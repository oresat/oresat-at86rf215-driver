// TESTING ONLY
use clap::Parser;
use oresat_at86rf215_driver::{
    freq::{Band, PllSettings},
    radio::Radio,
    registers::{BbcnPc, DevicePartNumber, RfnCmd, RfnPadfe, TransceiverCmd},
    spi::{self},
};
use std::{io, time::Duration};


#[derive(Parser, Debug)]
struct Args {
    #[arg(long, default_value_t = 436_500_000)]
    freq_hz: u64,

    #[arg(long, default_value_t = 0)] // PA off by default
    pa: u8,
}

fn init_radio(radio: &mut Radio, dev: &mut spidev::Spidev, freq_hz: u64, pa: u8) -> io::Result<()> {
    let (pn, vn) = spi::reset_and_identify(dev, radio)?;
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

    let pll = PllSettings::fine(Band::Sub1GHz, freq_hz).map_err(io::Error::other)?;
    eprintln!("frequency: {} Hz ", freq_hz,);
    spi::apply_channel_rf09(dev, radio, pll)?;

    radio.rf09_padfe.value = RfnPadfe::new().with_padfe(pa);
    spi::write_register(dev, &radio.rf09_padfe)?;

    radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::TxPrep);
    spi::write_register(dev, &radio.rf09_cmd)?;
    spi::wait_rf09_txprep_locked(dev, radio, Duration::from_millis(5))?;

    radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::Tx);
    spi::write_register(dev, &radio.rf09_cmd)?;

    eprintln!("radio initialised - listening on RF09");
    Ok(())
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    let mut radio = Radio::new();
    let spi_dev = "/dev/spidev0.0".to_string();
    let hack = true;
    let mut spidev: Option<spidev::Spidev> = if hack {
        let dev = spi::open_with_speed(&spi_dev, spi::DEFAULT_SPI_HZ)?;
        eprintln!("SPI opened: {} @ {} Hz", spi_dev, spi::DEFAULT_SPI_HZ);
        Some(dev)
    } else {
        eprintln!("dry-run mode - no hardware");
        None
    };

    radio.bbc0_pc.value = radio.bbc0_pc.value.with_pt(0).with_ctx(true);
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

    if let Some(ref mut dev) = spidev {
        let _ = spi::write_register(dev, &radio.bbc0_pc);
        let _ = spi::write_register(dev, &radio.rf09_txdaci);
        let _ = spi::write_register(dev, &radio.rf09_txdacq);
    }

    eprintln!("radio initialised - listening on RF09");
    if let Some(ref mut dev) = spidev {
        let mut attempt = 0u32;
        loop {
            match init_radio(&mut radio, dev, args.freq_hz, args.pa) {
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
    loop {
        // Check for SIGINT/SIGTERM before blocking.
        use signal_hook::consts::{SIGINT, SIGTERM};
        let signal_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        signal_hook::flag::register(SIGINT, signal_flag.clone()).map_err(io_err)?;
        signal_hook::flag::register(SIGTERM, signal_flag.clone()).map_err(io_err)?;

        if signal_flag.load(std::sync::atomic::Ordering::Relaxed) {
            eprintln!("\nsignal received - shutting down");
            // Leave the radio safe: drop the external front-end and stop the PA so
            // a restart/stop never leaves it keyed.
            if let Some(ref mut dev) = spidev {
                radio.rf09_padfe.value = radio.rf09_padfe.value.with_padfe(0);
                let _ = spi::write_register(dev, &radio.rf09_padfe);
                radio.rf09_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::TrxOff);
                let _ = spi::write_register(dev, &radio.rf09_cmd);
                radio.rf24_cmd.value = RfnCmd::new().with_cmd(TransceiverCmd::Sleep);
                let _ = spi::write_register(dev, &radio.rf24_cmd);
                radio.bbc0_pc.value = BbcnPc::new().with_ctx(false);
                let _ = spi::write_register(dev, &radio.bbc0_pc);
                eprintln!("radio safed: front-end off (PADFE=0), RF09 TrxOff, RF24 Sleep");
                return Ok(());
            }
        }
    }
}

fn io_err<E: std::error::Error + Send + Sync + 'static>(e: E) -> io::Error {
    io::Error::other(e)
}
