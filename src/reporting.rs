use core::fmt::{self, Write};

pub const MAX_RECORD_BYTES: usize = 1024;

/// Identity and sequence assigned at acquisition, before latest-reading overwrite.
pub struct IndoorSource {
    boot_id: [u8; 16],
    seq: u32,
}

impl IndoorSource {
    /// The caller supplies bytes drawn once per boot with hardware entropy enabled.
    pub fn new(boot_id: [u8; 16]) -> Self {
        Self { boot_id, seq: 0 }
    }

    pub fn produce(
        &mut self,
        temperature_celsius: f32,
        relative_humidity_percent: f32,
    ) -> IndoorReading {
        self.seq = self.seq.wrapping_add(1);
        IndoorReading {
            boot_id: self.boot_id,
            seq: self.seq,
            temperature_celsius,
            relative_humidity_percent,
        }
    }
}

pub struct IndoorReading {
    boot_id: [u8; 16],
    seq: u32,
    temperature_celsius: f32,
    relative_humidity_percent: f32,
}

impl IndoorReading {
    /// Only a successful result may be transmitted, in one synchronized write.
    pub fn encode<'a>(&self, buffer: &'a mut [u8]) -> Result<&'a [u8], fmt::Error> {
        if !self.temperature_celsius.is_finite() || !self.relative_humidity_percent.is_finite() {
            return Err(fmt::Error);
        }
        let capacity = buffer.len().min(MAX_RECORD_BYTES);
        let mut output = RecordBuffer {
            bytes: &mut buffer[..capacity],
            len: 0,
        };
        write!(output, "DATA {{\"v\":1,\"type\":\"indoor\",\"boot_id\":\"")?;
        for byte in self.boot_id {
            write!(output, "{byte:02x}")?;
        }
        let temperature_celsius = normalize_one_decimal_zero(self.temperature_celsius);
        let relative_humidity_percent = normalize_one_decimal_zero(self.relative_humidity_percent);
        writeln!(
            output,
            "\",\"seq\":{},\"temperature_celsius\":{:.1},\"relative_humidity_percent\":{:.1}}}",
            self.seq, temperature_celsius, relative_humidity_percent
        )?;
        let len = output.len;
        Ok(&buffer[..len])
    }
}

fn normalize_one_decimal_zero(value: f32) -> f32 {
    if value > -0.05 && value < 0.05 {
        0.0
    } else {
        value
    }
}

struct RecordBuffer<'a> {
    bytes: &'a mut [u8],
    len: usize,
}

impl Write for RecordBuffer<'_> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        let end = self.len + text.len();
        let destination = self.bytes.get_mut(self.len..end).ok_or(fmt::Error)?;
        destination.copy_from_slice(text.as_bytes());
        self.len = end;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sequence(reading: IndoorReading) -> u32 {
        let mut buffer = [0; MAX_RECORD_BYTES];
        let wire = core::str::from_utf8(reading.encode(&mut buffer).unwrap()).unwrap();
        wire.split("\"seq\":")
            .nth(1)
            .unwrap()
            .split(',')
            .next()
            .unwrap()
            .parse()
            .unwrap()
    }

    #[test]
    fn unread_measurements_still_advance_the_wire_counter() {
        let mut source = IndoorSource::new([0; 16]);
        assert_eq!(sequence(source.produce(10.0, 20.0)), 1);
        let _overwritten = source.produce(11.0, 21.0);
        assert_eq!(sequence(source.produce(12.0, 22.0)), 3);
    }

    #[test]
    fn counter_wraps_without_changing_boot_identity() {
        let mut source = IndoorSource {
            boot_id: [0xab; 16],
            seq: u32::MAX - 1,
        };
        assert_eq!(sequence(source.produce(10.0, 20.0)), 4294967295);
        let mut buffer = [0; MAX_RECORD_BYTES];
        let reading = source.produce(10.0, 20.0);
        let wire = core::str::from_utf8(reading.encode(&mut buffer).unwrap()).unwrap();
        assert!(wire.contains("\"boot_id\":\"abababababababababababababababab\",\"seq\":0,"));
    }

    #[test]
    fn format_failure_returns_no_record_to_transmit() {
        let mut source = IndoorSource::new([0; 16]);
        assert!(source.produce(21.5, 48.2).encode(&mut [0; 32]).is_err());
        for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert!(
                source
                    .produce(invalid, 48.2)
                    .encode(&mut [0; MAX_RECORD_BYTES])
                    .is_err()
            );
            assert!(
                source
                    .produce(21.5, invalid)
                    .encode(&mut [0; MAX_RECORD_BYTES])
                    .is_err()
            );
        }
    }

    #[test]
    fn finite_extremes_fit_the_bounded_wire_without_new_range_filtering() {
        let mut source = IndoorSource::new([0xff; 16]);
        let mut buffer = [0; 256];
        let reading = source.produce(f32::MIN, f32::MAX);
        let wire = reading.encode(&mut buffer).unwrap();
        assert!(wire.len() <= 1024);
        assert!(wire.starts_with(b"DATA {"));
        assert!(wire.ends_with(b"}\n"));
        assert!(!wire[..wire.len() - 1].contains(&b'\n'));
        assert!(!wire.contains(&b'\r'));
        assert!(!wire.contains(&0x1b));
    }

    #[test]
    fn paired_measurement_becomes_a_complete_indoor_data_record() {
        let mut source = IndoorSource::new([
            0x6a, 0x9d, 0x3c, 0x1f, 0x80, 0xb2, 0x4e, 0x67, 0xa5, 0x11, 0xd9, 0x2c, 0xb8, 0x37,
            0x04, 0x6e,
        ]);
        let reading = source.produce(21.5, 48.2);
        let mut buffer = [0; MAX_RECORD_BYTES];
        let wire = reading.encode(&mut buffer).unwrap();
        assert_eq!(
            core::str::from_utf8(wire).unwrap(),
            concat!(
                "DATA {\"v\":1,\"type\":\"indoor\",",
                "\"boot_id\":\"6a9d3c1f80b24e67a511d92cb837046e\",\"seq\":1,",
                "\"temperature_celsius\":21.5,\"relative_humidity_percent\":48.2}\n"
            )
        );
    }

    #[test]
    fn indoor_data_record_formats_both_measurements_with_one_decimal_digit() {
        let mut source = IndoorSource::new([0; 16]);
        let mut buffer = [0; MAX_RECORD_BYTES];

        let rounded = source.produce(21.26, 48.24);
        let wire = core::str::from_utf8(rounded.encode(&mut buffer).unwrap()).unwrap();
        assert!(wire.contains("\"temperature_celsius\":21.3,\"relative_humidity_percent\":48.2"));

        let whole = source.produce(21.0, 48.0);
        let wire = core::str::from_utf8(whole.encode(&mut buffer).unwrap()).unwrap();
        assert!(wire.contains("\"temperature_celsius\":21.0,\"relative_humidity_percent\":48.0"));
    }

    #[test]
    fn indoor_data_record_normalizes_negative_zero_after_rounding() {
        let mut source = IndoorSource::new([0; 16]);
        let reading = source.produce(-0.04, -0.0);
        let mut buffer = [0; MAX_RECORD_BYTES];

        let wire = core::str::from_utf8(reading.encode(&mut buffer).unwrap()).unwrap();

        assert!(wire.contains("\"temperature_celsius\":0.0,\"relative_humidity_percent\":0.0"));
    }
}
