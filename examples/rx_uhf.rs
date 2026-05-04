//! RX - UHF example.
//!
//! Listens on RF09 (sub-1 GHz) and prints received frames until interrupted.
//!
//! Usage:
//!   cargo run --example rx_uhf -- --spi /dev/spidev0.0 --freq 868300000
//!
//! Press Ctrl-C to stop.

use std::{io, time::Duration};

use clap::Parser;

use oresat_at86rf215_driver::{
    freq::{Band, PllSettings},
    radio::Radio,
    registers::{RfnCmd, TransceiverCmd},
    spi,
};

#[derive(Parser)]
#[command(name = "rx_uhf", about = "Listen on RF09 (sub-1 GHz) and print received frames")]
struct Args {
    /// SPI device path
    #[arg(long, default_value = "/dev/spidev0.0")]
    spi: String,

    /// Centre frequency in Hz (default 868.3 MHz)
    #[arg(long, default_value = "868300000")]
    freq: u64,

    /// GPIO chip for the radio IRQ line
    #[arg(long, default_value = "/dev/gpiochip0")]
    gpio_chip: String,

    /// GPIO line number for the radio IRQ (rising edge)
    #[arg(long, default_value = "30")]
    gpio_line: u32,
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    // Open SPI
    let mut dev = spi::open(&args.spi)?;
    let mut radio = Radio::new();

    // Chip reset + identity
    let (pn, vn) = spi::reset_and_identify(&mut dev, &mut radio)?;
    eprintln!("chip: {:?} v{}", pn, vn);

    // Configure frequency
    let pll = PllSettings::fine(Band::Sub1GHz, args.freq)
        .map_err(io::Error::other)?;
    spi::apply_channel_rf09(&mut dev, &mut radio, pll)?;
    eprintln!("frequency: {} Hz", args.freq);

    // Enable baseband + FCS filter
    radio.bbc0_pc.value = radio.bbc0_pc.value
        .with_bben(true)
        .with_fcsfe(true);
    spi::write_register(&mut dev, &radio.bbc0_pc)?;

    // Enable RXFE interrupt
    radio.bbc0_irqm.value = radio.bbc0_irqm.value.with_rxfe(true);
    spi::write_register(&mut dev, &radio.bbc0_irqm)?;

    // Enter Rx
    radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::TxPrep);
    spi::write_register(&mut dev, &radio.rf09_cmd)?;
    spi::wait_rf09_txprep_locked(&mut dev, &mut radio, Duration::from_millis(5))?;

    radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::Rx);
    spi::write_register(&mut dev, &radio.rf09_cmd)?;
    eprintln!("listening (Ctrl-C to stop)...");

    // GPIO IRQ setup
    let irq = gpiocdev::Request::builder()
        .on_chip(&args.gpio_chip)
        .with_line(args.gpio_line)
        .with_edge_detection(gpiocdev::line::EdgeDetection::RisingEdge)
        .request()
        .map_err(io::Error::other)?;

    // Signal handling. signal_hook::flag::register *sets* the flag on signal
    // receipt; sentinel is false-until-Ctrl-C, loop exits when it flips true.
    let term = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let r = term.clone();
    signal_hook::flag::register(signal_hook::consts::SIGINT, term.clone())
        .map_err(io::Error::other)?;

    // Receive loop
    let mut rx_count: u64 = 0;

    while !r.load(std::sync::atomic::Ordering::Relaxed) {
        // Block until an edge event arrives (or check periodically)
        if irq.wait_edge_event(Duration::from_millis(250)).is_err() {
            continue;
        }
        while irq.has_edge_event().unwrap_or(false) {
            let _ = irq.read_edge_event();
        }

        // Read and clear BBC0 IRQ status
        spi::read_register(&mut dev, &mut radio.bbc0_irqs)?;
        if !radio.bbc0_irqs.value.rxfe() {
            continue;
        }

        // Check CRC
        spi::read_register(&mut dev, &mut radio.bbc0_pc)?;
        if !radio.bbc0_pc.value.fcsok() {
            eprintln!("  [bad CRC - dropped]");
            continue;
        }

        // Read frame length and data
        spi::read_register(&mut dev, &mut radio.bbc0_fbl)?;
        let len = radio.bbc0_fbl.value.fbl() as usize;
        if len == 0 {
            continue;
        }
        let data = spi::read_rx_fifo(&mut dev, spi::Bbc::Bbc0, len)?;

        // Read RSSI
        spi::read_register(&mut dev, &mut radio.rf09_rssi)?;
        let rssi = radio.rf09_rssi.value.rssi();

        rx_count += 1;
        let hex: String = data.iter().take(32).map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ");
        let suffix = if data.len() > 32 { "..." } else { "" };
        eprintln!("  #{rx_count}: {len} B  RSSI={rssi} dBm  {hex}{suffix}");
    }

    eprintln!("\n{rx_count} frames received");

    // Shutdown
    radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::TrxOff);
    spi::write_register(&mut dev, &radio.rf09_cmd)?;

    Ok(())
}
