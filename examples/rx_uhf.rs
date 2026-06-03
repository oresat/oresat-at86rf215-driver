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
    registers::{EnergyDetectionMode, RfnCmd, TransceiverCmd},
    spi,
};

#[derive(Parser)]
#[command(
    name = "rx_uhf",
    about = "Listen on RF09 (sub-1 GHz) and print received frames"
)]
struct Args {
    /// SPI device path
    #[arg(long, default_value = "/dev/spidev0.0")]
    spi: String,

    /// SPI clock in Hz (default 10 MHz).
    #[arg(long, default_value = "10000000")]
    spi_hz: u32,

    /// Centre frequency in Hz (default 868.3 MHz)
    #[arg(long, default_value = "463.500000")]
    freq: u64,

    /// GPIO chip for the radio IRQ line
    #[arg(long, default_value = "/dev/gpiochip0")]
    gpio_chip: String,

    /// GPIO line number for the radio IRQ (rising edge)
    #[arg(long, default_value = "30")]
    gpio_line: u32,

    /// Skip GPIO and poll BBC0_IRQS over SPI every millisecond.
    #[arg(long)]
    poll: bool,

    /// Print chip state (rf09_state, pll.ls, bbc0_pc, accumulated IRQS,
    /// RSSI) every second.
    #[arg(long)]
    verbose: bool,
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    eprintln!(
        "rx_uhf v{} (build {})",
        env!("CARGO_PKG_VERSION"),
        env!("BUILD_ID")
    );

    // Open SPI
    let mut dev = spi::open_with_speed(&args.spi, args.spi_hz)?;
    eprintln!("SPI: {} @ {} Hz", args.spi, args.spi_hz);
    let mut radio = Radio::new();

    // Chip reset + identity
    let (pn, vn) = spi::reset_and_identify(&mut dev, &mut radio)?;
    eprintln!("chip: {:?} v{}", pn, vn);

    // Configure frequency
    let pll = PllSettings::fine(Band::Sub1GHz, args.freq).map_err(io::Error::other)?;
    spi::apply_channel_rf09(&mut dev, &mut radio, pll)?;
    eprintln!("frequency: {} Hz", args.freq);

    // Match the FSK modulation programmed by tx_uhf's defaults:
    // 2-FSK, h=0.5 (MSK), 50 kHz symbol rate. Silicon reset for FSKC0 is
    // 0xD6 (h=1.0), so a fresh-reset RX won't demod MSK without this.
    radio.bbc0_fskc0.value = radio
        .bbc0_fskc0
        .value
        .with_mord(false)
        .with_midx(1)
        .with_midxs(1)
        .with_bt(0);
    spi::write_register(&mut dev, &radio.bbc0_fskc0)?;
    radio.bbc0_fskc1.value = radio.bbc0_fskc1.value.with_srate(0);
    spi::write_register(&mut dev, &radio.bbc0_fskc1)?;

    // RX digital frontend for 50 kHz MSK, sub-1GHz (datasheet Table 6-51 +
    // Table 6-60, modulation index 1/2). Must be set in TRXOFF.
    radio.rf09_rxdfe.value = radio.rf09_rxdfe.value.with_sr(10).with_rcut(0);
    spi::write_register(&mut dev, &radio.rf09_rxdfe)?;
    radio.rf09_rxbwc.value = radio
        .rf09_rxbwc
        .value
        .with_bw(0) // 160 kHz @ 250 kHz IF
        .with_ifs(false);
    spi::write_register(&mut dev, &radio.rf09_rxbwc)?;
    radio.rf09_agcc.value = radio.rf09_agcc.value.with_en(true).with_avgs(0);
    spi::write_register(&mut dev, &radio.rf09_agcc)?;
    radio.rf09_agcs.value = radio.rf09_agcs.value.with_tgt(1);
    spi::write_register(&mut dev, &radio.rf09_agcs)?;

    // Enable baseband + PHY type = MR-FSK.
    // PT=0 is PHY OFF (datasheet 6.10.3): chip enters Rx with PLL locked
    // but the demodulator never runs, so RXFE never fires.
    // FCSFE (FCS filter) left OFF: with it on, a frame whose CRC fails is
    // discarded WITHOUT raising RXFE, so a CRC/config mismatch looks
    // identical to "no frame". Off means every completed frame raises RXFE
    // and shows bytes + FCSOK to tell a bit-error from a real config mismatch.
    radio.bbc0_pc.value = radio
        .bbc0_pc
        .value
        .with_pt(1)
        .with_bben(true)
        .with_fcsfe(false);
    spi::write_register(&mut dev, &radio.bbc0_pc)?;

    // Read back to confirm PT=1 / BBEN stuck (catches a chip that silently
    // refuses the write, example: wrong CS or a not-in-TrxOff state).
    spi::read_register(&mut dev, &mut radio.bbc0_pc)?;
    eprintln!(
        "bbc0_pc readback = {:#04x} (pt={}, bben={})",
        u8::from(radio.bbc0_pc.value),
        radio.bbc0_pc.value.pt(),
        radio.bbc0_pc.value.bben(),
    );

    // Read back the modulation config. Must match tx_uhf: FSKC0 should decode
    // MIDX=1,MIDXS=1 (h=0.5). The silicon reset is 0xD6 (MIDX=3 -> h=1.0); if
    // the readback shows 0xD6 the write didn't take and the demod is running
    // at the wrong modulation index, which garbles the PSDU.
    spi::read_register(&mut dev, &mut radio.bbc0_fskc0)?;
    spi::read_register(&mut dev, &mut radio.bbc0_fskc1)?;
    spi::read_register(&mut dev, &mut radio.rf09_rxdfe)?;
    spi::read_register(&mut dev, &mut radio.rf09_rxbwc)?;
    eprintln!(
        "fsk readback: fskc0={:#04x} (mord={} midx={} midxs={}) fskc1={:#04x} (srate={}) \
         rxdfe={:#04x} (sr={}) rxbwc={:#04x} (bw={})",
        u8::from(radio.bbc0_fskc0.value),
        radio.bbc0_fskc0.value.mord() as u8,
        radio.bbc0_fskc0.value.midx(),
        radio.bbc0_fskc0.value.midxs(),
        u8::from(radio.bbc0_fskc1.value),
        radio.bbc0_fskc1.value.srate(),
        u8::from(radio.rf09_rxdfe.value),
        radio.rf09_rxdfe.value.sr(),
        u8::from(radio.rf09_rxbwc.value),
        radio.rf09_rxbwc.value.bw(),
    );

    // Enable RX interrupts. RXFS (frame start) in addition to RXFE lets the
    // diagnostic distinguish "preamble/PHR detected but CRC failed" (RXFS
    // fires, RXFE may or may not) from "nothing detected at all" (neither).
    radio.bbc0_irqm.value = radio
        .bbc0_irqm
        .value
        .with_rxfs(true)
        .with_rxfe(true)
        .with_agch(true)
        .with_agcr(true);
    spi::write_register(&mut dev, &radio.bbc0_irqm)?;

    // Energy detection: AUTO triggers an ED measurement when the AGC is held
    // (For example: during frame reception) and latches the result in RF09_EDV. That
    // value stays readable after the frame ends, unlike RF09_RSSI which only
    // holds a valid number while the receiver/AGC is live and reads 0x7F (127,
    // "invalid") once the chip drops to TxPrep at frame-end. AUTO is the reset
    // default but set it explicitly so a stale EDC can't leave reading 127.
    radio.rf09_edc.value = radio.rf09_edc.value.with_edm(EnergyDetectionMode::Auto);
    spi::write_register(&mut dev, &radio.rf09_edc)?;

    // Enter Rx
    radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::TxPrep);
    spi::write_register(&mut dev, &radio.rf09_cmd)?;
    spi::wait_rf09_txprep_locked(&mut dev, &mut radio, Duration::from_millis(5))?;

    radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::Rx);
    spi::write_register(&mut dev, &radio.rf09_cmd)?;

    spi::read_register(&mut dev, &mut radio.rf09_state)?;
    eprintln!(
        "rf09_state after Rx cmd = {:?}",
        radio.rf09_state.value.state()
    );
    eprintln!(
        "listening (mode={}, Ctrl-C to stop)...",
        if args.poll { "SPI-poll" } else { "GPIO-IRQ" },
    );

    // Signal handling. signal_hook::flag::register *sets* the flag on signal
    // receipt; sentinel is false-until-Ctrl-C, loop exits when it flips true.
    let term = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let r = term.clone();
    signal_hook::flag::register(signal_hook::consts::SIGINT, term.clone())
        .map_err(io::Error::other)?;

    // GPIO IRQ setup (skipped in --poll mode).
    let irq = if args.poll {
        None
    } else {
        Some(
            gpiocdev::Request::builder()
                .on_chip(&args.gpio_chip)
                .with_line(args.gpio_line)
                .with_edge_detection(gpiocdev::line::EdgeDetection::RisingEdge)
                .request()
                .map_err(io::Error::other)?,
        )
    };

    // Receive loop
    let mut rx_count: u64 = 0;
    let mut last_status = std::time::Instant::now();
    // Accumulate IRQS bits seen since the last status print. Reading IRQS
    // clears it, so a 1 ms poll would otherwise drop RXFS/AGC events that
    // arrive between the once-a-second verbose status line.
    let mut acc_irqs: u8 = 0;

    while !r.load(std::sync::atomic::Ordering::Relaxed) {
        // Wait for a frame-ready signal: either a GPIO edge or, in --poll
        // mode, just a short sleep before re-reading IRQS over SPI.
        match &irq {
            Some(irq) => {
                if irq.wait_edge_event(Duration::from_millis(250)).is_err() {
                    // No edge: fall through so verbose status can still print.
                    if !args.verbose {
                        continue;
                    }
                } else {
                    while irq.has_edge_event().unwrap_or(false) {
                        let _ = irq.read_edge_event();
                    }
                }
            }
            None => std::thread::sleep(Duration::from_millis(1)),
        }

        // Read and clear BBC0 IRQ status.
        spi::read_register(&mut dev, &mut radio.bbc0_irqs)?;
        let irqs_now = u8::from(radio.bbc0_irqs.value);
        acc_irqs |= irqs_now;

        // Log on frame-start only (RXFS). AGCH/AGCR fire constantly and just
        // flood the log, so they're folded into acc_irqs but not printed.
        if args.verbose && radio.bbc0_irqs.value.rxfs() {
            eprintln!("  [rxfs] frame start detected (irqs={irqs_now:#04x})");
        }

        if args.verbose && last_status.elapsed() >= Duration::from_secs(1) {
            spi::read_register(&mut dev, &mut radio.rf09_state)?;
            spi::read_register(&mut dev, &mut radio.rf09_pll)?;
            spi::read_register(&mut dev, &mut radio.rf09_rssi)?;
            eprintln!(
                "  [status] state={:?} pll.ls={} rssi={} dBm acc_irqs={:#04x}",
                radio.rf09_state.value.state(),
                radio.rf09_pll.value.ls(),
                radio.rf09_rssi.value.rssi(),
                acc_irqs,
            );
            last_status = std::time::Instant::now();
            acc_irqs = 0;
        }

        if !radio.bbc0_irqs.value.rxfe() {
            continue;
        }

        // Frame complete. Read CRC status, length, data - but print bytes
        // even on a bad FCS so it is visable whether the payload is intact
        // (CRC/config mismatch) or garbled (modulation/bit-error).
        spi::read_register(&mut dev, &mut radio.bbc0_pc)?;
        let fcsok = radio.bbc0_pc.value.fcsok();

        spi::read_register(&mut dev, &mut radio.bbc0_fbl)?;
        let len = radio.bbc0_fbl.value.fbl() as usize;
        if len == 0 {
            eprintln!("  [rxfe] len=0 (fcsok={fcsok})");
            continue;
        }
        let data = spi::read_rx_fifo(&mut dev, spi::Bbc::Bbc0, len)?;

        // Per-frame signal level. Read EDV (energy detection value), not
        // RF09_RSSI: by frame-end the chip has dropped to TxPrep and RF09_RSSI
        // reads 0x7F (127, invalid). EDV was latched during reception (EDC=AUTO)
        // and still holds the real dBm value here.
        spi::read_register(&mut dev, &mut radio.rf09_edv)?;
        let rssi = radio.rf09_edv.value.edv();

        // RXFL/len includes the FCS field (datasheet 6.13.3). Split it off so
        // the displayed bytes are the actual payload. PC.FCST=0 => 32-bit FCS
        // (4 octets), =1 => 16-bit (2 octets).
        let fcs_len = if radio.bbc0_pc.value.fcst() { 2 } else { 4 };
        let payload_len = len.saturating_sub(fcs_len);
        let (payload, fcs) = data.split_at(payload_len.min(data.len()));

        rx_count += 1;
        let crc = if fcsok { "OK " } else { "BAD" };
        let hex: String = payload
            .iter()
            .take(32)
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ");
        let suffix = if payload.len() > 32 { "..." } else { "" };
        let fcs_hex: String = fcs
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ");
        eprintln!(
            "  #{rx_count}: {payload_len} B  CRC={crc}  RSSI={rssi} dBm  {hex}{suffix}  [FCS {fcs_hex}]"
        );

        // Re-arm Rx. Empirically the chip drops to TxPrep after a frame and
        // stops receiving, so explicitly command Rx again (PLL stays locked,
        // so a direct CMD=Rx from TxPrep is enough).
        radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::Rx);
        spi::write_register(&mut dev, &radio.rf09_cmd)?;
    }

    eprintln!("\n{rx_count} frames received");

    // Shutdown
    radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::TrxOff);
    spi::write_register(&mut dev, &radio.rf09_cmd)?;

    Ok(())
}
