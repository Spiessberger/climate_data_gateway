# WH24 receiver

The ESP32-C6 receives Fine Offset WH24 transmissions through a CC1101 with a
26 MHz crystal. Connect the radio to 3.3 V and ground, with these signal pins:

| CC1101 | ESP32-C6 GPIO |
| --- | --- |
| MOSI / SI | 18 |
| MISO / SO | 20 |
| SCK | 19 |
| CSn | 0 |
| GDO0 | 1 |

The profile uses 868.30 MHz, NRZ 2-FSK, 17.258 kBaud, 34.912 kHz deviation,
and 203.125 kHz receive bandwidth. The packet engine checks the `0x2DD4` sync
word and delivers 17 payload bytes plus RSSI/LQI. The software requires the WH24
family code, CRC-8, and additive checksum before accepting a reading. Field
conversions follow the [rtl_433 WH24 decoder](https://github.com/merbanan/rtl_433/blob/master/src/devices/fineoffset.c);
register settings follow the [CC1101 datasheet](https://www.ti.com/lit/ds/symlink/cc1101.pdf).

The first valid packet selects a station. Every weather log includes its ID;
after 82 seconds without a valid packet from that station, selection expires.
The next valid packet can select any station. Selection is kept only in memory
and its deadline continues through radio retries. See [the selection decision](adr/0002-weather-station-selection.md).

Weather readings use their own overwrite-on-publish signal, consumed alongside
indoor readings by `log_readings`. `None` in a weather log denotes an
unavailable quantity. Rain is the station's cumulative counter converted to mm;
the gateway does not calculate rainfall deltas or persist totals.

On startup, `CC1101 ready` confirms plausible chip identity, configuration
readback, and entry into RX. It does not prove RF reception or GDO0 wiring.
Initialization/runtime errors warn and retry after five seconds while local
sensor acquisition continues. A single warning follows 60 seconds without an
accepted weather reading, and recovery is logged when a reading is accepted.
The indoor sensor samples every 60 seconds.

Build and validate from the repository root:

```sh
cargo check
cargo build --release
cargo fmt --check
cargo clippy --lib --bin climate-data-gateway -- -D warnings
cargo test --lib --target x86_64-unknown-linux-gnu
```

The host test target exercises decoding and station timing without ESP32
dependencies. Use the appropriate host target on other development machines.
The embedded target has no Rust test harness; do not run host tests against it.

With the board attached, flash and monitor using the installed `espflash`:

```sh
espflash flash --port /dev/ttyACM0 --chip esp32c6 --monitor --log-format serial target/riscv32imac-unknown-none-elf/release/climate-data-gateway
```

Allow at least 90 seconds to observe several nominal 16-second transmissions
and the silence warning. Confirm that indoor reading logs continue even if the
CC1101 is disconnected. Absent weather logs alone cannot distinguish poor
reception from wiring, antenna, or station problems. The session hardware test
record lives in `.scratch/wh24-receiver/hardware-test.md`.
