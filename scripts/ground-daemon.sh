#!/bin/bash
# Example: ./ground-daemon.sh --tx-bind 0.0.0.0:10025 --rx-peer <yamcs-host>:10016
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DAEMON_BIN="${DAEMON_BIN:-${HERE}/../target/release/daemon}"
CONFIG="${CONFIG:-${HERE}/../configs/ground.toml}"

exec "${DAEMON_BIN}" \
    --config "${CONFIG}" --freq "${FREQ:-436500000}" \
    --spi "${SPI:-/dev/spidev0.0}" --spi-hz "${SPI_HZ:-1000000}" \
    --gpio-chip "${GPIO_CHIP:-/dev/gpiochip0}" --gpio-line "${GPIO_LINE:-25}" \
    --tx-port "${TX_PORT:-10025}" --rx-port "${RX_PORT:-10016}" "$@"
