//! Continuous-wave (CW) transmission on RF09.
//!
//! Outputs an unmodulated carrier at the specified frequency using the TX
//! DAC direct-mode override.  Useful for antenna tuning, spectrum analyser
//! verification, and regulatory pre-compliance spot checks.
//!
//! Usage:
//!   cargo run --example cw_uhf -- --spi /dev/spidev0.0 --freq 868300000
//!
//! Press Ctrl-C to stop.  The radio is returned to TrxOff on shutdown.

use std::{io, time::Duration};

use clap::Parser;

use oresat_at86rf215_driver::{
    freq::{Band, PllSettings},
    radio::Radio,
    registers::{RfnCmd, TransceiverCmd},
    spi,
};

#[derive(Parser)]
#[command(name = "cw_uhf", about = "Continuous-wave TX on RF09 (sub-1 GHz)")]
struct Args {
    /// SPI device path.
    #[arg(long, default_value = "/dev/spidev0.0")]
    spi: String,

    /// Center frequency in Hz (default 868.3 MHz).
    #[arg(long, default_value = "868300000")]
    freq: u64,
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    // ── open SPI ───────────────────────────────────────────────────────
    let mut dev = spi::open(&args.spi)?;
    let mut radio = Radio::new();

    // ── chip reset + identity ──────────────────────────────────────────
    let (pn, vn) = spi::reset_and_identify(&mut dev, &mut radio)?;
    eprintln!("chip: {:?} v{}", pn, vn);

    // ── configure frequency ────────────────────────────────────────────
    let pll = PllSettings::fine(Band::Sub1GHz, args.freq)
        .map_err(io::Error::other)?;
    spi::apply_channel_rf09(&mut dev, &mut radio, pll)?;
    eprintln!("frequency: {} Hz", args.freq);

    // ── TX DAC direct mode - unmodulated carrier ───────────────────────
    // Set I = max positive (0x3F), Q = 0.  This produces a pure tone at
    // the configured center frequency with no modulation sidebands.
    radio.rf09_txdaci.value = radio.rf09_txdaci.value
        .with_entxdacid(true)
        .with_txdacid(0x3F);
    radio.rf09_txdacq.value = radio.rf09_txdacq.value
        .with_entxdacqd(true)
        .with_txdacqd(0x00);
    spi::write_register(&mut dev, &radio.rf09_txdaci)?;
    spi::write_register(&mut dev, &radio.rf09_txdacq)?;

    // ── enable continuous TX in the baseband ───────────────────────────
    radio.bbc0_pc.value = radio.bbc0_pc.value
        .with_bben(true)
        .with_ctx(true);
    spi::write_register(&mut dev, &radio.bbc0_pc)?;

    // ── transition to Tx: TrxOff -> TxPrep -> Tx ────────────────────────
    radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::TxPrep);
    spi::write_register(&mut dev, &radio.rf09_cmd)?;
    std::thread::sleep(Duration::from_micros(200)); // PLL lock

    radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::Tx);
    spi::write_register(&mut dev, &radio.rf09_cmd)?;

    eprintln!("CW active at {} Hz - Ctrl-C to stop", args.freq);

    // ── signal handling ────────────────────────────────────────────────
    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    signal_hook::flag::register(signal_hook::consts::SIGINT, running.clone())
        .map_err(io::Error::other)?;

    while running.load(std::sync::atomic::Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(100));
    }

    // ── shutdown ───────────────────────────────────────────────────────
    eprintln!("\nshutting down");
    radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::TrxOff);
    spi::write_register(&mut dev, &radio.rf09_cmd)?;

    // Disable DAC override.
    radio.rf09_txdaci.value = radio.rf09_txdaci.value.with_entxdacid(false);
    radio.rf09_txdacq.value = radio.rf09_txdacq.value.with_entxdacqd(false);
    spi::write_register(&mut dev, &radio.rf09_txdaci)?;
    spi::write_register(&mut dev, &radio.rf09_txdacq)?;

    Ok(())
}
