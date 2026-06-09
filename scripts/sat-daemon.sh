#!/bin/bash
# Example: ./sat-daemon.sh --tx-bind 0.0.0.0:10016 --rx-peer <c3-host>:10025
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DAEMON_BIN="${DAEMON_BIN:-${HERE}/../target/release/daemon}"
CONFIG="${CONFIG:-${HERE}/../configs/sat.toml}"
BEACON_PORT="${BEACON_PORT-10015}"
RSSI_PEER="${RSSI_PEER-127.0.0.1:10030}"

args=(--config "${CONFIG}" --freq "${FREQ:-436500000}"
      --spi "${SPI:-/dev/spidev0.0}" --spi-hz "${SPI_HZ:-1000000}"
      --gpio-chip "${GPIO_CHIP:-/dev/gpiochip0}" --gpio-line "${GPIO_LINE:-25}"
      --tx-port "${TX_PORT:-10016}" --rx-port "${RX_PORT:-10025}")
if [ -n "${BEACON_PORT}" ]; then args+=(--beacon-port "${BEACON_PORT}"); else args+=(--no-beacon); fi
if [ -n "${RSSI_PEER}" ];   then args+=(--rssi-peer "${RSSI_PEER}");     else args+=(--no-rssi);   fi

exec "${DAEMON_BIN}" "${args[@]}" "$@"
