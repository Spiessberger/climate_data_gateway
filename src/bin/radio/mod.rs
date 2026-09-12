mod cc1101;

use climate_data_gateway::weather::Receiver;
use embassy_time::{Duration, Instant, Timer};
use esp_hal::peripherals::{GPIO0, GPIO1, GPIO18, GPIO19, GPIO20, SPI2};
use log::{debug, info, warn};

use crate::WEATHER_READING;
use cc1101::Cc1101;

#[embassy_executor::task]
pub async fn receive_weather(
    spi: SPI2<'static>,
    mosi: GPIO18<'static>,
    miso: GPIO20<'static>,
    sck: GPIO19<'static>,
    cs: GPIO0<'static>,
    gdo0: GPIO1<'static>,
) {
    let mut radio = Cc1101::new(spi, mosi, miso, sck, cs, gdo0);
    // Keep selection across radio resets, but keep advancing its silence clock.
    let mut receiver = Receiver::new(Instant::now().as_millis());
    loop {
        match radio.initialize().await {
            Ok(version) => {
                info!("CC1101 ready: part=0x00 version=0x{version:02x}, WH24 RX at 868.30 MHz");
                loop {
                    let result = radio.receive().await;
                    report(
                        &mut receiver,
                        result
                            .as_ref()
                            .ok()
                            .and_then(|packet| packet.as_ref().map(|p| &p[..])),
                    );
                    if let Err(error) = result {
                        warn!("CC1101 receive failed: {error}; retrying in five seconds");
                        break;
                    }
                }
            }
            Err(error) => {
                warn!("CC1101 initialization failed: {error}; retrying in five seconds")
            }
        }

        let retry_at = Instant::now() + Duration::from_secs(5);
        while Instant::now() < retry_at {
            report(&mut receiver, None);
            Timer::at(core::cmp::min(
                Instant::now() + Duration::from_secs(1),
                retry_at,
            ))
            .await;
        }
    }
}

fn report(receiver: &mut Receiver, packet: Option<&[u8]>) {
    let update = receiver.update(Instant::now().as_millis(), packet);
    if update.silence_started {
        warn!("No accepted weather reading for 60 seconds");
    }
    if let Some(id) = update.released {
        info!("Released weather station: station ID={id}");
    }
    if let Some(id) = update.selected {
        info!("Selected weather station: station ID={id}");
    }
    if update.recovered {
        info!("Weather reception recovered");
    }
    if let Some(error) = update.rejected {
        debug!("Rejected WH24 packet: {error:?}");
    }
    if let Some(reading) = update.reading {
        WEATHER_READING.signal(reading);
    }
}
