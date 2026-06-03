//! Interactive TUI driver for RF09 transmit, combining the CW-tone and
//! 2-FSK frame-TX paths in one tool. Switch between modes live with the
//! `Mode` field; each mode exposes its own controls.
//!
//! - CW mode: DAC-override carrier with
//!   center frequency and PA controls (txpwr/pacur).
//! - Frame mode: 2-FSK frame TX with
//!   center frequency, TX power, symbol rate, modulation index, data
//!   whitening, repeat gap, and an editable payload.
//!
//! Usage:
//!   cargo run --release --features tui --example uhf_tui
//!
//! Keys:
//!   up/down    select field
//!   left/right decrement / increment selected field (or switch mode)
//!   PgUp/PgDn  coarser step on freq (x10)
//!   [ / ]      finer / coarser freq step
//!   t          transmit one frame (Frame mode)
//!   Space      toggle TX (CW tone on/off, or frame repeat on/off)
//!   e / Enter  edit payload (when Payload selected, Frame mode)
//!   a          apply current settings to the chip
//!   q / Esc    quit (clean shutdown if currently transmitting)
//!

use std::{
    io::{self, Stdout},
    time::{Duration, Instant},
};

use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use oresat_at86rf215_driver::{
    freq::{Band, PllSettings},
    radio::Radio,
    registers::{BbcnTxfl, ChipMode, RfnCmd, TransceiverCmd, TransceiverState},
    spi::{self, Bbc},
};

#[derive(Parser)]
#[command(
    name = "uhf_tui",
    about = "Interactive CW + frame TX on RF09 with live control"
)]
struct Args {
    /// SPI device path.
    #[arg(long, default_value = "/dev/spidev0.0")]
    spi: String,

    /// SPI clock in Hz.
    #[arg(long, default_value = "10000000")]
    spi_hz: u32,

    /// Start mode: "cw" or "frame".
    #[arg(long, default_value = "frame")]
    mode: String,

    /// Initial center frequency in Hz.
    #[arg(long, default_value = "463500000")]
    freq: u64,

    /// Initial PA output power, 0..31 (~1 dB steps; 31 = max).
    #[arg(long, default_value = "24")]
    txpwr: u8,

    /// Initial PA bias current, 0..3 (3 = no gain reduction). CW mode.
    #[arg(long, default_value = "3")]
    pacur: u8,

    /// Initial FSK symbol rate in kHz. One of 50/100/150/200/300/400.
    #[arg(long, default_value = "50")]
    srate_khz: u16,

    /// Initial modulation index h. 0.5 = MSK, 1.0 = Sunde 2-FSK.
    #[arg(long, default_value = "0.5")]
    h: f32,

    /// Enable IEEE 802.15.4g PN9 data whitening on the PSDU.
    #[arg(long)]
    whiten: bool,

    /// Frame payload as hex string (e.g. "0BADCAFE"). Default: 16-byte ramp.
    #[arg(long)]
    payload: Option<String>,

    /// Gap between retransmissions in repeat mode (milliseconds).
    #[arg(long, default_value = "5")]
    gap_ms: u64,
}

// Selectable symbol rates and their FSKC1.SRATE field values.
const SRATES_KHZ: [u16; 6] = [50, 100, 150, 200, 300, 400];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Cw,
    Frame,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Field {
    Mode,
    Freq,
    TxPwr,
    PaCur,
    Srate,
    ModIndex,
    Whiten,
    Gap,
    Payload,
}

impl Field {
    /// Fields visible (and navigable) in the given mode, in display order.
    fn visible(mode: Mode) -> &'static [Field] {
        match mode {
            Mode::Cw => &[Field::Mode, Field::Freq, Field::TxPwr, Field::PaCur],
            Mode::Frame => &[
                Field::Mode,
                Field::Freq,
                Field::TxPwr,
                Field::Srate,
                Field::ModIndex,
                Field::Whiten,
                Field::Gap,
                Field::Payload,
            ],
        }
    }

    fn next(self, mode: Mode) -> Self {
        let v = Self::visible(mode);
        let i = v.iter().position(|f| *f == self).unwrap_or(0);
        v[(i + 1) % v.len()]
    }

    fn prev(self, mode: Mode) -> Self {
        let v = Self::visible(mode);
        let i = v.iter().position(|f| *f == self).unwrap_or(0);
        v[(i + v.len() - 1) % v.len()]
    }

    fn label(self) -> &'static str {
        match self {
            Field::Mode => "Mode",
            Field::Freq => "Frequency",
            Field::TxPwr => "TX Power",
            Field::PaCur => "PA Current",
            Field::Srate => "Symbol Rate",
            Field::ModIndex => "Mod Index",
            Field::Whiten => "Whitening",
            Field::Gap => "Repeat Gap",
            Field::Payload => "Payload",
        }
    }
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
/// (h <= 0.75) and Table 6-54 (h > 0.75). Returns (PARAMP, LPFCUT, RCUT).
fn recommended_tx_filters(srate_khz: u16, h: f32) -> Option<(u8, u8, u8)> {
    let idx = SRATES_KHZ.iter().position(|&s| s == srate_khz)?;
    let paramp = [3u8, 2, 2, 2, 1, 1][idx];
    if h <= 0.75 {
        let lpfcut = [0u8, 1, 3, 4, 6, 7][idx];
        Some((paramp, lpfcut, 0))
    } else {
        let lpfcut = [0u8, 3, 5, 6, 8, 9][idx];
        Some((paramp, lpfcut, 4))
    }
}

fn srate_field(khz: u16) -> Option<u8> {
    SRATES_KHZ.iter().position(|&s| s == khz).map(|i| i as u8)
}

/// TX DAC sample rate (RFn_TXDFE.SR) per datasheet Table 6-51, indexed by
/// FSK symbol rate. SR must track the symbol rate.
fn tx_dfe_sr(srate_khz: u16) -> Option<u8> {
    let idx = SRATES_KHZ.iter().position(|&s| s == srate_khz)?;
    Some([8u8, 4, 2, 2, 1, 1][idx])
}

fn parse_hex(s: &str) -> Result<Vec<u8>, String> {
    // Allow whitespace separators between bytes for friendlier editing.
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if !s.len().is_multiple_of(2) {
        return Err("hex string must have even length".into());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.to_string()))
        .collect()
}

struct State {
    mode: Mode,
    freq_hz: u64,
    txpwr: u8,
    pacur: u8,
    srate_khz: u16,
    h: f32,
    whiten: bool,
    gap_ms: u64,
    payload: Vec<u8>,
    freq_step: u64,
    selected: Field,
    /// CW tone on (CW mode) or frame repeat on (Frame mode).
    tx_active: bool,
    dirty: bool,
    count: u64,
    last_tx: Instant,
    /// Hex edit buffer for the payload; Some while editing.
    editing: Option<String>,
    log: Vec<String>,
}

impl State {
    fn new(a: &Args, payload: Vec<u8>) -> Self {
        let (_, _, actual_h) = pick_midx_midxs(a.h);
        let mode = if a.mode.eq_ignore_ascii_case("cw") {
            Mode::Cw
        } else {
            Mode::Frame
        };
        Self {
            mode,
            freq_hz: a.freq,
            txpwr: a.txpwr.min(31),
            pacur: a.pacur.min(3),
            srate_khz: if SRATES_KHZ.contains(&a.srate_khz) {
                a.srate_khz
            } else {
                50
            },
            h: actual_h,
            whiten: a.whiten,
            gap_ms: a.gap_ms,
            payload,
            freq_step: 1_000,
            selected: Field::Mode,
            tx_active: false,
            dirty: false,
            count: 0,
            last_tx: Instant::now(),
            editing: None,
            log: vec![],
        }
    }

    fn say(&mut self, msg: impl Into<String>) {
        self.log.push(msg.into());
        if self.log.len() > 50 {
            self.log.remove(0);
        }
    }

    /// PSDU length including the 4-byte FCS placeholder (PC.FCST=0).
    fn frame(&self) -> Vec<u8> {
        let mut frame = self.payload.clone();
        frame.resize(self.payload.len() + 4, 0x00);
        frame
    }
}

// ---------------------------------------------------------------------------
// CW path (DAC override).
// ---------------------------------------------------------------------------

/// Program the CW TX path (chip mode, DAC clock, channel, PA) from state.
fn write_config_cw(dev: &mut spidev::Spidev, radio: &mut Radio, s: &State) -> io::Result<()> {
    // CHPM=IqRadioMode is required for DAC-override CW on this part: the
    // LVDS data input is bypassed by the override, but the LVDS clock
    // domain is what actually clocks the TX DAC. BasebandMode (CHPM=0)
    // produces no carrier even with state=Tx pll=locked.
    radio.rf_iqifc1.value = radio.rf_iqifc1.value.with_chpm(ChipMode::IqRadioMode);
    spi::write_register(dev, &radio.rf_iqifc1)?;

    // TXDFE.SR resets to 0 (reserved) on some parts; un-clocked DAC =
    // silent CW. Set known-good values matching cariboulite's working
    // path: 4 MHz sample rate, 80 kHz analog LPF, 32 us PA ramp.
    radio.rf09_txdfe.value = radio.rf09_txdfe.value.with_sr(1).with_rcut(2).with_dm(false);
    spi::write_register(dev, &radio.rf09_txdfe)?;
    radio.rf09_txcutc.value = radio.rf09_txcutc.value.with_lpfcut(0).with_paramp(3);
    spi::write_register(dev, &radio.rf09_txcutc)?;

    let pll = PllSettings::fine(Band::Sub1GHz, s.freq_hz).map_err(io::Error::other)?;
    spi::apply_channel_rf09(dev, radio, pll)?;

    radio.rf09_pac.value = radio
        .rf09_pac
        .value
        .with_txpwr(s.txpwr & 0x1F)
        .with_pacur(s.pacur & 0x03);
    spi::write_register(dev, &radio.rf09_pac)
}

/// Engage CW: DAC override + transition TrxOff -> TxPrep -> Tx.
fn start_cw(dev: &mut spidev::Spidev, radio: &mut Radio) -> io::Result<(TransceiverState, bool)> {
    // Datasheet: TXDACID/TXDACQD are 7-bit unsigned, 0x00..0x7E.
    // 0x3F = zero, 0x00 = min, 0x7E = max. I=max-positive, Q=zero gives
    // a full-amplitude tone at the LO.
    radio.rf09_txdaci.value = radio
        .rf09_txdaci
        .value
        .with_entxdacid(true)
        .with_txdacid(0x7E);
    radio.rf09_txdacq.value = radio
        .rf09_txdacq
        .value
        .with_entxdacqd(true)
        .with_txdacqd(0x3F);
    spi::write_register(dev, &radio.rf09_txdaci)?;
    spi::write_register(dev, &radio.rf09_txdacq)?;

    radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::TxPrep);
    spi::write_register(dev, &radio.rf09_cmd)?;

    let deadline = Instant::now() + Duration::from_millis(5);
    loop {
        spi::read_register(dev, &mut radio.rf09_state)?;
        spi::read_register(dev, &mut radio.rf09_pll)?;
        if radio.rf09_state.value.state() == TransceiverState::TxPrep && radio.rf09_pll.value.ls() {
            break;
        }
        if Instant::now() >= deadline {
            break;
        }
        std::thread::sleep(Duration::from_micros(100));
    }

    radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::Tx);
    spi::write_register(dev, &radio.rf09_cmd)?;

    spi::read_register(dev, &mut radio.rf09_state)?;
    spi::read_register(dev, &mut radio.rf09_pll)?;
    Ok((radio.rf09_state.value.state(), radio.rf09_pll.value.ls()))
}

fn stop_cw(dev: &mut spidev::Spidev, radio: &mut Radio) -> io::Result<()> {
    radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::TrxOff);
    spi::write_register(dev, &radio.rf09_cmd)?;

    radio.rf09_txdaci.value = radio.rf09_txdaci.value.with_entxdacid(false);
    radio.rf09_txdacq.value = radio.rf09_txdacq.value.with_entxdacqd(false);
    spi::write_register(dev, &radio.rf09_txdaci)?;
    spi::write_register(dev, &radio.rf09_txdacq)
}

/// Round-trip through the PLL math so the user sees what the chip actually
/// tunes to (within crystal accuracy). Returns the achievable frequency.
fn realised_freq(target_hz: u64) -> Option<u64> {
    let pll = PllSettings::fine(Band::Sub1GHz, target_hz).ok()?;
    let (base, span) = if target_hz < 600_000_000 {
        (377_000_000u64, 6_500_000u64)
    } else {
        (754_000_000u64, 13_000_000u64)
    };
    let n = ((pll.ccf0 as u64) << 8) | (pll.cn as u64 & 0xFF);
    Some(base + (span * n + 32_768) / 65_536)
}

// ---------------------------------------------------------------------------
// Frame path (2-FSK).
// ---------------------------------------------------------------------------

/// Program the full 2-FSK TX path from staged state.
fn write_config_frame(dev: &mut spidev::Spidev, radio: &mut Radio, s: &State) -> io::Result<()> {
    // Ensure we are out of any CW IqRadioMode left over from a prior session.
    radio.rf_iqifc1.value = radio.rf_iqifc1.value.with_chpm(ChipMode::BasebandMode);
    spi::write_register(dev, &radio.rf_iqifc1)?;

    let pll = PllSettings::fine(Band::Sub1GHz, s.freq_hz).map_err(io::Error::other)?;
    spi::apply_channel_rf09(dev, radio, pll)?;

    // TX DAC clock + direct modulation (FSK requires both DM bits).
    let tx_sr = tx_dfe_sr(s.srate_khz).ok_or_else(|| io::Error::other("bad srate"))?;
    radio.rf09_txdfe.value = radio.rf09_txdfe.value.with_sr(tx_sr).with_dm(true);
    spi::write_register(dev, &radio.rf09_txdfe)?;

    // 2-FSK mod order + index + symbol rate.
    let (midx, midxs, _h) = pick_midx_midxs(s.h);
    radio.bbc0_fskc0.value = radio
        .bbc0_fskc0
        .value
        .with_mord(false)
        .with_midx(midx)
        .with_midxs(midxs)
        .with_bt(0);
    spi::write_register(dev, &radio.bbc0_fskc0)?;
    let srate = srate_field(s.srate_khz).ok_or_else(|| io::Error::other("bad srate"))?;
    radio.bbc0_fskc1.value = radio.bbc0_fskc1.value.with_srate(srate);
    spi::write_register(dev, &radio.bbc0_fskc1)?;
    radio.bbc0_fskdm.value = radio.bbc0_fskdm.value.with_en(true);
    spi::write_register(dev, &radio.bbc0_fskdm)?;

    // Recommended TX-frontend filters (datasheet Table 6-53 / 6-54).
    if let Some((paramp, lpfcut, rcut)) = recommended_tx_filters(s.srate_khz, s.h) {
        radio.rf09_txcutc.value = radio
            .rf09_txcutc
            .value
            .with_paramp(paramp)
            .with_lpfcut(lpfcut);
        spi::write_register(dev, &radio.rf09_txcutc)?;
        radio.rf09_txdfe.value = radio.rf09_txdfe.value.with_rcut(rcut);
        spi::write_register(dev, &radio.rf09_txdfe)?;
    }

    // PSDU data whitening (PN9 scrambler).
    radio.bbc0_fskphrtx.value = radio.bbc0_fskphrtx.value.with_dw(s.whiten);
    spi::write_register(dev, &radio.bbc0_fskphrtx)?;

    // Baseband + auto-FCS, MR-FSK PHY.
    radio.bbc0_pc.value = radio
        .bbc0_pc
        .value
        .with_pt(1)
        .with_bben(true)
        .with_txafcs(true);
    spi::write_register(dev, &radio.bbc0_pc)?;

    // PA power.
    radio.rf09_pac.value = radio
        .rf09_pac
        .value
        .with_txpwr(s.txpwr.min(31))
        .with_pacur(3);
    spi::write_register(dev, &radio.rf09_pac)?;

    Ok(())
}

/// Transmit one frame: TxPrep + PLL lock, load FIFO + length, Tx, poll TXFE.
fn send_frame(dev: &mut spidev::Spidev, radio: &mut Radio, s: &State) -> io::Result<()> {
    let frame = s.frame();
    let frame_len = frame.len();

    radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::TxPrep);
    spi::write_register(dev, &radio.rf09_cmd)?;
    spi::wait_rf09_txprep_locked(dev, radio, Duration::from_millis(5))?;

    spi::write_tx_fifo(dev, Bbc::Bbc0, &frame)?;
    radio.bbc0_txfl.value = BbcnTxfl::new().with_txfl(frame_len as u16);
    spi::write_register(dev, &radio.bbc0_txfl)?;

    radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::Tx);
    spi::write_register(dev, &radio.rf09_cmd)?;

    // Poll BBC0_IRQS.TXFE; OR-accumulate so a transient set isn't lost.
    let deadline = Instant::now() + Duration::from_millis(500);
    let mut acc_bbc_irqs: u8 = 0;
    loop {
        spi::read_register(dev, &mut radio.bbc0_irqs)?;
        acc_bbc_irqs |= u8::from(radio.bbc0_irqs.value);
        if radio.bbc0_irqs.value.txfe() || (acc_bbc_irqs & 0x10) != 0 {
            break;
        }
        if Instant::now() >= deadline {
            return Err(io::Error::other("timed out waiting for TXFE"));
        }
        std::thread::sleep(Duration::from_micros(200));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Mode-dispatching config helper.
// ---------------------------------------------------------------------------

fn write_config(dev: &mut spidev::Spidev, radio: &mut Radio, s: &State) -> io::Result<()> {
    match s.mode {
        Mode::Cw => write_config_cw(dev, radio, s),
        Mode::Frame => write_config_frame(dev, radio, s),
    }
}

// ---------------------------------------------------------------------------
// Rendering.
// ---------------------------------------------------------------------------

fn render(f: &mut Frame, s: &State) {
    let area = f.area();
    let n_control_lines = match s.mode {
        Mode::Cw => 4,    // Mode, Freq, TxPwr, PaCur
        Mode::Frame => 7, // Mode, Freq, TxPwr, Srate, ModIndex, Whiten, Gap
    } as u16;

    let mut constraints = vec![
        Constraint::Length(3),                   // title/state
        Constraint::Length(n_control_lines + 2), // controls
    ];
    if s.mode == Mode::Frame {
        constraints.push(Constraint::Length(3)); // payload
    }
    constraints.push(Constraint::Min(3));        // log
    constraints.push(Constraint::Length(2));     // footer

    let chunks = Layout::vertical(constraints).split(area);

    let mut i = 0;
    render_title(f, chunks[i], s);
    i += 1;
    render_controls(f, chunks[i], s);
    i += 1;
    if s.mode == Mode::Frame {
        render_payload(f, chunks[i], s);
        i += 1;
    }
    render_log(f, chunks[i], s);
    i += 1;
    render_footer(f, chunks[i], s);
}

fn render_title(f: &mut Frame, area: Rect, s: &State) {
    let mode_str = match s.mode {
        Mode::Cw => "CW",
        Mode::Frame => "Frame",
    };
    let state_str = match (s.mode, s.tx_active) {
        (Mode::Cw, true) => Span::styled(
            " TX ON ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Red)
                .add_modifier(Modifier::BOLD),
        ),
        (Mode::Cw, false) => Span::styled(" off ", Style::default().fg(Color::DarkGray)),
        (Mode::Frame, true) => Span::styled(
            " REPEAT ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Red)
                .add_modifier(Modifier::BOLD),
        ),
        (Mode::Frame, false) => Span::styled(" idle ", Style::default().fg(Color::DarkGray)),
    };
    let dirty = if s.dirty {
        Span::styled(" [pending]", Style::default().fg(Color::Yellow))
    } else {
        Span::raw("")
    };
    let title = Line::from(vec![
        Span::styled(
            format!("AT86RF215 {} - RF09 (sub-1GHz)  ", mode_str),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        state_str,
        Span::styled(
            format!("  sent: {}", s.count),
            Style::default().fg(Color::Gray),
        ),
        dirty,
    ]);
    f.render_widget(
        Paragraph::new(title).block(Block::default().borders(Borders::BOTTOM)),
        area,
    );
}

fn render_controls(f: &mut Frame, area: Rect, s: &State) {
    let realised = realised_freq(s.freq_hz);
    let shift_khz = s.h * s.srate_khz as f32;
    let mod_label = if (s.h - 0.5).abs() < 0.01 {
        " (MSK)"
    } else if (s.h - 1.0).abs() < 0.01 {
        " (Sunde 2-FSK)"
    } else {
        ""
    };

    let val = Style::default()
        .fg(Color::Green)
        .add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(Color::DarkGray);

    let value_for = |field: Field| -> Vec<Span<'static>> {
        match field {
            Field::Mode => vec![
                Span::styled(
                    match s.mode {
                        Mode::Cw => "CW (carrier tone)",
                        Mode::Frame => "Frame (2-FSK)",
                    }
                    .to_string(),
                    val,
                ),
                Span::styled("  <-/-> to switch", dim),
            ],
            Field::Freq => vec![
                Span::styled(format!("{:.6}", s.freq_hz as f64 / 1e6), val),
                Span::styled(
                    format!(
                        " MHz  (chip -> {:.6} MHz, step {} Hz)",
                        realised.unwrap_or(s.freq_hz) as f64 / 1e6,
                        s.freq_step
                    ),
                    dim,
                ),
            ],
            Field::TxPwr => vec![
                Span::styled(format!("{}", s.txpwr), val),
                Span::styled(" / 31", dim),
            ],
            Field::PaCur => vec![
                Span::styled(format!("{}", s.pacur), val),
                Span::styled(" / 3   ", dim),
                Span::styled(
                    format!(
                        "({})",
                        match s.pacur {
                            3 => "no gain reduction",
                            2 => "-1 dB",
                            1 => "-2 dB",
                            _ => "-3 dB",
                        }
                    ),
                    dim,
                ),
            ],
            Field::Srate => vec![
                Span::styled(format!("{}", s.srate_khz), val),
                Span::styled(" kHz", dim),
            ],
            Field::ModIndex => vec![
                Span::styled(format!("{:.3}", s.h), val),
                Span::styled(
                    format!("{}  -> {:.1} kHz pk-pk shift", mod_label, shift_khz),
                    dim,
                ),
            ],
            Field::Whiten => vec![Span::styled(
                if s.whiten { "on" } else { "off" }.to_string(),
                val,
            )],
            Field::Gap => vec![
                Span::styled(format!("{}", s.gap_ms), val),
                Span::styled(" ms", dim),
            ],
            Field::Payload => vec![],
        }
    };

    let lines: Vec<Line> = Field::visible(s.mode)
        .iter()
        .filter(|&&field| field != Field::Payload)
        .map(|&field| field_line(s, field, field.label(), value_for(field)))
        .collect();

    f.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(Span::styled(" controls ", Style::default().fg(Color::Cyan))),
        ),
        area,
    );
}

fn field_line<'a>(s: &State, f: Field, label: &'a str, value_spans: Vec<Span<'a>>) -> Line<'a> {
    let selected = s.selected == f;
    let arrow = if selected { "> " } else { "  " };
    let label_style = if selected {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().add_modifier(Modifier::BOLD)
    };
    let mut spans = vec![
        Span::styled(arrow, Style::default().fg(Color::Cyan)),
        Span::styled(format!("{:<13}", label), label_style),
        Span::raw("  "),
    ];
    spans.extend(value_spans);
    Line::from(spans)
}

fn render_payload(f: &mut Frame, area: Rect, s: &State) {
    let selected = s.selected == Field::Payload;
    let border_color = if selected { Color::Cyan } else { Color::DarkGray };

    let line = if let Some(buf) = &s.editing {
        // Editing: show the raw hex buffer with a block cursor.
        Line::from(vec![
            Span::styled("edit: ", Style::default().fg(Color::Yellow)),
            Span::styled(buf.clone(), Style::default().fg(Color::White)),
            Span::styled(
                "_",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::SLOW_BLINK | Modifier::REVERSED),
            ),
        ])
    } else {
        let hex: String = s
            .payload
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ");
        Line::from(vec![
            Span::styled(
                format!("{} bytes  ", s.payload.len()),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(hex, Style::default().fg(Color::Gray)),
        ])
    };

    let title = if selected {
        " payload (e/Enter to edit) "
    } else {
        " payload "
    };
    f.render_widget(
        Paragraph::new(line).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .title(Span::styled(title, Style::default().fg(Color::Cyan))),
        ),
        area,
    );
}

fn render_log(f: &mut Frame, area: Rect, s: &State) {
    let lines: Vec<Line> = s
        .log
        .iter()
        .rev()
        .take(area.height.saturating_sub(2) as usize)
        .rev()
        .map(|m| {
            let style = if m.contains("failed") || m.contains("timed out") {
                Style::default().fg(Color::Red)
            } else if m.starts_with("TX") || m.contains("sent") {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Gray)
            };
            Line::styled(m.as_str(), style)
        })
        .collect();
    f.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(Span::styled(" log ", Style::default().fg(Color::Cyan))),
        ),
        area,
    );
}

fn render_footer(f: &mut Frame, area: Rect, s: &State) {
    let key = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(Color::DarkGray);

    if s.editing.is_some() {
        let l = Line::from(vec![
            Span::styled("0-9 a-f", key),
            Span::styled(" type hex  ", dim),
            Span::styled("Backspace", key),
            Span::styled(" delete  ", dim),
            Span::styled("Enter", key),
            Span::styled(" commit  ", dim),
            Span::styled("Esc", key),
            Span::styled(" cancel", dim),
        ]);
        f.render_widget(Paragraph::new(vec![l, Line::raw("")]), area);
        return;
    }

    let mut l1 = vec![
        Span::styled("up/dn", key),
        Span::styled(" select  ", dim),
        Span::styled("<-/->", key),
        Span::styled(" adjust  ", dim),
        Span::styled("PgUp/Dn", key),
        Span::styled(" coarse  ", dim),
    ];
    match s.mode {
        Mode::Cw => {
            l1.push(Span::styled("Space", key));
            l1.push(Span::styled(" tone  ", dim));
        }
        Mode::Frame => {
            l1.push(Span::styled("t", key));
            l1.push(Span::styled(" send  ", dim));
            l1.push(Span::styled("Space", key));
            l1.push(Span::styled(" repeat  ", dim));
        }
    }
    l1.push(Span::styled("a", key));
    l1.push(Span::styled(" apply", dim));

    let l2 = Line::from(vec![
        Span::styled("q", key),
        Span::styled("/", dim),
        Span::styled("Esc", key),
        Span::styled(" quit", dim),
    ]);
    f.render_widget(Paragraph::new(vec![Line::from(l1), l2]), area);
}

// ---------------------------------------------------------------------------
// Input handling.
// ---------------------------------------------------------------------------

/// Adjust the modulation index to the next/previous achievable h value.
fn nudge_h(h: f32, delta: i64) -> f32 {
    const H_BASE: [f32; 8] = [0.375, 0.5, 0.75, 1.0, 1.25, 1.5, 1.75, 2.0];
    const SCALE: [f32; 4] = [7.0 / 8.0, 1.0, 9.0 / 8.0, 10.0 / 8.0];
    let mut hs: Vec<f32> = H_BASE
        .iter()
        .flat_map(|hb| SCALE.iter().map(move |sc| hb * sc))
        .collect();
    hs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    hs.dedup_by(|a, b| (*a - *b).abs() < 0.001);
    let cur = hs.iter().position(|&x| (x - h).abs() < 0.01).unwrap_or(0) as i64;
    let next = (cur + delta).clamp(0, hs.len() as i64 - 1) as usize;
    hs[next]
}

fn nudge(s: &mut State, delta: i64, coarse: bool) {
    match s.selected {
        Field::Mode => {} // handled in key loop (needs chip side effects)
        Field::Freq => {
            let step = if coarse {
                s.freq_step.saturating_mul(10)
            } else {
                s.freq_step
            };
            let step_signed = step as i64 * delta.signum();
            s.freq_hz = (s.freq_hz as i64 + step_signed).max(389_500_000) as u64;
            s.freq_hz = s.freq_hz.min(1_020_000_000);
            s.dirty = true;
        }
        Field::TxPwr => {
            s.txpwr = (s.txpwr as i32 + delta as i32).clamp(0, 31) as u8;
            s.dirty = true;
        }
        Field::PaCur => {
            s.pacur = (s.pacur as i32 + delta as i32).clamp(0, 3) as u8;
            s.dirty = true;
        }
        Field::Srate => {
            let cur = SRATES_KHZ
                .iter()
                .position(|&r| r == s.srate_khz)
                .unwrap_or(0) as i64;
            let next = (cur + delta).clamp(0, SRATES_KHZ.len() as i64 - 1) as usize;
            s.srate_khz = SRATES_KHZ[next];
            s.dirty = true;
        }
        Field::ModIndex => {
            s.h = nudge_h(s.h, delta);
            s.dirty = true;
        }
        Field::Whiten => {
            s.whiten = !s.whiten;
            s.dirty = true;
        }
        Field::Gap => {
            s.gap_ms = (s.gap_ms as i64 + delta).clamp(0, 10_000) as u64;
            s.dirty = true;
        }
        Field::Payload => {} // edited via the editor
    }
}

/// Switch CW <-> Frame: stop any active TX, change mode, re-apply the
/// mode-appropriate config so the chip and UI stay consistent.
fn switch_mode(dev: &mut spidev::Spidev, radio: &mut Radio, state: &mut State, new_mode: Mode) {
    if new_mode == state.mode {
        return;
    }
    if state.tx_active {
        match state.mode {
            Mode::Cw => {
                let _ = stop_cw(dev, radio);
            }
            Mode::Frame => {}
        }
        state.tx_active = false;
    }
    state.mode = new_mode;
    // Keep the selection on a field that exists in the new mode.
    if !Field::visible(new_mode).contains(&state.selected) {
        state.selected = Field::Mode;
    }
    match write_config(dev, radio, state) {
        Ok(()) => {
            state.dirty = false;
            state.say(format!(
                "mode -> {}",
                match new_mode {
                    Mode::Cw => "CW",
                    Mode::Frame => "Frame",
                }
            ));
        }
        Err(e) => state.say(format!("mode switch apply failed: {}", e)),
    }
}

fn do_start_cw(dev: &mut spidev::Spidev, radio: &mut Radio, state: &mut State) {
    match (|| -> io::Result<(TransceiverState, bool)> {
        write_config_cw(dev, radio, state)?;
        start_cw(dev, radio)
    })() {
        Err(e) => state.say(format!("start failed: {}", e)),
        Ok((trx_state, pll_locked)) => {
            state.tx_active = true;
            state.dirty = false;
            state.say(format!(
                "TX on @ {:.6} MHz, txpwr={} pacur={} state={:?} pll={}",
                state.freq_hz as f64 / 1e6,
                state.txpwr,
                state.pacur,
                trx_state,
                if pll_locked { "locked" } else { "UNLOCKED" },
            ));
            if trx_state != TransceiverState::Tx {
                state.say(format!("warning: requested Tx but chip is in {:?}", trx_state));
            }
        }
    }
}

fn do_stop_cw(dev: &mut spidev::Spidev, radio: &mut Radio, state: &mut State) {
    if !state.tx_active {
        return;
    }
    if let Err(e) = stop_cw(dev, radio) {
        state.say(format!("stop failed: {}", e));
    } else {
        state.tx_active = false;
        state.say("TX off");
    }
}

fn do_send(dev: &mut spidev::Spidev, radio: &mut Radio, state: &mut State) {
    if state.dirty {
        if let Err(e) = write_config_frame(dev, radio, state) {
            state.say(format!("apply failed: {}", e));
            return;
        }
        state.dirty = false;
    }
    match send_frame(dev, radio, state) {
        Ok(()) => {
            state.count += 1;
            state.last_tx = Instant::now();
            if !state.tx_active {
                state.say(format!(
                    "TX: {} bytes sent (#{})",
                    state.payload.len(),
                    state.count
                ));
            }
        }
        Err(e) => {
            state.say(format!("TX failed: {}", e));
            state.tx_active = false;
        }
    }
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    let payload = match &args.payload {
        Some(hex) => parse_hex(hex).map_err(io::Error::other)?,
        None => (0..16).collect(),
    };

    let mut dev = spi::open_with_speed(&args.spi, args.spi_hz)?;
    let mut radio = Radio::new();
    let (pn, vn) = spi::reset_and_identify(&mut dev, &mut radio)?;

    let mut state = State::new(&args, payload);
    state.say(format!("chip: {:?} v{}", pn, vn));
    state.say(format!("SPI: {} @ {} Hz", args.spi, args.spi_hz));

    write_config(&mut dev, &mut radio, &state)?;
    state.dirty = false;
    state.say("config applied");

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut term = Terminal::new(CrosstermBackend::new(stdout))?;

    let res = run_loop(&mut term, &mut dev, &mut radio, &mut state);

    // Clean shutdown: drop any CW tone, then park the transceiver.
    if state.mode == Mode::Cw && state.tx_active {
        let _ = stop_cw(&mut dev, &mut radio);
    }
    radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::TrxOff);
    let _ = spi::write_register(&mut dev, &radio.rf09_cmd);

    disable_raw_mode()?;
    execute!(term.backend_mut(), LeaveAlternateScreen)?;
    term.show_cursor()?;

    res
}

/// Handle a keystroke while editing the payload hex buffer.
fn handle_edit_key(state: &mut State, code: KeyCode) {
    let buf = state.editing.as_mut().unwrap();
    match code {
        KeyCode::Esc => {
            state.editing = None;
            state.say("payload edit cancelled");
        }
        KeyCode::Enter => {
            let text = buf.clone();
            match parse_hex(&text) {
                Ok(bytes) => {
                    state.payload = bytes;
                    state.editing = None;
                    state.dirty = true;
                    state.say(format!("payload set: {} bytes", state.payload.len()));
                }
                Err(e) => state.say(format!("bad hex: {}", e)),
            }
        }
        KeyCode::Backspace => {
            buf.pop();
        }
        KeyCode::Char(c) if c.is_ascii_hexdigit() || c == ' ' => {
            buf.push(c.to_ascii_uppercase());
        }
        _ => {}
    }
}

fn run_loop(
    term: &mut Terminal<CrosstermBackend<Stdout>>,
    dev: &mut spidev::Spidev,
    radio: &mut Radio,
    state: &mut State,
) -> io::Result<()> {
    loop {
        term.draw(|f| render(f, state))?;

        // In frame repeat mode, poll briefly so the loop keeps firing frames.
        let timeout = if state.mode == Mode::Frame && state.tx_active {
            Duration::from_millis(1)
        } else {
            Duration::from_millis(200)
        };

        if event::poll(timeout)?
            && let Event::Key(k) = event::read()?
            && k.kind == KeyEventKind::Press
        {
            if k.modifiers.contains(KeyModifiers::CONTROL) && k.code == KeyCode::Char('c') {
                return Ok(());
            }

            // Payload editor swallows all input while active.
            if state.editing.is_some() {
                handle_edit_key(state, k.code);
            } else {
                match k.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Up => state.selected = state.selected.prev(state.mode),
                    KeyCode::Down => state.selected = state.selected.next(state.mode),
                    KeyCode::Left => {
                        if state.selected == Field::Mode {
                            switch_mode(dev, radio, state, Mode::Cw);
                        } else {
                            nudge(state, -1, false);
                        }
                    }
                    KeyCode::Right => {
                        if state.selected == Field::Mode {
                            switch_mode(dev, radio, state, Mode::Frame);
                        } else {
                            nudge(state, 1, false);
                        }
                    }
                    KeyCode::PageUp => nudge(state, 1, true),
                    KeyCode::PageDown => nudge(state, -1, true),
                    KeyCode::Char('[') => {
                        if state.selected == Field::Freq {
                            state.freq_step = (state.freq_step / 10).max(1);
                        }
                    }
                    KeyCode::Char(']') => {
                        if state.selected == Field::Freq {
                            state.freq_step = (state.freq_step * 10).min(10_000_000);
                        }
                    }
                    KeyCode::Char('e') | KeyCode::Enter
                        if state.mode == Mode::Frame && state.selected == Field::Payload =>
                    {
                        let hex: String = state
                            .payload
                            .iter()
                            .map(|b| format!("{:02X}", b))
                            .collect::<Vec<_>>()
                            .join(" ");
                        state.editing = Some(hex);
                    }
                    KeyCode::Char('t') if state.mode == Mode::Frame => do_send(dev, radio, state),
                    KeyCode::Char(' ') => match state.mode {
                        Mode::Cw => {
                            if state.tx_active {
                                do_stop_cw(dev, radio, state);
                            } else {
                                do_start_cw(dev, radio, state);
                            }
                        }
                        Mode::Frame => {
                            state.tx_active = !state.tx_active;
                            if state.tx_active {
                                state.say("repeat on");
                                state.last_tx =
                                    Instant::now() - Duration::from_millis(state.gap_ms);
                            } else {
                                state.say(format!("repeat off ({} frames)", state.count));
                            }
                        }
                    },
                    KeyCode::Char('a') => apply(dev, radio, state),
                    _ => {}
                }
            }
        }

        if state.mode == Mode::Frame
            && state.tx_active
            && state.last_tx.elapsed() >= Duration::from_millis(state.gap_ms)
        {
            do_send(dev, radio, state);
        }
    }
}

/// Apply the staged settings to the chip. In CW mode with the tone running,
/// this does a stop -> apply -> start re-key so changes take effect live.
fn apply(dev: &mut spidev::Spidev, radio: &mut Radio, state: &mut State) {
    match state.mode {
        Mode::Cw => {
            let was_on = state.tx_active;
            let res = (|| -> io::Result<Option<(TransceiverState, bool)>> {
                if was_on {
                    stop_cw(dev, radio)?;
                }
                write_config_cw(dev, radio, state)?;
                if was_on {
                    Ok(Some(start_cw(dev, radio)?))
                } else {
                    Ok(None)
                }
            })();
            match res {
                Ok(None) => {
                    state.dirty = false;
                    state.say("applied");
                }
                Ok(Some((trx_state, pll_locked))) => {
                    state.dirty = false;
                    state.say(format!(
                        "applied (re-keyed) state={:?} pll={}",
                        trx_state,
                        if pll_locked { "locked" } else { "UNLOCKED" },
                    ));
                }
                Err(e) => state.say(format!("apply failed: {}", e)),
            }
        }
        Mode::Frame => match write_config_frame(dev, radio, state) {
            Ok(()) => {
                state.dirty = false;
                state.say("applied");
            }
            Err(e) => state.say(format!("apply failed: {}", e)),
        },
    }
}
