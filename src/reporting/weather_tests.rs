use crate::{
    reporting::{MAX_RECORD_BYTES, WeatherSource},
    weather::Receiver,
};

// Published rtl_433 WH24 example and independently sealed unavailable fixture,
// as used at the existing decoder/receiver seam, with appended CC1101 status.
const EXAMPLE: [u8; 19] = [
    0x24, 0xbf, 0x0a, 0xe2, 0x06, 0x4e, 0x08, 0x02, 0, 0x4a, 0, 1, 0, 0, 0, 0x8f, 7, 0xe0, 0xaa,
];
const BOOT: [u8; 16] = [
    0x6a, 0x9d, 0x3c, 0x1f, 0x80, 0xb2, 0x4e, 0x67, 0xa5, 0x11, 0xd9, 0x2c, 0xb8, 0x37, 4, 0x6e,
];

#[test]
fn accepted_published_weather_transmission_becomes_complete_data_record() {
    let mut receiver = Receiver::new(0);
    let mut source = WeatherSource::new(BOOT);
    let reading = source.produce(receiver.update(1000, Some(&EXAMPLE)).reading.unwrap());
    let mut buffer = [0; MAX_RECORD_BYTES];
    assert_eq!(
        reading.encode(&mut buffer).unwrap(),
        include_bytes!("../../tests/fixtures/weather.data")
    );
}

const MISSING: [u8; 19] = [
    0x24, 1, 0xff, 0x9f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x19,
    0xd1, 0, 0,
];

#[test]
fn released_station_changes_source_without_resetting_the_weather_counter() {
    let mut receiver = Receiver::new(0);
    let mut source = WeatherSource::new(BOOT);
    let mut indoor = super::IndoorSource::new(BOOT);
    let mut buffer = [0; MAX_RECORD_BYTES];
    let _unread = source.produce(receiver.update(0, Some(&EXAMPLE)).reading.unwrap());
    assert!(receiver.update(81_999, Some(&MISSING)).reading.is_none());
    let update = receiver.update(82_000, Some(&MISSING));
    assert_eq!(update.released, Some(191));
    assert_eq!(update.selected, Some(1));
    let report = source.produce(update.reading.unwrap());
    let wire = core::str::from_utf8(report.encode(&mut buffer).unwrap()).unwrap();
    assert_eq!(
        wire,
        include_str!("../../tests/fixtures/weather-unavailable.data")
    );
    let wire = indoor.produce(21.5, 48.2);
    assert!(
        core::str::from_utf8(wire.encode(&mut buffer).unwrap())
            .unwrap()
            .contains("\"seq\":1,")
    );
}

#[test]
fn weather_wrap_and_format_failures_preserve_bounded_reporting() {
    let reading = crate::weather::WeatherReading::decode(&EXAMPLE).unwrap();
    let mut source = WeatherSource {
        boot_id: BOOT,
        seq: u32::MAX,
    };
    let report = source.produce(reading);
    let mut buffer = [0; MAX_RECORD_BYTES];
    let wire = core::str::from_utf8(report.encode(&mut buffer).unwrap()).unwrap();
    assert!(wire.contains("\"seq\":0,"));
    assert!(report.encode(&mut [0; 32]).is_err());
    for field in 0..6 {
        for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut invalid_reading = reading;
            match field {
                0 => invalid_reading.temperature_celsius = Some(invalid),
                1 => invalid_reading.wind_speed_mps = Some(invalid),
                2 => invalid_reading.gust_speed_mps = Some(invalid),
                3 => invalid_reading.light_lux = Some(invalid),
                4 => invalid_reading.rain_mm = invalid,
                _ => invalid_reading.rssi_dbm = invalid,
            }
            assert!(source.produce(invalid_reading).encode(&mut buffer).is_err());
        }
    }
    let extreme = crate::weather::WeatherReading {
        temperature_celsius: Some(f32::MIN),
        wind_speed_mps: Some(f32::MAX),
        gust_speed_mps: Some(f32::MAX),
        light_lux: Some(f32::MAX),
        rain_mm: f32::MAX,
        rssi_dbm: f32::MIN,
        ..reading
    };
    let report = source.produce(extreme);
    let wire = report.encode(&mut buffer).unwrap();
    assert!(wire.len() <= 1024);
    assert!(wire.ends_with(b"}\n"));
    assert!(!wire[..wire.len() - 1].contains(&b'\n'));
    assert!(!wire.contains(&b'\r'));
    assert!(!wire.contains(&0x1b));
}
