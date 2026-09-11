#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

mod sensor;

use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use esp_backtrace as _;
use esp_hal::timer::timg::TimerGroup;
use log::info;

struct ClimateReading {
    temperature_celsius: f32,
    relative_humidity_percent: f32,
}

// One consumer; publishing replaces any reading it has not consumed yet.
static CLIMATE_READING: Signal<CriticalSectionRawMutex, ClimateReading> = Signal::new();

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    spawner.spawn(log_readings().unwrap());
    spawner.spawn(
        sensor::read_climate(peripherals.I2C0, peripherals.GPIO22, peripherals.GPIO23).unwrap(),
    );
}

#[embassy_executor::task]
async fn log_readings() {
    loop {
        let reading = CLIMATE_READING.wait().await;
        info!(
            "Climate reading: temperature={:.2} °C, relative humidity={:.2} %RH",
            reading.temperature_celsius, reading.relative_humidity_percent
        );
    }
}
