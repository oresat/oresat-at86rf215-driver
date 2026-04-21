//! Ratatui `Widget` impls for individual AT86RF215 register values.
//!
//! Each register renders itself as a small bordered table. Dashboard layout
//! (how widgets are arranged in a frame) lives in the runner, not here -
//! this module only exposes the pieces.

use crate::registers::*;
use crate::units;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Cell, Row, Table, Widget},
};

/// Highlight a boolean flag: set bits pop, cleared bits dim.
fn onoff(set: bool) -> Style {
    if set {
        Style::default().bg(Color::Gray).fg(Color::Black)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

/// One-row/one-column helper for registers that just display a single value.
fn single_cell(title: &'static str, value: String) -> Table<'static> {
    Table::new(
        vec![Row::new(vec![Cell::from(value)])],
        [Constraint::Min(0)],
    )
    .block(Block::default().borders(Borders::ALL).title(title))
}

impl Widget for RfPn {
    fn render(self, area: Rect, buf: &mut Buffer) {
        single_cell("RF_PN", format!("{:?}", self.pn())).render(area, buf);
    }
}

impl Widget for RfVn {
    fn render(self, area: Rect, buf: &mut Buffer) {
        single_cell("RF_VN", format!("0x{:02X}", self.vn())).render(area, buf);
    }
}

impl Widget for RfCfg {
    fn render(self, area: Rect, buf: &mut Buffer) {
        #[rustfmt::skip]
        let w = Table::new(
            vec![Row::new(vec![
                Cell::from(format!("drv={}", self.drv())),
                Cell::from("irqp" ).style(onoff(self.irqp())),
                Cell::from("irqmm").style(onoff(self.irqmm())),
            ])],
            [Constraint::Length(6), Constraint::Length(5), Constraint::Length(6)],
        )
        .block(Block::default().borders(Borders::ALL).title("RF_CFG"));
        w.render(area, buf);
    }
}

impl Widget for RfnState {
    fn render(self, area: Rect, buf: &mut Buffer) {
        single_cell("RFn_STATE", format!("{:?}", self.state())).render(area, buf);
    }
}

/// Lay out IRQ bits in a fixed order so IRQS and IRQM can be read side-by-side.
fn irq_row<'a>(flags: [(&'a str, bool); 6]) -> Row<'a> {
    Row::new(
        flags
            .into_iter()
            .map(|(name, set)| Cell::from(name).style(onoff(set)))
            .collect::<Vec<_>>(),
    )
}

const IRQ_WIDTHS: [Constraint; 6] = [
    Constraint::Length(6),  // wakeup
    Constraint::Length(6),  // trxrdy
    Constraint::Length(3),  // edc
    Constraint::Length(6),  // batlow
    Constraint::Length(6),  // trxerr
    Constraint::Length(6),  // iqifsf
];

impl Widget for RfnIrqs {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Table::new(
            vec![irq_row([
                ("wakeup", self.wakeup()),
                ("trxrdy", self.trxrdy()),
                ("edc",    self.edc()),
                ("batlow", self.batlow()),
                ("trxerr", self.trxerr()),
                ("iqifsf", self.iqifsf()),
            ])],
            IRQ_WIDTHS,
        )
        .block(Block::default().borders(Borders::ALL).title("RFn_IRQS"))
        .render(area, buf);
    }
}

impl Widget for RfnIrqm {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Table::new(
            vec![irq_row([
                ("wakeup", self.wakeup()),
                ("trxrdy", self.trxrdy()),
                ("edc",    self.edc()),
                ("batlow", self.batlow()),
                ("trxerr", self.trxerr()),
                ("iqifsf", self.iqifsf()),
            ])],
            IRQ_WIDTHS,
        )
        .block(Block::default().borders(Borders::ALL).title("RFn_IRQM"))
        .render(area, buf);
    }
}

impl Widget for RfnCcf0 {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // CCF0 is 25 kHz steps; display the raw count and the resulting MHz.
        let hz = self.ccf0() as u64 * 25_000;
        single_cell(
            "RFn_CCF0",
            format!("{} ({:.3} MHz)", self.ccf0(), hz as f64 / 1e6),
        )
        .render(area, buf);
    }
}

impl Widget for RfnCn {
    fn render(self, area: Rect, buf: &mut Buffer) {
        single_cell(
            "RFn_CN",
            format!("cn={} cm={}", self.cn(), self.cm()),
        )
        .render(area, buf);
    }
}

impl Widget for RfnCs {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let khz = self.cs() as u32 * 25;
        single_cell("RFn_CS", format!("{} ({} kHz)", self.cs(), khz)).render(area, buf);
    }
}

impl Widget for RfnPll {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Table::new(
            vec![Row::new(vec![
                Cell::from("lock").style(onoff(self.ls())),
                Cell::from(format!("lbw={}", self.lbw())),
            ])],
            [Constraint::Length(4), Constraint::Length(7)],
        )
        .block(Block::default().borders(Borders::ALL).title("RFn_PLL"))
        .render(area, buf);
    }
}

impl Widget for RfnEdv {
    fn render(self, area: Rect, buf: &mut Buffer) {
        single_cell("RFn_EDV", format!("{} dBm", self.edv())).render(area, buf);
    }
}

impl Widget for RfnPac {
    fn render(self, area: Rect, buf: &mut Buffer) {
        single_cell(
            "RFn_PAC",
            format!("txpwr={} pacur={}", self.txpwr(), self.pacur()),
        )
        .render(area, buf);
    }
}

// =============================================================================
// Chip-level configuration registers
// =============================================================================

impl Widget for RfClko {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // os: 0=off, 1=26MHz, 2=32MHz, 3=16MHz, 4=8MHz, 5=4MHz, 6=2MHz, 7=1MHz
        let os = match self.os() {
            0 => "off",
            1 => "26 MHz",
            2 => "32 MHz",
            3 => "16 MHz",
            4 => "8 MHz",
            5 => "4 MHz",
            6 => "2 MHz",
            7 => "1 MHz",
            _ => "?",
        };
        // drv: 0=2mA .. 3=8mA in 2mA steps
        let drv_ma = (self.drv() as u16 + 1) * 2;
        single_cell("RF_CLKO", format!("{} drv={}mA", os, drv_ma)).render(area, buf);
    }
}

impl Widget for RfBmdvc {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let range = if self.bmr() { "2.0-3.6V" } else { "1.7-3.0V" };
        Table::new(
            vec![Row::new(vec![
                Cell::from("en").style(onoff(self.bmen())),
                Cell::from(format!("th={}", self.bmth())),
                Cell::from(range),
            ])],
            [
                Constraint::Length(2),
                Constraint::Length(5),
                Constraint::Length(9),
            ],
        )
        .block(Block::default().borders(Borders::ALL).title("RF_BMDVC"))
        .render(area, buf);
    }
}

impl Widget for RfXoc {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // fs is reserved beyond 0; print as-is.
        single_cell("RF_XOC", format!("trim={} fs={}", self.trim(), self.fs())).render(area, buf);
    }
}

impl Widget for RfIqifc0 {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // cmv: 0=150mV, 1=200mV, 2=250mV, 3=300mV
        let cmv_mv = 150 + (self.cmv() as u16) * 50;
        // drv: 0=1mA .. 3=4mA
        let drv_ma = self.drv() as u16 + 1;
        Table::new(
            vec![Row::new(vec![
                Cell::from(format!("cmv={}mV", cmv_mv)),
                Cell::from(format!("drv={}mA", drv_ma)),
                Cell::from("cmv1v2").style(onoff(self.cmv1v2())),
                Cell::from("eec").style(onoff(self.eec())),
                Cell::from("extlb").style(onoff(self.extlb())),
                Cell::from("sf").style(onoff(self.sf())),
            ])],
            [
                Constraint::Length(9),
                Constraint::Length(8),
                Constraint::Length(6),
                Constraint::Length(3),
                Constraint::Length(5),
                Constraint::Length(2),
            ],
        )
        .block(Block::default().borders(Borders::ALL).title("RF_IQIFC0"))
        .render(area, buf);
    }
}

impl Widget for RfIqifc1 {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Table::new(
            vec![Row::new(vec![
                Cell::from(format!("chpm={:?}", self.chpm())),
                Cell::from(format!("skew={}", self.skewdrv())),
                Cell::from("failsf").style(onoff(self.failsf())),
            ])],
            [
                Constraint::Min(14),
                Constraint::Length(7),
                Constraint::Length(6),
            ],
        )
        .block(Block::default().borders(Borders::ALL).title("RF_IQIFC1"))
        .render(area, buf);
    }
}

impl Widget for RfIqifc2 {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Table::new(
            vec![Row::new(vec![Cell::from("sync").style(onoff(self.sync()))])],
            [Constraint::Length(4)],
        )
        .block(Block::default().borders(Borders::ALL).title("RF_IQIFC2"))
        .render(area, buf);
    }
}

// =============================================================================
// Radio analog / RX / TX path
// =============================================================================

impl Widget for RfnAuxs {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // pavc: 0=2.0V, 1=2.2V, 2=2.4V
        let pavc_v = 2.0 + (self.pavc() as f32) * 0.2;
        Table::new(
            vec![Row::new(vec![
                Cell::from(format!("pavc={:.1}V", pavc_v)),
                Cell::from("ave").style(onoff(self.ave())),
                Cell::from("aven").style(onoff(self.aven())),
                Cell::from("agcmap").style(onoff(self.agcmap())),
                Cell::from("extlnabyp").style(onoff(self.extlnabyp())),
            ])],
            [
                Constraint::Length(9),
                Constraint::Length(3),
                Constraint::Length(4),
                Constraint::Length(6),
                Constraint::Length(9),
            ],
        )
        .block(Block::default().borders(Borders::ALL).title("RFn_AUXS"))
        .render(area, buf);
    }
}

impl Widget for RfnRxbwc {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let bw = self.bw();
        let bw_label = units::rxbwc_khz(bw)
            .map(|k| format!("{} kHz", k))
            .unwrap_or_else(|| format!("?({})", bw));
        Table::new(
            vec![Row::new(vec![
                Cell::from(bw_label),
                Cell::from("ifs").style(onoff(self.ifs())),
                Cell::from("ifi").style(onoff(self.ifi())),
            ])],
            [
                Constraint::Length(10),
                Constraint::Length(3),
                Constraint::Length(3),
            ],
        )
        .block(Block::default().borders(Borders::ALL).title("RFn_RXBWC"))
        .render(area, buf);
    }
}

fn fmt_sr(sr: u8) -> String {
    match units::dfe_sr_khz(sr) {
        Some(k) => format!("{} kHz", k),
        None => format!("?({})", sr),
    }
}

impl Widget for RfnRxdfe {
    fn render(self, area: Rect, buf: &mut Buffer) {
        single_cell(
            "RFn_RXDFE",
            format!("sr={} rcut={}", fmt_sr(self.sr()), self.rcut()),
        )
        .render(area, buf);
    }
}

impl Widget for RfnAgcc {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let avg_n = units::agcc_avg_samples(self.avgs());
        Table::new(
            vec![Row::new(vec![
                Cell::from("en").style(onoff(self.en())),
                Cell::from("frzc").style(onoff(self.frzc())),
                Cell::from("frzs").style(onoff(self.frzs())),
                Cell::from("rst").style(onoff(self.rst())),
                Cell::from(format!("avg={}", avg_n)),
                Cell::from("agci").style(onoff(self.agci())),
            ])],
            [
                Constraint::Length(2),
                Constraint::Length(4),
                Constraint::Length(4),
                Constraint::Length(3),
                Constraint::Length(7),
                Constraint::Length(4),
            ],
        )
        .block(Block::default().borders(Borders::ALL).title("RFn_AGCC"))
        .render(area, buf);
    }
}

impl Widget for RfnAgcs {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let tgt_dbfs = units::agcs_target_dbfs(self.tgt());
        single_cell(
            "RFn_AGCS",
            format!("gcw={} tgt={}dB", self.gcw(), tgt_dbfs),
        )
        .render(area, buf);
    }
}

impl Widget for RfnRssi {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // 127 indicates invalid per datasheet.
        let v = self.rssi();
        let label = if v == 127 {
            "invalid".to_string()
        } else {
            format!("{} dBm", v)
        };
        single_cell("RFn_RSSI", label).render(area, buf);
    }
}

impl Widget for RfnEdc {
    fn render(self, area: Rect, buf: &mut Buffer) {
        single_cell("RFn_EDC", format!("{:?}", self.edm())).render(area, buf);
    }
}

impl Widget for RfnEdd {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let dtb_us = 2u32 << (2 * self.dtb() as u32);
        let total_us = units::edd_us(self.dtb(), self.df());
        single_cell(
            "RFn_EDD",
            format!("df={} dtb={}us total={}us", self.df(), dtb_us, total_us),
        )
        .render(area, buf);
    }
}

impl Widget for RfnPllcf {
    fn render(self, area: Rect, buf: &mut Buffer) {
        single_cell("RFn_PLLCF", format!("cf={}", self.cf())).render(area, buf);
    }
}

impl Widget for RfnTxcutc {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let lpf = self.lpfcut();
        let lpf_label = units::txcutc_khz(lpf)
            .map(|k| format!("lpf={}kHz", k))
            .unwrap_or_else(|| format!("lpf=?({})", lpf));
        // paramp: 0=4us, 1=8us, 2=16us, 3=32us
        let paramp_us = 4u16 << self.paramp();
        single_cell(
            "RFn_TXCUTC",
            format!("{} paramp={}us", lpf_label, paramp_us),
        )
        .render(area, buf);
    }
}

impl Widget for RfnTxdfe {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Table::new(
            vec![Row::new(vec![
                Cell::from(format!("sr={}", fmt_sr(self.sr()))),
                Cell::from("dm").style(onoff(self.dm())),
                Cell::from(format!("rcut={}", self.rcut())),
            ])],
            [
                Constraint::Length(13),
                Constraint::Length(2),
                Constraint::Length(7),
            ],
        )
        .block(Block::default().borders(Borders::ALL).title("RFn_TXDFE"))
        .render(area, buf);
    }
}

// =============================================================================
// Baseband (BBCn) registers
// =============================================================================

impl Widget for BbcnPc {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // pt: 0=FSK, 1=OFDM, 2=OQPSK, 3=Legacy O-QPSK
        let pt = match self.pt() {
            0 => "FSK",
            1 => "OFDM",
            2 => "OQPSK",
            3 => "OQPSK-leg",
            _ => "?",
        };
        let fcs = if self.fcst() { "32-bit" } else { "16-bit" };
        Table::new(
            vec![Row::new(vec![
                Cell::from(pt),
                Cell::from("bben").style(onoff(self.bben())),
                Cell::from(format!("fcs={}", fcs)),
                Cell::from("txafcs").style(onoff(self.txafcs())),
                Cell::from("fcsfe").style(onoff(self.fcsfe())),
                Cell::from("fcsok").style(onoff(self.fcsok())),
                Cell::from("ctx").style(onoff(self.ctx())),
            ])],
            [
                Constraint::Length(9),
                Constraint::Length(4),
                Constraint::Length(10),
                Constraint::Length(6),
                Constraint::Length(5),
                Constraint::Length(5),
                Constraint::Length(3),
            ],
        )
        .block(Block::default().borders(Borders::ALL).title("BBCn_PC"))
        .render(area, buf);
    }
}

impl Widget for BbcnPs {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Table::new(
            vec![Row::new(vec![Cell::from("txur").style(onoff(self.txur()))])],
            [Constraint::Length(4)],
        )
        .block(Block::default().borders(Borders::ALL).title("BBCn_PS"))
        .render(area, buf);
    }
}

/// Lay out the 8 baseband IRQ bits in a fixed order so IRQS and IRQM line up.
fn bb_irq_row<'a>(flags: [(&'a str, bool); 8]) -> Row<'a> {
    Row::new(
        flags
            .into_iter()
            .map(|(name, set)| Cell::from(name).style(onoff(set)))
            .collect::<Vec<_>>(),
    )
}

const BB_IRQ_WIDTHS: [Constraint; 8] = [
    Constraint::Length(4), // rxfs
    Constraint::Length(4), // rxfe
    Constraint::Length(4), // rxam
    Constraint::Length(4), // rxem
    Constraint::Length(4), // txfe
    Constraint::Length(4), // agch
    Constraint::Length(4), // agcr
    Constraint::Length(4), // fbli
];

impl Widget for BbcnIrqs {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Table::new(
            vec![bb_irq_row([
                ("rxfs", self.rxfs()),
                ("rxfe", self.rxfe()),
                ("rxam", self.rxam()),
                ("rxem", self.rxem()),
                ("txfe", self.txfe()),
                ("agch", self.agch()),
                ("agcr", self.agcr()),
                ("fbli", self.fbli()),
            ])],
            BB_IRQ_WIDTHS,
        )
        .block(Block::default().borders(Borders::ALL).title("BBCn_IRQS"))
        .render(area, buf);
    }
}

impl Widget for BbcnIrqm {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Table::new(
            vec![bb_irq_row([
                ("rxfs", self.rxfs()),
                ("rxfe", self.rxfe()),
                ("rxam", self.rxam()),
                ("rxem", self.rxem()),
                ("txfe", self.txfe()),
                ("agch", self.agch()),
                ("agcr", self.agcr()),
                ("fbli", self.fbli()),
            ])],
            BB_IRQ_WIDTHS,
        )
        .block(Block::default().borders(Borders::ALL).title("BBCn_IRQM"))
        .render(area, buf);
    }
}

impl Widget for BbcnRxfl {
    fn render(self, area: Rect, buf: &mut Buffer) {
        single_cell("BBCn_RXFL", format!("{} B", self.rxfl())).render(area, buf);
    }
}

impl Widget for BbcnTxfl {
    fn render(self, area: Rect, buf: &mut Buffer) {
        single_cell("BBCn_TXFL", format!("{} B", self.txfl())).render(area, buf);
    }
}

impl Widget for BbcnFbl {
    fn render(self, area: Rect, buf: &mut Buffer) {
        single_cell("BBCn_FBL", format!("{} B", self.fbl())).render(area, buf);
    }
}

impl Widget for BbcnFbli {
    fn render(self, area: Rect, buf: &mut Buffer) {
        single_cell("BBCn_FBLI", format!("{} B", self.fbli())).render(area, buf);
    }
}

impl Widget for BbcnCnt {
    fn render(self, area: Rect, buf: &mut Buffer) {
        single_cell("BBCn_CNT", format!("{}", self.cnt())).render(area, buf);
    }
}

impl Widget for BbcnCntc {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Table::new(
            vec![Row::new(vec![
                Cell::from("en").style(onoff(self.en())),
                Cell::from("rstrxs").style(onoff(self.rstrxs())),
                Cell::from("rsttxs").style(onoff(self.rsttxs())),
                Cell::from("caprxs").style(onoff(self.caprxs())),
                Cell::from("captxs").style(onoff(self.captxs())),
            ])],
            [
                Constraint::Length(2),
                Constraint::Length(6),
                Constraint::Length(6),
                Constraint::Length(6),
                Constraint::Length(6),
            ],
        )
        .block(Block::default().borders(Borders::ALL).title("BBCn_CNTC"))
        .render(area, buf);
    }
}

impl Widget for BbcnOfdmc {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // opt: 0=Opt1, 1=Opt2, 2=Opt3, 3=Opt4
        Table::new(
            vec![Row::new(vec![
                Cell::from(format!("opt={}", self.opt() + 1)),
                Cell::from("poi").style(onoff(self.poi())),
                Cell::from("lfo").style(onoff(self.lfo())),
                Cell::from(format!("sstx={}", self.sstx())),
                Cell::from(format!("ssrx={}", self.ssrx())),
            ])],
            [
                Constraint::Length(5),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(7),
                Constraint::Length(7),
            ],
        )
        .block(Block::default().borders(Borders::ALL).title("BBCn_OFDMC"))
        .render(area, buf);
    }
}

impl Widget for BbcnOqpskc0 {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // fchip: 0=100, 1=200, 2=1000, 3=2000 kchip/s
        let fchip = match self.fchip() {
            0 => "100k",
            1 => "200k",
            2 => "1M",
            3 => "2M",
            _ => "?",
        };
        let mod_kind = if self.mod_() { "RRC-0.8" } else { "RC-0.8" };
        Table::new(
            vec![Row::new(vec![
                Cell::from(format!("fchip={}cps", fchip)),
                Cell::from(format!("mod={}", mod_kind)),
                Cell::from("dm").style(onoff(self.dm())),
            ])],
            [
                Constraint::Length(13),
                Constraint::Length(12),
                Constraint::Length(2),
            ],
        )
        .block(Block::default().borders(Borders::ALL).title("BBCn_OQPSKC0"))
        .render(area, buf);
    }
}

impl Widget for BbcnFskc0 {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let order = if self.mord() { "4FSK" } else { "2FSK" };
        single_cell(
            "BBCn_FSKC0",
            format!(
                "{} midx={} midxs={} bt={}",
                order,
                self.midx(),
                self.midxs(),
                self.bt()
            ),
        )
        .render(area, buf);
    }
}

impl Widget for BbcnPmuval {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let raw = self.pmuval();
        let deg = units::pmuval_degrees(raw);
        single_cell("BBCn_PMUVAL", format!("{} ({:.1}°)", raw, deg)).render(area, buf);
    }
}

// =============================================================================
// Remaining RF registers
// =============================================================================

impl Widget for RfnRndv {
    fn render(self, area: Rect, buf: &mut Buffer) {
        single_cell("RFn_RNDV", format!("0x{:02X}", self.rndv())).render(area, buf);
    }
}

impl Widget for RfnPadfe {
    fn render(self, area: Rect, buf: &mut Buffer) {
        single_cell("RFn_PADFE", format!("padfe={}", self.padfe() as u8)).render(area, buf);
    }
}

impl Widget for RfnTxci {
    fn render(self, area: Rect, buf: &mut Buffer) {
        single_cell("RFn_TXCI", format!("dcoi={}", self.dcoi())).render(area, buf);
    }
}

impl Widget for RfnTxcq {
    fn render(self, area: Rect, buf: &mut Buffer) {
        single_cell("RFn_TXCQ", format!("dcoq={}", self.dcoq())).render(area, buf);
    }
}

impl Widget for RfnTxdaci {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Table::new(
            vec![Row::new(vec![
                Cell::from("en").style(onoff(self.entxdacid())),
                Cell::from(format!("val={}", self.txdacid())),
            ])],
            [Constraint::Length(2), Constraint::Length(8)],
        )
        .block(Block::default().borders(Borders::ALL).title("RFn_TXDACI"))
        .render(area, buf);
    }
}

impl Widget for RfnTxdacq {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Table::new(
            vec![Row::new(vec![
                Cell::from("en").style(onoff(self.entxdacqd())),
                Cell::from(format!("val={}", self.txdacqd())),
            ])],
            [Constraint::Length(2), Constraint::Length(8)],
        )
        .block(Block::default().borders(Borders::ALL).title("RFn_TXDACQ"))
        .render(area, buf);
    }
}

// =============================================================================
// OFDM PHR + switch registers
// =============================================================================

impl Widget for BbcnOfdmphrtx {
    fn render(self, area: Rect, buf: &mut Buffer) {
        single_cell("BBCn_OFDMPHRTX", format!("mcs={}", self.mcs())).render(area, buf);
    }
}

impl Widget for BbcnOfdmphrrx {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Table::new(
            vec![Row::new(vec![
                Cell::from(format!("mcs={}", self.mcs())),
                Cell::from("spc").style(onoff(self.spc())),
            ])],
            [Constraint::Length(5), Constraint::Length(3)],
        )
        .block(Block::default().borders(Borders::ALL).title("BBCn_OFDMPHRRX"))
        .render(area, buf);
    }
}

impl Widget for BbcnOfdmsw {
    fn render(self, area: Rect, buf: &mut Buffer) {
        single_cell(
            "BBCn_OFDMSW",
            format!("rxo={} pdt={}", self.rxo(), self.pdt()),
        )
        .render(area, buf);
    }
}

// =============================================================================
// OQPSK extended config + PHR
// =============================================================================

impl Widget for BbcnOqpskc1 {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Table::new(
            vec![Row::new(vec![
                Cell::from(format!("pdt0={}", self.pdt0())),
                Cell::from(format!("pdt1={}", self.pdt1())),
                Cell::from("rxoleg").style(onoff(self.rxoleg())),
                Cell::from("rxo").style(onoff(self.rxo())),
            ])],
            [
                Constraint::Length(6),
                Constraint::Length(6),
                Constraint::Length(6),
                Constraint::Length(3),
            ],
        )
        .block(Block::default().borders(Borders::ALL).title("BBCn_OQPSKC1"))
        .render(area, buf);
    }
}

impl Widget for BbcnOqpskc2 {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Table::new(
            vec![Row::new(vec![
                Cell::from(format!("rxm={}", self.rxm())),
                Cell::from("fcstleg").style(onoff(self.fcstleg())),
                Cell::from("enprop").style(onoff(self.enprop())),
                Cell::from("rpc").style(onoff(self.rpc())),
                Cell::from("spc").style(onoff(self.spc())),
            ])],
            [
                Constraint::Length(5),
                Constraint::Length(7),
                Constraint::Length(6),
                Constraint::Length(3),
                Constraint::Length(3),
            ],
        )
        .block(Block::default().borders(Borders::ALL).title("BBCn_OQPSKC2"))
        .render(area, buf);
    }
}

impl Widget for BbcnOqpskc3 {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Table::new(
            vec![Row::new(vec![
                Cell::from(format!("nsfd={}", self.nsfd())),
                Cell::from("hrleg").style(onoff(self.hrleg())),
            ])],
            [Constraint::Length(6), Constraint::Length(5)],
        )
        .block(Block::default().borders(Borders::ALL).title("BBCn_OQPSKC3"))
        .render(area, buf);
    }
}

impl Widget for BbcnOqpskphrtx {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Table::new(
            vec![Row::new(vec![
                Cell::from("leg").style(onoff(self.leg())),
                Cell::from(format!("mod={}", self.mod_())),
                Cell::from("ppdut").style(onoff(self.ppdut())),
            ])],
            [
                Constraint::Length(3),
                Constraint::Length(5),
                Constraint::Length(5),
            ],
        )
        .block(Block::default().borders(Borders::ALL).title("BBCn_OQPSKPHRTX"))
        .render(area, buf);
    }
}

impl Widget for BbcnOqpskphrrx {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Table::new(
            vec![Row::new(vec![
                Cell::from("leg").style(onoff(self.leg())),
                Cell::from(format!("mod={}", self.mod_())),
                Cell::from("ppdut").style(onoff(self.ppdut())),
            ])],
            [
                Constraint::Length(3),
                Constraint::Length(5),
                Constraint::Length(5),
            ],
        )
        .block(Block::default().borders(Borders::ALL).title("BBCn_OQPSKPHRRX"))
        .render(area, buf);
    }
}

// =============================================================================
// Address filtering / MAC registers
// =============================================================================

impl Widget for BbcnAfc0 {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Table::new(
            vec![Row::new(vec![
                Cell::from("af0").style(onoff(self.afen0())),
                Cell::from("af1").style(onoff(self.afen1())),
                Cell::from("af2").style(onoff(self.afen2())),
                Cell::from("af3").style(onoff(self.afen3())),
                Cell::from("pm").style(onoff(self.pm())),
            ])],
            [
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(2),
            ],
        )
        .block(Block::default().borders(Borders::ALL).title("BBCn_AFC0"))
        .render(area, buf);
    }
}

impl Widget for BbcnAfc1 {
    fn render(self, area: Rect, buf: &mut Buffer) {
        single_cell(
            "BBCn_AFC1",
            format!("panc=0x{:X} mrft=0x{:X}", self.panc(), self.mrft()),
        )
        .render(area, buf);
    }
}

impl Widget for BbcnAfftm {
    fn render(self, area: Rect, buf: &mut Buffer) {
        single_cell("BBCn_AFFTM", format!("0x{:02X}", self.afftm())).render(area, buf);
    }
}

impl Widget for BbcnAffvm {
    fn render(self, area: Rect, buf: &mut Buffer) {
        single_cell("BBCn_AFFVM", format!("0x{:X}", self.affvm())).render(area, buf);
    }
}

impl Widget for BbcnAfs {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Table::new(
            vec![Row::new(vec![
                Cell::from("am0").style(onoff(self.am0())),
                Cell::from("am1").style(onoff(self.am1())),
                Cell::from("am2").style(onoff(self.am2())),
                Cell::from("am3").style(onoff(self.am3())),
                Cell::from("em").style(onoff(self.em())),
            ])],
            [
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(2),
            ],
        )
        .block(Block::default().borders(Borders::ALL).title("BBCn_AFS"))
        .render(area, buf);
    }
}

impl Widget for BbcnMacea {
    fn render(self, area: Rect, buf: &mut Buffer) {
        single_cell("BBCn_MACEA", format!("0x{:016X}", self.macea())).render(area, buf);
    }
}

impl Widget for BbcnMacpid {
    fn render(self, area: Rect, buf: &mut Buffer) {
        single_cell("BBCn_MACPID", format!("0x{:04X}", self.macpid())).render(area, buf);
    }
}

impl Widget for BbcnMacsha {
    fn render(self, area: Rect, buf: &mut Buffer) {
        single_cell("BBCn_MACSHA", format!("0x{:04X}", self.macsha())).render(area, buf);
    }
}

// =============================================================================
// Auto-ACK / AMCS registers
// =============================================================================

impl Widget for BbcnAmcs {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Table::new(
            vec![Row::new(vec![
                Cell::from("tx2rx").style(onoff(self.tx2rx())),
                Cell::from("ccatx").style(onoff(self.ccatx())),
                Cell::from("ccaed").style(onoff(self.ccaed())),
                Cell::from("aack").style(onoff(self.aack())),
                Cell::from("aacks").style(onoff(self.aacks())),
                Cell::from("aackdr").style(onoff(self.aackdr())),
                Cell::from("aackfa").style(onoff(self.aackfa())),
                Cell::from("aackft").style(onoff(self.aackft())),
            ])],
            [
                Constraint::Length(5),
                Constraint::Length(5),
                Constraint::Length(5),
                Constraint::Length(4),
                Constraint::Length(5),
                Constraint::Length(6),
                Constraint::Length(6),
                Constraint::Length(6),
            ],
        )
        .block(Block::default().borders(Borders::ALL).title("BBCn_AMCS"))
        .render(area, buf);
    }
}

impl Widget for BbcnAmedt {
    fn render(self, area: Rect, buf: &mut Buffer) {
        single_cell("BBCn_AMEDT", format!("{} dBm", self.amedt() as i8)).render(area, buf);
    }
}

impl Widget for BbcnAmaackpd {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Table::new(
            vec![Row::new(vec![
                Cell::from("pd0").style(onoff(self.pd0())),
                Cell::from("pd1").style(onoff(self.pd1())),
                Cell::from("pd2").style(onoff(self.pd2())),
                Cell::from("pd3").style(onoff(self.pd3())),
            ])],
            [
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
            ],
        )
        .block(Block::default().borders(Borders::ALL).title("BBCn_AMAACKPD"))
        .render(area, buf);
    }
}

impl Widget for BbcnAmaackt {
    fn render(self, area: Rect, buf: &mut Buffer) {
        single_cell("BBCn_AMAACKT", format!("{} µs", self.amaackt())).render(area, buf);
    }
}

// =============================================================================
// FSK extended config registers
// =============================================================================

impl Widget for BbcnFskc1 {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Table::new(
            vec![Row::new(vec![
                Cell::from(format!("srate={}", self.srate())),
                Cell::from("fi").style(onoff(self.fi())),
                Cell::from(format!("fskplh={}", self.fskplh())),
            ])],
            [
                Constraint::Length(8),
                Constraint::Length(2),
                Constraint::Length(9),
            ],
        )
        .block(Block::default().borders(Borders::ALL).title("BBCn_FSKC1"))
        .render(area, buf);
    }
}

impl Widget for BbcnFskc2 {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Table::new(
            vec![Row::new(vec![
                Cell::from("fecie").style(onoff(self.fecie())),
                Cell::from("fecs").style(onoff(self.fecs())),
                Cell::from("pri").style(onoff(self.pri())),
                Cell::from("mse").style(onoff(self.mse())),
                Cell::from("rxpto").style(onoff(self.rxpto())),
                Cell::from(format!("rxo={}", self.rxo())),
                Cell::from("pdtm").style(onoff(self.pdtm())),
            ])],
            [
                Constraint::Length(5),
                Constraint::Length(4),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(5),
                Constraint::Length(5),
                Constraint::Length(4),
            ],
        )
        .block(Block::default().borders(Borders::ALL).title("BBCn_FSKC2"))
        .render(area, buf);
    }
}

impl Widget for BbcnFskc3 {
    fn render(self, area: Rect, buf: &mut Buffer) {
        single_cell(
            "BBCn_FSKC3",
            format!("pdt={} sfdt={}", self.pdt(), self.sfdt()),
        )
        .render(area, buf);
    }
}

impl Widget for BbcnFskc4 {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Table::new(
            vec![Row::new(vec![
                Cell::from(format!("csfd0={}", self.csfd0())),
                Cell::from(format!("csfd1={}", self.csfd1())),
                Cell::from("rawrbit").style(onoff(self.rawrbit())),
                Cell::from("sfd32").style(onoff(self.sfd32())),
                Cell::from("sfdq").style(onoff(self.sfdq())),
            ])],
            [
                Constraint::Length(7),
                Constraint::Length(7),
                Constraint::Length(7),
                Constraint::Length(5),
                Constraint::Length(4),
            ],
        )
        .block(Block::default().borders(Borders::ALL).title("BBCn_FSKC4"))
        .render(area, buf);
    }
}

impl Widget for BbcnFskpll {
    fn render(self, area: Rect, buf: &mut Buffer) {
        single_cell("BBCn_FSKPLL", format!("0x{:02X}", self.fskpll())).render(area, buf);
    }
}

impl Widget for BbcnFsksfd {
    fn render(self, area: Rect, buf: &mut Buffer) {
        single_cell("BBCn_FSKSFD", format!("0x{:04X}", self.fsksfd())).render(area, buf);
    }
}

impl Widget for BbcnFskphrtx {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Table::new(
            vec![Row::new(vec![
                Cell::from("dw").style(onoff(self.dw())),
                Cell::from("sfd").style(onoff(self.sfd())),
            ])],
            [Constraint::Length(2), Constraint::Length(3)],
        )
        .block(Block::default().borders(Borders::ALL).title("BBCn_FSKPHRTX"))
        .render(area, buf);
    }
}

impl Widget for BbcnFskphrrx {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Table::new(
            vec![Row::new(vec![
                Cell::from("dw").style(onoff(self.dw())),
                Cell::from("sfd").style(onoff(self.sfd())),
                Cell::from("ms").style(onoff(self.ms())),
                Cell::from("fcst").style(onoff(self.fcst())),
            ])],
            [
                Constraint::Length(2),
                Constraint::Length(3),
                Constraint::Length(2),
                Constraint::Length(4),
            ],
        )
        .block(Block::default().borders(Borders::ALL).title("BBCn_FSKPHRRX"))
        .render(area, buf);
    }
}

impl Widget for BbcnFskrpc {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Table::new(
            vec![Row::new(vec![
                Cell::from("en").style(onoff(self.en())),
                Cell::from(format!("baset={}", self.baset())),
            ])],
            [Constraint::Length(2), Constraint::Length(8)],
        )
        .block(Block::default().borders(Borders::ALL).title("BBCn_FSKRPC"))
        .render(area, buf);
    }
}

impl Widget for BbcnFskrpcont {
    fn render(self, area: Rect, buf: &mut Buffer) {
        single_cell("BBCn_FSKRPCONT", format!("0x{:02X}", self.fskrpcont())).render(area, buf);
    }
}

impl Widget for BbcnFskrpcofft {
    fn render(self, area: Rect, buf: &mut Buffer) {
        single_cell("BBCn_FSKRPCOFFT", format!("0x{:02X}", self.fskrpcofft())).render(area, buf);
    }
}

impl Widget for BbcnFskrrxfl {
    fn render(self, area: Rect, buf: &mut Buffer) {
        single_cell("BBCn_FSKRRXFL", format!("{} B", self.fskrrxfl())).render(area, buf);
    }
}

impl Widget for BbcnFskdm {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Table::new(
            vec![Row::new(vec![
                Cell::from("en").style(onoff(self.en())),
                Cell::from("pe").style(onoff(self.pe())),
            ])],
            [Constraint::Length(2), Constraint::Length(2)],
        )
        .block(Block::default().borders(Borders::ALL).title("BBCn_FSKDM"))
        .render(area, buf);
    }
}

impl Widget for BbcnFskpe {
    fn render(self, area: Rect, buf: &mut Buffer) {
        single_cell("BBCn_FSKPE", format!("0x{:02X}", self.fskpe())).render(area, buf);
    }
}

// =============================================================================
// PMU remaining registers
// =============================================================================

impl Widget for BbcnPmuc {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Table::new(
            vec![Row::new(vec![
                Cell::from("en").style(onoff(self.en())),
                Cell::from("avg").style(onoff(self.avg())),
                Cell::from(format!("sync={}", self.sync())),
                Cell::from("fed").style(onoff(self.fed())),
                Cell::from("iqsel").style(onoff(self.iqsel())),
                Cell::from("ccfts").style(onoff(self.ccfts())),
            ])],
            [
                Constraint::Length(2),
                Constraint::Length(3),
                Constraint::Length(7),
                Constraint::Length(3),
                Constraint::Length(5),
                Constraint::Length(5),
            ],
        )
        .block(Block::default().borders(Borders::ALL).title("BBCn_PMUC"))
        .render(area, buf);
    }
}

impl Widget for BbcnPmuqf {
    fn render(self, area: Rect, buf: &mut Buffer) {
        single_cell("BBCn_PMUQF", format!("{}", self.pmuqf())).render(area, buf);
    }
}

impl Widget for BbcnPmui {
    fn render(self, area: Rect, buf: &mut Buffer) {
        single_cell("BBCn_PMUI", format!("{}", self.pmui())).render(area, buf);
    }
}

impl Widget for BbcnPmuq {
    fn render(self, area: Rect, buf: &mut Buffer) {
        single_cell("BBCn_PMUQ", format!("{}", self.pmuq())).render(area, buf);
    }
}
