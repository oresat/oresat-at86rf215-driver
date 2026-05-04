//! TX/RX - UHF example.
//!
//! Sends a beacon frame periodically on RF09 (sub-1 GHz) and listens for
//! incoming frames between transmissions.  Demonstrates half-duplex
//! bidirectional radio operation.
//!
//! Usage:
//!   cargo run --example txrx_uhf -- --spi /dev/spidev0.0 --freq 868300000
//!
//! The beacon payload defaults to "PING" followed by a 2-byte sequence number.
//! Incoming frames are printed with RSSI and hex dump.  Press Ctrl-C to stop.

use std::{io, time::Duration, time::Instant};

use clap::Parser;

use oresat_at86rf215_driver::{
    freq::{Band, PllSettings},
    radio::Radio,
    registers::{BbcnTxfl, RfnCmd, TransceiverCmd},
    spi::{self, Bbc},
};

#[derive(Parser)]
#[command(name = "txrx_uhf", about = "Periodic beacon TX + RX on RF09 (sub-1 GHz)")]
struct Args {
    /// SPI device path.
    #[arg(long, default_value = "/dev/spidev0.0")]
    spi: String,

    /// Centre frequency in Hz (default 868.3 MHz).
    #[arg(long, default_value = "868300000")]
    freq: u64,

    /// Beacon interval in milliseconds.
    #[arg(long, default_value = "2000")]
    interval_ms: u64,

    /// GPIO chip for the radio IRQ line.
    #[arg(long, default_value = "/dev/gpiochip0")]
    gpio_chip: String,

    /// GPIO line number for the radio IRQ (rising edge).
    #[arg(long, default_value = "30")]
    gpio_line: u32,
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

    // ── enable baseband + auto-FCS + FCS filter ────────────────────────
    radio.bbc0_pc.value = radio.bbc0_pc.value
        .with_bben(true)
        .with_txafcs(true)
        .with_fcsfe(true);
    spi::write_register(&mut dev, &radio.bbc0_pc)?;

    // ── enable RXFE interrupt ──────────────────────────────────────────
    radio.bbc0_irqm.value = radio.bbc0_irqm.value.with_rxfe(true);
    spi::write_register(&mut dev, &radio.bbc0_irqm)?;

    // ── GPIO IRQ setup ─────────────────────────────────────────────────
    let irq = gpiocdev::Request::builder()
        .on_chip(&args.gpio_chip)
        .with_line(args.gpio_line)
        .with_edge_detection(gpiocdev::line::EdgeDetection::RisingEdge)
        .request()
        .map_err(io::Error::other)?;

    // ── signal handling ────────────────────────────────────────────────
    // signal_hook::flag::register *sets* the flag on signal receipt, so the
    // sentinel is false-until-Ctrl-C and the loop exits when it flips to true.
    let term = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGINT, term.clone())
        .map_err(io::Error::other)?;

    // ── enter Rx ───────────────────────────────────────────────────────
    radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::TxPrep);
    spi::write_register(&mut dev, &radio.rf09_cmd)?;
    spi::wait_rf09_txprep_locked(&mut dev, &mut radio, Duration::from_millis(5))?;
    radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::Rx);
    spi::write_register(&mut dev, &radio.rf09_cmd)?;

    eprintln!(
        "TX/RX active - beacon every {} ms (Ctrl-C to stop)",
        args.interval_ms,
    );

    // ── main loop ──────────────────────────────────────────────────────
    let interval = Duration::from_millis(args.interval_ms);
    let mut next_tx = Instant::now() + interval;
    let mut seq: u16 = 0;
    let mut tx_count: u64 = 0;
    let mut rx_count: u64 = 0;

    while !term.load(std::sync::atomic::Ordering::Relaxed) {
        // ── check for RX ───────────────────────────────────────────────
        if irq.wait_edge_event(Duration::from_millis(50)).is_ok() {
            while irq.has_edge_event().unwrap_or(false) {
                let _ = irq.read_edge_event();
            }

            spi::read_register(&mut dev, &mut radio.bbc0_irqs)?;
            if radio.bbc0_irqs.value.rxfe() {
                spi::read_register(&mut dev, &mut radio.bbc0_pc)?;
                if radio.bbc0_pc.value.fcsok() {
                    spi::read_register(&mut dev, &mut radio.bbc0_fbl)?;
                    let len = radio.bbc0_fbl.value.fbl() as usize;
                    if len > 0 {
                        let data = spi::read_rx_fifo(&mut dev, Bbc::Bbc0, len)?;
                        spi::read_register(&mut dev, &mut radio.rf09_rssi)?;
                        let rssi = radio.rf09_rssi.value.rssi();
                        rx_count += 1;

                        let hex: String = data.iter().take(32)
                            .map(|b| format!("{:02X}", b))
                            .collect::<Vec<_>>()
                            .join(" ");
                        let suffix = if data.len() > 32 { "..." } else { "" };
                        eprintln!("  RX #{rx_count}: {len} B  RSSI={rssi} dBm  {hex}{suffix}");
                    }
                }
            }
        }

        // ── periodic TX beacon ─────────────────────────────────────────
        if Instant::now() >= next_tx {
            // Build beacon: "PING" + 2-byte sequence number.
            let mut beacon = Vec::with_capacity(6);
            beacon.extend_from_slice(b"PING");
            beacon.extend_from_slice(&seq.to_be_bytes());

            // TxPrep -> load FIFO -> wait for PLL -> TX -> back to Rx.
            // The FIFO + TXFL writes already take ~hundreds of µs at 10 MHz
            // SPI, but on a fast bus the PLL may not be locked yet when we
            // try to transition to Tx - poll explicitly.
            radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::TxPrep);
            spi::write_register(&mut dev, &radio.rf09_cmd)?;

            spi::write_tx_fifo(&mut dev, Bbc::Bbc0, &beacon)?;
            radio.bbc0_txfl.value = BbcnTxfl::new().with_txfl(beacon.len() as u16);
            spi::write_register(&mut dev, &radio.bbc0_txfl)?;

            spi::wait_rf09_txprep_locked(&mut dev, &mut radio, Duration::from_millis(5))?;
            radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::Tx);
            spi::write_register(&mut dev, &radio.rf09_cmd)?;

            // Re-enter Rx after TX completes.
            radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::Rx);
            spi::write_register(&mut dev, &radio.rf09_cmd)?;

            tx_count += 1;
            seq = seq.wrapping_add(1);
            eprintln!("  TX #{tx_count}: beacon seq={}", seq.wrapping_sub(1));
            next_tx = Instant::now() + interval;
        }
    }

    eprintln!("\nshutdown: tx={tx_count} rx={rx_count}");
    radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::TrxOff);
    spi::write_register(&mut dev, &radio.rf09_cmd)?;

    Ok(())
}
