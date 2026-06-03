//! TX - UHF example.
//!
//! Transmits one or more frames on RF09 (sub-1 GHz) at a configurable
//! frequency. With `--repeat`, retransmits in a loop until Ctrl-C.
//!
//! Usage:
//!   cargo run --example tx_uhf -- --spi /dev/spidev0.0 --freq 463500000
//!   cargo run --example tx_uhf -- --config configs/sat.toml --freq 463500000
//!   cargo run --example tx_uhf -- --repeat --gap-ms 5
//!   cargo run --example tx_uhf -- --repeat --h 1.0 --whiten      # Sunde 2-FSK
//!   cargo run --example tx_uhf -- --repeat --h 1.5 --whiten      # wide FSK
//!
//! The frame payload defaults to a short test pattern. Use `--payload` to
//! specify hex bytes (example: `--payload "0BADCAFE"`). A `--config <toml>`
//! applies a RadioConfig (example: PA settings from `configs/sat.toml`) before
//! the channel is programmed.

use std::{
    fs::read_to_string,
    io,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use clap::Parser;

use oresat_at86rf215_driver::{
    config::RadioConfig,
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

    /// SPI clock in Hz (default 10 MHz).
    #[arg(long, default_value = "10000000")]
    spi_hz: u32,

    /// Center frequency. Accepts an optional unit suffix (k/M/G, with or
    /// without "Hz", case-insensitive); a bare number is Hz. Examples:
    /// `463.5MHz`, `463.5m`, `463500000`.
    #[arg(long, default_value = "463500000", value_parser = parse_freq_hz)]
    freq: u64,

    /// Frame payload as hex string (e.g. "0BADCAFE"). Default: 16-byte ramp.
    #[arg(long)]
    payload: Option<String>,

    /// TOML RadioConfig to apply before programming the channel.
    #[arg(long)]
    config: Option<String>,

    /// Loop forever, retransmitting until Ctrl-C.
    #[arg(long)]
    repeat: bool,

    /// Gap between retransmissions in `--repeat` mode (milliseconds).
    /// Shorter gaps = higher duty cycle / easier to see on the analyzer.
    #[arg(long, default_value = "5")]
    gap_ms: u64,

    /// PA output power (RFn_PAC.TXPWR, 0..31, ~1 dB steps). Default 24.
    #[arg(long, default_value = "24")]
    txpwr: u8,

    /// PA bias current (RFn_PAC.PACUR, 0..3). 3 = no gain reduction (default);
    /// 2 = ~1 dB, 1 = ~2 dB, 0 = ~3 dB reduction.
    #[arg(long, default_value = "3")]
    pacur: u8,

    /// FSK symbol rate in kHz. One of: 50, 100, 150, 200, 300, 400.
    /// With MSK (h=0.5) the peak-to-peak shift is srate/2 kHz.
    #[arg(long, default_value = "50")]
    srate_khz: u16,

    /// Modulation index h. 0.5 = MSK.
    /// The actual h used is the closest pair available from FSKC0.MIDX
    /// (0.375/0.5/0.75/1.0/1.25/1.5/1.75/2.0) times FSKC0.MIDXS
    /// (7/8, 1, 9/8, 10/8).
    #[arg(long, default_value = "0.5")]
    h: f32,

    /// Enable IEEE 802.15.4g PN9 data whitening on the PSDU (FSKPHRTX.DW=1).
    #[arg(long)]
    whiten: bool,

    /// Skip the recommended TX filter tuning. By default the example
    /// programs TXCUTC.PARAMP/LPFCUT and TXDFE.RCUT per datasheet
    /// Table 6-53 (h<=0.75) or Table 6-54 (h>0.75) for the chosen srate.
    /// Disable with --no-tune-filters.
    #[arg(long)]
    no_tune_filters: bool,

    /// Disable FSK direct modulation (TXDFE.DM=0, FSKDM.EN=0) and use the
    /// normal baseband modulator instead.
    #[arg(long)]
    no_direct_mod: bool,

    /// External front-end control configuration (RFn_PADFE.PADFE, 0..3).
    /// 0 = off (FEA/FEB held low, no external FE); 1 = Config 1 (TX/RX
    /// switch + LNA bypass); 2 = Config 2 (enable + TX/RX switch);
    /// 3 = Config 3 (TX/RX switch + LNA bypass, MCU-gated enable).
    /// Datasheet sect 6.5. Overrides any value from --config.
    #[arg(long)]
    padfe: Option<u8>,
}

/// Pick the closest (MIDX, MIDXS) pair to a target modulation index.
/// Returns (MIDX, MIDXS, actual_h). Datasheet sect 6.10.7.1.
fn pick_midx_midxs(target_h: f32) -> (u8, u8, f32) {
    const H_BASE: [f32; 8] = [0.375, 0.5, 0.75, 1.0, 1.25, 1.5, 1.75, 2.0];
    const SCALE: [f32; 4] = [7.0 / 8.0, 1.0, 9.0 / 8.0, 10.0 / 8.0];
    let mut best = (0u8, 0u8, f32::INFINITY, 0.0f32);
    for (midx, hb) in H_BASE.iter().enumerate() {
        for (midxs, sc) in SCALE.iter().enumerate() {
            let h = hb * sc;
            let err = (h - target_h).abs();
            if err < best.2 {
                best = (midx as u8, midxs as u8, err, h);
            }
        }
    }
    (best.0, best.1, best.3)
}

/// Recommended TX-frontend filter settings from datasheet Table 6-53
/// (modulation index 0.5) and Table 6-54 (modulation index 1).
/// Returns (PARAMP, LPFCUT, RCUT). Picks Table 6-53 for h <= 0.75 and
/// Table 6-54 otherwise; h > 1 has no datasheet table.
fn recommended_tx_filters(srate_khz: u16, h: f32) -> Option<(u8, u8, u8)> {
    let idx = match srate_khz {
        50 => 0,
        100 => 1,
        150 => 2,
        200 => 3,
        300 => 4,
        400 => 5,
        _ => return None,
    };
    let paramp = [3u8, 2, 2, 2, 1, 1][idx];
    if h <= 0.75 {
        let lpfcut = [0u8, 1, 3, 4, 6, 7][idx];
        Some((paramp, lpfcut, 0))
    } else {
        let lpfcut = [0u8, 3, 5, 6, 8, 9][idx];
        Some((paramp, lpfcut, 4))
    }
}

fn srate_field(khz: u16) -> Result<u8, String> {
    match khz {
        50 => Ok(0),
        100 => Ok(1),
        150 => Ok(2),
        200 => Ok(3),
        300 => Ok(4),
        400 => Ok(5),
        n => Err(format!(
            "unsupported srate {n} kHz; pick 50/100/150/200/300/400"
        )),
    }
}

/// TX DAC sample rate (RFn_TXDFE.SR) per datasheet Table 6-51 for AT86RF215
/// v.3, indexed by FSK symbol rate.
fn tx_dfe_sr(srate_khz: u16) -> Result<u8, String> {
    match srate_khz {
        50 => Ok(8),
        100 => Ok(4),
        150 => Ok(2),
        200 => Ok(2),
        300 => Ok(1),
        400 => Ok(1),
        n => Err(format!(
            "unsupported srate {n} kHz; pick 50/100/150/200/300/400"
        )),
    }
}

/// Parse a frequency with an optional unit suffix into Hz. A bare number is
/// Hz; suffixes k/M/G (optionally followed by "Hz") scale by 1e3/1e6/1e9 and
/// are case-insensitive. The numeric part may be fractional ("463.5MHz").
fn parse_freq_hz(s: &str) -> Result<u64, String> {
    let s = s.trim();
    let num_end = s
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(s.len());
    let (num, unit) = s.split_at(num_end);
    let value: f64 = num
        .parse()
        .map_err(|_| format!("invalid frequency number: {num:?}"))?;
    let scale = match unit.trim().trim_end_matches(['H', 'h', 'Z', 'z']) {
        "" => 1.0,
        "k" | "K" => 1e3,
        "m" | "M" => 1e6,
        "g" | "G" => 1e9,
        other => return Err(format!("unknown frequency unit: {other:?}")),
    };
    Ok((value * scale) as u64)
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

    eprintln!(
        "tx_uhf v{} (build {})",
        env!("CARGO_PKG_VERSION"),
        env!("BUILD_ID")
    );

    let payload = match args.payload {
        Some(hex) => parse_hex(&hex).map_err(io::Error::other)?,
        None => (0..16).collect(), // 0x00..0x0F ramp
    };

    // -- open SPI -------------------------------------------------------
    let mut dev = spi::open_with_speed(&args.spi, args.spi_hz)?;
    eprintln!("SPI: {} @ {} Hz", args.spi, args.spi_hz);
    let mut radio = Radio::new();

    // -- chip reset + identity ------------------------------------------
    let (pn, vn) = spi::reset_and_identify(&mut dev, &mut radio)?;
    eprintln!("chip: {:?} v{}", pn, vn);

    // -- apply optional TOML config -------------------------------------
    // Flushes the RF09 TX-path registers (txcutc, txdfe, pac, padfe) which
    // is the subset sat.toml is expected to set. apply_channel_rf09 below
    // will overwrite rf09_cs/ccf0/cn from the TOML - intentional.
    if let Some(path) = &args.config {
        let toml_str = read_to_string(path)?;
        let cfg: RadioConfig = toml::from_str(&toml_str).map_err(io::Error::other)?;
        radio.apply_config(&cfg);
        spi::write_register(&mut dev, &radio.rf09_txcutc)?; // TX filter cutoff + PA ramp
        spi::write_register(&mut dev, &radio.rf09_txdfe)?; // TX DAC sample-rate + IF
        spi::write_register(&mut dev, &radio.rf09_pac)?; // PA power control
        spi::write_register(&mut dev, &radio.rf09_padfe)?; // PA driver front-end
        eprintln!("applied config: {}", path);
    }

    // -- external front-end control (RFn_PADFE) -------------------------
    // FEA/FEB are driven by the state machine per the selected config
    // (datasheet sect 6.5). Set in TRXOFF (where the chip sits after reset)
    // before going to TXPREP/TX. Overrides any value applied from --config.
    if let Some(padfe) = args.padfe {
        if padfe > 3 {
            return Err(io::Error::other(format!(
                "invalid --padfe {padfe}; must be 0..3 (RFn_PADFE.PADFE is 2 bits)"
            )));
        }
        radio.rf09_padfe.value = radio.rf09_padfe.value.with_padfe(padfe);
        spi::write_register(&mut dev, &radio.rf09_padfe)?;
        eprintln!("front-end: RFn_PADFE.PADFE={} (Config {})", padfe, padfe);
    }

    // -- configure frequency --------------------------------------------
    let pll = PllSettings::fine(Band::Sub1GHz, args.freq).map_err(io::Error::other)?;
    eprintln!(
        "frequency: {} Hz (CCF0={}, CN={}, CS={})",
        args.freq, pll.ccf0, pll.cn, pll.cs,
    );
    spi::apply_channel_rf09(&mut dev, &mut radio, pll)?;

    // -- clock the TX DAC + enable direct modulation --------------------
    // RFn_TXDFE.SR resets to 0 (reserved/invalid) - leaves the TX DAC
    // unclocked, so the baseband stalls and TXFE never fires. See NOTES.md.
    // SR must match the symbol rate (Table 6-51): too-fast SR transmits the
    // SHR/PHR but not the FIFO-sourced PSDU. DM=1 pairs with FSKDM.EN below.
    let tx_sr = tx_dfe_sr(args.srate_khz).map_err(io::Error::other)?;
    let direct_mod = !args.no_direct_mod;
    radio.rf09_txdfe.value = radio.rf09_txdfe.value.with_sr(tx_sr).with_dm(direct_mod);
    spi::write_register(&mut dev, &radio.rf09_txdfe)?; // TX DAC clock + direct-mod
    eprintln!(
        "TXDFE.SR={} direct_mod={} (Table 6-51 for {} kHz)",
        tx_sr, direct_mod, args.srate_khz
    );

    // -- 2-FSK with selectable modulation index -------------------------
    // Datasheet sect 6.10.4.2: direct modulation shall be used for all FSK
    // modes (both FSKDM.EN and TXDFE.DM).
    let (midx, midxs, actual_h) = pick_midx_midxs(args.h);
    radio.bbc0_fskc0.value = radio
        .bbc0_fskc0
        .value
        .with_mord(false)
        .with_midx(midx)
        .with_midxs(midxs)
        .with_bt(0);
    spi::write_register(&mut dev, &radio.bbc0_fskc0)?; // FSK mod-order + index
    let srate = srate_field(args.srate_khz).map_err(io::Error::other)?;
    radio.bbc0_fskc1.value = radio.bbc0_fskc1.value.with_srate(srate);
    spi::write_register(&mut dev, &radio.bbc0_fskc1)?; // FSK symbol rate
    let shift_khz = actual_h * args.srate_khz as f32;
    let label = if (actual_h - 0.5).abs() < 0.01 {
        " (MSK)"
    } else if (actual_h - 1.0).abs() < 0.01 {
        " (Sunde 2-FSK)"
    } else {
        ""
    };
    eprintln!(
        "2-FSK: srate={} kHz, h={:.3}{} -> peak-to-peak shift {:.1} kHz",
        args.srate_khz, actual_h, label, shift_khz,
    );
    radio.bbc0_fskdm.value = radio.bbc0_fskdm.value.with_en(direct_mod);
    spi::write_register(&mut dev, &radio.bbc0_fskdm)?; // FSK direct-mod enable

    // -- Recommended TX-frontend filters (datasheet Table 6-53 / 6-54) --
    // Hardware-reset values (PARAMP=0, LPFCUT=0, RCUT=0) are not the
    // recommended setup for any srate above 50 kHz, and even at 50 kHz
    // PARAMP=0 is a 4 us PA ramp vs. the recommended 32 us.
    if !args.no_tune_filters {
        if let Some((paramp, lpfcut, rcut)) = recommended_tx_filters(args.srate_khz, actual_h) {
            radio.rf09_txcutc.value = radio
                .rf09_txcutc
                .value
                .with_paramp(paramp)
                .with_lpfcut(lpfcut);
            spi::write_register(&mut dev, &radio.rf09_txcutc)?;
            radio.rf09_txdfe.value = radio.rf09_txdfe.value.with_rcut(rcut);
            spi::write_register(&mut dev, &radio.rf09_txdfe)?;
            eprintln!(
                "tx-filters: PARAMP={} LPFCUT={} RCUT={} (Table {})",
                paramp,
                lpfcut,
                rcut,
                if actual_h <= 0.75 { "6-53" } else { "6-54" },
            );
        }
    }

    // -- PSDU data whitening (PN9 scrambler per IEEE 802.15.4g) ---------
    // Off by default to match prior behaviour.
    radio.bbc0_fskphrtx.value = radio.bbc0_fskphrtx.value.with_dw(args.whiten);
    spi::write_register(&mut dev, &radio.bbc0_fskphrtx)?;
    if args.whiten {
        eprintln!("whitening: PSDU PN9 scrambler enabled (FSKPHRTX.DW=1)");
    }

    // -- enable baseband + auto-FCS -------------------------------------
    // PT=1 is MR-FSK; PT=0 is "PHY OFF" (datasheet sec 6.10.3), which
    // looks superficially valid (chip enters Tx, PLL locks) but emits
    // no symbols and TXFE never fires.
    radio.bbc0_pc.value = radio
        .bbc0_pc
        .value
        .with_pt(1)
        .with_bben(true)
        .with_txafcs(true);
    spi::write_register(&mut dev, &radio.bbc0_pc)?; // PHY type + BB enable + auto-FCS

    // -- PA power -------------------------------------------------------
    // RFn_PAC reset is txpwr=0 (minimum) + pacur=0 (3 dB gain reduction).
    // pacur=3 = no reduction.
    radio.rf09_pac.value = radio
        .rf09_pac
        .value
        .with_txpwr(args.txpwr.min(31))
        .with_pacur(args.pacur.min(3));
    spi::write_register(&mut dev, &radio.rf09_pac)?; // PA TX power + current
    eprintln!(
        "PA: txpwr={}/31 pacur={}",
        args.txpwr.min(31),
        args.pacur.min(3)
    );

    // Read RFn_PAC back and confirm TXPWR/PACUR actually stuck. The reset
    // value is txpwr=0 (~25 dB below max); if the write lands during a state
    // transition or gets clobbered, the chip transmits ~25 dB low with no
    // other symptom.
    spi::read_register(&mut dev, &mut radio.rf09_pac)?;
    let got_txpwr = radio.rf09_pac.value.txpwr();
    let got_pacur = radio.rf09_pac.value.pacur();
    let want_txpwr = args.txpwr.min(31);
    let want_pacur = args.pacur.min(3);
    if got_txpwr != want_txpwr || got_pacur != want_pacur {
        return Err(io::Error::other(format!(
            "RFn_PAC readback mismatch: wrote txpwr={want_txpwr} pacur={want_pacur}, \
             read txpwr={got_txpwr} pacur={got_pacur} (txpwr=0 transmits ~25 dB low)"
        )));
    }

    // -- modulator config readback --------------------------------------
    // Read back what actually stuck in the chip's TX/FSK path. The PSDU
    // garble bug means the modulator transmits something other than the
    // FIFO; this surfaces any FSK config (FEC/interleave/raw/SFD) or
    // chip-mode (CHPM must be 0 = baseband) that would divert the data
    // path away from the frame buffer.
    spi::read_register(&mut dev, &mut radio.bbc0_fskc0)?;
    spi::read_register(&mut dev, &mut radio.bbc0_fskc1)?;
    spi::read_register(&mut dev, &mut radio.bbc0_fskc2)?;
    spi::read_register(&mut dev, &mut radio.bbc0_fskc3)?;
    spi::read_register(&mut dev, &mut radio.bbc0_fskc4)?;
    spi::read_register(&mut dev, &mut radio.bbc0_fskdm)?;
    spi::read_register(&mut dev, &mut radio.bbc0_fskphrtx)?;
    spi::read_register(&mut dev, &mut radio.bbc0_pc)?;
    spi::read_register(&mut dev, &mut radio.rf09_txdfe)?;
    spi::read_register(&mut dev, &mut radio.rf_iqifc1)?;
    eprintln!(
        "TX cfg readback: fskc0={:#04x}(midx={} midxs={}) fskc1={:#04x}(srate={}) \
         fskc2={:#04x}(fecie={} fecs={}) fskc3={:#04x} fskc4={:#04x}(rawrbit={} sfd32={}) \
         fskdm={:#04x}(en={} pe={}) fskphrtx={:#04x}(dw={} sfd={}) \
         pc={:#04x}(pt={} bben={} txafcs={}) txdfe={:#04x}(sr={} dm={}) iqifc1={:#04x}(chpm={:?})",
        u8::from(radio.bbc0_fskc0.value),
        radio.bbc0_fskc0.value.midx(),
        radio.bbc0_fskc0.value.midxs(),
        u8::from(radio.bbc0_fskc1.value),
        radio.bbc0_fskc1.value.srate(),
        u8::from(radio.bbc0_fskc2.value),
        radio.bbc0_fskc2.value.fecie(),
        radio.bbc0_fskc2.value.fecs(),
        u8::from(radio.bbc0_fskc3.value),
        u8::from(radio.bbc0_fskc4.value),
        radio.bbc0_fskc4.value.rawrbit(),
        radio.bbc0_fskc4.value.sfd32(),
        u8::from(radio.bbc0_fskdm.value),
        radio.bbc0_fskdm.value.en(),
        radio.bbc0_fskdm.value.pe(),
        u8::from(radio.bbc0_fskphrtx.value),
        radio.bbc0_fskphrtx.value.dw(),
        radio.bbc0_fskphrtx.value.sfd(),
        u8::from(radio.bbc0_pc.value),
        radio.bbc0_pc.value.pt(),
        radio.bbc0_pc.value.bben(),
        radio.bbc0_pc.value.txafcs(),
        u8::from(radio.rf09_txdfe.value),
        radio.rf09_txdfe.value.sr(),
        radio.rf09_txdfe.value.dm(),
        u8::from(radio.rf_iqifc1.value),
        radio.rf_iqifc1.value.chpm(),
    );

    // -- SIGINT handling for --repeat -----------------------------------
    // Init false; signal_hook flips false->true on Ctrl-C
    let term = Arc::new(AtomicBool::new(false));
    if args.repeat {
        signal_hook::flag::register(signal_hook::consts::SIGINT, term.clone())
            .map_err(io::Error::other)?;
        eprintln!("repeat mode: gap={} ms, Ctrl-C to stop", args.gap_ms);
        eprintln!("payload: {:02X?}", payload);
    } else {
        eprintln!("payload: {:02X?}", payload);
    }

    // TXFL is the total PSDU length INCLUDING the FCS field (datasheet 6.13.3).
    // With TXAFCS=1 the chip overwrites the last fcs_len octets of the frame
    // buffer with the computed FCS, so the buffer must reserve room for them.
    // PC.FCST=0 => 32-bit FCS (4 octets); =1 => 16-bit (2 octets). We leave
    // FCST=0. Omitting this reservation truncates the real data by fcs_len
    // bytes (the tail of the payload is silently replaced by the FCS).
    let fcs_len = if radio.bbc0_pc.value.fcst() { 2 } else { 4 };
    let mut frame = payload.clone();
    frame.resize(payload.len() + fcs_len, 0x00); // FCS placeholder (any value)
    let frame_len = frame.len();

    // -- transmit loop --------------------------------------------------
    let mut count: u64 = 0;
    loop {
        // TxPrep + wait for PLL lock.
        radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::TxPrep);
        spi::write_register(&mut dev, &radio.rf09_cmd)?; // cmd: TxPrep
        spi::wait_rf09_txprep_locked(&mut dev, &mut radio, Duration::from_millis(5))?;

        // Load TX FIFO + length (length covers payload + FCS placeholder).
        spi::write_tx_fifo(&mut dev, Bbc::Bbc0, &frame)?;
        radio.bbc0_txfl.value = BbcnTxfl::new().with_txfl(frame_len as u16);
        spi::write_register(&mut dev, &radio.bbc0_txfl)?;

        // Diagnostic (first frame only): read the data portion back to confirm
        // the payload landed in the TX frame buffer.
        if count == 0 {
            let back = spi::read_tx_fifo(&mut dev, Bbc::Bbc0, payload.len())?;
            eprintln!("TX FIFO readback: {:02X?} (+{} FCS bytes)", back, fcs_len);
        }

        // Issue Tx.
        radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::Tx);
        spi::write_register(&mut dev, &radio.rf09_cmd)?; // cmd: Tx

        // Poll BBC0_IRQS.TXFE (reading IRQS clears it; loop on fresh reads).
        // OR-accumulate so a transient bit set isn't lost when we also dump
        // diagnostics on timeout.
        let deadline = Instant::now() + Duration::from_millis(500);
        let mut acc_bbc_irqs: u8 = 0;
        let mut acc_rf_irqs: u8 = 0;
        loop {
            spi::read_register(&mut dev, &mut radio.bbc0_irqs)?;
            spi::read_register(&mut dev, &mut radio.rf09_irqs)?;
            acc_bbc_irqs |= u8::from(radio.bbc0_irqs.value);
            acc_rf_irqs |= u8::from(radio.rf09_irqs.value);
            if radio.bbc0_irqs.value.txfe() || (acc_bbc_irqs & 0x10) != 0 {
                break;
            }
            if Instant::now() >= deadline {
                spi::read_register(&mut dev, &mut radio.rf09_state)?;
                spi::read_register(&mut dev, &mut radio.rf09_pll)?;
                spi::read_register(&mut dev, &mut radio.bbc0_pc)?;
                spi::read_register(&mut dev, &mut radio.bbc0_ps)?;
                spi::read_register(&mut dev, &mut radio.rf09_txdfe)?;
                spi::read_register(&mut dev, &mut radio.rf_iqifc1)?;
                eprintln!(
                    "TXFE timeout: rf09_state={:?} pll.ls={} bbc0_pc={:#04x} \
                     bbc0_ps={:#04x} txdfe={:#04x} iqifc1={:#04x} \
                     acc_bbc_irqs={:#04x} acc_rf_irqs={:#04x}",
                    radio.rf09_state.value.state(),
                    radio.rf09_pll.value.ls(),
                    u8::from(radio.bbc0_pc.value),
                    u8::from(radio.bbc0_ps.value),
                    u8::from(radio.rf09_txdfe.value),
                    u8::from(radio.rf_iqifc1.value),
                    acc_bbc_irqs,
                    acc_rf_irqs,
                );
                return Err(io::Error::other("timed out waiting for TXFE"));
            }
            std::thread::sleep(Duration::from_micros(200));
        }

        count += 1;
        if !args.repeat {
            eprintln!("TX: {} bytes sent", payload.len());
            break;
        }

        // Brief status every ~1 s without flooding stderr.
        if count.is_multiple_of(50) {
            eprintln!("TX: {} frames sent", count);
        }

        if term.load(Ordering::Relaxed) {
            eprintln!("\nstopping after {} frames", count);
            break;
        }
        if args.gap_ms > 0 {
            std::thread::sleep(Duration::from_millis(args.gap_ms));
        }
    }

    // -- back to TrxOff -------------------------------------------------
    radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::TrxOff);
    spi::write_register(&mut dev, &radio.rf09_cmd)?; // cmd: TrxOff

    Ok(())
}
