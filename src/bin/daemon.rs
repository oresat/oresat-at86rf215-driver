//! AT86RF215 radio daemon.
//!
//! Bridges UDP *or* Unix-datagram sockets to the radio over SPI:
//!
//! - **TX socket** (default UDP port 10020, or `--tx-uds <path>` for UDS):
//!   datagrams received here are transmitted by the RF09 sub-1 GHz transceiver.
//! - **RX socket** (default UDP port 10021, or `--rx-uds <path>` for UDS):
//!   frames received by the radio are forwarded here as datagrams.
//! - **Telemetry socket** (optional): periodic CBOR-encoded `CommState` snapshots
//!   for the TUI viewer. UDP only - the TUI typically lives across the network.
//!
//! The event loop uses `mio::Poll` (same pattern as the ax5043 daemon) - no
//! async runtime required.
//!
//! ## Running without hardware
//!
//! With `--dry-run` the daemon skips SPI/GPIO initialisation and runs a
//! loopback: TX packets are echoed straight to the RX socket so the full
//! socket path can be exercised on a dev machine.

use std::{
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
    time::Duration,
};

use clap::Parser;
use mio::net::{UdpSocket, UnixDatagram};
use mio::{Events, Interest, Poll, Registry, Token};

use oresat_at86rf215_driver::{
    comm::{BbcStatus, CommState, RfStatus, RxPacket},
    radio::Radio,
    registers::{BbcnTxfl, RfnCmd, TransceiverCmd},
    spi::{self, Bbc},
    stats::RadioStats,
};

// ── mio tokens ──────────────────────────────────────────────────────────────

const TX_SOCKET: Token = Token(0);
const TIMER: Token = Token(1);
#[allow(dead_code)] // reserved for future mio-based signal source
const SIGNAL: Token = Token(2);
const GPIO_IRQ: Token = Token(3);

// ── CLI ─────────────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(name = "at86rf215-daemon", about = "AT86RF215 radio <-> UDP/UDS daemon")]
struct Args {
    /// UDP port to receive TX datagrams on (ignored when `--tx-uds` is set).
    #[arg(long, default_value = "10020")]
    tx_port: u16,

    /// UDP port to send RX frames to (ignored when `--rx-uds` is set).
    #[arg(long, default_value = "10021")]
    rx_port: u16,

    /// Unix-domain datagram path for TX input (overrides `--tx-port`).
    #[arg(long)]
    tx_uds: Option<PathBuf>,

    /// Unix-domain datagram path for RX output (overrides `--rx-port`).
    #[arg(long)]
    rx_uds: Option<PathBuf>,

    /// Optional telemetry destination (e.g. "127.0.0.1:10035").
    #[arg(long)]
    telemetry: Option<String>,

    /// SPI device path (e.g. /dev/spidev0.0).
    #[arg(long, default_value = "/dev/spidev0.0")]
    spi: String,

    /// Run without hardware - TX packets are looped back as RX.
    #[arg(long)]
    dry_run: bool,

    /// Telemetry interval in milliseconds.
    #[arg(long, default_value = "100")]
    telemetry_ms: u64,

    /// GPIO chip for the radio IRQ line (e.g. /dev/gpiochip0).
    #[arg(long, default_value = "/dev/gpiochip0")]
    gpio_chip: String,

    /// GPIO line number for the radio IRQ (rising edge).
    #[arg(long, default_value = "30")]
    gpio_line: u32,

    /// TOML config file to load at startup (applies register values before Rx).
    #[arg(long)]
    config: Option<String>,
}

// ── socket abstractions ────────────────────────────────────────────────────
//
// The daemon accepts either UDP or Unix-datagram on both the TX (incoming)
// and RX (outgoing) sides. Two small enums dispatch across the two kinds so
// the event-loop body stays single-branched.

enum TxListener {
    Udp(UdpSocket),
    Uds(UnixDatagram),
}

impl TxListener {
    fn recv(&self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            TxListener::Udp(s) => s.recv(buf),
            TxListener::Uds(s) => s.recv(buf),
        }
    }

    fn register(&mut self, registry: &Registry, token: Token) -> io::Result<()> {
        match self {
            TxListener::Udp(s) => registry.register(s, token, Interest::READABLE),
            TxListener::Uds(s) => registry.register(s, token, Interest::READABLE),
        }
    }

    fn describe(&self, port: u16, uds: &Option<PathBuf>) -> String {
        match (self, uds) {
            (TxListener::Uds(_), Some(p)) => format!("uds:{}", p.display()),
            _ => format!(":{port}"),
        }
    }
}

enum RxSender {
    Udp(std::net::UdpSocket),
    Uds(std::os::unix::net::UnixDatagram),
}

impl RxSender {
    fn send(&self, data: &[u8]) -> io::Result<usize> {
        match self {
            RxSender::Udp(s) => s.send(data),
            RxSender::Uds(s) => s.send(data),
        }
    }

    fn describe(&self, port: u16, uds: &Option<PathBuf>) -> String {
        match (self, uds) {
            (RxSender::Uds(_), Some(p)) => format!("uds:{}", p.display()),
            _ => format!(":{port}"),
        }
    }
}

/// Remove a stale socket file at `path` if one exists. Ignores not-found.
/// Without this, a second `bind()` on an abandoned UDS path fails with
/// `EADDRINUSE` even when no process holds it.
fn remove_stale_uds(path: &std::path::Path) -> io::Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

// ── main ────────────────────────────────────────────────────────────────────

fn main() -> io::Result<()> {
    let args = Args::parse();

    // ── sockets ─────────────────────────────────────────────────────────
    let tx_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), args.tx_port);
    let rx_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), args.rx_port);

    let mut tx_sock = match args.tx_uds.as_ref() {
        Some(path) => {
            remove_stale_uds(path)?;
            TxListener::Uds(UnixDatagram::bind(path)?)
        }
        None => TxListener::Udp(UdpSocket::bind(tx_addr)?),
    };

    let rx_sock = match args.rx_uds.as_ref() {
        Some(path) => {
            let sock = std::os::unix::net::UnixDatagram::unbound()?;
            sock.connect(path)?;
            RxSender::Uds(sock)
        }
        None => {
            let sock = std::net::UdpSocket::bind("0.0.0.0:0")?;
            sock.connect(rx_addr)?;
            RxSender::Udp(sock)
        }
    };

    let telemetry_sock = args.telemetry.as_ref().map(|dest| {
        let sock = std::net::UdpSocket::bind("0.0.0.0:0").expect("bind telemetry");
        sock.connect(dest).expect("connect telemetry");
        sock
    });

    // ── timer (periodic telemetry) ──────────────────────────────────────
    let mut tfd = timerfd::TimerFd::new().map_err(io_err)?;
    tfd.set_state(
        timerfd::TimerState::Periodic {
            current: Duration::from_secs(1),
            interval: Duration::from_millis(args.telemetry_ms),
        },
        timerfd::SetTimeFlags::Default,
    );

    // ── signal handling (SIGINT for clean shutdown) ─────────────────────
    use signal_hook::consts::SIGINT;
    let signal_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    signal_hook::flag::register(SIGINT, signal_flag.clone())
        .map_err(io_err)?;

    // ── poll registry ───────────────────────────────────────────────────
    let mut poll = Poll::new()?;
    tx_sock.register(poll.registry(), TX_SOCKET)?;

    // Register timerfd as a raw FD source.
    use std::os::fd::AsRawFd;
    let tfd_raw = tfd.as_raw_fd();
    poll.registry().register(
        &mut mio::unix::SourceFd(&tfd_raw),
        TIMER,
        Interest::READABLE,
    )?;

    // ── radio ───────────────────────────────────────────────────────────
    let mut radio = Radio::new();
    let mut spidev: Option<spidev::Spidev> = if !args.dry_run {
        match spi::open(&args.spi) {
            Ok(dev) => {
                eprintln!("SPI opened: {}", args.spi);
                Some(dev)
            }
            Err(e) => {
                eprintln!("warning: failed to open SPI ({}), falling back to dry-run", e);
                None
            }
        }
    } else {
        eprintln!("dry-run mode - no hardware");
        None
    };

    // ── TOML config (optional) ───────────────────────────────────────
    if let Some(ref path) = args.config {
        let contents = std::fs::read_to_string(path)?;
        let config: oresat_at86rf215_driver::config::RadioConfig =
            toml::from_str(&contents).map_err(io::Error::other)?;
        radio.apply_config(&config);
        eprintln!("config loaded: {}", path);
    }

    // ── radio initialisation (hardware only) ───────────────────────────
    if let Some(ref mut dev) = spidev {
        init_radio(&mut radio, dev)?;
    }

    // ── GPIO IRQ (hardware only) ───────────────────────────────────────
    let irq_req = if spidev.is_some() {
        match gpiocdev::Request::builder()
            .on_chip(&args.gpio_chip)
            .with_line(args.gpio_line)
            .with_edge_detection(gpiocdev::line::EdgeDetection::RisingEdge)
            .request()
        {
            Ok(req) => {
                let irq_fd = req.as_raw_fd();
                poll.registry().register(
                    &mut mio::unix::SourceFd(&irq_fd),
                    GPIO_IRQ,
                    Interest::READABLE,
                )?;
                eprintln!("GPIO IRQ: {}:{}", args.gpio_chip, args.gpio_line);
                Some(req)
            }
            Err(e) => {
                eprintln!("warning: failed to open GPIO IRQ ({}), RX disabled", e);
                None
            }
        }
    } else {
        None
    };

    // ── event loop ──────────────────────────────────────────────────────
    let mut events = Events::with_capacity(64);
    let mut pkt_buf = [0u8; 2048];
    let mut stats = RadioStats::new();

    eprintln!(
        "listening: TX on {}, RX forwarded to {}",
        tx_sock.describe(args.tx_port, &args.tx_uds),
        rx_sock.describe(args.rx_port, &args.rx_uds),
    );

    loop {
        // Check for SIGINT before blocking.
        if signal_flag.load(std::sync::atomic::Ordering::Relaxed) {
            eprintln!(
                "\nSIGINT received - shutting down (tx={}, rx={}, crc_fail={}, ticks={})",
                stats.tx_count, stats.rx_count, stats.rx_crc_fail, stats.ticks,
            );
            return Ok(());
        }

        poll.poll(&mut events, Some(Duration::from_millis(250)))?;

        for event in events.iter() {
            match event.token() {
                TX_SOCKET => {
                    // Read datagrams from the TX socket and transmit (or loopback).
                    loop {
                        match tx_sock.recv(&mut pkt_buf) {
                            Ok(n) if n > 0 => {
                                let frame = &pkt_buf[..n];

                                if let Some(ref mut dev) = spidev {
                                    match transmit_frame(&mut radio, dev, frame) {
                                        Ok(()) => stats.record_tx(),
                                        Err(e) => {
                                            stats.record_tx_error();
                                            eprintln!("TX error: {}", e);
                                        }
                                    }
                                } else {
                                    // Dry-run: loopback to RX socket.
                                    stats.record_tx();
                                    let pkt = RxPacket {
                                        data: frame.to_vec(),
                                        rssi: -40,
                                        edv: -40,
                                    };
                                    stats.record_rx(-40);
                                    let _ = rx_sock.send(frame)?;
                                    if let Some(ref ts) = telemetry_sock {
                                        let _ = CommState::Rx(pkt).send(ts);
                                    }
                                }
                            }
                            Ok(_) => break,
                            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => break,
                            Err(e) => return Err(e),
                        }
                    }
                }

                TIMER => {
                    // Drain the timerfd so it doesn't fire again immediately.
                    let _ = tfd.read();
                    stats.tick();

                    // In hardware mode, refresh status registers from SPI.
                    if let Some(ref mut dev) = spidev {
                        let _ = spi::read_register(dev, &mut radio.rf09_state);
                        let _ = spi::read_register(dev, &mut radio.rf09_rssi);
                        let _ = spi::read_register(dev, &mut radio.rf09_edv);
                        let _ = spi::read_register(dev, &mut radio.rf09_agcs);
                        stats.update_rssi(radio.rf09_rssi.value.rssi());
                    }

                    if let Some(ref ts) = telemetry_sock {
                        if spidev.is_some() {
                            let _ = CommState::Rf09Status(rf_status(&radio)).send(ts);
                            let _ = CommState::Bbc0Status(bbc_status(&radio)).send(ts);
                        } else {
                            let _ = CommState::Rf09Status(synthetic_rf(stats.ticks)).send(ts);
                            let _ = CommState::Bbc0Status(synthetic_bbc(stats.ticks, stats.tx_count)).send(ts);
                        }
                        let _ = CommState::Stats(stats).send(ts);
                    }
                }

                GPIO_IRQ => {
                    // Drain all pending edge events from the GPIO line.
                    if let Some(ref req) = irq_req {
                        while req.has_edge_event().unwrap_or(false) {
                            let _ = req.read_edge_event();
                        }
                    }

                    if let Some(ref mut dev) = spidev {
                        // Read and clear BBC0 IRQ status - this single read
                        // captures both RXFE and TXFE before clearing them.
                        if let Err(e) = spi::read_register(dev, &mut radio.bbc0_irqs) {
                            eprintln!("IRQ read error: {}", e);
                            continue;
                        }
                        let irqs = radio.bbc0_irqs.value;

                        // TXFE: transmission complete - re-enter Rx.
                        if irqs.txfe() {
                            radio.rf09_cmd.value = RfnCmd::new()
                                .with_cmd(TransceiverCmd::Rx);
                            let _ = spi::write_register(dev, &radio.rf09_cmd);
                        }

                        // RXFE: frame received - read it out.
                        if irqs.rxfe() {
                            match receive_frame(&mut radio, dev, &mut stats) {
                                Ok(Some(pkt)) => {
                                    stats.record_rx(pkt.rssi);
                                    let _ = rx_sock.send(&pkt.data);
                                    if let Some(ref ts) = telemetry_sock {
                                        let _ = CommState::Rx(pkt).send(ts);
                                    }
                                }
                                Ok(None) => {}
                                Err(e) => eprintln!("RX error: {}", e),
                            }
                        }
                    }
                }

                _ => {}
            }
        }
    }
}

// ── radio init ─────────────────────────────────────────────────────────────

/// Initialise the AT86RF215 for operation.
///
/// Sequence:
/// 1. Chip reset (write 0x7 to RF_RST).
/// 2. Wait for the chip to come out of reset (~1 ms typ.).
/// 3. Read RF_PN / RF_VN to verify SPI communication.
/// 4. Write all configurable registers (from TOML config or defaults).
/// 5. Enable BBC0 baseband with auto-FCS append and FCS filter.
/// 6. Enable RXFE interrupt in BBCn_IRQM.
/// 7. Transition RF09 from TrxOff -> TxPrep -> Rx.
fn init_radio(radio: &mut Radio, dev: &mut spidev::Spidev) -> io::Result<()> {
    // 1-2. Chip reset + identity check.
    let (pn, vn) = spi::reset_and_identify(dev, radio)?;
    eprintln!("chip: {:?} v{}", pn, vn);

    // 3. Write configurable registers to SPI (values set by --config or defaults).
    //    Uses BulkWrites to coalesce contiguous registers into fewer SPI transactions.
    write_rf09_config(radio, dev)?;
    write_bbc0_config(radio, dev)?;

    // 4. Enable BBC0 baseband + auto-FCS + FCS filter.
    //    These are layered on top of whatever the TOML config set.
    radio.bbc0_pc.value = radio.bbc0_pc.value
        .with_bben(true)
        .with_txafcs(true)
        .with_fcsfe(true);
    spi::write_register(dev, &radio.bbc0_pc)?;

    // 5. Enable RXFE + TXFE interrupts so the GPIO fires on frame
    //    reception and transmission completion.
    radio.bbc0_irqm.value = radio.bbc0_irqm.value
        .with_rxfe(true)
        .with_txfe(true);
    spi::write_register(dev, &radio.bbc0_irqm)?;

    // 6. Transition to Rx: TrxOff -> TxPrep, then TxPrep -> Rx.
    radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::TxPrep);
    spi::write_register(dev, &radio.rf09_cmd)?;
    spi::wait_rf09_txprep_locked(dev, radio, Duration::from_millis(5))?;

    radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::Rx);
    spi::write_register(dev, &radio.rf09_cmd)?;

    eprintln!("radio initialised - listening on RF09");
    Ok(())
}

/// Write RF09 configuration registers (channel plan, RX/TX filters, AGC, PA).
fn write_rf09_config(radio: &mut Radio, dev: &mut spidev::Spidev) -> io::Result<()> {
    use std::io::Write;
    use oresat_at86rf215_driver::registers::BulkWrites;

    let mut bw = BulkWrites::new();
    bw.add(&mut radio.rf09_cs);
    bw.add(&mut radio.rf09_ccf0);
    bw.add(&mut radio.rf09_cn);
    bw.add(&mut radio.rf09_rxbwc);
    bw.add(&mut radio.rf09_rxdfe);
    bw.add(&mut radio.rf09_agcc);
    bw.add(&mut radio.rf09_agcs);
    bw.add(&mut radio.rf09_txcutc);
    bw.add(&mut radio.rf09_txdfe);
    bw.add(&mut radio.rf09_pac);
    for cmd in bw.generate_commands() {
        dev.write_all(&cmd)?;
    }
    Ok(())
}

/// Write BBC0 PHY configuration registers (FSK/OFDM/OQPSK settings).
fn write_bbc0_config(radio: &mut Radio, dev: &mut spidev::Spidev) -> io::Result<()> {
    use std::io::Write;
    use oresat_at86rf215_driver::registers::BulkWrites;

    let mut bw = BulkWrites::new();
    bw.add(&mut radio.bbc0_fskc0);
    bw.add(&mut radio.bbc0_fskc1);
    bw.add(&mut radio.bbc0_fskc2);
    bw.add(&mut radio.bbc0_fskc3);
    bw.add(&mut radio.bbc0_fskc4);
    bw.add(&mut radio.bbc0_fskpll);
    bw.add(&mut radio.bbc0_fskphrtx);
    bw.add(&mut radio.bbc0_ofdmphrtx);
    bw.add(&mut radio.bbc0_ofdmc);
    bw.add(&mut radio.bbc0_oqpskc0);
    for cmd in bw.generate_commands() {
        dev.write_all(&cmd)?;
    }
    Ok(())
}

// ── TX path ─────────────────────────────────────────────────────────────────

/// Write a frame to the TX FIFO and issue the TX command.
///
/// Sequence (datasheet §5.1.5):
/// 1. Transition to TxPrep (required before loading the FIFO).
/// 2. Write frame data to the TX frame buffer (BBC0_FBTXS).
/// 3. Write frame length to BBCn_TXFL.
/// 4. Issue CMD=TX to start transmission.
///
/// The chip auto-transitions Tx -> TxPrep when the frame ends, firing
/// a TXFE interrupt.  The GPIO_IRQ handler re-enters Rx on TXFE,
/// so this function does **not** issue CMD=Rx itself.
fn transmit_frame(
    radio: &mut Radio,
    dev: &mut spidev::Spidev,
    frame: &[u8],
) -> io::Result<()> {
    // 1. Transition to TxPrep (from Rx or TrxOff).
    radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::TxPrep);
    spi::write_register(dev, &radio.rf09_cmd)?;

    // 2. Write frame data to TX buffer.
    spi::write_tx_fifo(dev, Bbc::Bbc0, frame)?;

    // 3. Set TX frame length register.
    radio.bbc0_txfl.value = BbcnTxfl::new().with_txfl(frame.len() as u16);
    spi::write_register(dev, &radio.bbc0_txfl)?;

    // 4. Issue TX command.
    radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::Tx);
    spi::write_register(dev, &radio.rf09_cmd)?;

    Ok(())
}

// ── RX path ────────────────────────────────────────────────────────────────

/// Read a received frame from the RX FIFO after RXFE has been confirmed.
///
/// The caller must have already read BBCn_IRQS and checked that RXFE is
/// set before calling this function.
///
/// Sequence (datasheet §5.1.4):
/// 1. Check BBCn_PC.fcsok to verify CRC.
/// 2. Read BBCn_FBL to get the received frame length.
/// 3. Read that many bytes from the RX frame buffer (BBC0_FBRXS).
/// 4. Capture RSSI/EDV from the RF status registers.
///
/// Returns `None` if CRC is invalid or frame length is zero.
fn receive_frame(
    radio: &mut Radio,
    dev: &mut spidev::Spidev,
    stats: &mut RadioStats,
) -> io::Result<Option<RxPacket>> {
    // 1. Check FCS (CRC) validity.
    spi::read_register(dev, &mut radio.bbc0_pc)?;
    if !radio.bbc0_pc.value.fcsok() {
        stats.record_crc_fail();
        eprintln!("RX: frame dropped (bad CRC)");
        return Ok(None);
    }

    // 3. Read frame buffer level (received frame length).
    spi::read_register(dev, &mut radio.bbc0_fbl)?;
    let len = radio.bbc0_fbl.value.fbl() as usize;
    if len == 0 {
        return Ok(None);
    }

    // 4. Read frame data from RX FIFO.
    let data = spi::read_rx_fifo(dev, Bbc::Bbc0, len)?;

    // 5. Capture RSSI/EDV (may already be refreshed by the telemetry tick,
    //    but re-read here for accuracy at the moment of reception).
    spi::read_register(dev, &mut radio.rf09_rssi)?;
    spi::read_register(dev, &mut radio.rf09_edv)?;

    Ok(Some(RxPacket {
        data,
        rssi: radio.rf09_rssi.value.rssi(),
        edv: radio.rf09_edv.value.edv(),
    }))
}

// ── telemetry snapshots ─────────────────────────────────────────────────────

fn rf_status(radio: &Radio) -> RfStatus {
    RfStatus {
        state: radio.rf09_state.value.state().into_bits(),
        rssi: radio.rf09_rssi.value.rssi(),
        edv: radio.rf09_edv.value.edv(),
        agc_gcw: radio.rf09_agcs.value.gcw(),
    }
}

fn bbc_status(radio: &Radio) -> BbcStatus {
    BbcStatus {
        phy_type: radio.bbc0_pc.value.pt(),
        rxfl: radio.bbc0_rxfl.value.rxfl(),
        txfl: radio.bbc0_txfl.value.txfl(),
        cnt: radio.bbc0_cnt.value.cnt(),
    }
}

/// Synthetic RF status for dry-run mode - simulates an idling receiver.
fn synthetic_rf(tick: u64) -> RfStatus {
    // Simulate RSSI wandering between -90 and -70 dBm.
    let rssi_base: i8 = -80;
    let wobble = ((tick % 20) as i8) - 10; // ±10
    RfStatus {
        state: 0x05, // Rx
        rssi: rssi_base.saturating_add(wobble),
        edv: rssi_base.saturating_add(wobble).saturating_add(5),
        agc_gcw: 23,
    }
}

/// Synthetic BBC status for dry-run mode.
fn synthetic_bbc(tick: u64, tx_count: u64) -> BbcStatus {
    BbcStatus {
        phy_type: 0, // FSK
        rxfl: 0,
        txfl: if tx_count > 0 { 64 } else { 0 },
        cnt: tick as u32,
    }
}

fn io_err<E: std::error::Error + Send + Sync + 'static>(e: E) -> io::Error {
    io::Error::other(e)
}
