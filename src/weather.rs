/// One WH24 transmission, including the CC1101's appended reception status.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WeatherReading {
    pub station_id: u8,
    pub temperature_celsius: Option<f32>,
    pub relative_humidity_percent: Option<u8>,
    pub wind_direction_degrees: Option<u16>,
    pub wind_speed_mps: Option<f32>,
    pub gust_speed_mps: Option<f32>,
    /// Cumulative station counter, not a gateway-maintained rainfall total.
    pub rain_mm: f32,
    pub uv_microwatts_per_cm2: Option<u16>,
    pub uv_index: Option<u8>,
    pub light_lux: Option<f32>,
    pub battery_low: bool,
    pub rssi_dbm: f32,
    pub lqi: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeError {
    Length,
    Family,
    Crc,
    Checksum,
}

impl WeatherReading {
    /// Decode 17 payload bytes followed by RSSI and LQI. WH24 scaling follows
    /// rtl_433's fineoffset.c; the similar WH65 uses different wind/rain factors.
    pub fn decode(packet: &[u8]) -> Result<Self, DecodeError> {
        if packet.len() != 19 {
            return Err(DecodeError::Length);
        }
        if packet[0] != 0x24 {
            return Err(DecodeError::Family);
        }
        let mut crc = 0_u8;
        for byte in &packet[..16] {
            crc ^= byte;
            for _ in 0..8 {
                crc = if crc & 0x80 != 0 {
                    (crc << 1) ^ 0x31
                } else {
                    crc << 1
                };
            }
        }
        if crc != 0 {
            return Err(DecodeError::Crc);
        }
        if packet[..16]
            .iter()
            .fold(0_u8, |sum, byte| sum.wrapping_add(*byte))
            != packet[16]
        {
            return Err(DecodeError::Checksum);
        }

        let temperature = u16::from(packet[3] & 7) << 8 | u16::from(packet[4]);
        let direction = u16::from(packet[3] & 0x80) << 1 | u16::from(packet[2]);
        let wind = u16::from(packet[3] & 0x10) << 4 | u16::from(packet[6]);
        let rain = u16::from_be_bytes([packet[8], packet[9]]);
        let uv = u16::from_be_bytes([packet[10], packet[11]]);
        let light = u32::from_be_bytes([0, packet[12], packet[13], packet[14]]);
        const UV_UPPER_BOUNDS: [u16; 13] = [
            432, 851, 1210, 1570, 2017, 2450, 2761, 3100, 3512, 3918, 4277, 4650, 5029,
        ];
        Ok(Self {
            station_id: packet[1],
            temperature_celsius: (temperature != 0x7ff)
                .then_some((temperature as f32 - 400.0) / 10.0),
            relative_humidity_percent: (packet[5] != 0xff).then_some(packet[5]),
            wind_direction_degrees: (direction != 0x1ff).then_some(direction),
            wind_speed_mps: (wind != 0x1ff).then_some(wind as f32 * 0.125 * 1.12),
            gust_speed_mps: (packet[7] != 0xff).then_some(packet[7] as f32 * 1.12),
            rain_mm: rain as f32 * 0.3,
            uv_microwatts_per_cm2: (uv != 0xffff).then_some(uv),
            uv_index: (uv != 0xffff)
                .then_some(UV_UPPER_BOUNDS.partition_point(|bound| *bound < uv) as u8),
            light_lux: (light != 0xffffff).then_some(light as f32 / 10.0),
            battery_low: packet[3] & 8 != 0,
            // TI CC1101 datasheet §17.3: signed two's complement and nominal
            // 74 dB offset. The high LQI bit is hardware CRC status, unused here.
            rssi_dbm: packet[17] as i8 as f32 / 2.0 - 74.0,
            lqi: packet[18] & 0x7f,
        })
    }
}

/// Changes to report after a packet or the passage of time.
#[derive(Debug, Default)]
pub struct ReceiverUpdate {
    pub reading: Option<WeatherReading>,
    pub selected: Option<u8>,
    pub released: Option<u8>,
    pub silence_started: bool,
    pub recovered: bool,
    pub rejected: Option<DecodeError>,
}

/// Volatile station selection. Times are monotonic milliseconds since boot;
/// calling `update` without a packet also advances silence/expiry deadlines.
pub struct Receiver {
    selected: Option<u8>,
    last_reading_ms: u64,
    silent: bool,
}

impl Receiver {
    pub fn new(now_ms: u64) -> Self {
        Self {
            selected: None,
            last_reading_ms: now_ms,
            silent: false,
        }
    }

    pub fn update(&mut self, now_ms: u64, packet: Option<&[u8]>) -> ReceiverUpdate {
        let mut update = ReceiverUpdate::default();
        let elapsed = now_ms.saturating_sub(self.last_reading_ms);
        if elapsed >= 60_000 && !self.silent {
            self.silent = true;
            update.silence_started = true;
        }
        if elapsed >= 82_000 {
            update.released = self.selected.take();
        }
        if let Some(packet) = packet {
            match WeatherReading::decode(packet) {
                Err(error) => update.rejected = Some(error),
                Ok(reading) if self.selected.is_none_or(|id| id == reading.station_id) => {
                    if self.selected.is_none() {
                        self.selected = Some(reading.station_id);
                        update.selected = self.selected;
                    }
                    update.recovered = self.silent;
                    self.silent = false;
                    self.last_reading_ms = now_ms;
                    update.reading = Some(reading);
                }
                Ok(_) => {}
            }
        }
        update
    }
}

#[cfg(test)]
mod tests;
