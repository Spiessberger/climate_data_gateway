# Indoor readings over USB to live HTTP

The indoor producer still measures paired temperature and humidity immediately
after SHT40 setup and every 60 seconds, with five-second retries on failure.
Successful measurements receive a sequence number before entering the existing
overwrite-on-publish signal. Weather acquisition, selection and text reporting
remain unchanged in this first service integration slice.

At boot, the firmware temporarily enables the ESP32-C6 ADC entropy source using
`TrngSource`, fills a 16-byte boot identifier, then drops `Trng` before its source.
The ADC is not used by the I²C sensor or external SPI radio. This follows the
[HAL's TRNG contract](https://docs.espressif.com/projects/rust/esp-hal/1.1.0/esp32c6/esp_hal/rng/struct.Trng.html).

Indoor reports bypass the logger and use a complete bounded record in one locked
`esp_println::Printer::write_bytes` operation. Operational logs retain their
existing formatting. A representative record is:

```text
DATA {"v":1,"type":"indoor","boot_id":"6a9d3c1f80b24e67a511d92cb837046e","seq":42,"temperature_celsius":21.5,"relative_humidity_percent":48.2}
```

All fields are required. Version and sequence are integers; measurements are
finite numbers in °C and percent. Boot identifiers have 32 lowercase hexadecimal
characters. The indoor counter starts at 1 and wraps from 4294967295 to 0.
Formatting errors emit a warning instead of a partial record. The whole DATA line
must fit 1024 bytes including prefix and LF; indoor reports use a tested 256-byte
buffer to keep the embedded stack small. USB may still drop or truncate output.

## Checks

```sh
cargo check --locked
cargo clippy --locked -- -D warnings
cargo test --locked --target x86_64-unknown-linux-gnu --lib
cargo build --locked --release
```

Use the actual host target if different from x86_64 Linux. Reporting tests cover
identity, paired values, framing, finite extremes, formatting failures, produced
counter gaps and wrap. Existing weather decoder/receiver tests remain in the host
library suite. These tests do not prove hardware entropy or physical USB behavior.

## Observe the service and actual gateway

The separate sibling repository `../climate-data-service` has an independent
Linux build and a pseudo-terminal demo described in its README. Its integration
boundary is the wire contract, not a relative Cargo dependency.

When the gateway is connected, identify its explicit serial path (prefer an OS
`/dev/serial/by-id/` symlink). Build/flash using the repository's ESP32-C6 toolchain
and `espflash` workflow, then stop the serial monitor before starting the service.
Flashing and this observation require available hardware.

```sh
# In the separate climate-data-service checkout:
cargo run --locked -- --serial /dev/serial/by-id/YOUR_GATEWAY
# In another terminal, poll once per second:
watch -n 1 curl -s http://127.0.0.1:8080/live
```

Check for an initial paired indoor reading and a subsequent reading after roughly
60 seconds, with a stable boot identifier and advancing sequence. Restarting the
gateway should produce a new boot identifier and start the counter at 1. Weather
remains text and cannot overwrite indoor live state. Retain actual hardware
observations separately from simulated results.

This completes only the indoor live path. Heartbeats, connection health/reconnect,
weather DATA, historical storage and permanent logs belong to later tickets.
