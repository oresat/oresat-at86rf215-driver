//! Live telemetry viewer - receives CBOR `CommState` from the daemon.
//!
//! Usage:
//!   cargo run --example live -- --port 10035
//!
//! Then start the daemon with:
//!   cargo run --bin daemon -- --dry-run --telemetry 127.0.0.1:10035

use std::{
    collections::VecDeque,
    io::{self, Stdout},
    net::{IpAddr, Ipv4Addr, SocketAddr},
    os::fd::AsRawFd,
    time::{Duration, Instant},
};

use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use mio::{Events, Interest, Poll, Token, net::UdpSocket, unix::SourceFd};
use oresat_at86rf215_driver::comm::{BbcStatus, CommState, RfStatus, RxPacket};
use oresat_at86rf215_driver::cpuload::CpuLoad;
use oresat_at86rf215_driver::stats::RadioStats;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Widget},
};

#[derive(Parser)]
#[command(name = "live", about = "Live telemetry viewer for the AT86RF215 daemon")]
struct Args {
    /// UDP port to listen on for telemetry.
    #[arg(long, default_value = "10035")]
    port: u16,
}

/// Accumulated telemetry state, updated from incoming CommState messages.
struct LiveState {
    rf09: Option<RfStatus>,
    bbc0: Option<BbcStatus>,
    stats: Option<RadioStats>,
    recent_rx: VecDeque<RxPacket>,
    rx_count: u64,
    tx_count: u64,
    msg_count: u64,
    cpu_percent: Option<f32>,
}

impl LiveState {
    fn new() -> Self {
        Self {
            rf09: None,
            bbc0: None,
            stats: None,
            recent_rx: VecDeque::with_capacity(8),
            rx_count: 0,
            tx_count: 0,
            msg_count: 0,
            cpu_percent: None,
        }
    }

    fn update(&mut self, msg: CommState) {
        self.msg_count += 1;
        match msg {
            CommState::Rf09Status(s) | CommState::Rf24Status(s) => self.rf09 = Some(s),
            CommState::Bbc0Status(s) | CommState::Bbc1Status(s) => self.bbc0 = Some(s),
            CommState::Rx(pkt) => {
                self.rx_count += 1;
                self.recent_rx.push_front(pkt);
                if self.recent_rx.len() > 6 {
                    self.recent_rx.pop_back();
                }
            }
            CommState::Tx(_) => {
                self.tx_count += 1;
            }
            CommState::Stats(s) => {
                self.stats = Some(s);
            }
        }
    }
}

struct LiveDashboard<'a>(&'a LiveState);

impl Widget for LiveDashboard<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let rows = Layout::vertical([
            Constraint::Length(5), // RF status
            Constraint::Length(5), // BBC status
            Constraint::Length(5), // stats
            Constraint::Min(4),   // recent RX packets
            Constraint::Length(1), // footer
        ])
        .split(area);

        // RF status
        render_rf_status(self.0.rf09.as_ref(), rows[0], buf);

        // BBC status
        render_bbc_status(self.0.bbc0.as_ref(), rows[1], buf);

        // Stats
        render_stats(self.0.stats.as_ref(), self.0.msg_count, rows[2], buf);

        // Recent RX
        render_recent_rx(&self.0.recent_rx, rows[3], buf);

        let cpu = self.0.cpu_percent
            .map(|p| format!("{:.1}%", p))
            .unwrap_or_else(|| "-".into());
        Paragraph::new(Line::from(format!("  q / Esc: quit    CPU: {cpu}")))
            .style(Style::default().fg(Color::DarkGray))
            .render(rows[4], buf);
    }
}

fn render_rf_status(rf: Option<&RfStatus>, area: Rect, buf: &mut Buffer) {
    let block = Block::default().borders(Borders::ALL).title("RF09 Status");
    match rf {
        Some(s) => {
            let state_name = match s.state {
                0x02 => "TrxOff",
                0x03 => "TxPrep",
                0x04 => "Tx",
                0x05 => "Rx",
                0x06 => "Transition",
                _ => "Reset/Sleep",
            };
            let rssi_label = if s.rssi == 127 {
                "invalid".to_string()
            } else {
                format!("{} dBm", s.rssi)
            };
            Table::new(
                vec![
                    Row::new(vec![
                        Cell::from(format!("State: {}", state_name)),
                        Cell::from(format!("RSSI: {}", rssi_label)),
                        Cell::from(format!("EDV: {} dBm", s.edv)),
                        Cell::from(format!("AGC GCW: {}", s.agc_gcw)),
                    ]),
                ],
                [
                    Constraint::Length(20),
                    Constraint::Length(18),
                    Constraint::Length(14),
                    Constraint::Min(14),
                ],
            )
            .block(block)
            .render(area, buf);
        }
        None => {
            Paragraph::new("  waiting for data...")
                .style(Style::default().fg(Color::DarkGray))
                .block(block)
                .render(area, buf);
        }
    }
}

fn render_bbc_status(bbc: Option<&BbcStatus>, area: Rect, buf: &mut Buffer) {
    let block = Block::default().borders(Borders::ALL).title("BBC0 Status");
    match bbc {
        Some(s) => {
            let phy = match s.phy_type {
                0 => "FSK",
                1 => "OFDM",
                2 => "OQPSK",
                3 => "OQPSK-leg",
                _ => "?",
            };
            Table::new(
                vec![
                    Row::new(vec![
                        Cell::from(format!("PHY: {}", phy)),
                        Cell::from(format!("RXFL: {} B", s.rxfl)),
                        Cell::from(format!("TXFL: {} B", s.txfl)),
                        Cell::from(format!("CNT: {}", s.cnt)),
                    ]),
                ],
                [
                    Constraint::Length(16),
                    Constraint::Length(14),
                    Constraint::Length(14),
                    Constraint::Min(14),
                ],
            )
            .block(block)
            .render(area, buf);
        }
        None => {
            Paragraph::new("  waiting for data...")
                .style(Style::default().fg(Color::DarkGray))
                .block(block)
                .render(area, buf);
        }
    }
}

fn render_recent_rx(packets: &VecDeque<RxPacket>, area: Rect, buf: &mut Buffer) {
    let rows: Vec<Row> = packets
        .iter()
        .enumerate()
        .map(|(i, pkt)| {
            let hex: String = pkt
                .data
                .iter()
                .take(16)
                .map(|b| format!("{:02X}", b))
                .collect::<Vec<_>>()
                .join(" ");
            let suffix = if pkt.data.len() > 16 { "..." } else { "" };
            Row::new(vec![
                Cell::from(format!("{}", i)),
                Cell::from(format!("{} B", pkt.data.len())),
                Cell::from(format!("{} dBm", pkt.rssi)),
                Cell::from(format!("{}{}", hex, suffix)),
            ])
        })
        .collect();

    Table::new(
        rows,
        [
            Constraint::Length(4),
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Min(20),
        ],
    )
    .header(
        Row::new(vec!["#", "Len", "RSSI", "Data (hex)"])
            .style(Style::default().fg(Color::Yellow)),
    )
    .block(Block::default().borders(Borders::ALL).title("Recent RX Packets"))
    .render(area, buf);
}

fn render_stats(stats: Option<&RadioStats>, msg_count: u64, area: Rect, buf: &mut Buffer) {
    let block = Block::default().borders(Borders::ALL).title("Radio Stats");
    match stats {
        Some(s) => {
            let rssi_mean = s.rssi_mean()
                .map(|v| format!("{:.1}", v))
                .unwrap_or_else(|| "-".into());
            let rssi_last = if s.rssi_last == 127 {
                "inv".to_string()
            } else {
                format!("{}", s.rssi_last)
            };
            Table::new(
                vec![
                    Row::new(vec![
                        Cell::from(format!("TX: {}", s.tx_count)),
                        Cell::from(format!("RX: {}", s.rx_count)),
                        Cell::from(format!("CRC fail: {}", s.rx_crc_fail)),
                        Cell::from(format!("TX err: {}", s.tx_errors)),
                    ]),
                    Row::new(vec![
                        Cell::from(format!("RSSI: {} dBm", rssi_last)),
                        Cell::from(format!("min/max: {}/{}", s.rssi_min, s.rssi_max)),
                        Cell::from(format!("mean: {} dBm", rssi_mean)),
                        Cell::from(format!("ticks: {}  msgs: {}", s.ticks, msg_count)),
                    ]),
                ],
                [
                    Constraint::Length(16),
                    Constraint::Length(20),
                    Constraint::Length(20),
                    Constraint::Min(20),
                ],
            )
            .block(block)
            .render(area, buf);
        }
        None => {
            Paragraph::new("  waiting for stats...")
                .style(Style::default().fg(Color::DarkGray))
                .block(block)
                .render(area, buf);
        }
    }
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), args.port);
    let mut sock = UdpSocket::bind(addr)?;
    eprintln!("listening for telemetry on :{}", args.port);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    let res = run(&mut terminal, &mut sock);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    res
}

const TELEMETRY: Token = Token(0);
const STDIN: Token = Token(1);

/// Event-driven TUI loop - blocks on `mio::Poll` until telemetry arrives,
/// the user presses a key, or the CPU-meter refresh deadline expires (1 s).
/// redraws only on events.
fn run(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    sock: &mut UdpSocket,
) -> io::Result<()> {
    let mut poll = Poll::new()?;
    poll.registry().register(sock, TELEMETRY, Interest::READABLE)?;
    let stdin_fd = io::stdin().as_raw_fd();
    poll.registry()
        .register(&mut SourceFd(&stdin_fd), STDIN, Interest::READABLE)?;

    let mut events = Events::with_capacity(32);
    let mut state = LiveState::new();
    let mut recv_buf = [0u8; 4096];

    let mut cpu = CpuLoad::new();
    // Seed the sampler so the first visible value is meaningful.
    let _ = cpu.sample();
    let cpu_refresh = Duration::from_secs(1);
    let mut last_cpu_sample = Instant::now();

    // Start "waiting for data..." so it shows even with no events.
    terminal.draw(|frame| frame.render_widget(LiveDashboard(&state), frame.area()))?;

    loop {
        let now = Instant::now();
        let timeout = cpu_refresh
            .checked_sub(now.duration_since(last_cpu_sample))
            .unwrap_or(Duration::ZERO);
        poll.poll(&mut events, Some(timeout))?;

        let mut dirty = false;

        for event in events.iter() {
            match event.token() {
                TELEMETRY => loop {
                    match sock.recv(&mut recv_buf) {
                        Ok(n) if n > 0 => {
                            if let Ok(msg) = CommState::decode(&recv_buf[..n]) {
                                state.update(msg);
                                dirty = true;
                            }
                        }
                        _ => break,
                    }
                },
                STDIN => {
                    while event::poll(Duration::ZERO)? {
                        if let Event::Key(key) = event::read()?
                            && key.kind == KeyEventKind::Press
                        {
                            match key.code {
                                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                                KeyCode::Char('c')
                                    if key.modifiers.contains(KeyModifiers::CONTROL) =>
                                {
                                    return Ok(());
                                }
                                _ => {}
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        if now.duration_since(last_cpu_sample) >= cpu_refresh {
            if let Ok(Some(pct)) = cpu.sample() {
                state.cpu_percent = Some(pct);
                dirty = true;
            }
            last_cpu_sample = Instant::now();
        }

        if dirty {
            terminal.draw(|frame| frame.render_widget(LiveDashboard(&state), frame.area()))?;
        }
    }
}
