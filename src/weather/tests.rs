use super::*;

// Published WH24 example in rtl_433 src/devices/fineoffset.c, followed by
// CC1101 status bytes: signed RSSI -32 / 2 - 74 = -90 dBm, LQI 42.
const EXAMPLE: [u8; 19] = [
    0x24, 0xbf, 0x0a, 0xe2, 0x06, 0x4e, 0x08, 0x02, 0x00, 0x4a, 0x00, 0x01, 0x00, 0x00, 0x00, 0x8f,
    0x07, 0xe0, 0xaa,
];

// Synthetic protocol fixtures, sealed independently using polynomial division.
const NEIGHBOR: [u8; 19] = [
    0x24, 0x6a, 0x0a, 0xe2, 0x06, 0x4e, 0x08, 0x02, 0x00, 0x4a, 0x00, 0x01, 0x00, 0x00, 0x00, 0x7b,
    0x9e, 0x00, 0x00,
];
const MISSING: [u8; 19] = [
    0x24, 0x01, 0xff, 0x9f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x19,
    0xd1, 0x00, 0x00,
];

#[test]
fn decodes_published_wh24_reading_and_radio_status() {
    let reading = WeatherReading::decode(&EXAMPLE).unwrap();
    assert_eq!(reading.station_id, 191);
    assert_eq!(reading.temperature_celsius, Some(11.8));
    assert_eq!(reading.relative_humidity_percent, Some(78));
    assert_eq!(reading.wind_direction_degrees, Some(266));
    assert_eq!(reading.wind_speed_mps, Some(1.12));
    assert_eq!(reading.gust_speed_mps, Some(2.24));
    assert!((reading.rain_mm - 22.2).abs() < 0.0001);
    assert_eq!(reading.uv_microwatts_per_cm2, Some(1));
    assert_eq!(reading.uv_index, Some(0));
    assert_eq!(reading.light_lux, Some(0.0));
    assert!(!reading.battery_low);
    assert_eq!(reading.rssi_dbm, -90.0);
    assert_eq!(reading.lqi, 42);
}

#[test]
fn selects_first_valid_station_and_releases_after_five_missed_transmissions() {
    let mut receiver = Receiver::new(0);
    let first = receiver.update(1_000, Some(&EXAMPLE));
    assert_eq!(first.selected, Some(191));
    assert_eq!(first.reading.unwrap().station_id, 191);
    assert!(!receiver.update(60_999, None).silence_started);
    assert!(receiver.update(61_000, None).silence_started);
    assert!(!receiver.update(62_000, None).silence_started);
    assert_eq!(receiver.update(82_999, None).released, None);
    assert_eq!(receiver.update(83_000, None).released, Some(191));
    let reacquired = receiver.update(84_000, Some(&EXAMPLE));
    assert_eq!(reacquired.selected, Some(191));
    assert!(reacquired.recovered);
}

#[test]
fn rejects_bad_length_family_crc_and_checksum_independently() {
    assert_eq!(
        WeatherReading::decode(&EXAMPLE[..18]),
        Err(DecodeError::Length)
    );
    let mut packet = EXAMPLE;
    packet[0] = 0x23;
    assert_eq!(WeatherReading::decode(&packet), Err(DecodeError::Family));
    packet = EXAMPLE;
    packet[4] ^= 1;
    assert_eq!(WeatherReading::decode(&packet), Err(DecodeError::Crc));
    packet = EXAMPLE;
    packet[16] ^= 1;
    assert_eq!(WeatherReading::decode(&packet), Err(DecodeError::Checksum));
}

#[test]
fn invalid_markers_leave_fields_missing_but_preserve_station_and_battery() {
    let reading = WeatherReading::decode(&MISSING).unwrap();
    assert_eq!(reading.station_id, 1);
    assert!(reading.battery_low);
    assert_eq!(reading.temperature_celsius, None);
    assert_eq!(reading.relative_humidity_percent, None);
    assert_eq!(reading.wind_direction_degrees, None);
    assert_eq!(reading.wind_speed_mps, None);
    assert_eq!(reading.gust_speed_mps, None);
    assert_eq!(reading.uv_microwatts_per_cm2, None);
    assert_eq!(reading.uv_index, None);
    assert_eq!(reading.light_lux, None);
    assert_eq!(reading.rain_mm, 19660.5);
}

#[test]
fn corrupt_and_other_station_packets_do_not_select_or_extend_selection() {
    let mut receiver = Receiver::new(0);
    let mut corrupt = EXAMPLE;
    corrupt[15] ^= 1;
    let rejected = receiver.update(0, Some(&corrupt));
    assert_eq!(rejected.selected, None);
    assert!(rejected.reading.is_none());
    receiver.update(1_000, Some(&EXAMPLE));
    assert!(receiver.update(50_000, Some(&NEIGHBOR)).reading.is_none());
    assert!(receiver.update(59_000, Some(&corrupt)).reading.is_none());
    let changed = receiver.update(83_000, Some(&NEIGHBOR));
    assert_eq!(changed.released, Some(191));
    assert_eq!(changed.selected, Some(106));
    assert_eq!(changed.reading.unwrap().station_id, 106);
}

#[test]
fn selected_station_refreshes_deadline_and_recovery_rearms_silence_warning() {
    let mut receiver = Receiver::new(0);
    receiver.update(0, Some(&EXAMPLE));
    receiver.update(16_000, Some(&EXAMPLE));
    assert!(!receiver.update(60_000, None).silence_started);
    assert!(receiver.update(76_000, None).silence_started);
    let recovered = receiver.update(80_000, Some(&EXAMPLE));
    assert!(recovered.recovered);
    assert_eq!(recovered.selected, None);
    assert_eq!(receiver.update(98_000, None).released, None);
    assert!(receiver.update(140_000, None).silence_started);
    assert_eq!(receiver.update(162_000, None).released, Some(191));
}

#[test]
fn silent_boot_warns_once_and_new_receiver_does_not_remember_station() {
    let mut receiver = Receiver::new(5_000);
    assert!(!receiver.update(64_999, None).silence_started);
    assert!(receiver.update(65_000, None).silence_started);
    assert!(!receiver.update(100_000, None).silence_started);
    assert!(receiver.update(101_000, Some(&EXAMPLE)).recovered);
    let mut rebooted = Receiver::new(0);
    assert_eq!(rebooted.update(0, Some(&NEIGHBOR)).selected, Some(106));
}

#[test]
fn decodes_negative_temperature_high_wind_bit_uv_cap_and_positive_rssi_byte() {
    let packet = [
        0x24, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x13, 0xa6, 0x2d, 0xc6, 0xc0,
        0x23, 0xcb, 0x00, 0x00,
    ];
    let reading = WeatherReading::decode(&packet).unwrap();
    assert_eq!(reading.temperature_celsius, Some(-40.0));
    assert_eq!(reading.wind_speed_mps, Some(35.84));
    assert_eq!(reading.uv_index, Some(13));
    assert_eq!(reading.light_lux, Some(300_000.0));
    assert_eq!(reading.rssi_dbm, -74.0);
}
