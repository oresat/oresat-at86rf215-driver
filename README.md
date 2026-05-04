# oresat-at86rf215-driver
Microchip AT86RF215 Rust driver and utilites

[Datasheet](https://ww1.microchip.com/downloads/aemDocuments/documents/OTH/ProductDocuments/DataSheets/Atmel-42415-WIRELESS-AT86RF215_Datasheet.pdf) (PDF)
Initial [design thoughts](https://docs.google.com/document/d/1zBbb-4qnycPR2GkD_XNbqpXR3gw3rPBdjSQARGNUBZ0/edit?tab=t.0)

## Executables

The `tui` feature enables the ratatui-based binaries (`tui`, `live`, `cw_uhf_tui`).

### Daemon

| Binary | Run | Description |
| --- | --- | --- |
| `daemon` | `cargo run --bin daemon` | Bridges UDP / Unix-datagram sockets to the radio over SPI. TX socket -> RF09; RX socket <- incoming frames.

### Hardware examples

Defaults: `--spi /dev/spidev0.0`, `--gpio-chip /dev/gpiochip0`, `--gpio-line 30`, `--freq 868300000`.

| Example | Run | Description |
| --- | --- | --- |
| `cw_uhf` | `cargo run --example cw_uhf` | Continuous-wave TX on RF09 via DAC override. |
| `cw_uhf_tui` | `cargo run --release --features tui --example cw_uhf_tui` | Interactive TUI for CW TX with live frequency |
| `rx_uhf` | `cargo run --example rx_uhf` | Listen on RF09 (sub-1 GHz), print received frames with RSSI and hex dump. |
| `tx_uhf` | `cargo run --example tx_uhf` | Transmit a single frame on RF09. `--payload <HEX_VAULE>` for custom hex. |
| `txrx_uhf` | `cargo run --example txrx_uhf` | Periodic `PING`-beacon TX + RX on RF09; demonstrates half-duplex operation. |
| `live` | `cargo run --features tui --example live` | TUI telemetry viewer; receives CBOR `CommState` from the daemon's telemetry socket (default UDP 10035). |


