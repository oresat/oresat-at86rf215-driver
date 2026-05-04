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
    registers::{ChipMode, RfnCmd, TransceiverCmd},
    spi,
};

#[derive(Parser)]
#[command(name = "cw_uhf", about = "Continuous-wave TX on RF09 (sub-1 GHz)", version)]
struct Args {
    /// SPI device path.
    #[arg(long, default_value = "/dev/spidev0.0")]
    spi: String,

    /// Center frequency in Hz (default 868.3 MHz).
    #[arg(long, default_value = "868300000")]
    freq: u64,

    /// PA output power, 0..31 (~1 dB steps; 31 = max).
    #[arg(long, default_value = "31")]
    txpwr: u8,

    /// PA bias current, 0..3 (3 = no gain reduction).
    #[arg(long, default_value = "3")]
    pacur: u8,

    /// Chip mode for IQIFC1.CHPM:
    /// "iq" = IqRadioMode (CHPM=1, default - required for DAC-override CW;
    /// 
    /// "bb" = BasebandMode
    #[arg(long, default_value = "iq", value_parser = ["bb", "iq"])]
    chpm: String,
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    eprintln!("cw_uhf v{}", env!("CARGO_PKG_VERSION"));

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

    // ── chip mode ──────────────────────────────────────────────────────
    // Selectable via --chpm so we can A/B-test if BasebandMode produces no carrier.
    let chpm = match args.chpm.as_str() {
        "iq" => ChipMode::IqRadioMode,
        _ => ChipMode::BasebandMode,
    };
    radio.rf_iqifc1.value = radio.rf_iqifc1.value.with_chpm(chpm);
    spi::write_register(&mut dev, &radio.rf_iqifc1)?;
    eprintln!("chip mode: {:?}", chpm);

    // ── TX path config (explicit; do not trust hardware reset) ─────────
    // The TX DAC is clocked from RFn_TXDFE.SR.
    radio.rf09_txdfe.value = radio.rf09_txdfe.value
        .with_sr(1)         // 4 MHz sample rate
        .with_rcut(2);      // 0.5 * fS/2
    spi::write_register(&mut dev, &radio.rf09_txdfe)?;

    radio.rf09_txcutc.value = radio.rf09_txcutc.value
        .with_lpfcut(0)     // 80 kHz analog LPF
        .with_paramp(3);    // 32 µs PA ramp
    spi::write_register(&mut dev, &radio.rf09_txcutc)?;

    // ── TX DAC direct mode - unmodulated carrier ───────────────────────
    radio.rf09_txdaci.value = radio.rf09_txdaci.value
        .with_entxdacid(true)
        .with_txdacid(0x7E);
    radio.rf09_txdacq.value = radio.rf09_txdacq.value
        .with_entxdacqd(true)
        .with_txdacqd(0x3F);
    spi::write_register(&mut dev, &radio.rf09_txdaci)?;
    spi::write_register(&mut dev, &radio.rf09_txdacq)?;

    // ── PA settings ────────────────────────────────────────────────────
    radio.rf09_pac.value = radio.rf09_pac.value
        .with_txpwr(args.txpwr & 0x1F)
        .with_pacur(args.pacur & 0x03);
    spi::write_register(&mut dev, &radio.rf09_pac)?;
    eprintln!("pac: txpwr={} pacur={}", args.txpwr, args.pacur);

    // DAC override drives constants into the analog modulator, bypassing
    // baseband. BBC0_PC.BBEN/CTX is intentionally left off - enabling them
    // with default-zero FSK config has historically caused the chip to
    // refuse Tx on some parts.

    // ── transition to Tx: TrxOff -> TxPrep -> Tx ────────────────────────
    radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::TxPrep);
    spi::write_register(&mut dev, &radio.rf09_cmd)?;
    spi::wait_rf09_txprep_locked(&mut dev, &mut radio, Duration::from_millis(5))?;

    radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::Tx);
    spi::write_register(&mut dev, &radio.rf09_cmd)?;

    // ── confirm Tx, surface TRXERR if PLL dropped or PA gating failed ──
    // Reading IRQS clears it, so any TRXERR raised between TxPrep and now
    // shows up here and only here. State should be 4 (Tx).
    spi::read_register(&mut dev, &mut radio.rf09_state)?;
    spi::read_register(&mut dev, &mut radio.rf09_irqs)?;
    eprintln!(
        "post-Tx: state={:?} pll_locked={} trxerr={} wakeup={}",
        radio.rf09_state.value.state(),
        radio.rf09_pll.value.ls(),
        radio.rf09_irqs.value.trxerr(),
        radio.rf09_irqs.value.wakeup(),
    );
    if radio.rf09_irqs.value.trxerr() {
        eprintln!("warning: TRXERR raised — the chip rejected the Tx command");
    }

    // ── readback every register ────────────────────────
    spi::read_register(&mut dev, &mut radio.rf_iqifc1)?;
    spi::read_register(&mut dev, &mut radio.rf09_txdfe)?;
    spi::read_register(&mut dev, &mut radio.rf09_txcutc)?;
    spi::read_register(&mut dev, &mut radio.rf09_txdaci)?;
    spi::read_register(&mut dev, &mut radio.rf09_txdacq)?;
    spi::read_register(&mut dev, &mut radio.rf09_pac)?;
    spi::read_register(&mut dev, &mut radio.rf09_cs)?;
    spi::read_register(&mut dev, &mut radio.rf09_ccf0)?;
    spi::read_register(&mut dev, &mut radio.rf09_cn)?;
    eprintln!(
        "readback: IQIFC1.chpm={:?} TXDFE(sr={} dm={} rcut={}) TXCUTC(lpf={} ramp={})",
        radio.rf_iqifc1.value.chpm(),
        radio.rf09_txdfe.value.sr(),
        radio.rf09_txdfe.value.dm(),
        radio.rf09_txdfe.value.rcut(),
        radio.rf09_txcutc.value.lpfcut(),
        radio.rf09_txcutc.value.paramp(),
    );
    eprintln!(
        "readback: TXDACI(en={} val=0x{:02X}) TXDACQ(en={} val=0x{:02X})",
        radio.rf09_txdaci.value.entxdacid(),
        radio.rf09_txdaci.value.txdacid(),
        radio.rf09_txdacq.value.entxdacqd(),
        radio.rf09_txdacq.value.txdacqd(),
    );
    eprintln!(
        "readback: PAC(txpwr={} pacur={}) channel(cs={} ccf0={} cn={} cm={})",
        radio.rf09_pac.value.txpwr(),
        radio.rf09_pac.value.pacur(),
        radio.rf09_cs.value.cs(),
        radio.rf09_ccf0.value.ccf0(),
        radio.rf09_cn.value.cn(),
        radio.rf09_cn.value.cm(),
    );

    eprintln!("CW active at {} Hz - Ctrl-C to stop", args.freq);

    // ── signal handling ────────────────────────────────────────────────
    // signal_hook::flag::register *sets* the flag on signal receipt, so the
    // sentinel is false-until-Ctrl-C and the loop exits when it flips to true.
    let term = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGINT, term.clone())
        .map_err(io::Error::other)?;

    while !term.load(std::sync::atomic::Ordering::Relaxed) {
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
