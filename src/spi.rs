//! SPI transport layer for the AT86RF215.
//!
//! Provides helpers for reading/writing registers and frame buffers over
//! `spidev`. Everything in this module is a thin wrapper around the byte-array
//! primitives in [`crate::registers`] and the FIFO addresses in
//! [`crate::radio`].

use std::io::{self, Write};
use std::time::{Duration, Instant};

use spidev::{SpiModeFlags, SpidevOptions, SpidevTransfer};

use crate::freq::PllSettings;
use crate::radio::{Radio, BBC0_FBRXS, BBC0_FBTXS, BBC1_FBRXS, BBC1_FBTXS};
use crate::registers::{
    generate_read_header, generate_write_header, BulkWrites, ChipResetCmd, DevicePartNumber,
    Readable, TransceiverState, Writable,
};

/// Open a spidev device with sensible defaults for the AT86RF215.
pub fn open(path: &str) -> io::Result<spidev::Spidev> {
    let mut dev = spidev::Spidev::open(path)?;
    let opts = SpidevOptions::new()
        .bits_per_word(8)
        .max_speed_hz(10_000_000)
        .mode(SpiModeFlags::SPI_MODE_0)
        .build();
    dev.configure(&opts)?;
    Ok(dev)
}

/// Write a single register over SPI. The register's in-memory `.value`
/// must already hold the desired contents.
pub fn write_register<W: Writable>(spi: &mut spidev::Spidev, reg: &W) -> io::Result<()> {
    spi.write_all(&reg.write_command())
}

/// Read a single register from SPI into the register's in-memory `.value`.
pub fn read_register<R: Readable>(spi: &mut spidev::Spidev, reg: &mut R) -> io::Result<()> {
    let cmd = reg.read_command();
    let mut rx = vec![0u8; cmd.len()];
    let mut transfer = SpidevTransfer::read_write(&cmd, &mut rx);
    spi.transfer(&mut transfer)?;
    reg.set_from_bytes(&rx[2..]);
    Ok(())
}

/// Which baseband core to target.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Bbc {
    /// BBC0 - sub-1 GHz.
    Bbc0,
    /// BBC1 - 2.4 GHz.
    Bbc1,
}

impl Bbc {
    const fn tx_base(self) -> u16 {
        match self {
            Bbc::Bbc0 => BBC0_FBTXS,
            Bbc::Bbc1 => BBC1_FBTXS,
        }
    }

    const fn rx_base(self) -> u16 {
        match self {
            Bbc::Bbc0 => BBC0_FBRXS,
            Bbc::Bbc1 => BBC1_FBRXS,
        }
    }
}

/// Build the SPI write command for the TX frame buffer (header + data).
///
/// This is the byte array that would be sent over SPI. Useful for callers
/// who batch operations or for testing without hardware.
pub fn tx_fifo_write_command(bbc: Bbc, data: &[u8]) -> Vec<u8> {
    let header = generate_write_header(bbc.tx_base());
    let mut cmd = Vec::with_capacity(2 + data.len());
    cmd.extend_from_slice(&header);
    cmd.extend_from_slice(data);
    cmd
}

/// Build the SPI read command for the RX frame buffer (header + zero padding).
///
/// The response bytes at positions `[2..]` will contain the frame data.
pub fn rx_fifo_read_command(bbc: Bbc, len: usize) -> Vec<u8> {
    let header = generate_read_header(bbc.rx_base());
    let mut cmd = Vec::with_capacity(2 + len);
    cmd.extend_from_slice(&header);
    cmd.resize(2 + len, 0x00);
    cmd
}

/// Write `data` into the TX frame buffer starting at offset 0.
///
/// The caller must also write `BBCn_TXFL` with the frame length
/// before issuing the TX command.
pub fn write_tx_fifo(spi: &mut spidev::Spidev, bbc: Bbc, data: &[u8]) -> io::Result<()> {
    spi.write_all(&tx_fifo_write_command(bbc, data))
}

/// Read `len` bytes from the RX frame buffer starting at offset 0.
///
/// The caller should read `BBCn_FBL` first to know how many bytes to read.
pub fn read_rx_fifo(spi: &mut spidev::Spidev, bbc: Bbc, len: usize) -> io::Result<Vec<u8>> {
    let cmd = rx_fifo_read_command(bbc, len);
    let mut rx = vec![0u8; cmd.len()];
    let mut transfer = SpidevTransfer::read_write(&cmd, &mut rx);
    spi.transfer(&mut transfer)?;
    Ok(rx[2..].to_vec())
}

/// Stage `pll` onto RF09 and flush the channel triple (CS, CCF0, CN) as a
/// single coalesced SPI write at address 0x0104.
pub fn apply_channel_rf09(
    spi: &mut spidev::Spidev,
    radio: &mut Radio,
    pll: PllSettings,
) -> io::Result<()> {
    pll.apply_rf09(radio);
    let mut bw = BulkWrites::new();
    bw.add(&mut radio.rf09_cs);
    bw.add(&mut radio.rf09_ccf0);
    bw.add(&mut radio.rf09_cn);
    for cmd in bw.generate_commands() {
        spi.write_all(&cmd)?;
    }
    Ok(())
}

/// RF24 counterpart of [`apply_channel_rf09`] - same three-register block,
/// address 0x0204.
pub fn apply_channel_rf24(
    spi: &mut spidev::Spidev,
    radio: &mut Radio,
    pll: PllSettings,
) -> io::Result<()> {
    pll.apply_rf24(radio);
    let mut bw = BulkWrites::new();
    bw.add(&mut radio.rf24_cs);
    bw.add(&mut radio.rf24_ccf0);
    bw.add(&mut radio.rf24_cn);
    for cmd in bw.generate_commands() {
        spi.write_all(&cmd)?;
    }
    Ok(())
}

/// Issue a chip reset, poll `RF09_IRQS.WAKEUP` until the chip is awake, then
/// read the `RF_PN` / `RF_VN` identification registers.
///
/// Returns `(part number, version)` for logging.
pub fn reset_and_identify(
    spi: &mut spidev::Spidev,
    radio: &mut Radio,
) -> io::Result<(DevicePartNumber, u8)> {
    radio.rf_rst.value = radio.rf_rst.value.with_cmd(ChipResetCmd::Reset);
    write_register(spi, &radio.rf_rst)?;

    let deadline = Instant::now() + Duration::from_millis(10);
    loop {
        read_register(spi, &mut radio.rf09_irqs)?;
        if radio.rf09_irqs.value.wakeup() {
            break;
        }
        if Instant::now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "AT86RF215 did not assert RF09_IRQS.WAKEUP within 10 ms of reset",
            ));
        }
        std::thread::sleep(Duration::from_micros(100));
    }

    read_register(spi, &mut radio.rf_pn)?;
    read_register(spi, &mut radio.rf_vn)?;

    Ok((radio.rf_pn.value.pn(), radio.rf_vn.value.vn()))
}

/// Poll `RFn_STATE` and `RFn_PLL.LS` until the transceiver reaches `TXPREP`
/// with the PLL locked, or `timeout` elapses.
/// 
/// Reads the per-call state into `radio.rf09_state` / `radio.rf09_pll` so
/// the caller can inspect them after a timeout.
pub fn wait_rf09_txprep_locked(
    spi: &mut spidev::Spidev,
    radio: &mut Radio,
    timeout: Duration,
) -> io::Result<()> {
    let deadline = Instant::now() + timeout;
    loop {
        read_register(spi, &mut radio.rf09_state)?;
        read_register(spi, &mut radio.rf09_pll)?;
        if radio.rf09_state.value.state() == TransceiverState::TxPrep
            && radio.rf09_pll.value.ls()
        {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!(
                    "RF09 TxPrep+PLL-lock not reached in {:?} (state={:?}, pll_locked={})",
                    timeout,
                    radio.rf09_state.value.state(),
                    radio.rf09_pll.value.ls(),
                ),
            ));
        }
        std::thread::sleep(Duration::from_micros(100));
    }
}

/// RF24 counterpart of [`wait_rf09_txprep_locked`].
pub fn wait_rf24_txprep_locked(
    spi: &mut spidev::Spidev,
    radio: &mut Radio,
    timeout: Duration,
) -> io::Result<()> {
    let deadline = Instant::now() + timeout;
    loop {
        read_register(spi, &mut radio.rf24_state)?;
        read_register(spi, &mut radio.rf24_pll)?;
        if radio.rf24_state.value.state() == TransceiverState::TxPrep
            && radio.rf24_pll.value.ls()
        {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!(
                    "RF24 TxPrep+PLL-lock not reached in {:?} (state={:?}, pll_locked={})",
                    timeout,
                    radio.rf24_state.value.state(),
                    radio.rf24_pll.value.ls(),
                ),
            ));
        }
        std::thread::sleep(Duration::from_micros(100));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::radio::{BBC0_FBTXS, BBC0_FBRXS, BBC1_FBTXS, BBC1_FBRXS};

    #[test]
    fn tx_fifo_write_command_bbc0_header() {
        let cmd = tx_fifo_write_command(Bbc::Bbc0, &[0xAA, 0xBB]);
        // Write bit (bit 15) set + address 0x2000.
        let expected_header = generate_write_header(BBC0_FBTXS);
        assert_eq!(&cmd[..2], &expected_header);
        assert_eq!(&cmd[2..], &[0xAA, 0xBB]);
    }

    #[test]
    fn tx_fifo_write_command_bbc1_header() {
        let cmd = tx_fifo_write_command(Bbc::Bbc1, &[0x01]);
        let expected_header = generate_write_header(BBC1_FBTXS);
        assert_eq!(&cmd[..2], &expected_header);
        assert_eq!(cmd.len(), 3); // 2 header + 1 data
    }

    #[test]
    fn rx_fifo_read_command_bbc0_length() {
        let cmd = rx_fifo_read_command(Bbc::Bbc0, 128);
        let expected_header = generate_read_header(BBC0_FBRXS);
        assert_eq!(&cmd[..2], &expected_header);
        assert_eq!(cmd.len(), 2 + 128);
        // Padding bytes should all be zero.
        assert!(cmd[2..].iter().all(|&b| b == 0));
    }

    #[test]
    fn rx_fifo_read_command_bbc1_header() {
        let cmd = rx_fifo_read_command(Bbc::Bbc1, 10);
        let expected_header = generate_read_header(BBC1_FBRXS);
        assert_eq!(&cmd[..2], &expected_header);
    }

    #[test]
    fn fifo_addresses_are_in_correct_ranges() {
        // Datasheet: BBC0 TX at 0x2000, RX at 0x3000
        //            BBC1 TX at 0x2800, RX at 0x3800
        assert_eq!(BBC0_FBTXS, 0x2000);
        assert_eq!(BBC0_FBRXS, 0x3000);
        assert_eq!(BBC1_FBTXS, 0x2800);
        assert_eq!(BBC1_FBRXS, 0x3800);
    }

    #[test]
    fn empty_tx_fifo_write_is_header_only() {
        let cmd = tx_fifo_write_command(Bbc::Bbc0, &[]);
        assert_eq!(cmd.len(), 2); // just the header, no payload
    }
}
