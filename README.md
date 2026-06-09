# oresat-at86rf215-driver
Microchip AT86RF215 Rust driver and utilites

[Datasheet](https://ww1.microchip.com/downloads/aemDocuments/documents/OTH/ProductDocuments/DataSheets/Atmel-42415-WIRELESS-AT86RF215_Datasheet.pdf) (PDF)
Initial [design thoughts](https://docs.google.com/document/d/1zBbb-4qnycPR2GkD_XNbqpXR3gw3rPBdjSQARGNUBZ0/edit?tab=t.0)

## Run
`daemon --freq 436500000 --config configs/sat.toml`  
`daemon --freq 436500000 --config configs/ground.toml`

```
# Ground
DAEMON_BIN=./daemon ./scripts/ground-daemon.sh --tx-bind 0.0.0.0:10025 --rx-peer <yamcs-host-ip>:10016

# Satellite
DAEMON_BIN=./daemon ./scripts/sat-daemon.sh
```