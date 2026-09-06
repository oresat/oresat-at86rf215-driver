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
//! - **Beacon socket** (opt-in; enable with `--beacon-port` / `--beacon-bind` /
//!   `--beacon-uds` or a `[beacon]` config table - default bind UDP
//!   `127.0.0.1:10015`, `--no-beacon` or
//!   `[beacon].enabled = false` forces it off): a second TX input.
//! - **Telemetry socket** (optional): periodic CBOR-encoded `CommState` snapshots
//!   for the TUI viewer. UDP only.
//! - **RSSI reporting** (optional, `--rssi-peer <addr>`): the daemon pushes the latest
//!   RSSI as a single raw int8 dBm byte (127 = invalid) to that UDP destination
//!   after every received frame.
//!
//! The beacon port, RSSI peer, and their enable flags can also be set
//! from the `--config` TOML (`[beacon]` / `[rssi]` tables) a CLI flag overrides
//! the TOML value.
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

use crate::{
    comm::{BbcStatus, CommState, RfStatus, RxPacket},
    freq::{Band, PllSettings},
    radio::Radio,
    registers::{
        BbcnTxfl, DevicePartNumber, EnergyDetectionMode, RfClko, RfnCmd, RfnIrqm, 
        TransceiverCmd, TransceiverState,
    },
    spi::{self, Bbc},
    stats::RadioStats,
};

/// Maximum RF frame the chip can buffer/transmit: BBCn_TXFL/FBL are 11-bit, so
/// payload + FCS must not exceed 2047.
const MAX_RF_FRAME: usize = 2047;

// -- mio tokens --------------------------------------------------------------

const TX_SOCKET: Token = Token(0);
const TIMER: Token = Token(1);
#[allow(dead_code)] // reserved for future mio-based signal source
const SIGNAL: Token = Token(2);
const GPIO_IRQ: Token = Token(3);
const BEACON_SOCKET: Token = Token(4);


// Profile - Which radio card the daemon drives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Profile {
    // UHF half duplex.
    Uhf,
    // L-Band receive only.
    Lband,
}

impl Profile {
    // L-Band can not transmit.
    const fn can_transmit(self) -> bool {
        matches!(self, Profile::Uhf)
    }

    // Clock is only on for Lband.
    const fn default_clko_os(self) -> u8 {
        match self {
            Profile::Uhf => 0,
            Profile::Lband => 3,
        }
    }

    // Adjust gain for L-Band, as increased gain compared to input.
    const fn default_rssi_offset_db(self) -> i16 {
        match self {
            Profile::Uhf => 0,
            Profile::Lband => -23,
        }
    }

    // Default frequency for each UHF and L-Band.
    const fn default_freq_hz(self) -> u64 {
        match self {
            Profile::Uhf => 436_500_000,
            Profile::Lband => 457_000_000,
        }
    }
}

// -- CLI ---------------------------------------------------------------------

#[derive(Parser, Debug)]
#[command(
    name = "daemon",
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

    /// UDP port to receive beacon datagrams on (default bind 10015, the OreSat
    /// C3 beacon-downlink). Setting this (or any `--beacon-*` flag /
    /// `[beacon]` config field) enables the optional (default off) beacon socket.
    /// Overrides `[beacon].port` in `--config`. Ignored when a beacon UDS path
    /// is set.
    #[arg(long)]
    beacon_port: Option<u16>,

    /// Full UDP bind address for the beacon socket, example: "0.0.0.0:10015".
    /// Overrides `--beacon-port` (which only ever binds 127.0.0.1). Overrides
    /// `[beacon].bind` in `--config`. Ignored when a beacon UDS path is set.
    #[arg(long)]
    beacon_bind: Option<String>,

    /// Unix-domain datagram path for beacon input (overrides
    /// `--beacon-port`/`--beacon-bind` and `[beacon]` in `--config`).
    #[arg(long)]
    beacon_uds: Option<PathBuf>,

    /// Disable the beacon socket (overrides `[beacon].enabled` in `--config`).
    #[arg(long)]
    no_beacon: bool,

    /// Optional telemetry destination (example: "127.0.0.1:10035").
    #[arg(long)]
    telemetry: Option<String>,

    /// UDP destination for the per-frame RSSI push, example: "127.0.0.1:10030".
    /// The satellite C3 reads this single int8 dBm byte (127 = invalid), sent
    /// after every received frame. Overrides `[rssi].peer` in `--config`.
    #[arg(long)]
    rssi_peer: Option<String>,

    /// Disable the RSSI push (overrides `[rssi].enabled` in `--config`).
    #[arg(long)]
    no_rssi: bool,

    /// SPI device path (default /dev/spidev0.0). Overrides `[spi].dev`.
    #[arg(long)]
    spi: Option<String>,

    /// SPI clock rate in Hz (default 1_000_000). Lower if the Pi's aux SPI
    /// (spidev1.x) misreads register values. Overrides `[spi].hz`.
    #[arg(long)]
    spi_hz: Option<u32>,

    /// Run without hardware - TX packets are looped back as RX.
    #[arg(long)]
    dry_run: bool,

    /// Which radio to drive.
    /// `uhf` = the TX+RX UHF.
    /// `lband` = the receive only 1.265 GHz.
    /// L-Band gates transmission, the CLKO default and RSSI offset.
    #[arg(long, value_enum)]
    profile: Option<Profile>,

    /// L-Band: frequency of the uplink.
    #[arg(long)]
    rf_hz: Option<u64>,

    /// L-Band: Si4112 synthesizer, oscillator frequency.
    #[arg(long, default_value_t = 808_000_000)]
    lo_hz: u64,

    /// RF_CLKO.OS: 0=off, 1=26MHz, 2=32MHz, 3=16 MHz, 4=8, 5=4, 6=2, 7=1.
    /// Defaults to 3 for profile `lband` and 0 for `uhf`.
    #[arg(long)]
    clko_os: Option<u8>,

    /// dB added to the reported RSSI/EDV.
    /// Defaults to -23 for L-Band, 0 for UHF. 
    #[arg(long)]
    rssi_offset_db: Option<i16>,

    /// Telemetry interval in milliseconds.
    #[arg(long, default_value = "100")]
    telemetry_ms: u64,

    /// GPIO chip path for the radio IRQ line (default /dev/gpiochip0).
    /// Overrides `[gpio].chip`.
    #[arg(long)]
    gpio_chip: Option<String>,

    /// GPIO line number for the radio IRQ, rising edge (default 25).
    /// Overrides `[gpio].line`.
    #[arg(long)]
    gpio_line: Option<u32>,

    /// TOML config file to load at startup.
    #[arg(long)]
    config: Option<String>,

    /// RF09 carrier frequency in Hz (sub-1 GHz).
    #[arg(long)]
    freq: Option<u64>,

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

/// How often the idle health backstop is allowed to act on a wedged radio.
const HEALTH_ACTION_INTERVAL: Duration = Duration::from_secs(1);

/// Consecutive bad health checks (each `HEALTH_ACTION_INTERVAL` apart) the radio
/// must be wedged before escalating from a light re-arm to a full re-init.
const HEALTH_REINIT_AFTER: u32 = 3;

/// Full re-init backoff: start here, double on each failed re-init up to
/// [`REINIT_BACKOFF_MAX`], reset on recovery. Stops reset-spamming a chip that
/// cannot come up (e.g. inadequate supply) while still recovering quickly from a
/// one-off fault.
const REINIT_BACKOFF_MIN: Duration = Duration::from_secs(2);
const REINIT_BACKOFF_MAX: Duration = Duration::from_secs(60);

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

// -- run --------------------------------------------------------------------

pub fn run(default_profile: Profile) -> io::Result<()> {
    let args = Args::parse();
    let profile = args.profile.unwrap_or(default_profile);

    // Identify the deployed binary up front: BUILD_ID is git sha (+ "-dirty")
    // and a build timestamp, stamped by build.rs. On a satellite with no console
    // this is the only way to confirm which image is actually running.
    eprintln!("{:?} daemon build {}", profile, env!("BUILD_ID"));

    // L-Band is receive only, refuse transmission flags.
    // This also blocks flags for beacon queues for transmission.
    if !profile.can_transmit() {
        for (flag, set) in [
            ("--tx-bind", args.tx_bind.is_some()),
            ("--tx-uds", args.tx_uds.is_some()),
            ("--beacon-port", args.beacon_port.is_some()),
            ("--beacon-bind", args.beacon_bind.is_some()),
            ("--beacon-uds", args.beacon_uds.is_some()),
        ] {
            if set {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("{flag} is not valid with --profile lband (receive only)"),
                ));
            }
        }
    }

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

    let mut tx_sock: Option<TxListener> = if profile.can_transmit() {
        Some(match args.tx_uds.as_ref() {
            Some(path) => {
                remove_stale_uds(path)?;
                TxListener::Uds(UnixDatagram::bind(path)?)
            }
            None => TxListener::Udp(UdpSocket::bind(tx_addr)?),
        })
    } else {
        None
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

    // -- signal handling (SIGINT/SIGTERM for clean shutdown) -------------
    // SIGTERM is what a supervisor (OreSat C3/olaf, systemd) sends to stop or
    // restart the daemon, so it must trigger the same radio-safe shutdown as a
    // console SIGINT - otherwise a routine restart leaves the PA/front-end keyed.
    use signal_hook::consts::{SIGINT, SIGTERM};
    let signal_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    signal_hook::flag::register(SIGINT, signal_flag.clone()).map_err(io_err)?;
    signal_hook::flag::register(SIGTERM, signal_flag.clone()).map_err(io_err)?;

    // -- poll registry ---------------------------------------------------
    let mut poll = Poll::new()?;
    if let Some(ref mut t) = tx_sock {
        t.register(poll.registry(), TX_SOCKET)?;
    }

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

    // -- TOML config (optional) ---------------------------------------
    // One reader for both registers and beacon/RSSI/SPI/GPIO settings. Read
    // BEFORE opening SPI so [spi]/[gpio] can choose the device the daemon binds.
    let mut net = crate::config::NetConfig::default();
    if let Some(ref path) = args.config {
        let contents = std::fs::read_to_string(path)?;
        // Fail loud on any typos in a table name. RadioConfig has no deny_unknown_fields
        // (the register and net passes share the file), so a misspelt table like
        // `[rf09_rxdfee]` would otherwise be silently dropped -> a deaf radio.
        crate::config::check_known_tables(&contents)
            .map_err(|m| io::Error::new(io::ErrorKind::InvalidData, m))?;
        let config: crate::config::RadioConfig =
            toml::from_str(&contents).map_err(io::Error::other)?;
        // An out-of-range register value panics inside the bitfield builders;
        // contain it so a bad config is a clean error (and a supervisor restart)
        // rather than an unwinding abort. Startup-only, so the cost is irrelevant.
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| radio.apply_config(&config)))
            .map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "config contains an out-of-range register field value \
                     (a bitfield setter panicked) - check the offending value against the datasheet",
                )
            })?;
        net = toml::from_str(&contents).map_err(io::Error::other)?;

        eprintln!("config loaded: {}", path);
    }

    let clko_os = args
        .clko_os
        .or(net.frontend.clko_os)
        .unwrap_or_else(|| profile.default_clko_os());
    let rssi_offset_db = args
        .rssi_offset_db
        .or(net.frontend.rssi_offset_db)
        .unwrap_or_else(|| profile.default_rssi_offset_db());

    if args.config.is_some() {
        // Reject a config that would boot a deaf/dead radio (PT/RXDFE.SR hard
        // errors; AGCC/PADFE warnings). Only enforced when a config is supplied -
        // bench/dry-run runs with built-in defaults are unaffected.
        validate_radio_for_flight(&radio, profile);
        if let Err(m) = flight_blocking_problems(&radio, profile, clko_os) {
            return Err(io::Error::new(io::ErrorKind::InvalidData, m));
        }
    }

    // -- SPI + GPIO device settings (CLI flag > TOML > default) ----------
    let spi_dev = args
        .spi
        .clone()
        .or(net.spi.dev.clone())
        .unwrap_or_else(|| "/dev/spidev0.0".to_string());
    let spi_hz = args.spi_hz.or(net.spi.hz).unwrap_or(spi::DEFAULT_SPI_HZ);
    let gpio_chip = args
        .gpio_chip
        .clone()
        .or(net.gpio.chip.clone())
        .unwrap_or_else(|| "/dev/gpiochip0".to_string());
    let gpio_line = args.gpio_line.or(net.gpio.line).unwrap_or(25);

    let mut spidev: Option<spidev::Spidev> = if !args.dry_run {
        let dev = spi::open_with_speed(&spi_dev, spi_hz)?;
        eprintln!("SPI opened: {} @ {} Hz", spi_dev, spi_hz);
        Some(dev)
    } else {
        eprintln!("dry-run mode - no hardware");
        None
    };

    // -- beacon + RSSI socket settings (CLI flag > TOML > default) -------
    // Beacon bind: --beacon-bind overides (any address), else 127.0.0.1:<port>,
    // where the port is --beacon-port, else [beacon].port, else 10015.
    let beacon_bind = args.beacon_bind.clone().or(net.beacon.bind.clone());
    let beacon_uds = args.beacon_uds.clone().or(net.beacon.uds.clone());
    let beacon_port = args.beacon_port.or(net.beacon.port).unwrap_or(10015);
    // The beacon socket is optional: it binds only when explicitly configured
    // (a --beacon-* flag or any [beacon] field). 
    // CLI --no-beacon, or [beacon].enabled forces it off/on regardless.
    let beacon_configured = args.beacon_port.is_some()
        || args.beacon_bind.is_some()
        || args.beacon_uds.is_some()
        || net.beacon.port.is_some()
        || net.beacon.bind.is_some()
        || net.beacon.uds.is_some();
    let beacon_enabled = !args.no_beacon && net.beacon.enabled.unwrap_or(beacon_configured) && profile.can_transmit();
    let beacon_addr: SocketAddr = match beacon_bind.as_ref() {
        Some(s) => s.parse().map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidInput, format!("beacon bind {s}: {e}"))
        })?,
        None => SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), beacon_port),
    };

    let rssi_peer = args.rssi_peer.clone().or(net.rssi.peer.clone());
    // RSSI push is on when a peer is resolved and not disabled. An explicit
    // [rssi].enabled=true with no peer can't push - warn rather than silently no-op.
    if !args.no_rssi && net.rssi.enabled == Some(true) && rssi_peer.is_none() {
        eprintln!("warning: [rssi].enabled is true but no rssi peer set - RSSI push disabled");
    }
    let rssi_enabled =
        !args.no_rssi && net.rssi.enabled.unwrap_or(rssi_peer.is_some()) && rssi_peer.is_some();

    // -- beacon listener socket (optional) ------------------------------
    let mut beacon_sock: Option<TxListener> = if beacon_enabled {
        Some(match beacon_uds.as_ref() {
            Some(path) => {
                remove_stale_uds(path)?;
                TxListener::Uds(UnixDatagram::bind(path)?)
            }
            None => TxListener::Udp(UdpSocket::bind(beacon_addr)?),
        })
    } else {
        None
    };
    if let Some(ref mut b) = beacon_sock {
        b.register(poll.registry(), BEACON_SOCKET)?;
    }

    // -- RSSI push socket (optional) ------------------------------------
    let rssi_sock = rssi_enabled.then(|| {
        let sock = std::net::UdpSocket::bind("0.0.0.0:0").expect("bind rssi");
        // connect() on a UDP addr does not need a live peer; a down peer only
        // shows up as ECONNREFUSED on send (logged, non-fatal - see the
        // per-frame push after the event dispatch in the main loop).
        sock.connect(rssi_peer.as_ref().expect("rssi peer set when enabled"))
            .expect("connect rssi");
        sock
    });

    // Set frequency from argument values.
    let freq_hz = match (args.freq, args.rf_hz) {
        (Some(f), _) => f,
        (None, Some(rf)) => rf.checked_sub(args.lo_hz).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("--rf-hz {rf} is below --lo-hz {}", args.lo_hz),
            )
        })?,
        (None, None) => profile.default_freq_hz(),
    };
    if let Some(rf) = args.rf_hz {
        eprintln!(
            "downconverter: RF {} Hz - LO {} Hz = IF {} HZ",
            rf, args.lo_hz, freq_hz,
        );
    }

    

    // -- radio initialisation (hardware only) ---------------------------
    // Retry a few times.
    if let Some(ref mut dev) = spidev {
        let mut attempt = 0u32;
        loop {
            match init_radio(&mut radio, dev, freq_hz, args.fcs_filter, args.verbose, profile, clko_os) {
                Ok(()) => break,
                Err(e) if attempt < 2 => {
                    attempt += 1;
                    eprintln!("radio init failed (attempt {attempt}/3): {e} - retrying in 200 ms");
                    std::thread::sleep(Duration::from_millis(200));
                }
                Err(e) => return Err(e),
            }
        }
    }

    // RX servicing mode: GPIO edge IRQ, or SPI polling on the timer tick. Starts
    // from --poll and falls back to polling if the GPIO line cannot be opened.
    let mut poll_mode = args.poll;

    // -- GPIO IRQ (hardware only; skipped in --poll mode) ----------------
    let irq_req = if spidev.is_some() && !poll_mode {
        match gpiocdev::Request::builder()
            .on_chip(&gpio_chip)
            .with_line(gpio_line)
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
                    .value(gpio_line)
                    .map(|v| if v == gpiocdev::line::Value::Active { "HIGH" } else { "LOW" })
                    .unwrap_or("?");
                eprintln!("GPIO IRQ: {}:{} (idle level: {})", gpio_chip, gpio_line, idle);
                Some(req)
            }
            Err(e) => {
                // Don't go deaf: fall back to SPI polling on the timer tick so a
                // wiring/permission fault on the IRQ line still yields a working RX.
                eprintln!(
                    "warning: failed to open GPIO IRQ ({}), falling back to --poll (SPI polling)",
                    e,
                );
                poll_mode = true;
                None
            }
        }
    } else {
        if poll_mode && spidev.is_some() {
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

    // Consecutive health checks (HEALTH_ACTION_INTERVAL apart) the radio has been
    // found wedged. Drives the backstop (light re-arm -> backed-off full re-init).
    let mut health_bad: u32 = 0;
    let mut last_health_action = std::time::Instant::now();
    let mut last_reinit = std::time::Instant::now();
    let mut reinit_backoff = REINIT_BACKOFF_MIN;

    // Beacons received here are queued onto the same TX backlog as `tx_sock`.
    let mut beacon_count: u64 = 0;
    // RSSI is pushed after every received frame: a push fires whenever
    // stats.rx_count has advanced past this watermark (see end of event loop).
    let mut last_rssi_rx_count: u64 = 0;
    // Tracks whether the RSSI peer is currently unreachable, so a consumer that
    // is simply not up yet does not flood stderr with one error per push
    // log only the transitions into and out of the failed state.
    let mut rssi_peer_down = false;

    match tx_sock {
        Some(ref t) => eprintln!(
            "listening: TX on {}, RX forwarded to {}",
            t.describe(&tx_addr.to_string(), &args.tx_uds),
            rx_sock.describe(&rx_peer, &args.rx_uds),
        ),
        None => eprintln!(
            "listening: receive only, RX forwarded to {}",
            rx_sock.describe(&rx_peer, &args.rx_uds),
        ),
    }


    if let Some(ref b) = beacon_sock {
        eprintln!("listening: beacon on {}", b.describe(&beacon_addr.to_string(), &beacon_uds));
    }
    if rssi_sock.is_some() {
        eprintln!(
            "rssi push -> {} after every received frame",
            rssi_peer.as_deref().unwrap_or("?"),
        );
    }

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
                    service_radio_irqs(&mut radio, dev, &rx_sock, &telemetry_sock, &mut stats, &mut link, profile, rssi_offset_db);

                let high = req
                    .value(gpio_line)
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
        // Check for SIGINT/SIGTERM before blocking.
        if signal_flag.load(std::sync::atomic::Ordering::Relaxed) {
            eprintln!(
                "\nsignal received - shutting down (tx={}, rx={}, crc_fail={}, trxerr={}, batlow={}, reinits={}, beacons={}, ticks={})",
                stats.tx_count, stats.rx_count, stats.rx_crc_fail, stats.trxerr_count,
                stats.batlow_count, stats.radio_reinits, beacon_count, stats.ticks,
            );
            // Leave the radio safe: drop the external front-end and stop the PA so
            // a restart/stop never leaves it keyed.
            if let Some(ref mut dev) = spidev {
                radio_safe_shutdown(&mut radio, dev);
            }
            return Ok(());
        }

        // A signal interrupting poll() surfaces as ErrorKind::Interrupted; loop so
        // the shutdown check above runs. Other poll errors are logged, not fatal.
        match poll.poll(&mut events, Some(Duration::from_millis(250))) {
            Ok(()) => {}
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => {
                eprintln!("poll error: {e}");
                continue;
            }
        }

        for event in events.iter() {
            match event.token() {
                TX_SOCKET => {
                    if let Some(ref tx_sock) = tx_sock {
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
                                Err(e) => {
                                    // A transient socket error must not kill the daemon
                                    // (the supervisor would only restart into it). Log
                                    // and stop draining this readiness; retry next wake.
                                    eprintln!("TX socket recv error: {e}");
                                    break;
                                }
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
                            profile,
                        )?;
                    }
                }

                BEACON_SOCKET => {
                    // Beacons are a second TX input: drain them onto the SAME
                    // transmit backlog as the TX socket.
                    if let Some(ref beacon_sock) = beacon_sock {
                        loop {
                            match beacon_sock.recv(&mut pkt_buf) {
                                Ok(n) if n > 0 => {
                                    let frame = &pkt_buf[..n];
                                    let preview: String = frame
                                        .iter()
                                        .take(16)
                                        .map(|b| format!("{:02X}", b))
                                        .collect::<Vec<_>>()
                                        .join(" ");
                                    let suffix = if frame.len() > 16 { "..." } else { "" };
                                    eprintln!("BEACON socket: {} B from client [{}{}]", n, preview, suffix);
                                    beacon_count += 1;
                                    link.enqueue(frame);
                                }
                                Ok(_) => break,
                                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => break,
                                Err(e) => {
                                    eprintln!("beacon socket recv error: {e}");
                                    break;
                                }
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
                            profile,
                        )?;
                    }
                }

                TIMER => {
                    // Drain the timerfd so it doesn't fire again immediately.
                    let _ = tfd.read();
                    stats.tick();

                    // --poll mode: service the radio by reading BBC0_IRQS here
                    // instead of waiting on the GPIO line. The rate is
                    // telemetry interval (--telemetry-ms, default 100 ms).
                    if poll_mode {
                        if let Some(ref mut dev) = spidev {
                            match service_radio_irqs(
                                &mut radio,
                                dev,
                                &rx_sock,
                                &telemetry_sock,
                                &mut stats,
                                &mut link,
                                profile, 
                                rssi_offset_db,
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
                        // Leave 127 the specified "invalid" value.
                        let raw = radio.rf09_rssi.value.rssi();
                        stats.update_rssi(if raw == 127 {
                            127
                        } else {
                            (raw as i16).saturating_add(rssi_offset_db).clamp(-128, 126) as i8
                        });
                        // Health backstop. The chip should be receiving when idle.
                        // If it is parked in an unexpected non-operational state
                        // (TrxOff/Reset - e.g. a brownout reset or a TRXERR whose
                        // IRQ was missed) and we are not mid-TX, try a light re-arm;
                        // only after HEALTH_REINIT_AFTER consecutive failures do a
                        // full re-init, and back that off (REINIT_BACKOFF_*) so a
                        // chip held down by power/contention is not reset-spammed.
                        let state = radio.rf09_state.value.state();
                        let operational = matches!(
                            state,
                            TransceiverState::Rx
                                | TransceiverState::Tx
                                | TransceiverState::TxPrep
                                | TransceiverState::Transition
                        );
                        if operational || link.tx_busy {
                            if health_bad != 0 {
                                eprintln!("RF09 healthy again (state {state:?})");
                            }
                            health_bad = 0;
                            reinit_backoff = REINIT_BACKOFF_MIN;
                            // Operational again without a reflash => the BATLOW was
                            // a transient dip that did not reset the registers, so
                            // the config is trustworthy: drop the suspect flag.
                            stats.clear_batlow();
                        } else if last_health_action.elapsed() >= HEALTH_ACTION_INTERVAL {
                            last_health_action = std::time::Instant::now();
                            health_bad += 1;
                            // Light re-arm first so the receiver comes back fast, then
                            // verify the chip did not lose its register config to a
                            // brownout reset. A CMD=Rx restores the state machine but
                            // NOT the channel/PHY/front-end, so after a reset the chip
                            // reports "Rx" yet is deaf on the wrong channel. init_radio
                            // programs RF09_CCF0 to a non-zero channel center; reading
                            // back 0 means the chip reset to defaults and must be
                            // reflashed. BATLOW (read-to-clear, easily missed) is only a
                            // hint - the CCF0 read-back is authoritative and catches
                            // every reset whether or not its BATLOW edge was captured.
                            let state_ok = match ensure_receiving(&mut radio, dev) {
                                Ok(()) => {
                                    let _ = spi::read_register(dev, &mut radio.rf09_state);
                                    matches!(
                                        radio.rf09_state.value.state(),
                                        TransceiverState::Rx | TransceiverState::TxPrep
                                    )
                                }
                                Err(_) => false,
                            };
                            let config_intact = state_ok && {
                                let _ = spi::read_register(dev, &mut radio.rf09_ccf0);
                                radio.rf09_ccf0.value.ccf0() != 0
                            };
                            // Chip alive but registers wiped, or a BATLOW we did capture:
                            // either way the config is suspect and needs a reflash. A
                            // global chip reset clears every register at once, so an
                            // intact CCF0 proves no reset occurred (any BATLOW was a
                            // transient dip) - treat that as a clean recovery.
                            let config_suspect = (state_ok && !config_intact) || stats.batlow_pending;
                            if state_ok && config_intact {
                                eprintln!("RF09 re-armed from {state:?} to Rx");
                                stats.clear_batlow();
                                health_bad = 0;
                            } else if (config_suspect || health_bad >= HEALTH_REINIT_AFTER)
                                && last_reinit.elapsed() >= reinit_backoff
                            {
                                last_reinit = std::time::Instant::now();
                                let reason = if state_ok && !config_intact {
                                    "config reset to defaults (brownout)"
                                } else if stats.batlow_pending {
                                    "BATLOW (brownout) - config may be lost"
                                } else {
                                    "stuck"
                                };
                                eprintln!(
                                    "RF09 {reason} in {state:?} for {health_bad}s - full re-init (backoff {:?})",
                                    reinit_backoff,
                                );
                                match init_radio(
                                    &mut radio,
                                    dev,
                                    freq_hz,
                                    args.fcs_filter,
                                    args.verbose,
                                    profile,
                                    clko_os,
                                ) {
                                    Ok(()) => {
                                        stats.record_reinit();
                                        // Config restored - the radio is trustworthy again.
                                        stats.clear_batlow();
                                        link.tx_busy = false;
                                        link.rx_busy = false;
                                        health_bad = 0;
                                        reinit_backoff = REINIT_BACKOFF_MIN;
                                    }
                                    Err(e) => {
                                        eprintln!("RF09 re-init failed: {e}");
                                        reinit_backoff =
                                            (reinit_backoff * 2).min(REINIT_BACKOFF_MAX);
                                    }
                                }
                            } else {
                                eprintln!("warning: RF09 in unexpected state {state:?} (x{health_bad})");
                            }
                        }
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
                        // Re-arm Rx state-aware: a wedged transmit can leave the
                        // chip in TxPrep OR TrxOff, and CMD=Rx is illegal from
                        // TrxOff, so route via TxPrep when needed (ensure_receiving).
                        if let Some(ref mut dev) = spidev
                            && let Err(e) = ensure_receiving(&mut radio, dev)
                        {
                            eprintln!("TX-timeout Rx re-arm failed: {e}");
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
                        profile,
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
                                profile,
                                rssi_offset_db,
                            ) {
                                Ok(irqs) => acc_irqs |= irqs,
                                Err(e) => eprintln!("IRQ service error: {}", e),
                            }

                            let still_high = irq_req
                                .as_ref()
                                .and_then(|req| req.value(gpio_line).ok())
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
                        profile,
                    )?;
                }

                _ => {}
            }
        }

        // RSSI push to the satellite C3: after every received frame.
        if stats.rx_count != last_rssi_rx_count {
            last_rssi_rx_count = stats.rx_count;
            if let Some(ref rs) = rssi_sock {
                match rs.send(&encode_rssi(&stats)) {
                    Ok(_) => {
                        if rssi_peer_down {
                            // Peer came back (a listener was started).
                            eprintln!("RSSI: push to rssi peer recovered");
                            rssi_peer_down = false;
                        }
                    }
                    Err(e) => {
                        // A down/absent peer returns ECONNREFUSED (ICMP port
                        // unreachable) on every push. Log only the first
                        // failure so a consumer that is simply not up yet does
                        // not flood stderr once per frame; never crash.
                        if !rssi_peer_down {
                            eprintln!(
                                "{}",
                                color(
                                    &format!("RSSI: push to rssi peer failed: {e} (suppressing repeats until it recovers)"),
                                    "31",
                                ),
                            );
                            rssi_peer_down = true;
                        }
                    }
                }
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
    profile: Profile,
    clko_os: u8,
) -> io::Result<()> {
    // 1-2. Chip reset + identity check. A garbage/floating part number (0xFF =
    // MISO floating: chip unpowered, wrong CS, or wiring fault) must fail loud so
    // the init-retry / supervisor path runs instead of the daemon driving a chip
    // that is not actually there.
    let (pn, vn) = spi::reset_and_identify(dev, radio)?;

    radio.rf_clko.value = RfClko::new().with_os(clko_os).with_drv(1);
    spi::write_register(dev, &radio.rf_clko)?;
    eprintln!(
        "CLKO: RF_CLKO.OS={} ({})",
        clko_os,
        match clko_os {
            0 => "off",
            1 => "26 MHz",
            2 => "32 MHz",
            3 => "16 MHz",
            4 => "8 MHz",
            5 => "4 MHz",
            6 => "2 MHz",
            _ => "1 MHz",
        },
    );

    eprintln!("chip: {:?} v{}", pn, vn);
    if let DevicePartNumber::Unknown(b) = pn {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "unrecognised AT86RF215 part number {b:#04x} (0xFF = MISO floating: \
                 chip unpowered / wrong CS / SPI wiring) - refusing to run blind"
            ),
        ));
    }

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

    // Report the external front-end mode just flushed (RF09_PADFE) so a config
    // that forgot to enable it (0 = FE disabled -> external PA/LNA never keyed)
    // is obvious at startup rather than looking like a weak link.
    let fe = radio.rf09_padfe.value.padfe();
    eprintln!(
        "front-end: RF09_PADFE={} ({})",
        fe,
        match (fe, profile) {
            (0, Profile::Lband) => "disabled (receive only)",
            (_, Profile::Lband) => "disabled (receive only) - no effect",
            (0, Profile::Uhf) => "disabled - external PA/LNA NOT keyed",
            (1, Profile::Uhf) => "FE_MODE_4",
            (2, Profile::Uhf) => "FE_MODE_5",
            (_, Profile::Uhf) => "FE_MODE_6",
        },
    );

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

    // Set TX PA to minimum power for receive only L-Band.
    if !profile.can_transmit() {
        radio.rf09_pac.value = radio.rf09_pac.value.with_txpwr(0);
        spi::write_register(dev, &radio.rf09_pac)?;
    }

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

    // 5c. Enable RF09-level fault interrupts (TRXERR = PLL lock fault, BATLOW =
    //     supply brownout). Without this they are masked off and a radiation/
    //     thermal PLL fault that drops the chip to TrxOff never reaches the IRQ
    //     line, so the daemon goes deaf with no signal. service_radio_irqs reads
    //     RF09_IRQS each pass and recovers on TRXERR.
    radio.rf09_irqm.value = RfnIrqm::new().with_trxerr(true).with_batlow(true);
    spi::write_register(dev, &radio.rf09_irqm)?;

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
    use crate::registers::BulkWrites;
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
    bw.add(&mut radio.rf09_padfe);
    for cmd in bw.generate_commands() {
        dev.write_all(&cmd)?;
    }
    Ok(())
}

/// Write BBC0 PHY configuration registers (FSK/OFDM/OQPSK settings).
fn write_bbc0_config(radio: &mut Radio, dev: &mut spidev::Spidev) -> io::Result<()> {
    use crate::registers::BulkWrites;
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
fn transmit_frame(radio: &mut Radio, dev: &mut spidev::Spidev, frame: &[u8], profile: Profile) -> io::Result<()> {
    // Last defense for transmission on L-Band: Fail.
    if !profile.can_transmit() {
        return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "transmit_frame called on receive only L-Band.",
        ));
    } 

    // 0. Bound the frame to the chip's 2047-octet TX buffer (payload + FCS).
    //    BBCn_TXFL/FBL are 11-bit, so an oversize frame would overrun the FIFO
    //    and TXFL would silently wrap - reject it loudly before touching the chip.
    //    FCST=0 => 32-bit FCS (4 octets); =1 => 16-bit (2).
    let fcs_len = if radio.bbc0_pc.value.fcst() { 2 } else { 4 };
    if frame.len() + fcs_len > MAX_RF_FRAME {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "frame {} B + {} B FCS exceeds the {} B RF frame buffer - dropping",
                frame.len(),
                fcs_len,
                MAX_RF_FRAME,
            ),
        ));
    }

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
    profile: Profile,
) -> io::Result<()> {
    // For L-Band drop any frames queued to send.
    if !profile.can_transmit() {
        if !link.tx_queue.is_empty() {
            eprintln!(
                "warning: {} frame(s) queued on a receive-only profile - dropping.",
                link.tx_queue.len(),
            );
            link.tx_queue.clear();
        }
        return Ok(());
    }
    
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
            match transmit_frame(radio, dev, &frame, profile) {
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
                    // transmit_frame may have left the chip in TxPrep (a PLL-lock
                    // timeout returns before CMD=Rx), which is deaf. Force it back
                    // to Rx so a failed transmit never silently kills reception.
                    if let Err(re) = ensure_receiving(radio, dev) {
                        eprintln!("post-TX-error Rx re-arm failed: {re}");
                    }
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
    profile: Profile,
    offset_db: i16,
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

    if irqs.txfe() && !profile.can_transmit() {
        eprintln!(
            "{}",
            color("ERROR, TXFE on receive only L-Band", "31")
        );
    }

    // TXFE: transmission complete - free the TX slot and re-enter Rx.
    if irqs.txfe() {
        link.tx_busy = false;
        radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::Rx);
        spi::write_register(dev, &radio.rf09_cmd)?;
    }

    // RXFE: frame received - read it out and forward.
    if irqs.rxfe() {
        match receive_frame(radio, dev, stats, offset_db) {
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

    // Sample the RF09-level IRQs too. The IRQ pin is the OR of all blocks and is
    // read-to-clear, so a pending RF09 source (TRXERR/BATLOW) left unread would
    // hold the shared line high and stall every later edge. On TRXERR (PLL lock
    // fault - radiation/thermal/brownout) the chip drops to TrxOff and goes deaf;
    // drive it back to Rx here. A full re-init, if needed, is the timer health
    // backstop's job.
    spi::read_register(dev, &mut radio.rf09_irqs)?;
    let rf_irqs = radio.rf09_irqs.value;
    if rf_irqs.trxerr() {
        stats.record_trxerr();
        eprintln!("{}", color("RF09 TRXERR (PLL lock fault) - recovering to Rx", "31"));
        link.tx_busy = false;
        link.rx_busy = false;
        if let Err(e) = ensure_receiving(radio, dev) {
            eprintln!("TRXERR recovery (ensure_receiving) failed: {e}");
        }
    }
    if rf_irqs.batlow() {
        // A brownout can reset the chip's registers to defaults. Flag the config
        // as suspect so the health backstop reflashes (restoring channel/PHY/
        // front-end) instead of a cosmetic CMD=Rx re-arm that would leave the
        // receiver deaf on the wrong channel. Cleared on the next good reflash.
        stats.record_batlow();
        eprintln!("{}", color("RF09 BATLOW - supply voltage below threshold (brownout warning)", "31"));
    }

    Ok(raw_irqs)
}

/// Drive RF09 back into the Rx state from wherever it currently is, taking the
/// legal path: from Rx it is a no-op; from TxPrep a single CMD=Rx; from TrxOff/
/// Tx/Transition/Reset it goes via TxPrep + PLL-lock first (CMD=Rx is illegal
/// directly from TrxOff). Used by the TX-error path, the TX-timeout backstop, and
/// TRXERR recovery so a fault can never silently leave the receiver off.
fn ensure_receiving(radio: &mut Radio, dev: &mut spidev::Spidev) -> io::Result<()> {
    spi::read_register(dev, &mut radio.rf09_state)?;
    match radio.rf09_state.value.state() {
        TransceiverState::Rx => Ok(()),
        TransceiverState::TxPrep => {
            radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::Rx);
            spi::write_register(dev, &radio.rf09_cmd)
        }
        _ => {
            radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::TxPrep);
            spi::write_register(dev, &radio.rf09_cmd)?;
            spi::wait_rf09_txprep_locked(dev, radio, Duration::from_millis(5))?;
            radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::Rx);
            spi::write_register(dev, &radio.rf09_cmd)
        }
    }
}

/// Leave the radio safe on shutdown: drop the external front-end (so the PA is
/// not left keyed), command RF09 to TrxOff, and put the unused RF24 to sleep.
/// Best-effort - failures are logged, not propagated (we are exiting anyway).
fn radio_safe_shutdown(radio: &mut Radio, dev: &mut spidev::Spidev) {
    radio.rf09_padfe.value = radio.rf09_padfe.value.with_padfe(0);
    let _ = spi::write_register(dev, &radio.rf09_padfe);
    radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::TrxOff);
    let _ = spi::write_register(dev, &radio.rf09_cmd);
    radio.rf24_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::Sleep);
    let _ = spi::write_register(dev, &radio.rf24_cmd);
    eprintln!("radio safed: front-end off (PADFE=0), RF09 TrxOff, RF24 Sleep");
    // RF_CLKO left alone.
}

/// Warn (non-fatal) about resolved register values that weaken the link but do
/// not kill it: AGC off, external front-end disabled.
fn validate_radio_for_flight(radio: &Radio, profile: Profile) {
    if !radio.rf09_agcc.value.en() {
        eprintln!("warning: RF09_AGCC.EN=0 - automatic gain control disabled (RX gain not tracked)");
    }
    if profile.can_transmit() && radio.rf09_padfe.value.padfe() == 0 {
        eprintln!("warning: RF09_PADFE=0 - external PA/LNA front-end will never be keyed");
    }
    if !profile.can_transmit() && radio.rf09_padfe.value.padfe() != 0 {
        eprintln!(
            "warning: RF09_PADFE={} on receive only card - the setting does nothing",
            radio.rf09_padfe.value.padfe(),
        );
    }
}

/// Hard flight invariants: a config that violates these would boot a deaf radio,
/// so refuse to start. Returns a combined message listing every violation.
fn flight_blocking_problems(radio: &Radio, profile: Profile, clko_os: u8) -> Result<(), String> {
    let mut problems = Vec::new();
    let pt = radio.bbc0_pc.value.pt();
    if !profile.can_transmit() && clko_os != 3 {
        problems.push(format!(
            "RF_CLKO.OS={clko_os} on --profile lband (expected 3 = 16 MHz)"
        ));
    }
    if pt != 1 {
        problems.push(format!(
            "BBC0_PC.PT={pt} (expected 1 = MR-FSK; 0 = PHY OFF = the modulator never runs)"
        ));
    }
    if radio.rf09_rxdfe.value.sr() == 0 {
        problems.push(
            "RF09_RXDFE.SR=0 (reset default maps to a 4 MHz sample rate - the receiver is deaf)"
                .to_string(),
        );
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "config fails flight sanity checks (would boot a deaf radio):\n  - {}",
            problems.join("\n  - "),
        ))
    }
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
    offset_db: i16,
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
    // Correct edv for L-Band.
    let edv = (radio.rf09_edv.value.edv() as i16)
        .saturating_add(offset_db)
        .clamp(i8::MIN as i16, i8::MAX as i16 - 1) as i8;

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

/// Encode the current RSSI for the `--rssi-peer`.
///
/// The format is a single raw int8 dBm byte (127 = invalid).
///
/// In dry-run there is no SPI read, so `stats.rssi_last` stays 127 until the
/// first loopback RX; substitute a synthetic wandering value so the feed is
/// non-empty.
fn encode_rssi(stats: &RadioStats) -> [u8; 1] {
    let rssi = if stats.rssi_last == 127 {
        synthetic_rf(stats.ticks).rssi
    } else {
        stats.rssi_last
    };
    [rssi as u8]
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
