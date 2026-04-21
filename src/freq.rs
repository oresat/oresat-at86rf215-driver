//! Frequency -> PLL/channel helpers (datasheet 6.3).
//!
//! Pure math: given a desired carrier frequency, produce the `(CCF0, CN, CS, CNM)`
//! register values the AT86RF215 needs. No SPI, no `Radio` coupling - stage the
//! result onto a `Radio` via [`PllSettings::apply_rf09`] / [`PllSettings::apply_rf24`].

use crate::registers::*;
use crate::radio::Radio;

/// Which transceiver the settings are for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Band {
    /// Sub-1 GHz transceiver (RF09). Valid 389.5–510 MHz or 779–1020 MHz.
    Sub1GHz,
    /// 2.4 GHz transceiver (RF24). Valid 2400–2483.5 MHz.
    Rf24,
}

/// Channel-setting mode (RFn_CNM.CM).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ChannelMode {
    /// IEEE compliant: `f = (CCF0 + CN·CS)·25 kHz + band_offset`.
    Ieee = 0,
    /// Fine mode, 389.5–510 MHz (~99 Hz step).
    Fine389 = 1,
    /// Fine mode, 779–1020 MHz (~198 Hz step).
    Fine779 = 2,
    /// Fine mode, 2400–2483.5 MHz (~397 Hz step).
    Fine2400 = 3,
}

/// Register values ready to write into RFn_CCF0 / RFn_CN / RFn_CS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PllSettings {
    pub ccf0: u16,
    pub cn: u16,
    pub cs: u8,
    pub mode: ChannelMode,
}

/// Errors produced when resolving a frequency to register values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreqError {
    /// Frequency not inside any supported range.
    OutOfRange,
    /// Frequency/spacing combination cannot be represented on an integer grid.
    NotRepresentable,
    /// Requested channel number exceeds the 9-bit field.
    ChannelTooLarge,
}

impl std::fmt::Display for FreqError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OutOfRange => write!(f, "frequency out of supported range"),
            Self::NotRepresentable => write!(f, "frequency/spacing not representable on 25 kHz grid"),
            Self::ChannelTooLarge => write!(f, "channel number exceeds 9-bit field"),
        }
    }
}

impl std::error::Error for FreqError {}

const STEP_HZ: u64 = 25_000;

impl PllSettings {
    /// Resolve an IEEE-mode channel plan: a base frequency plus `channel` steps
    /// of `spacing_hz`. Used for 802.15.4-style channel rasters.
    ///
    /// This is `const fn` - an out-of-range `const` frequency becomes a
    /// compile error when unwrapped inside a `const` context. See
    /// [`PllSettings::fine`] for an example of that pattern.
    #[allow(clippy::manual_is_multiple_of)] // is_multiple_of is not const-stable
    pub const fn ieee(
        band: Band,
        base_hz: u64,
        spacing_hz: u64,
        channel: u16,
    ) -> Result<Self, FreqError> {
        let base_offset = match band_offset(band, base_hz) {
            Ok(v) => v,
            Err(e) => return Err(e),
        };

        if spacing_hz % STEP_HZ != 0 {
            return Err(FreqError::NotRepresentable);
        }
        let cs_steps = spacing_hz / STEP_HZ;
        if cs_steps == 0 || cs_steps > u8::MAX as u64 {
            return Err(FreqError::NotRepresentable);
        }
        if base_offset % STEP_HZ != 0 {
            return Err(FreqError::NotRepresentable);
        }
        let ccf0 = base_offset / STEP_HZ;
        if ccf0 > u16::MAX as u64 {
            return Err(FreqError::OutOfRange);
        }
        if channel >= 512 {
            return Err(FreqError::ChannelTooLarge);
        }
        Ok(Self {
            ccf0: ccf0 as u16,
            cn: channel,
            cs: cs_steps as u8,
            mode: ChannelMode::Ieee,
        })
    }

    /// Fine-mode plan: picks the mode implied by `band` and `freq_hz`, then
    /// rounds to the nearest N-step. `CN` is not used in the datasheet sense
    /// here - the 24-bit N value is split across (CCF0<<8 | CN_lo).
    ///
    /// `const fn` so the PDF's ask - "pass it a frequency and have it come up
    /// with the appropriate settings (or compile error if no appropriate one
    /// is found)" - is satisfied when the frequency is a compile-time constant:
    ///
    /// ```
    /// # use oresat_at86rf215_driver::freq::{Band, PllSettings};
    /// const PLAN: PllSettings = match PllSettings::fine(Band::Sub1GHz, 868_300_000) {
    ///     Ok(v) => v,
    ///     Err(_) => panic!("frequency out of range"),
    /// };
    /// assert_eq!(PLAN.mode as u8, 2); // Fine779
    /// ```
    ///
    /// An out-of-range frequency fails to compile in `const` context:
    ///
    /// ```compile_fail
    /// # use oresat_at86rf215_driver::freq::{Band, PllSettings};
    /// const BAD: PllSettings = match PllSettings::fine(Band::Sub1GHz, 100_000_000) {
    ///     Ok(v) => v,
    ///     Err(_) => panic!("frequency out of range"),
    /// };
    /// ```
    pub const fn fine(band: Band, freq_hz: u64) -> Result<Self, FreqError> {
        let (mode, base_hz, span_hz) = match band {
            Band::Sub1GHz if freq_hz >= 389_500_000 && freq_hz <= 510_000_000 => {
                (ChannelMode::Fine389, 377_000_000u64, 6_500_000u64)
            }
            Band::Sub1GHz if freq_hz >= 779_000_000 && freq_hz <= 1_020_000_000 => {
                (ChannelMode::Fine779, 754_000_000u64, 13_000_000u64)
            }
            Band::Rf24 if freq_hz >= 2_400_000_000 && freq_hz <= 2_483_500_000 => {
                (ChannelMode::Fine2400, 2_366_000_000u64, 26_000_000u64)
            }
            _ => return Err(FreqError::OutOfRange),
        };

        // N = round((freq - base) * 2^16 / span)
        let delta = freq_hz - base_hz;
        let n = (delta * 65_536 + span_hz / 2) / span_hz;
        if n > 0x00FF_FFFF {
            return Err(FreqError::OutOfRange);
        }
        // 24-bit N laid across CCF0H:CCF0L:CN (the fine-mode packing).
        let ccf0 = ((n >> 8) & 0xFFFF) as u16;
        let cn = (n & 0xFF) as u16;
        Ok(Self { ccf0, cn, cs: 0, mode })
    }

    /// Stage these settings onto the RF09 channel registers.
    pub fn apply_rf09(self, radio: &mut Radio) {
        radio.rf09_ccf0.value = RfnCcf0::new().with_ccf0(self.ccf0);
        radio.rf09_cs.value = RfnCs::new().with_cs(self.cs);
        radio.rf09_cn.value = RfnCn::new().with_cn(self.cn).with_cm(self.mode as u8);
    }

    /// Stage these settings onto the RF24 channel registers.
    pub fn apply_rf24(self, radio: &mut Radio) {
        radio.rf24_ccf0.value = RfnCcf0::new().with_ccf0(self.ccf0);
        radio.rf24_cs.value = RfnCs::new().with_cs(self.cs);
        radio.rf24_cn.value = RfnCn::new().with_cn(self.cn).with_cm(self.mode as u8);
    }
}

/// IEEE-mode band offset: RF24 subtracts 1.5 GHz before it is encoded in CCF0.
const fn band_offset(band: Band, base_hz: u64) -> Result<u64, FreqError> {
    match band {
        Band::Sub1GHz => {
            if base_hz >= 389_500_000 && base_hz <= 1_020_000_000 {
                Ok(base_hz)
            } else {
                Err(FreqError::OutOfRange)
            }
        }
        Band::Rf24 => {
            if base_hz >= 2_400_000_000 && base_hz <= 2_483_500_000 {
                Ok(base_hz - 1_500_000_000)
            } else {
                Err(FreqError::OutOfRange)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ieee_sub1ghz_802154() {
        // 802.15.4 channel 0 at 868.3 MHz, 2 MHz spacing.
        let s = PllSettings::ieee(Band::Sub1GHz, 868_300_000, 2_000_000, 0).unwrap();
        assert_eq!(s.mode, ChannelMode::Ieee);
        assert_eq!(s.cs, 80);
        assert_eq!(s.ccf0 as u64 * 25_000, 868_300_000);
        assert_eq!(s.cn, 0);
    }

    #[test]
    fn ieee_rf24_channel_subtracts_offset() {
        // 2405 MHz channel 11, 5 MHz spacing.
        let s = PllSettings::ieee(Band::Rf24, 2_405_000_000, 5_000_000, 0).unwrap();
        assert_eq!(s.cs, 200);
        // (2405 - 1500) MHz / 25 kHz = 36200
        assert_eq!(s.ccf0, 36_200);
    }

    #[test]
    fn ieee_out_of_range() {
        assert_eq!(
            PllSettings::ieee(Band::Sub1GHz, 100_000_000, 200_000, 0),
            Err(FreqError::OutOfRange)
        );
    }

    #[test]
    fn fine_mode1_endpoints() {
        let lo = PllSettings::fine(Band::Sub1GHz, 389_500_000).unwrap();
        assert_eq!(lo.mode, ChannelMode::Fine389);
        let hi = PllSettings::fine(Band::Sub1GHz, 510_000_000).unwrap();
        assert_eq!(hi.mode, ChannelMode::Fine389);
        // Rounded N must reproduce the frequency to within one LSB (~99 Hz).
        let n = ((hi.ccf0 as u64) << 8) | (hi.cn as u64 & 0xFF);
        let f = 377_000_000 + (6_500_000u64 * n + 32_768) / 65_536;
        assert!((f as i64 - 510_000_000).abs() < 200);
    }

    #[test]
    fn fine_mode3_2400_midband() {
        let s = PllSettings::fine(Band::Rf24, 2_450_000_000).unwrap();
        assert_eq!(s.mode, ChannelMode::Fine2400);
        // N ~= (2450 − 2366) MHz · 65536 / 26 MHz
        let n = ((s.ccf0 as u64) << 8) | (s.cn as u64 & 0xFF);
        let f = 2_366_000_000 + (26_000_000u64 * n + 32_768) / 65_536;
        assert!((f as i64 - 2_450_000_000).abs() < 500);
    }

    #[test]
    fn freq_error_display() {
        assert_eq!(FreqError::OutOfRange.to_string(), "frequency out of supported range");
        assert_eq!(FreqError::NotRepresentable.to_string(), "frequency/spacing not representable on 25 kHz grid");
        assert_eq!(FreqError::ChannelTooLarge.to_string(), "channel number exceeds 9-bit field");
    }
}
