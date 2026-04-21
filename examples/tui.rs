use std::{
    io::{self, Stdout},
    time::Duration,
};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use oresat_at86rf215_driver::{
    freq::{Band, PllSettings},
    radio::Radio,
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Paragraph, Widget},
};

/// Active dashboard tab
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Rf09,
    Bbc0Phy,
    Bbc0Mac,
}

impl Tab {
    fn label(self) -> &'static str {
        match self {
            Tab::Rf09 => "1:RF09",
            Tab::Bbc0Phy => "2:BBC0 PHY",
            Tab::Bbc0Mac => "3:BBC0 MAC/PMU",
        }
    }

    fn next(self) -> Self {
        match self {
            Tab::Rf09 => Tab::Bbc0Phy,
            Tab::Bbc0Phy => Tab::Bbc0Mac,
            Tab::Bbc0Mac => Tab::Rf09,
        }
    }

    fn prev(self) -> Self {
        match self {
            Tab::Rf09 => Tab::Bbc0Mac,
            Tab::Bbc0Phy => Tab::Rf09,
            Tab::Bbc0Mac => Tab::Bbc0Phy,
        }
    }
}


struct Dashboard<'a> {
    radio: &'a Radio,
    tab: Tab,
}

impl Widget for Dashboard<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Tab bar at top and footer at bottom
        let outer = Layout::vertical([
            Constraint::Length(1), // tab bar
            Constraint::Min(0),   // content
            Constraint::Length(1), // footer
        ])
        .split(area);

        render_tab_bar(self.tab, outer[0], buf);

        match self.tab {
            Tab::Rf09 => render_rf09(self.radio, outer[1], buf),
            Tab::Bbc0Phy => render_bbc0_phy(self.radio, outer[1], buf),
            Tab::Bbc0Mac => render_bbc0_mac(self.radio, outer[1], buf),
        }

        Paragraph::new(Line::from("  q/Esc: quit  |  1/2/3 or <-/->: switch tab"))
            .style(Style::default().fg(Color::DarkGray))
            .render(outer[2], buf);
    }
}

fn section_header(label: &str, area: Rect, buf: &mut Buffer) {
    let width = area.width as usize;
    let prefix = format!("── {} ", label);
    let fill = width.saturating_sub(prefix.chars().count());
    let line = format!("{}{}", prefix, "─".repeat(fill));
    Paragraph::new(Line::from(line))
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .render(area, buf);
}

fn render_tab_bar(active: Tab, area: Rect, buf: &mut Buffer) {
    let tabs = [Tab::Rf09, Tab::Bbc0Phy, Tab::Bbc0Mac];
    let cols = Layout::horizontal(tabs.map(|_| Constraint::Length(18))).split(area);
    for (i, &tab) in tabs.iter().enumerate() {
        let style = if tab == active {
            Style::default().bg(Color::White).fg(Color::Black)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        Paragraph::new(Line::from(format!(" {} ", tab.label())))
            .style(style)
            .render(cols[i], buf);
    }
}

// =============================================================================
// Tab 1: Chip-level + RF09 transceiver
// =============================================================================

fn render_rf09(radio: &Radio, area: Rect, buf: &mut Buffer) {
    let rows = Layout::vertical([
        // Chip
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(1),
        // IQ interface
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(1),
        // Transceiver state
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(1),
        // Channel / PLL
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(1),
        // RX path
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(1),
        // ED measurement
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(1),
        // TX path
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Min(0),
    ])
    .split(area);

    section_header("Chip", rows[0], buf);
    let top = Layout::horizontal([
        Constraint::Length(14),
        Constraint::Length(10),
        Constraint::Min(24),
    ])
    .split(rows[1]);
    radio.rf_pn.value.render(top[0], buf);
    radio.rf_vn.value.render(top[1], buf);
    radio.rf_cfg.value.render(top[2], buf);

    let chip = Layout::horizontal([
        Constraint::Length(20),
        Constraint::Length(24),
        Constraint::Min(20),
    ])
    .split(rows[2]);
    radio.rf_clko.value.render(chip[0], buf);
    radio.rf_bmdvc.value.render(chip[1], buf);
    radio.rf_xoc.value.render(chip[2], buf);

    section_header("IQ interface", rows[4], buf);
    let iq = Layout::horizontal([
        Constraint::Min(48),
        Constraint::Length(36),
        Constraint::Length(12),
    ])
    .split(rows[5]);
    radio.rf_iqifc0.value.render(iq[0], buf);
    radio.rf_iqifc1.value.render(iq[1], buf);
    radio.rf_iqifc2.value.render(iq[2], buf);

    section_header("Transceiver state", rows[7], buf);
    let irq = Layout::horizontal([
        Constraint::Length(16),
        Constraint::Min(42),
        Constraint::Min(42),
    ])
    .split(rows[8]);
    radio.rf09_state.value.render(irq[0], buf);
    radio.rf09_irqs.value.render(irq[1], buf);
    radio.rf09_irqm.value.render(irq[2], buf);

    section_header("Channel / PLL", rows[10], buf);
    let chan = Layout::horizontal([
        Constraint::Length(24),
        Constraint::Length(16),
        Constraint::Length(18),
        Constraint::Min(18),
    ])
    .split(rows[11]);
    radio.rf09_ccf0.value.render(chan[0], buf);
    radio.rf09_cn.value.render(chan[1], buf);
    radio.rf09_cs.value.render(chan[2], buf);
    radio.rf09_pll.value.render(chan[3], buf);

    section_header("RX path", rows[13], buf);
    let rx = Layout::horizontal([
        Constraint::Length(20),
        Constraint::Length(22),
        Constraint::Min(34),
        Constraint::Length(20),
        Constraint::Length(16),
    ])
    .split(rows[14]);
    radio.rf09_rxbwc.value.render(rx[0], buf);
    radio.rf09_rxdfe.value.render(rx[1], buf);
    radio.rf09_agcc.value.render(rx[2], buf);
    radio.rf09_agcs.value.render(rx[3], buf);
    radio.rf09_rssi.value.render(rx[4], buf);

    section_header("ED measurement", rows[16], buf);
    let ed = Layout::horizontal([
        Constraint::Length(18),
        Constraint::Length(28),
        Constraint::Length(14),
        Constraint::Min(14),
    ])
    .split(rows[17]);
    radio.rf09_edc.value.render(ed[0], buf);
    radio.rf09_edd.value.render(ed[1], buf);
    radio.rf09_edv.value.render(ed[2], buf);
    radio.rf09_pllcf.value.render(ed[3], buf);

    section_header("TX path", rows[19], buf);
    let tx = Layout::horizontal([
        Constraint::Length(24),
        Constraint::Min(38),
        Constraint::Length(28),
        Constraint::Length(28),
    ])
    .split(rows[20]);
    radio.rf09_pac.value.render(tx[0], buf);
    radio.rf09_auxs.value.render(tx[1], buf);
    radio.rf09_txcutc.value.render(tx[2], buf);
    radio.rf09_txdfe.value.render(tx[3], buf);

    let cal = Layout::horizontal([
        Constraint::Length(16),
        Constraint::Length(14),
        Constraint::Length(14),
        Constraint::Length(14),
        Constraint::Length(16),
        Constraint::Min(16),
    ])
    .split(rows[21]);
    radio.rf09_padfe.value.render(cal[0], buf);
    radio.rf09_rndv.value.render(cal[1], buf);
    radio.rf09_txci.value.render(cal[2], buf);
    radio.rf09_txcq.value.render(cal[3], buf);
    radio.rf09_txdaci.value.render(cal[4], buf);
    radio.rf09_txdacq.value.render(cal[5], buf);
}

// =============================================================================
// Tab 2: BBC0 core + PHY config (OFDM / OQPSK / FSK)
// =============================================================================

fn render_bbc0_phy(radio: &Radio, area: Rect, buf: &mut Buffer) {
    let rows = Layout::vertical([
        // BBC core
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(1),
        // Frame buffers
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(1),
        // OFDM
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(1),
        // OQPSK
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(1),
        // FSK
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Min(0),
    ])
    .split(area);

    section_header("BBC core", rows[0], buf);
    let bbc_irq = Layout::horizontal([
        Constraint::Min(50),
        Constraint::Length(12),
        Constraint::Length(38),
        Constraint::Length(38),
    ])
    .split(rows[1]);
    radio.bbc0_pc.value.render(bbc_irq[0], buf);
    radio.bbc0_ps.value.render(bbc_irq[1], buf);
    radio.bbc0_irqs.value.render(bbc_irq[2], buf);
    radio.bbc0_irqm.value.render(bbc_irq[3], buf);

    section_header("Frame buffers", rows[3], buf);
    let bbc_buf = Layout::horizontal([
        Constraint::Length(16),
        Constraint::Length(16),
        Constraint::Length(16),
        Constraint::Length(16),
        Constraint::Length(20),
        Constraint::Min(28),
    ])
    .split(rows[4]);
    radio.bbc0_rxfl.value.render(bbc_buf[0], buf);
    radio.bbc0_txfl.value.render(bbc_buf[1], buf);
    radio.bbc0_fbl.value.render(bbc_buf[2], buf);
    radio.bbc0_fbli.value.render(bbc_buf[3], buf);
    radio.bbc0_cnt.value.render(bbc_buf[4], buf);
    radio.bbc0_cntc.value.render(bbc_buf[5], buf);

    section_header("OFDM", rows[6], buf);
    let ofdm = Layout::horizontal([
        Constraint::Length(20),
        Constraint::Length(16),
        Constraint::Length(30),
        Constraint::Min(18),
    ])
    .split(rows[7]);
    radio.bbc0_ofdmphrtx.value.render(ofdm[0], buf);
    radio.bbc0_ofdmphrrx.value.render(ofdm[1], buf);
    radio.bbc0_ofdmc.value.render(ofdm[2], buf);
    radio.bbc0_ofdmsw.value.render(ofdm[3], buf);

    section_header("OQPSK", rows[9], buf);
    let oqpsk = Layout::horizontal([
        Constraint::Length(32),
        Constraint::Length(28),
        Constraint::Length(30),
        Constraint::Min(18),
    ])
    .split(rows[10]);
    radio.bbc0_oqpskc0.value.render(oqpsk[0], buf);
    radio.bbc0_oqpskc1.value.render(oqpsk[1], buf);
    radio.bbc0_oqpskc2.value.render(oqpsk[2], buf);
    radio.bbc0_oqpskc3.value.render(oqpsk[3], buf);

    let oqpsk_phr = Layout::horizontal([
        Constraint::Length(22),
        Constraint::Min(22),
    ])
    .split(rows[11]);
    radio.bbc0_oqpskphrtx.value.render(oqpsk_phr[0], buf);
    radio.bbc0_oqpskphrrx.value.render(oqpsk_phr[1], buf);

    section_header("FSK", rows[13], buf);
    let fsk = Layout::horizontal([
        Constraint::Length(30),
        Constraint::Length(24),
        Constraint::Length(36),
        Constraint::Min(20),
    ])
    .split(rows[14]);
    radio.bbc0_fskc0.value.render(fsk[0], buf);
    radio.bbc0_fskc1.value.render(fsk[1], buf);
    radio.bbc0_fskc2.value.render(fsk[2], buf);
    radio.bbc0_fskc3.value.render(fsk[3], buf);

    let fsk2 = Layout::horizontal([
        Constraint::Length(36),
        Constraint::Length(16),
        Constraint::Length(18),
        Constraint::Min(18),
    ])
    .split(rows[15]);
    radio.bbc0_fskc4.value.render(fsk2[0], buf);
    radio.bbc0_fskpll.value.render(fsk2[1], buf);
    radio.bbc0_fsksfd0.value.render(fsk2[2], buf);
    radio.bbc0_fsksfd1.value.render(fsk2[3], buf);

    let fsk3 = Layout::horizontal([
        Constraint::Length(16),
        Constraint::Length(18),
        Constraint::Length(16),
        Constraint::Min(12),
    ])
    .split(rows[16]);
    radio.bbc0_fskphrtx.value.render(fsk3[0], buf);
    radio.bbc0_fskphrrx.value.render(fsk3[1], buf);
    radio.bbc0_fskrpc.value.render(fsk3[2], buf);
    radio.bbc0_fskdm.value.render(fsk3[3], buf);

    let fsk4 = Layout::horizontal([
        Constraint::Length(20),
        Constraint::Length(22),
        Constraint::Length(18),
        Constraint::Length(14),
        Constraint::Length(14),
        Constraint::Min(14),
    ])
    .split(rows[17]);
    radio.bbc0_fskrpcont.value.render(fsk4[0], buf);
    radio.bbc0_fskrpcofft.value.render(fsk4[1], buf);
    radio.bbc0_fskrrxfl.value.render(fsk4[2], buf);
    radio.bbc0_fskpe0.value.render(fsk4[3], buf);
    radio.bbc0_fskpe1.value.render(fsk4[4], buf);
    radio.bbc0_fskpe2.value.render(fsk4[5], buf);
}

// =============================================================================
// Tab 3: BBC0 AFC / MAC filtering / AMCS / PMU
// =============================================================================

fn render_bbc0_mac(radio: &Radio, area: Rect, buf: &mut Buffer) {
    let rows = Layout::vertical([
        // Address filter
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(1),
        // MAC addresses
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(1),
        // Auto-ACK
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(1),
        // PMU
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Min(0),
    ])
    .split(area);

    section_header("Address filter", rows[0], buf);
    let afc = Layout::horizontal([
        Constraint::Length(22),
        Constraint::Length(24),
        Constraint::Length(14),
        Constraint::Length(14),
        Constraint::Min(22),
    ])
    .split(rows[1]);
    radio.bbc0_afc0.value.render(afc[0], buf);
    radio.bbc0_afc1.value.render(afc[1], buf);
    radio.bbc0_afftm.value.render(afc[2], buf);
    radio.bbc0_affvm.value.render(afc[3], buf);
    radio.bbc0_afs.value.render(afc[4], buf);

    section_header("MAC addresses", rows[3], buf);
    let mac0 = Layout::horizontal([
        Constraint::Min(28),
        Constraint::Length(18),
        Constraint::Length(18),
    ])
    .split(rows[4]);
    radio.bbc0_macea.value.render(mac0[0], buf);
    radio.bbc0_macpidf0.value.render(mac0[1], buf);
    radio.bbc0_macshaf0.value.render(mac0[2], buf);

    let mac1 = Layout::horizontal([
        Constraint::Length(18),
        Constraint::Length(18),
        Constraint::Length(18),
        Constraint::Length(18),
        Constraint::Length(18),
        Constraint::Min(18),
    ])
    .split(rows[5]);
    radio.bbc0_macpidf1.value.render(mac1[0], buf);
    radio.bbc0_macshaf1.value.render(mac1[1], buf);
    radio.bbc0_macpidf2.value.render(mac1[2], buf);
    radio.bbc0_macshaf2.value.render(mac1[3], buf);
    radio.bbc0_macpidf3.value.render(mac1[4], buf);
    radio.bbc0_macshaf3.value.render(mac1[5], buf);

    section_header("Auto-ACK", rows[7], buf);
    let amcs = Layout::horizontal([Constraint::Min(60)])
        .split(rows[8]);
    radio.bbc0_amcs.value.render(amcs[0], buf);

    let amcs2 = Layout::horizontal([
        Constraint::Length(16),
        Constraint::Length(22),
        Constraint::Min(20),
    ])
    .split(rows[9]);
    radio.bbc0_amedt.value.render(amcs2[0], buf);
    radio.bbc0_amaackpd.value.render(amcs2[1], buf);
    radio.bbc0_amaackt.value.render(amcs2[2], buf);

    section_header("PMU", rows[11], buf);
    let pmu = Layout::horizontal([
        Constraint::Min(34),
        Constraint::Length(20),
        Constraint::Length(16),
    ])
    .split(rows[12]);
    radio.bbc0_pmuc.value.render(pmu[0], buf);
    radio.bbc0_pmuval.value.render(pmu[1], buf);
    radio.bbc0_pmuqf.value.render(pmu[2], buf);

    let pmu2 = Layout::horizontal([
        Constraint::Length(16),
        Constraint::Min(16),
    ])
    .split(rows[13]);
    radio.bbc0_pmui.value.render(pmu2[0], buf);
    radio.bbc0_pmuq.value.render(pmu2[1], buf);
}

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    let mut radio = Radio::new();
    // Set fields so the dashboard is not empty on first render
    radio.rf_cfg.value = radio.rf_cfg.value.with_drv(3).with_irqmm(true);
    radio.rf09_irqm.value = radio.rf09_irqm.value.with_trxrdy(true).with_trxerr(true);

    PllSettings::ieee(Band::Sub1GHz, 868_300_000, 200_000, 0)
        .unwrap()
        .apply_rf09(&mut radio);
    radio.rf09_pac.value = radio.rf09_pac.value.with_txpwr(25).with_pacur(3);

    let res = run(&mut terminal, &radio);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    res
}

fn run(terminal: &mut Terminal<CrosstermBackend<Stdout>>, radio: &Radio) -> io::Result<()> {
    let mut tab = Tab::Rf09;
    loop {
        terminal.draw(|frame| {
            frame.render_widget(Dashboard { radio, tab }, frame.area());
        })?;
        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                KeyCode::Char('1') => tab = Tab::Rf09,
                KeyCode::Char('2') => tab = Tab::Bbc0Phy,
                KeyCode::Char('3') => tab = Tab::Bbc0Mac,
                KeyCode::Right | KeyCode::Tab => tab = tab.next(),
                KeyCode::Left | KeyCode::BackTab => tab = tab.prev(),
                _ => {}
            }
        }
    }
}
