# oresat-at86rf215-driver
Microchip AT86RF215 Rust driver and utilites

[Datasheet](https://ww1.microchip.com/downloads/aemDocuments/documents/OTH/ProductDocuments/DataSheets/Atmel-42415-WIRELESS-AT86RF215_Datasheet.pdf) (PDF)
Initial [design thoughts](https://docs.google.com/document/d/1zBbb-4qnycPR2GkD_XNbqpXR3gw3rPBdjSQARGNUBZ0/edit?tab=t.0)

## Setup/Dependecies

Install the necessary dependecies:

* `arm-linux-gnueabihf-gcc`
* `rustup`
* `cargo-deb`

> [Note]
> Distros vary in naming schemes for the cross-compiler of the `armv7-unknown-linux-gnueabihf` target
> Ensure your cross-compiler binary name matches the value in `.cargo/config.toml`


Install the necessary rust target:

```
rustup target add armv7-unknown-linux-gnueabihf
```

## Run Daemon on c3
```
# Ground
uhf_daemon --config ground.toml --tx-bind 0.0.0.0:10025 --rx-peer <yamcs-host-ip>:10016

# Satellite
sudo uhf_daemon --rx-port 10025 --tx-port 10016 --config sat.toml
```
### Build .deb package
```
cargo deb --target=armv7-unknown-linux-gnueabihf

# On c3
sudo apt install ./.deb
```
