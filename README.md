
# Skipper

Skipper is an over the air (OTA) update manager, primarily for use in embedded Linux systems.

[![build status](https://github.com/mikeframpo/skipper/actions/workflows/rust.yml/badge.svg)](https://github.com/mikeframpo/skipper/actions/workflows/rust.yml)

Skipper is currently in development, and will eventually support the following features:
- Dual (A/B) partition update strategy, i.e., the running partition(s) updates the unused partitions. Similar to the system used by [android](https://source.android.com/devices/tech/ota/ab/).
- Update process can run in the background when system resources are not in demand.
- Update archive is streamed directly into destination filesystem, removing need for extra storage overhead.
- Update download supports resuming via http-ranges.
- Archive signing via X509 certificate.
- Bootloader support for U-boot, and possibly others in the future.
- Support for re-partitioning a Debian/Raspbian system into a skipper-compatible configuration.
- Eventual support for Yocto and Buildroot, for *from scratch* configurations.
