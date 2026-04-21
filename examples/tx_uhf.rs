//! TX - UHF example.
//!
//! Transmits a single frame on RF09 (sub-1 GHz) at a configurable frequency.
//!
//! Usage:
//!   cargo run --example tx_uhf -- --spi /dev/spidev0.0 --freq 868300000
//!
//! The frame payload defaults to a short test pattern. Use `--payload` to
//! specify hex bytes (e.g. `--payload "DEADBEEF"`).

use std::{io, time::Duration};

use clap::Parser;

use oresat_at86rf215_driver::{
    freq::{Band, PllSettings},
    radio::Radio,
    registers::{BbcnTxfl, RfnCmd, TransceiverCmd},
    spi::{self, Bbc},
};

#[derive(Parser)]
#[command(name = "tx_uhf", about = "Transmit a single frame on RF09 (sub-1 GHz)")]
struct Args {
    /// SPI device path.
    #[arg(long, default_value = "/dev/spidev0.0")]
    spi: String,

    /// Centre frequency in Hz (default 868.3 MHz).
    #[arg(long, default_value = "868300000")]
    freq: u64,

    /// Frame payload as hex string (e.g. "DEADBEEF"). Default: 16-byte ramp.
    #[arg(long)]
    payload: Option<String>,
}

fn parse_hex(s: &str) -> Result<Vec<u8>, String> {
    if !s.len().is_multiple_of(2) {
        return Err("hex string must have even length".into());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.to_string()))
        .collect()
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    let payload = match args.payload {
        Some(hex) => parse_hex(&hex).map_err(io::Error::other)?,
        None => (0..16).collect(), // 0x00..0x0F ramp
    };

    // ── open SPI ───────────────────────────────────────────────────────
    let mut dev = spi::open(&args.spi)?;
    let mut radio = Radio::new();

    // ── chip reset + identity ──────────────────────────────────────────
    let (pn, vn) = spi::reset_and_identify(&mut dev, &mut radio)?;
    eprintln!("chip: {:?} v{}", pn, vn);

    // ── configure frequency ────────────────────────────────────────────
    let pll = PllSettings::fine(Band::Sub1GHz, args.freq)
        .map_err(io::Error::other)?;
    eprintln!(
        "frequency: {} Hz (CCF0={}, CN={}, CS={})",
        args.freq, pll.ccf0, pll.cn, pll.cs,
    );
    spi::apply_channel_rf09(&mut dev, &mut radio, pll)?;

    // ── enable baseband + auto-FCS ─────────────────────────────────────
    radio.bbc0_pc.value = radio.bbc0_pc.value
        .with_bben(true)
        .with_txafcs(true);
    spi::write_register(&mut dev, &radio.bbc0_pc)?;

    // ── transition to TxPrep ───────────────────────────────────────────
    radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::TxPrep);
    spi::write_register(&mut dev, &radio.rf09_cmd)?;
    std::thread::sleep(Duration::from_micros(200)); // PLL lock

    // ── load TX FIFO ───────────────────────────────────────────────────
    spi::write_tx_fifo(&mut dev, Bbc::Bbc0, &payload)?;
    radio.bbc0_txfl.value = BbcnTxfl::new().with_txfl(payload.len() as u16);
    spi::write_register(&mut dev, &radio.bbc0_txfl)?;

    // ── transmit ───────────────────────────────────────────────────────
    radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::Tx);
    spi::write_register(&mut dev, &radio.rf09_cmd)?;

    eprintln!("TX: {} bytes sent", payload.len());
    eprintln!("payload: {:02X?}", payload);

    // ── back to TrxOff ─────────────────────────────────────────────────
    // Wait for TX to complete (chip auto-transitions to TxPrep).
    std::thread::sleep(Duration::from_millis(10));
    radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::TrxOff);
    spi::write_register(&mut dev, &radio.rf09_cmd)?;

    Ok(())
}
