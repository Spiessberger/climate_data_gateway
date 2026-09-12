#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

mod indoor;
mod radio;

use climate_data_gateway::{
    reporting::{Heartbeat, IndoorReading},
    weather::WeatherReading,
};
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Duration, Ticker};
use esp_backtrace as _;
use esp_hal::{
    rng::{Trng, TrngSource},
    timer::timg::TimerGroup,
};
use log::{info, warn};

// One consumer; publishing replaces any reading it has not consumed yet.
static INDOOR_READING: Signal<CriticalSectionRawMutex, IndoorReading> = Signal::new();
static WEATHER_READING: Signal<CriticalSectionRawMutex, WeatherReading> = Signal::new();

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    esp_println::logger::init_logger_from_env();
    let p = esp_hal::init(esp_hal::Config::default());

    let mut boot_id = [0; 16];
    {
        // The external SHT40 and CC1101 do not enable the ESP's RF entropy source.
        // Declaration order ensures Trng is dropped before its ADC entropy source.
        let _entropy = TrngSource::new(p.RNG, p.ADC1);
        let trng = Trng::try_new().expect("ADC entropy source enabled");
        trng.read(&mut boot_id);
    }

    let timg0 = TimerGroup::new(p.TIMG0);
    let sw_interrupt = esp_hal::interrupt::software::SoftwareInterruptControl::new(p.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    spawner.spawn(log_readings().unwrap());
    spawner.spawn(report_heartbeats(boot_id).unwrap());
    spawner.spawn(
        indoor::read_indoor(p.I2C0.into(), p.GPIO22.into(), p.GPIO23.into(), boot_id).unwrap(),
    );
    spawner.spawn(
        radio::receive_weather(
            p.SPI2.into(),
            p.GPIO18.into(),
            p.GPIO20.into(),
            p.GPIO19.into(),
            p.GPIO0.into(),
            p.GPIO1.into(),
        )
        .unwrap(),
    );
}

#[embassy_executor::task]
async fn report_heartbeats(boot_id: [u8; 16]) {
    let heartbeat = Heartbeat::new(boot_id);
    let mut ticker = Ticker::every(Duration::from_secs(5));
    let mut buffer = [0; 128];
    loop {
        ticker.next().await;
        match heartbeat.encode(&mut buffer) {
            Ok(record) => esp_println::Printer::write_bytes(record),
            Err(_) => warn!("Could not format heartbeat DATA record"),
        }
    }
}

#[embassy_executor::task]
async fn log_readings() {
    // Indoor fields, including finite f32 extremes, fit in 256 bytes (wire-tested).
    let mut buffer = [0; 256];
    loop {
        match select(INDOOR_READING.wait(), WEATHER_READING.wait()).await {
            Either::First(reading) => match reading.encode(&mut buffer) {
                // Printer holds the same lock as operational logging for the whole record.
                Ok(record) => esp_println::Printer::write_bytes(record),
                Err(_) => warn!("Could not format indoor DATA record"),
            },
            Either::Second(reading) => log_weather(reading),
        }
    }
}

fn log_weather(reading: WeatherReading) {
    info!(
        "Weather reading: station ID={}, temperature={:?} °C, relative humidity={:?} %RH, wind direction={:?} degrees, wind speed={:?} m/s, gust={:?} m/s, cumulative rain={:.1} mm, UV={:?} µW/cm², UV index={:?}, light={:?} lux, battery low={}, RSSI={:.1} dBm, LQI={}",
        reading.station_id,
        reading.temperature_celsius,
        reading.relative_humidity_percent,
        reading.wind_direction_degrees,
        reading.wind_speed_mps,
        reading.gust_speed_mps,
        reading.rain_mm,
        reading.uv_microwatts_per_cm2,
        reading.uv_index,
        reading.light_lux,
        reading.battery_low,
        reading.rssi_dbm,
        reading.lqi,
    );
}
