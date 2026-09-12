use embassy_time::{Delay, Duration, Instant, Timer};
use esp_hal::{
    gpio::AnyPin,
    i2c::master::{AnyI2c, BusTimeout, Config, I2c},
};
use log::warn;
use sht4x::{Precision, Sht4xAsync};

use crate::{CLIMATE_READING, ClimateReading};

const RETRY_INTERVAL: Duration = Duration::from_secs(5);
const SAMPLE_INTERVAL: Duration = Duration::from_secs(60);

#[embassy_executor::task]
#[allow(
    clippy::large_stack_frames,
    reason = "Embassy stores task state statically; the release poll frame was verified at 288 bytes"
)]
pub async fn read_climate(i2c: AnyI2c<'static>, sda: AnyPin<'static>, scl: AnyPin<'static>) {
    let bus = I2c::new(i2c, Config::default().with_timeout(BusTimeout::Maximum))
        .expect("valid I2C configuration")
        .with_sda(sda)
        .with_scl(scl)
        .into_async();
    let mut sensor = Sht4xAsync::new(bus);
    let mut delay = Delay;

    loop {
        if let Err(error) = sensor.soft_reset(&mut delay).await {
            warn!("SHT40 initialization failed: {error:?}");
            Timer::after(RETRY_INTERVAL).await;
            continue;
        }

        loop {
            let next_sample = Instant::now() + SAMPLE_INTERVAL;
            match sensor.measure(Precision::High, &mut delay).await {
                Ok(measurement) => CLIMATE_READING.signal(ClimateReading {
                    temperature_celsius: measurement.temperature_celsius().to_num(),
                    relative_humidity_percent: measurement.humidity_percent().to_num(),
                }),
                Err(error) => {
                    warn!("SHT40 measurement failed: {error:?}");
                    Timer::after(RETRY_INTERVAL).await;
                    break;
                }
            }
            Timer::at(next_sample).await;
        }
    }
}
