// TESTING ONLY
use clap::Parser;
use gpiocdev::line::Value::{Active, Inactive};
use oresat_at86rf215_driver::{
    freq::{Band, PllSettings},
    radio::Radio,
    registers::{BbcnPc, ChipMode, ChipResetCmd, RfnCmd, TransceiverCmd},
    spi::{self, read_register, write_register},
};
use std::{io, time::Duration};

#[derive(Parser, Debug)]
struct Args {
    /// SPI device path.
    #[arg(long, default_value = "/dev/spidev0.0")]
    spi: String,

    /// SPI clock in Hz (default 10 MHz).
    #[arg(long, default_value = "10000000")]
    spi_hz: u32,

    /// Center frequency. Accepts an optional unit suffix (k/M/G, with or
    /// without "Hz", case-insensitive); a bare number is Hz. Examples:
    /// `436.5MHz`, `436.5m`, `436500000`.
    #[arg(long, default_value_t = 436_500_000)]
    freq: u64,

    /// initial pa output power, 0..31 (~1 db steps; 31 = max).
    #[arg(long, default_value = "24")]
    txpwr: u8,

    /// External front-end control configuration (RFn_PADFE.PADFE, 0..3).
    /// 0 = off (FEA/FEB held low, no external FE); 1 = Config 1 (TX/RX
    /// switch + LNA bypass); 2 = Config 2 (enable + TX/RX switch);
    /// 3 = Config 3 (TX/RX switch + LNA bypass, MCU-gated enable).
    /// Datasheet sect 6.5. Overrides any value from --config.
    #[arg(long)]
    padfe: Option<u8>,

    // Turn on GPIO0 line 27
    #[arg(long)]
    pa: bool,

    // Switch to LBand
    #[arg(long)]
    lband: bool,
}
fn set_gpio(
    val: gpiocdev::line::Value,
    chip: &str,
    line: u32,
) -> Result<gpiocdev::Request, gpiocdev::Error> {
    let req = gpiocdev::Request::builder()
        .on_chip(chip)
        .with_line(line)
        .with_direction(gpiocdev::line::Direction::Output)
        .with_value(val)
        .request()?;
    return Ok(req);
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    let mut radio = Radio::new();

    let mut attempt = 0;
    let radio_status = set_gpio(Active, "/dev/gpiochip1", 22);

    eprintln!("Radio Enable Active {:?}", radio_status);
    if !(args.lband) {
        let band_status = set_gpio(Active, "/dev/gpiochip0", 16);
        eprintln!("UHF Radio Enabled {:?}", band_status);
    } else {
        let band_status = set_gpio(Active, "/dev/gpiochip0", 19);
        eprintln!("L-band Radio Enabled {:?}", band_status);
    }
    let mut dev = spi::open_with_speed(&args.spi, args.spi_hz)?;
    eprintln!("SPI opened: {} @ {} Hz", &args.spi, args.spi_hz);

    radio.rf_rst.value = radio.rf_rst.value.with_cmd(ChipResetCmd::Reset);
    write_register(&mut dev, &radio.rf_rst)?;

    loop {
        read_register(&mut dev, &mut radio.rf09_irqs)?;
        if radio.rf09_irqs.value.wakeup() {
            break;
        }
        if attempt == 30 {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "AT86RF215 did not assert RF09_IRQS.WAKEUP within 100 ms of reset",
            ));
        }
        std::thread::sleep(Duration::from_micros(1000));
        attempt = attempt + 1;
    }

    let pn = read_register(&mut dev, &mut radio.rf_pn)?;
    let vn = read_register(&mut dev, &mut radio.rf_vn)?;
    eprintln!("chip: {:?} v{:?}", pn, vn);

    // radio.bbc0_pc.value = radio.bbc0_pc.value.with_pt(1).with_ctx(true);
    // radio.bbc0_txfl.value = radio.bbc0_txfl.value.with_txfl(0x22);
    radio.rf_iqifc1.value = radio.rf_iqifc1.value.with_chpm(ChipMode::IqRadioMode);
    radio.rf09_txdaci.value = radio
        .rf09_txdaci
        .value
        .with_txdacid(0x7E)
        .with_entxdacid(true);
    radio.rf09_txdacq.value = radio
        .rf09_txdacq
        .value
        .with_txdacqd(0x3F)
        .with_entxdacqd(true);

    radio.rf09_pac.value = radio.rf09_pac.value.with_txpwr(args.txpwr);

    // spi::write_register(dev, &radio.bbc0_pc);
    // spi::write_register(dev, &radio.bbc0_txfl);
    spi::write_register(&mut dev, &radio.rf_iqifc1)?;
    spi::write_register(&mut dev, &radio.rf09_txdaci)?;
    spi::write_register(&mut dev, &radio.rf09_txdacq)?;
    spi::write_register(&mut dev, &radio.rf09_pac)?;

    eprintln!("CW Register configured");
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

    let pll = PllSettings::fine(Band::Sub1GHz, args.freq).map_err(io::Error::other)?;
    eprintln!(
        "frequency: {} Hz (CCF0={}, CN={}, CS={})",
        args.freq, pll.ccf0, pll.cn, pll.cs,
    );
    spi::apply_channel_rf09(&mut dev, &mut radio, pll)?;

    radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::TxPrep);
    spi::write_register(&mut dev, &radio.rf09_cmd)?;
    spi::wait_rf09_txprep_locked(&mut dev, &mut radio, Duration::from_millis(5))?;

    radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::Tx);
    spi::write_register(&mut dev, &radio.rf09_cmd)?;

    spi::read_register(&mut dev, &mut radio.rf_iqifc1)?;
    spi::read_register(&mut dev, &mut radio.rf09_txdaci)?;
    spi::read_register(&mut dev, &mut radio.rf09_txdacq)?;
    spi::read_register(&mut dev, &mut radio.rf09_pac)?;
    spi::read_register(&mut dev, &mut radio.bbc0_pc)?;
    eprintln!(
        "CT cfg: chipmode = {:?}, txdaci = {}, {:#04x}, txdaci = {}, {:#04x} \
        txpwr = {}, ctx = {}, phy = {}
        ",
        radio.rf_iqifc1.value.chpm(),
        u8::from(radio.rf09_txdaci.value.entxdacid()),
        u8::from(radio.rf09_txdaci.value.txdacid()),
        u8::from(radio.rf09_txdacq.value.entxdacqd()),
        u8::from(radio.rf09_txdacq.value.txdacqd()),
        u8::from(radio.rf09_pac.value.txpwr()),
        u8::from(radio.bbc0_pc.value.ctx()),
        u8::from(radio.bbc0_pc.value.pt()),
    );
    if args.pa {
        let status = set_gpio(Active, "/dev/gpiochip0", 27);
        eprintln!("GPIO Status: {:?}", status)
    }
    loop {
        use signal_hook::consts::{SIGINT, SIGTERM};
        let signal_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        signal_hook::flag::register(SIGINT, signal_flag.clone()).map_err(io_err)?;
        signal_hook::flag::register(SIGTERM, signal_flag.clone()).map_err(io_err)?;

        if signal_flag.load(std::sync::atomic::Ordering::Relaxed) {
            eprintln!("\nsignal received - shutting down");
            // Leave the radio safe: drop the external front-end and stop the PA so
            // a restart/stop never leaves it keyed.
            radio.rf09_padfe.value = radio.rf09_padfe.value.with_padfe(0);
            let _ = spi::write_register(&mut dev, &radio.rf09_padfe);
            radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::TrxOff);
            let _ = spi::write_register(&mut dev, &radio.rf09_cmd);
            radio.rf24_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::Sleep);
            let _ = spi::write_register(&mut dev, &radio.rf09_cmd);
            radio.bbc0_pc.value = BbcnPc::new().with_ctx(false);
            let _ = spi::write_register(&mut dev, &radio.bbc0_pc);
            let _ = eprintln!("radio safed: front-end off (PADFE=0), RF09 TrxOff, RF24 Sleep");
            let pa_status = set_gpio(Inactive, "/dev/gpiochip0", 27);
            let radio_en_status = set_gpio(Inactive, "/dev/gpiochip1", 22);
            let uhf_en_status = set_gpio(Inactive, "/dev/gpiochip0", 16);
            let lband_en_status = set_gpio(Inactive, "/dev/gpiochip0", 19);
            eprintln!(
                "PA {:?}, Radio Enable {:?}, UHF EN {:?}, Lband EN{:?}",
                pa_status, radio_en_status, uhf_en_status, lband_en_status
            );

            return Ok(());
        }
    }
}
fn io_err<E: std::error::Error + Send + Sync + 'static>(e: E) -> io::Error {
    io::Error::other(e)
}
