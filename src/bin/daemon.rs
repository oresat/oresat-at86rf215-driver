//! AT86RF215 radio daemon.
//!
//! Bridges UDP or Unix-datagram sockets to the radio over SPI:
//!
//! - **TX socket** (default UDP `127.0.0.1:10020`; `--tx-bind <addr>` to bind a
//!   different address such as `0.0.0.0:10025`, or `--tx-uds <path>` for UDS):
//!   datagrams received here are transmitted by the RF09 sub-1 GHz transceiver.
//! - **RX socket** (default UDP `127.0.0.1:10021`; `--rx-peer <addr>` to forward
//!   to a different host, or `--rx-uds <path>` for UDS):
//!   frames received by the radio are forwarded here as datagrams.
//! - **Telemetry socket** (optional): periodic CBOR-encoded `CommState` snapshots
//!   for the TUI viewer. UDP only.
//!
//!
//! ## Running without hardware
//!
//! With `--dry-run` the daemon skips SPI/GPIO initialisation
//! Runs loopback: TX packets are echoed straight to the RX socket 

use std::{
    collections::VecDeque,
    io,
    io::IsTerminal,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
    time::{Duration, Instant},
};

use clap::Parser;
use mio::net::{UdpSocket, UnixDatagram};
use mio::{Events, Interest, Poll, Registry, Token};

use oresat_at86rf215_driver::{
    comm::{BbcStatus, CommState, RfStatus, RxPacket},
    freq::{Band, PllSettings},
    radio::Radio,
    registers::{BbcnTxfl, EnergyDetectionMode, RfnCmd, TransceiverCmd},
    spi::{self, Bbc},
    stats::RadioStats,
};

// -- mio tokens --------------------------------------------------------------

const TX_SOCKET: Token = Token(0);
const TIMER: Token = Token(1);
#[allow(dead_code)] // reserved for future mio-based signal source
const SIGNAL: Token = Token(2);
const GPIO_IRQ: Token = Token(3);

// -- CLI ---------------------------------------------------------------------

#[derive(Parser, Debug)]
#[command(
    name = "at86rf215-daemon",
    about = "AT86RF215 radio <-> UDP/UDS daemon"
)]
struct Args {
    /// UDP port to receive TX datagrams on (ignored when `--tx-uds` is set).
    #[arg(long, default_value = "10020")]
    tx_port: u16,

    /// UDP port to send RX frames to (ignored when `--rx-uds` is set).
    #[arg(long, default_value = "10021")]
    rx_port: u16,

    /// Full UDP bind address for the TX socket, example: "0.0.0.0:10025". Overrides
    /// `--tx-port` (which only ever binds 127.0.0.1). Use this to accept TX
    /// datagrams from another host (example: the daemon runs on a Pi while yamcs/C3
    /// run on a workstation). Ignored when `--tx-uds` is set.
    #[arg(long)]
    tx_bind: Option<String>,

    /// Full UDP destination for forwarded RX frames, example: "192.168.1.5:10016"
    /// (a hostname is resolved). Overrides `--rx-port` (which only ever targets
    /// 127.0.0.1). Use this to forward received frames to a consumer on another
    /// host. Ignored when `--rx-uds` is set.
    #[arg(long)]
    rx_peer: Option<String>,

    /// Unix-domain datagram path for TX input (overrides `--tx-port`).
    #[arg(long)]
    tx_uds: Option<PathBuf>,

    /// Unix-domain datagram path for RX output (overrides `--rx-port`).
    #[arg(long)]
    rx_uds: Option<PathBuf>,

    /// Optional telemetry destination (example: "127.0.0.1:10035").
    #[arg(long)]
    telemetry: Option<String>,

    /// SPI device path (example: /dev/spidev0.0).
    #[arg(long, default_value = "/dev/spidev0.0")]
    spi: String,

    /// SPI clock rate in Hz. Lower (example: 1_000_000) if the Pi's aux SPI
    /// (spidev1.x) misreads register values.
    #[arg(long, default_value_t = spi::DEFAULT_SPI_HZ)]
    spi_hz: u32,

    /// Run without hardware - TX packets are looped back as RX.
    #[arg(long)]
    dry_run: bool,

    /// Telemetry interval in milliseconds.
    #[arg(long, default_value = "100")]
    telemetry_ms: u64,

    /// GPIO chip path for the radio IRQ line.
    #[arg(long, default_value = "/dev/gpiochip0")]
    gpio_chip: String,

    /// GPIO line number for the radio IRQ (rising edge).
    #[arg(long, default_value = "25")]
    gpio_line: u32,

    /// TOML config file to load at startup.
    #[arg(long)]
    config: Option<String>,

    /// RF09 carrier frequency in Hz (sub-1 GHz).
    #[arg(long, default_value_t = 463_500_000)]
    freq: u64,

    /// SPI-Poll BBC0_IRQS on the telemetry tick instead of waiting on the GPIO IRQ line. 
    #[arg(long)]
    poll: bool,

    /// Enable the hardware FCS filter (BBC0_PC.FCSFE). OFF by default so that
    /// every completed frame raises RXFE and is logged even on a bad CRC -
    /// during bring-up a CRC/config mismatch otherwise looks identical to "no frame".
    #[arg(long)]
    fcs_filter: bool,

    /// Print radio state (rf09_state, pll.ls, rssi, accumulated BBC0_IRQS) once a second.
    #[arg(long)]
    verbose: bool,

    /// Disable listen-before-talk. By default a queued TX frame is held back
    /// while the radio reports a reception in progress (AGC held / frame start
    /// seen, no frame end yet) so an outgoing frame does not abort an inbound
    /// one on this half-duplex link.
    #[arg(long)]
    no_carrier_sense: bool,
}

// -- link timing --------------------------------------------------------------

/// Backstop for a lost TXFE: if the chip never signals transmit-complete within
/// this window, force the TX slot free (and re-arm Rx) so a single dropped IRQ
/// cannot hold the transmit queue forever.
const TX_COMPLETE_TIMEOUT: Duration = Duration::from_millis(250);

/// Backstop for a stuck carrier-sense flag: a false preamble or an aborted
/// reception can raise AGC-hold / RXFS without a matching RXFE/AGC-release, which
/// would otherwise pin `rx_busy` and block TX. Clear it after this long.
const RX_CARRIER_TIMEOUT: Duration = Duration::from_millis(500);

/// Bound the transmit backlog so a stalled link cannot grow it without limit.
const TX_QUEUE_MAX: usize = 256;

// -- link state ----------------------------------------------------------------

/// Half-duplex transmit scheduling state.
///
/// The radio is a single transceiver: it can transmit or receive, not both. The
/// old TX path drained every queued datagram and called `transmit_frame` for
/// each back-to-back, but `transmit_frame` only *starts* a transmission (issues
/// CMD=TX and returns) - so the next call's CMD=TXPREP aborted the previous
/// frame mid-air. A burst of Metadata+FileData+EOF therefore collapsed to just
/// the last frame on the air. This struct serialises transmission: at most one
/// frame is keyed at a time, the next is sent only after TXFE.
struct LinkState {
    /// Frames waiting to be transmitted, in arrival order.
    tx_queue: VecDeque<Vec<u8>>,
    /// A frame has been keyed (CMD=TX issued) and TXFE not yet observed.
    tx_busy: bool,
    /// When the in-flight TX was keyed (for `TX_COMPLETE_TIMEOUT`).
    tx_started: Instant,
    /// A reception is in progress (AGC held / RXFS seen, no RXFE/AGC-release yet).
    rx_busy: bool,
    /// When `rx_busy` was last asserted (for `RX_CARRIER_TIMEOUT`).
    rx_started: Instant,
}

impl LinkState {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            tx_queue: VecDeque::new(),
            tx_busy: false,
            tx_started: now,
            rx_busy: false,
            rx_started: now,
        }
    }

    /// Enqueue a frame for transmission, dropping it (with a warning) if the
    /// backlog is full rather than growing memory without bound.
    fn enqueue(&mut self, frame: &[u8]) {
        if self.tx_queue.len() >= TX_QUEUE_MAX {
            eprintln!(
                "warning: TX queue full ({} frames) - dropping {} B frame",
                TX_QUEUE_MAX,
                frame.len(),
            );
            return;
        }
        self.tx_queue.push_back(frame.to_vec());
    }
}

// -- socket abstractions ----------------------------------------------------
//
// The daemon accepts either UDP or Unix-datagram on both the TX (incoming)
// and RX (outgoing) sides. Two small enums dispatch across the two kinds.

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

    fn describe(&self, addr: &str, uds: &Option<PathBuf>) -> String {
        match (self, uds) {
            (TxListener::Uds(_), Some(p)) => format!("uds:{}", p.display()),
            _ => addr.to_string(),
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

    fn describe(&self, addr: &str, uds: &Option<PathBuf>) -> String {
        match (self, uds) {
            (RxSender::Uds(_), Some(p)) => format!("uds:{}", p.display()),
            _ => addr.to_string(),
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

// -- main --------------------------------------------------------------------

fn main() -> io::Result<()> {
    let args = Args::parse();

    // -- sockets ---------------------------------------------------------
    // TX bind: --tx-bind wins (any address), else 127.0.0.1:--tx-port.
    let tx_addr: SocketAddr = match args.tx_bind.as_ref() {
        Some(s) => s.parse().map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidInput, format!("--tx-bind {s}: {e}"))
        })?,
        None => SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), args.tx_port),
    };
    // RX peer: --rx-peer wins (host:port, hostname resolved at connect), else
    // 127.0.0.1:--rx-port. Kept as a string so connect() can resolve a hostname.
    let rx_peer: String = match args.rx_peer.as_ref() {
        Some(s) => s.clone(),
        None => format!("127.0.0.1:{}", args.rx_port),
    };

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
            sock.connect(rx_peer.as_str())?;
            RxSender::Udp(sock)
        }
    };

    let telemetry_sock = args.telemetry.as_ref().map(|dest| {
        let sock = std::net::UdpSocket::bind("0.0.0.0:0").expect("bind telemetry");
        sock.connect(dest).expect("connect telemetry");
        sock
    });

    // -- timer (periodic telemetry) --------------------------------------
    let mut tfd = timerfd::TimerFd::new().map_err(io_err)?;
    tfd.set_state(
        timerfd::TimerState::Periodic {
            current: Duration::from_secs(1),
            interval: Duration::from_millis(args.telemetry_ms),
        },
        timerfd::SetTimeFlags::Default,
    );

    // -- signal handling (SIGINT for clean shutdown) ---------------------
    use signal_hook::consts::SIGINT;
    let signal_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    signal_hook::flag::register(SIGINT, signal_flag.clone()).map_err(io_err)?;

    // -- poll registry ---------------------------------------------------
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

    // -- radio -----------------------------------------------------------
    let mut radio = Radio::new();
    let mut spidev: Option<spidev::Spidev> = if !args.dry_run {
        let dev = spi::open_with_speed(&args.spi, args.spi_hz)?;
        eprintln!("SPI opened: {} @ {} Hz", args.spi, args.spi_hz);
        Some(dev)
    } else {
        eprintln!("dry-run mode - no hardware");
        None
    };

    // -- TOML config (optional) ---------------------------------------
    if let Some(ref path) = args.config {
        let contents = std::fs::read_to_string(path)?;
        let config: oresat_at86rf215_driver::config::RadioConfig =
            toml::from_str(&contents).map_err(io::Error::other)?;
        radio.apply_config(&config);
        eprintln!("config loaded: {}", path);
    }

    // -- radio initialisation (hardware only) ---------------------------
    if let Some(ref mut dev) = spidev {
        init_radio(&mut radio, dev, args.freq, args.fcs_filter, args.verbose)?;
    }

    // -- GPIO IRQ (hardware only; skipped in --poll mode) ----------------
    let irq_req = if spidev.is_some() && !args.poll {
        match gpiocdev::Request::builder()
            .on_chip(&args.gpio_chip)
            .with_line(args.gpio_line)
            .with_edge_detection(gpiocdev::line::EdgeDetection::RisingEdge)
            .with_bias(gpiocdev::line::Bias::PullDown)
            .request()
        {
            Ok(req) => {
                let irq_fd = req.as_raw_fd();
                poll.registry().register(
                    &mut mio::unix::SourceFd(&irq_fd),
                    GPIO_IRQ,
                    Interest::READABLE,
                )?;
                let idle = req
                    .value(args.gpio_line)
                    .map(|v| if v == gpiocdev::line::Value::Active { "HIGH" } else { "LOW" })
                    .unwrap_or("?");
                eprintln!("GPIO IRQ: {}:{} (idle level: {})", args.gpio_chip, args.gpio_line, idle);
                Some(req)
            }
            Err(e) => {
                eprintln!("warning: failed to open GPIO IRQ ({}), RX disabled", e);
                None
            }
        }
    } else {
        if args.poll && spidev.is_some() {
            eprintln!("RX serviced by SPI polling on the timer tick (--poll)");
        }
        None
    };

    // -- event loop ------------------------------------------------------
    let mut events = Events::with_capacity(64);
    let mut pkt_buf = [0u8; 2048];
    let mut stats = RadioStats::new();
    let mut link = LinkState::new();

    // --verbose: accumulate BBC0_IRQS between status prints (reading IRQS
    // clears it, so RXFS/AGC events that arrive between the once-a-second
    // status line would otherwise be lost) and print state once a second.
    let mut acc_irqs: u8 = 0;
    let mut last_status = std::time::Instant::now();

    eprintln!(
        "listening: TX on {}, RX forwarded to {}",
        tx_sock.describe(&tx_addr.to_string(), &args.tx_uds),
        rx_sock.describe(&rx_peer, &args.rx_uds),
    );

    // Clear any IRQ pending from the RX-enable->IRQ-arm window before the loop.
    // The AT86RF215 IRQ pin is level-held: it stays asserted while any enabled
    // IRQS bit is pending and only drops when IRQS is read over SPI. If a frame
    // (or noise-triggered RXFS) latched the pin high before edge detection was
    // armed, the rising-edge watcher would never see a fresh edge and the
    // handler would never run to clear it - a deadlock that forces --poll. The
    // IRQ pin is the OR of all four blocks, so read every IRQS register (not
    // just BBC0) and loop until the line reads low.
    if let Some(ref req) = irq_req {
        if let Some(ref mut dev) = spidev {
            for pass in 0..16 {
                // Read all four status registers so a pending RF09/RF24/BBC1
                // source can't hold the shared pin high behind BBC0.
                let _ = spi::read_register(dev, &mut radio.rf09_irqs);
                let _ = spi::read_register(dev, &mut radio.rf24_irqs);
                let _ = spi::read_register(dev, &mut radio.bbc1_irqs);
                let _ =
                    service_radio_irqs(&mut radio, dev, &rx_sock, &telemetry_sock, &mut stats, &mut link);

                let high = req
                    .value(args.gpio_line)
                    .map(|v| v == gpiocdev::line::Value::Active)
                    .unwrap_or(false);
                if pass == 0 {
                    eprintln!(
                        "startup IRQS: RF09={:#04x} RF24={:#04x} BBC0={:#04x} BBC1={:#04x}",
                        u8::from(radio.rf09_irqs.value),
                        u8::from(radio.rf24_irqs.value),
                        u8::from(radio.bbc0_irqs.value),
                        u8::from(radio.bbc1_irqs.value),
                    );
                }
                if !high {
                    eprintln!("IRQ line cleared to LOW after {} pass(es)", pass + 1);
                    break;
                }
                if pass == 15 {
                    eprintln!(
                        "warning: IRQ line still HIGH after 16 clear passes - a source \
                         outside RF09/RF24/BBC0/BBC1 IRQS is holding it (RX edges will be missed)"
                    );
                }
            }
        }
    }

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
                    // Drain every queued datagram into the transmit backlog. Do
                    // NOT transmit inline: the radio sends one frame at a time and
                    // the next is keyed only after TXFE (see pump_tx / LinkState),
                    // so a burst is never collapsed by keying over an in-flight
                    // frame. The mio UDP source is edge-triggered, so the recv loop
                    // must run until WouldBlock or later datagrams are missed.
                    loop {
                        match tx_sock.recv(&mut pkt_buf) {
                            Ok(n) if n > 0 => {
                                let frame = &pkt_buf[..n];

                                // Confirm the local client -> daemon UDP/UDS hop:
                                // log every datagram accepted on the TX socket
                                // before it is queued for the radio.
                                let preview: String = frame
                                    .iter()
                                    .take(16)
                                    .map(|b| format!("{:02X}", b))
                                    .collect::<Vec<_>>()
                                    .join(" ");
                                let suffix = if frame.len() > 16 { "..." } else { "" };
                                eprintln!("TX socket: {} B from client [{}{}]", n, preview, suffix);

                                link.enqueue(frame);
                            }
                            Ok(_) => break,
                            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => break,
                            Err(e) => return Err(e),
                        }
                    }
                    pump_tx(
                        &mut link,
                        &mut radio,
                        &mut spidev,
                        &rx_sock,
                        &telemetry_sock,
                        &mut stats,
                        !args.no_carrier_sense,
                    )?;
                }

                TIMER => {
                    // Drain the timerfd so it doesn't fire again immediately.
                    let _ = tfd.read();
                    stats.tick();

                    // --poll mode: service the radio by reading BBC0_IRQS here
                    // instead of waiting on the GPIO line. The rate is
                    // telemetry interval (--telemetry-ms, default 100 ms).
                    if args.poll {
                        if let Some(ref mut dev) = spidev {
                            match service_radio_irqs(
                                &mut radio,
                                dev,
                                &rx_sock,
                                &telemetry_sock,
                                &mut stats,
                                &mut link,
                            ) {
                                Ok(irqs) => acc_irqs |= irqs,
                                Err(e) => eprintln!("poll service error: {}", e),
                            }
                        }
                    }

                    // --verbose: once a second, dump receiver state so
                    // no RXFS (flat noise floor) is distinguishable from
                    // "frames arrive but fail CRC" (RXFS/RXFE set in acc_irqs).
                    if args.verbose && last_status.elapsed() >= Duration::from_secs(1) {
                        if let Some(ref mut dev) = spidev {
                            let _ = spi::read_register(dev, &mut radio.rf09_state);
                            let _ = spi::read_register(dev, &mut radio.rf09_pll);
                            let _ = spi::read_register(dev, &mut radio.rf09_rssi);
                            eprintln!(
                                "[status] state={:?} pll.ls={} rssi={} dBm acc_irqs={:#04x} (rx={}, crc_fail={})",
                                radio.rf09_state.value.state(),
                                radio.rf09_pll.value.ls(),
                                radio.rf09_rssi.value.rssi(),
                                acc_irqs,
                                stats.rx_count,
                                stats.rx_crc_fail,
                            );
                        }
                        acc_irqs = 0;
                        last_status = std::time::Instant::now();
                    }

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
                            let _ =
                                CommState::Bbc0Status(synthetic_bbc(stats.ticks, stats.tx_count))
                                    .send(ts);
                        }
                        let _ = CommState::Stats(stats).send(ts);
                    }

                    // Backstops for lost IRQs, then service the transmit queue.
                    // The tick is the only wakeup when nothing arrives on the TX
                    // socket and no RF IRQ fires, so a frame deferred by carrier
                    // sense (or stalled behind a lost TXFE) is retried here.
                    if link.tx_busy && link.tx_started.elapsed() > TX_COMPLETE_TIMEOUT {
                        eprintln!(
                            "warning: TXFE not seen in {:?} - forcing TX ready (frame may be lost)",
                            TX_COMPLETE_TIMEOUT,
                        );
                        link.tx_busy = false;
                        // The chip should have auto-dropped to TxPrep; re-arm Rx
                        // so a wedged transmit cannot leave the receiver off.
                        if let Some(ref mut dev) = spidev {
                            radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::Rx);
                            let _ = spi::write_register(dev, &radio.rf09_cmd);
                        }
                    }
                    if link.rx_busy && link.rx_started.elapsed() > RX_CARRIER_TIMEOUT {
                        // Stale carrier (false preamble / aborted frame): release
                        // the guard so it cannot pin TX indefinitely.
                        link.rx_busy = false;
                    }
                    pump_tx(
                        &mut link,
                        &mut radio,
                        &mut spidev,
                        &rx_sock,
                        &telemetry_sock,
                        &mut stats,
                        !args.no_carrier_sense,
                    )?;
                }

                GPIO_IRQ => {
                    // Drain all pending edge events from the GPIO line.
                    if let Some(ref req) = irq_req {
                        while req.has_edge_event().unwrap_or(false) {
                            let _ = req.read_edge_event();
                        }
                    }

                    if let Some(ref mut dev) = spidev {
                        // Service repeatedly until the IRQ line reads inactive.
                        // The pin is level-held and read-to-clear: an event that
                        // arrives while we are mid-service re-asserts the line
                        // without producing a new rising edge, so a single pass
                        // could leave the pin stuck high and miss every later
                        // frame. Re-read the line after each pass and loop while
                        // it is still asserted (capped to bound a runaway).
                        for _ in 0..16 {
                            match service_radio_irqs(
                                &mut radio,
                                dev,
                                &rx_sock,
                                &telemetry_sock,
                                &mut stats,
                                &mut link,
                            ) {
                                Ok(irqs) => acc_irqs |= irqs,
                                Err(e) => eprintln!("IRQ service error: {}", e),
                            }

                            let still_high = irq_req
                                .as_ref()
                                .and_then(|req| req.value(args.gpio_line).ok())
                                .map(|v| v == gpiocdev::line::Value::Active)
                                .unwrap_or(false);
                            if !still_high {
                                break;
                            }
                        }
                    }

                    // A reception just ended (rx_busy cleared) and/or a TX
                    // completed (TXFE -> tx_busy cleared): service the queue now
                    // so a frame held back by carrier sense goes out immediately.
                    pump_tx(
                        &mut link,
                        &mut radio,
                        &mut spidev,
                        &rx_sock,
                        &telemetry_sock,
                        &mut stats,
                        !args.no_carrier_sense,
                    )?;
                }

                _ => {}
            }
        }
    }
}

// -- radio init -------------------------------------------------------------

/// Initialise the AT86RF215 for operation.
///
/// Sequence:
/// 1. Chip reset (write 0x7 to RF_RST).
/// 2. Wait for the chip to come out of reset (~1 ms typ.).
/// 3. Read RF_PN / RF_VN to verify SPI communication.
/// 4. Write all configurable registers (from TOML config or defaults).
/// 5. Enable BBC0 baseband with auto-FCS append and FCS filter.
/// 6. Enable RXFE (+ optionally RXFS) and TXFE interrupts in BBCn_IRQM.
/// 7. Transition RF09 from TrxOff -> TxPrep -> Rx.
fn init_radio(
    radio: &mut Radio,
    dev: &mut spidev::Spidev,
    freq_hz: u64,
    fcs_filter: bool,
    rxfs_irq: bool,
) -> io::Result<()> {
    // 1-2. Chip reset + identity check.
    let (pn, vn) = spi::reset_and_identify(dev, radio)?;
    eprintln!("chip: {:?} v{}", pn, vn);

    // 3. Write configurable registers to SPI (values set by --config or defaults).
    //    Uses BulkWrites to combine contiguous registers into fewer SPI transactions.
    write_rf09_config(radio, dev)?;
    write_bbc0_config(radio, dev)?;

    // 3b. Program the RF09 channel/PLL from --freq. The TOML config deliberately
    //     leaves CS/CCF0/CN unset (they default to 0 = reset channel), so this
    //     must run or both ends sit at the wrong frequency and never link.
    //     Overwrites the cs/ccf0/cn just flushed by write_rf09_config - intended.
    let pll = PllSettings::fine(Band::Sub1GHz, freq_hz).map_err(io::Error::other)?;
    eprintln!(
        "frequency: {} Hz (CCF0={}, CN={}, CS={})",
        freq_hz, pll.ccf0, pll.cn, pll.cs,
    );
    spi::apply_channel_rf09(dev, radio, pll)?;

    // 4. Enable BBC0 baseband + auto-FCS + FCS filter.
    //    These are layered on top of whatever the TOML config set.
    radio.bbc0_pc.value = radio
        .bbc0_pc
        .value
        .with_bben(true)
        .with_txafcs(true)
        .with_fcsfe(fcs_filter);
    spi::write_register(dev, &radio.bbc0_pc)?;
    eprintln!("FCS filter: {}", if fcs_filter { "on" } else { "off (bad-CRC frames still raise RXFE)" });

    // 5. Enable RXFE + TXFE interrupts, plus AGC hold/release (AGCH/AGCR) which
    //    drive carrier sense: AGCH fires the instant the receiver locks onto an
    //    inbound signal and AGCR when it lets go, bracketing a reception so the
    //    TX scheduler can hold a queued frame rather than key over it on this
    //    half-duplex link. RXFS (frame start) is enabled only when asked
    //    (rxfs_irq, set by --verbose): it lets the diagnostic show the receiver
    //    detected a preamble/PHR even when no complete frame (RXFE) follows, but
    //    it also fires on noise/false preambles, so leave it off for normal RX
    //    (AGCH already covers the carrier-sense window).
    radio.bbc0_irqm.value = radio
        .bbc0_irqm
        .value
        .with_rxfs(rxfs_irq)
        .with_rxfe(true)
        .with_txfe(true)
        .with_agch(true)
        .with_agcr(true);
    spi::write_register(dev, &radio.bbc0_irqm)?;

    // 5b. Energy detection AUTO: latches an ED measurement into RF09_EDV while
    //     the AGC is held during reception. RF09_RSSI reads 0x7F (invalid) once
    //     the chip drops to TxPrep at frame-end, so the per-frame level must
    //     come from EDV.
    radio.rf09_edc.value = radio.rf09_edc.value.with_edm(EnergyDetectionMode::Auto);
    spi::write_register(dev, &radio.rf09_edc)?;

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
    use oresat_at86rf215_driver::registers::BulkWrites;
    use std::io::Write;

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
    use oresat_at86rf215_driver::registers::BulkWrites;
    use std::io::Write;

    let mut bw = BulkWrites::new();
    bw.add(&mut radio.bbc0_fskc0);
    bw.add(&mut radio.bbc0_fskc1);
    bw.add(&mut radio.bbc0_fskc2);
    bw.add(&mut radio.bbc0_fskc3);
    bw.add(&mut radio.bbc0_fskc4);
    bw.add(&mut radio.bbc0_fskpll);
    bw.add(&mut radio.bbc0_fskphrtx);
    bw.add(&mut radio.bbc0_fskdm); // FSK direct modulation (TX); paired with TXDFE.DM
    bw.add(&mut radio.bbc0_ofdmphrtx);
    bw.add(&mut radio.bbc0_ofdmc);
    bw.add(&mut radio.bbc0_oqpskc0);
    for cmd in bw.generate_commands() {
        dev.write_all(&cmd)?;
    }
    Ok(())
}

// -- TX path -----------------------------------------------------------------

/// Write a frame to the TX FIFO and issue the TX command.
///
/// Sequence (datasheet 5.1.5):
/// 1. Transition to TxPrep (required before loading the FIFO).
/// 2. Write frame data to the TX frame buffer (BBC0_FBTXS).
/// 3. Write frame length to BBCn_TXFL.
/// 4. Issue CMD=TX to start transmission.
///
/// The chip auto-transitions Tx -> TxPrep when the frame ends, firing
/// a TXFE interrupt.  The GPIO_IRQ handler re-enters Rx on TXFE,
/// so this function does **not** issue CMD=Rx itself.
fn transmit_frame(radio: &mut Radio, dev: &mut spidev::Spidev, frame: &[u8]) -> io::Result<()> {
    // 1. Transition to TxPrep and wait for the PLL to relock. Issuing CMD=TX
    //    before TxPrep+PLL-lock risks transmitting before the
    //    synthesizer is settled - TXFE may never fire.
    radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::TxPrep);
    spi::write_register(dev, &radio.rf09_cmd)?;
    spi::wait_rf09_txprep_locked(dev, radio, Duration::from_millis(5))?;

    // 2. Reserve room for the FCS. With BBC0_PC.TXAFCS=1 the chip overwrites
    //    the last fcs_len octets of the frame buffer with the computed FCS, and
    //    TXFL counts those octets (datasheet 6.13.3). Without the reservation
    //    the chip clobbers the last fcs_len bytes of real payload. 
    //    FCST=0 => 32-bit FCS (4 octets); =1 => 16-bit (2).
    let fcs_len = if radio.bbc0_pc.value.fcst() { 2 } else { 4 };
    let mut buf = frame.to_vec();
    buf.resize(frame.len() + fcs_len, 0x00); // FCS placeholder

    // 3. Write frame data (+ FCS placeholder) to TX buffer.
    spi::write_tx_fifo(dev, Bbc::Bbc0, &buf)?;

    // 4. Set TX frame length register (payload + FCS).
    radio.bbc0_txfl.value = BbcnTxfl::new().with_txfl(buf.len() as u16);
    spi::write_register(dev, &radio.bbc0_txfl)?;

    // 5. Issue TX command.
    radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::Tx);
    spi::write_register(dev, &radio.rf09_cmd)?;

    Ok(())
}

/// Service the transmit backlog: key the next queued frame when the radio is
/// free to send it.
///
/// On hardware this keys **at most one** frame per call and then returns - the
/// next is sent only after TXFE clears `tx_busy` (via `service_radio_irqs`). A
/// frame is held back while `tx_busy` (a transmission is in flight) or, unless
/// `carrier_sense` is false, while `rx_busy` (a reception is in progress), so an
/// outgoing frame never aborts an in-flight one on this half-duplex link.
///
/// In dry-run (no SPI device) there is no half-duplex constraint, so the whole
/// backlog is looped straight back to the RX socket.
fn pump_tx(
    link: &mut LinkState,
    radio: &mut Radio,
    spidev: &mut Option<spidev::Spidev>,
    rx_sock: &RxSender,
    telemetry_sock: &Option<std::net::UdpSocket>,
    stats: &mut RadioStats,
    carrier_sense: bool,
) -> io::Result<()> {
    match spidev.as_mut() {
        Some(dev) => {
            // One frame in flight at a time; wait for TXFE before the next.
            if link.tx_busy {
                return Ok(());
            }
            // Listen before talk: hold the queue while a reception is underway.
            if carrier_sense && link.rx_busy {
                return Ok(());
            }
            let Some(frame) = link.tx_queue.pop_front() else {
                return Ok(());
            };
            match transmit_frame(radio, dev, &frame) {
                Ok(()) => {
                    link.tx_busy = true;
                    link.tx_started = Instant::now();
                    stats.record_tx();
                    eprintln!(
                        "TX: {} B keyed on RF09 ({} queued)",
                        frame.len(),
                        link.tx_queue.len(),
                    );
                }
                Err(e) => {
                    // Leave tx_busy false so the next pump retries the queue; the
                    // dropped frame is not requeued (CFDP will retransmit).
                    stats.record_tx_error();
                    eprintln!("TX error: {}", e);
                }
            }
            Ok(())
        }
        None => {
            // Dry-run loopback: no radio, no half-duplex limit - drain it all.
            while let Some(frame) = link.tx_queue.pop_front() {
                stats.record_tx();
                stats.record_rx(-40);
                let _ = rx_sock.send(&frame);
                if let Some(ts) = telemetry_sock {
                    let pkt = RxPacket {
                        data: frame,
                        rssi: -40,
                        edv: -40,
                    };
                    let _ = CommState::Rx(pkt).send(ts);
                }
            }
            Ok(())
        }
    }
}

// -- IRQ servicing ---------------------------------------------------------

/// Read and clear BBC0_IRQS, then act on it. Shared by the GPIO-IRQ path and
/// the `--poll` timer path. Also maintains the half-duplex `LinkState` flags so
/// the caller's `pump_tx` knows when it is safe to key a queued frame.
///
/// - **AGC hold / RXFS**: a reception started - mark the channel busy so
///   carrier sense holds back any queued TX (don't talk over an inbound frame).
/// - **TXFE**: transmission finished (chip auto-dropped to TxPrep) - free the TX
///   slot and re-enter Rx.
/// - **RXFE**: a frame arrived - read it out, forward it, then re-arm Rx. The
///   chip leaves Rx for TxPrep after a received frame, so without the re-arm
///   the daemon would go deaf after the first packet.
/// - **RXFE / AGC release**: the reception ended - mark the channel free.
fn service_radio_irqs(
    radio: &mut Radio,
    dev: &mut spidev::Spidev,
    rx_sock: &RxSender,
    telemetry_sock: &Option<std::net::UdpSocket>,
    stats: &mut RadioStats,
    link: &mut LinkState,
) -> io::Result<u8> {
    // A single read captures every event (AGC/RXFS/RXFE/TXFE) before clearing.
    spi::read_register(dev, &mut radio.bbc0_irqs)?;
    let irqs = radio.bbc0_irqs.value;
    let raw_irqs = u8::from(irqs);

    // Carrier sense: AGC held or a frame start means a reception is in progress.
    // Set this before the clear below so a single poll-mode read that captures
    // both start and end nets out to "free" (cleared last, see RXFE/AGCR).
    if irqs.agch() || irqs.rxfs() {
        link.rx_busy = true;
        link.rx_started = Instant::now();
    }

    // TXFE: transmission complete - free the TX slot and re-enter Rx.
    if irqs.txfe() {
        link.tx_busy = false;
        radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::Rx);
        spi::write_register(dev, &radio.rf09_cmd)?;
    }

    // RXFE: frame received - read it out and forward.
    if irqs.rxfe() {
        match receive_frame(radio, dev, stats) {
            Ok(Some(pkt)) => {
                stats.record_rx(pkt.rssi);
                let preview: String = pkt
                    .data
                    .iter()
                    .take(16)
                    .map(|b| format!("{:02X}", b))
                    .collect::<Vec<_>>()
                    .join(" ");
                let suffix = if pkt.data.len() > 16 { "..." } else { "" };
                eprintln!(
                    "{}",
                    color(
                        &format!(
                            "RX: {} B from RF09 (EDV={} dBm) -> rx socket [{}{}]",
                            pkt.data.len(),
                            pkt.edv,
                            preview,
                            suffix,
                        ),
                        "32",
                    ),
                );
                // Log forward failures instead of swallowing them. A connected
                // UDP socket whose peer (the nc listener) is not up yet returns
                // ECONNREFUSED here after an ICMP port-unreachable - the giveaway
                // that the rx-socket consumer was started too late or on the
                // wrong address.
                if let Err(e) = rx_sock.send(&pkt.data) {
                    eprintln!("{}", color(&format!("RX: forward to rx socket failed: {e}"), "31"));
                }
                if let Some(ts) = telemetry_sock {
                    let _ = CommState::Rx(pkt).send(ts);
                }
            }
            Ok(None) => {}
            Err(e) => eprintln!("RX error: {}", e),
        }

        // Re-arm Rx: the chip dropped to TxPrep when the frame completed.
        radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::Rx);
        spi::write_register(dev, &radio.rf09_cmd)?;
    }

    // Channel free again once the frame ended (RXFE) or the AGC released. Clear
    // last so a combined start+end read (poll mode) ends up "free".
    if irqs.rxfe() || irqs.agcr() {
        link.rx_busy = false;
    }

    Ok(raw_irqs)
}

// -- RX path ----------------------------------------------------------------

/// Read a received frame from the RX FIFO after RXFE has been confirmed.
///
/// The caller must have already read BBCn_IRQS and checked that RXFE is
/// set before calling this function.
///
/// Sequence (datasheet 5.1.4):
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
    // 1. Read FCS (CRC) validity, but defer the drop until after we have read
    //    the bytes - on a bad CRC we still want to log them so a real-but-
    //    corrupt frame is distinguishable from noise that merely tripped RXFE
    spi::read_register(dev, &mut radio.bbc0_pc)?;
    let fcsok = radio.bbc0_pc.value.fcsok();

    // 2. Read frame buffer level (received frame length).
    spi::read_register(dev, &mut radio.bbc0_fbl)?;
    let len = radio.bbc0_fbl.value.fbl() as usize;
    if len == 0 {
        return Ok(None);
    }

    // 3. Read frame data from RX FIFO.
    let mut data = spi::read_rx_fifo(dev, Bbc::Bbc0, len)?;

    // 4. Capture RSSI/EDV. Report EDV, not RF09_RSSI: by frame-end the chip has
    //    dropped to TxPrep and RF09_RSSI reads 0x7F (127, "invalid"). EDV was
    //    latched during reception (EDC=AUTO) and still holds the real dBm value
    spi::read_register(dev, &mut radio.rf09_rssi)?;
    spi::read_register(dev, &mut radio.rf09_edv)?;
    let edv = radio.rf09_edv.value.edv();

    // 5. On a bad CRC, log the raw bytes + level so noise (random/garbled) is
    //    distinguishable from a real frame with a CRC/config mismatch, then drop.
    if !fcsok {
        stats.record_crc_fail();
        let preview: String = data
            .iter()
            .take(16)
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ");
        let suffix = if data.len() > 16 { "..." } else { "" };
        eprintln!(
            "{}",
            color(
                &format!("RX: frame dropped (bad CRC) len={len} EDV={edv} dBm [{preview}{suffix}]"),
                "31",
            ),
        );
        return Ok(None);
    }

    // 6. Strip the trailing FCS. BBCn_FBL/len includes the FCS field
    //    (datasheet 6.13.3); forwarding it would hand the application 4 extra
    //    (or 2) bytes of CRC as if they were payload. FCST=0
    //    => 32-bit FCS (4 octets); =1 => 16-bit (2).
    let fcs_len = if radio.bbc0_pc.value.fcst() { 2 } else { 4 };
    data.truncate(data.len().saturating_sub(fcs_len));

    Ok(Some(RxPacket {
        data,
        rssi: edv,
        edv,
    }))
}

// -- telemetry snapshots -----------------------------------------------------

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
    let wobble = ((tick % 20) as i8) - 10; // +/- 10
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

/// Wrap `text` in an ANSI color (`"31"` red, `"32"` green) when stderr is a
/// terminal. Returns it unchanged when stderr is piped/redirected, so log
/// files don't fill with escape codes.
fn color(text: &str, code: &str) -> String {
    if std::io::stderr().is_terminal() {
        format!("\x1b[{code}m{text}\x1b[0m")
    } else {
        text.to_string()
    }
}
