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

use climate_data_gateway::weather::WeatherReading;
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use esp_backtrace as _;
use esp_hal::timer::timg::TimerGroup;
use log::info;

struct IndoorReading {
    temperature_celsius: f32,
    relative_humidity_percent: f32,
}

// One consumer; publishing replaces any reading it has not consumed yet.
static INDOOR_READING: Signal<CriticalSectionRawMutex, IndoorReading> = Signal::new();
static WEATHER_READING: Signal<CriticalSectionRawMutex, WeatherReading> = Signal::new();

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    esp_println::logger::init_logger_from_env();
    let p = esp_hal::init(esp_hal::Config::default());

    let timg0 = TimerGroup::new(p.TIMG0);
    let sw_interrupt = esp_hal::interrupt::software::SoftwareInterruptControl::new(p.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    spawner.spawn(log_readings().unwrap());
    spawner.spawn(indoor::read_indoor(p.I2C0.into(), p.GPIO22.into(), p.GPIO23.into()).unwrap());
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
async fn log_readings() {
    loop {
        match select(INDOOR_READING.wait(), WEATHER_READING.wait()).await {
            Either::First(reading) => info!(
                "Indoor reading: temperature={:.2} °C, relative humidity={:.2} %RH",
                reading.temperature_celsius, reading.relative_humidity_percent
            ),
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
