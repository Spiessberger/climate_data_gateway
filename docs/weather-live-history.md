# Complete weather readings over USB

Accepted weather readings now receive their own wrapping sequence counter before
publication to the existing weather signal. Indoor and weather share the
hardware-entropy-backed boot identifier; their counters and pending slots remain
independent. The weather source lives across radio reinitialization and station
release/reselection. The decoder, automatic selection, 60-second silence warning,
82-second release, and operational text events retain their existing behavior.

The reporting consumer emits one bounded `DATA ` JSON line through the same
synchronized printer operation as indoor readings and heartbeats. A static
1024-byte buffer keeps the full record off the embedded stack. Failed formatting
warns without submitting a partial record; physical USB delivery may still fail.

The [complete fixture](../tests/fixtures/weather.data) comes from the existing
published WH24 packet. The [unavailable fixture](../tests/fixtures/weather-unavailable.data)
comes from an independently sealed missing-quantity packet accepted after station
release. Both include every quantity, station ID, battery status, RSSI and LQI.
Integer quantities remain integer JSON values, unavailable quantities are explicit
nulls, and finite fractional values preserve the decoder's quantities. `rain_mm`
is the cumulative station counter, never an interval rainfall calculation.

The separate `climate-data-service` accepts these records into independent weather
live state and retained SQLite history. Poll `GET /live`; query weather history at
`GET /history/weather` or `GET /history/weather/updates` using the service's
bounded range/incremental query parameters. Its `docs/weather.md` documents the
field units, nullability, cursors and per-stream health. Run its finite demonstration:

```sh
# From the separate climate-data-service checkout:
cargo build --locked
python3 scripts/weather_demo.py
```

Gateway host tests exercise the reporting and existing receiver boundaries:

```sh
cargo test --locked --target x86_64-unknown-linux-gnu --lib
cargo check --locked
cargo clippy --locked -- -D warnings
cargo build --locked --release
```

The service independently retains copies of these fixture files for serial-to-HTTP
and real SQLite integration tests, without a relative Cargo dependency. Host tests
and the pseudo-terminal demo do not establish physical USB/radio or hardware
entropy behavior. Full hardware smoke acceptance remains a later slice.
