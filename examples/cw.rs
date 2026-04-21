//! Continuous-wave (CW) TX plan for RF09.
//!
//! Doesn't talk to real SPI - generates the byte arrays a spidev caller would
//! send, showing how `freq::PllSettings` and `typestate::Transceiver` compose.

use oresat_at86rf215_driver::{
    freq::{Band, PllSettings},
    radio::Radio,
    registers::BulkWrites,
    typestate::{Rf09, Transceiver, TrxOff},
};

fn main() {
    let mut radio = Radio::new();

    // 868.3 MHz, 200 kHz spacing (IEEE 802.15.4 SUN sub-GHz channel 0).
    let pll = PllSettings::ieee(Band::Sub1GHz, 868_300_000, 200_000, 0)
        .expect("frequency must be in-range");
    println!(
        "PLL plan: ccf0={} cn={} cs={} mode={:?}",
        pll.ccf0, pll.cn, pll.cs, pll.mode
    );
    pll.apply_rf09(&mut radio);

    // Type-state: TrxOff -> TxPrep -> Tx. Each call writes the matching command
    // into `rf09_cmd`; a real driver would flush those writes to SPI between
    // transitions (here we just show the last one).
    let trx: Transceiver<Rf09, TrxOff> = Transceiver::new(radio);
    let trx = trx.tx_prep().tx();
    let mut radio = trx.into_radio();

    // Flush the channel plan in one shot - CCF0, CS, CN are contiguous so
    // BulkWrites will coalesce them into a single SPI transaction.
    let mut pending = BulkWrites::new();
    pending.add(&mut radio.rf09_ccf0);
    pending.add(&mut radio.rf09_cs);
    pending.add(&mut radio.rf09_cn);
    pending.add(&mut radio.rf09_cmd);
    for (i, cmd) in pending.generate_commands().into_iter().enumerate() {
        println!("bulk[{i}]: {cmd:02X?}");
    }
}
