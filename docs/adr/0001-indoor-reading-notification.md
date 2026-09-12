---
status: accepted
---

# Deliver the latest indoor reading to one consumer

The indoor sensor producer and reporting consumer communicate through a single pending indoor reading. New readings overwrite an unread reading: the user chose freshness over preserving every measurement, so future serial reporting must not assume that every measurement is delivered.

## User decisions

- Connect the SHT40 over I²C using GPIO22 for SDA and GPIO23 for SCL.
- An Embassy task sets up the sensor and measures both temperature and relative humidity immediately after setup, then every 60 seconds.
- Publish both quantities together in one slot; overwrite its contents when another reading arrives before consumption.
- The consumer logs each received reading at info level.
- On initialization or measurement failure, warn, publish no reading, and retry after five seconds.
- Serial messaging is future work; its consumer will receive readings through this notification boundary.
- Reduce generated initialization to what the application needs.

## Delegated implementation decisions

- Use Embassy's `Signal` as the single-consumer, overwrite-on-publish notification primitive; an ordinary bounded queue would not implement the chosen overwrite behavior.
- Keep sensor acquisition in a producer task and logging in a separate consumer task, allowing serial reporting to be added to that consumer later.
- Keep the logger, HAL, Embassy runtime setup, bootloader descriptor, and panic support. Remove generated demonstration code, unused setup, and unused direct dependencies where verified safe.

These decisions were captured during bounded grill-with-docs and confirmed by the user's request to implement the assembled design.
