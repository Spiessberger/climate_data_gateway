---
status: accepted
---

# Select a weather station automatically and release it after silence

The WH24 station ID can change after battery replacement. The user chose automatic selection of the first received station ID and release after consecutive missed transmissions, avoiding a configured ID at the cost of potentially selecting a neighboring station after loss of reception.

## User decisions

- Carry and log the station ID with every weather reading so changes of source are visible.
- Receive one selected station at a time and release its selection after 82 seconds without a valid packet from that station: five missed 16-second transmissions plus two seconds of tolerance. Invalid packets and other station IDs do not refresh the deadline.
- Log selection and release at info level. After release, select the first subsequently received valid frame's station ID.
- Keep receiver state entirely in memory, with no persistent data on the ESP32. Reboot starts fresh selection; radio retries retain selection only until its existing silence deadline expires.
- Deliver all decoded weather quantities, battery status, RSSI, and LQI through a signal consumed by `log_readings` at info level.

## Delegated decisions

- Require valid family code, CRC-8, and additive checksum before an incoming frame can select a station or produce a reading.
- Give weather readings their own latest-reading signal, following ADR 0001's delivery convention. A new weather reading overwrites an unread weather reading; it cannot overwrite a local climate reading.

These choices were captured during bounded grill-with-docs and confirmed by the user's request to implement the assembled design. The complete implementation specification is in `.scratch/wh24-receiver/spec.md`.
