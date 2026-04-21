//! Easy to read decoders for raw register field values.
//!
//! Where a code maps onto a fixed lookup table the unmapped slots return
//! `None` rather than fabricating a value - the chip silently allows
//! those codes but the datasheet leaves their meaning reserved.

/// PMUVAL -> degrees. The 8-bit `pmuval` field encodes a phase from
/// 0 to 360° in 256 steps, so each LSB ≈ 1.406°.
pub fn pmuval_degrees(pmuval: u8) -> f32 {
    pmuval as f32 * 360.0 / 256.0
}

/// Degrees -> PMUVAL. Wraps into `[0, 360)` and rounds to the nearest
/// 1.406° step. The PDF explicitly notes this direction has rounding
/// loss - round-trip via `pmuval_degrees` reproduces within one LSB.
pub fn pmuval_from_degrees(deg: f32) -> u8 {
    let wrapped = deg.rem_euclid(360.0);
    let raw = (wrapped * 256.0 / 360.0).round() as i32;
    raw.rem_euclid(256) as u8
}

/// 12-entry RFn_RXBWC bandwidth lookup. Returns the receiver bandwidth
/// in kHz for codes 0..=11, or `None` for the four reserved codes.
pub fn rxbwc_khz(bw: u8) -> Option<u16> {
    const TABLE: [u16; 12] = [
        160, 200, 250, 320, 400, 500, 630, 800, 1000, 1250, 1600, 2000,
    ];
    TABLE.get(bw as usize).copied()
}

/// 12-entry RFn_TXCUTC LPF cutoff lookup. Returns the transmitter
/// low-pass cutoff in kHz for codes 0..=11, or `None` for reserved codes.
pub fn txcutc_khz(lpfcut: u8) -> Option<u16> {
    const TABLE: [u16; 12] = [
        80, 100, 125, 160, 200, 250, 315, 400, 500, 625, 800, 1000,
    ];
    TABLE.get(lpfcut as usize).copied()
}

/// RFn_RXDFE.sr / RFn_TXDFE.sr -> digital frontend sample rate in kHz.
/// Eight valid codes; everything else is reserved.
pub fn dfe_sr_khz(sr: u8) -> Option<u32> {
    match sr {
        1 => Some(4000),
        2 => Some(2000),
        3 => Some(4000 / 3),
        4 => Some(1000),
        5 => Some(800),
        6 => Some(2000 / 3),
        8 => Some(500),
        10 => Some(400),
        _ => None,
    }
}

/// RFn_EDD averaging window in microseconds.
///
/// `dtb` (Duration Time Basis) selects 2/8/32/128 µs. `df` (Duration Factor)
/// scales it. Total = `dtb * df` µs. With `df = 0` the chip disables ED
/// averaging; this returns 0 in that case.
pub fn edd_us(dtb: u8, df: u8) -> u32 {
    let dtb_us = 2u32 << (2 * dtb as u32);
    dtb_us * df as u32
}

/// RFn_AGCC.avgs -> number of samples in the AGC averaging window.
/// Codes 0..=3 map to 8/16/32/64 samples.
pub fn agcc_avg_samples(avgs: u8) -> u16 {
    8u16 << (avgs & 0x3)
}

/// RFn_AGCS.tgt -> target level in dB relative to ADC full scale.
/// Code 0 = -21 dB, then -3 dB per step down to -42 dB at code 7.
pub fn agcs_target_dbfs(tgt: u8) -> i16 {
    -21 - 3 * (tgt & 0x7) as i16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pmuval_cardinal_angles() {
        assert_eq!(pmuval_degrees(0), 0.0);
        assert_eq!(pmuval_degrees(64), 90.0);
        assert_eq!(pmuval_degrees(128), 180.0);
        assert_eq!(pmuval_degrees(192), 270.0);
    }

    #[test]
    fn pmuval_from_degrees_roundtrip() {
        // Every raw value should round-trip through degrees -> raw -> degrees
        // with at most one LSB of slop.
        for raw in 0u8..=255 {
            let deg = pmuval_degrees(raw);
            let back = pmuval_from_degrees(deg);
            assert_eq!(back, raw, "deg={} raw={}", deg, raw);
        }
    }

    #[test]
    fn pmuval_from_degrees_wraps_negative_and_overrange() {
        // 360° wraps to 0; -90° wraps to 270°.
        assert_eq!(pmuval_from_degrees(360.0), 0);
        assert_eq!(pmuval_from_degrees(-90.0), pmuval_from_degrees(270.0));
    }

    #[test]
    fn rxbwc_table_endpoints() {
        assert_eq!(rxbwc_khz(0), Some(160));
        assert_eq!(rxbwc_khz(11), Some(2000));
        assert_eq!(rxbwc_khz(12), None);
    }

    #[test]
    fn txcutc_table_endpoints() {
        assert_eq!(txcutc_khz(0), Some(80));
        assert_eq!(txcutc_khz(11), Some(1000));
        assert_eq!(txcutc_khz(12), None);
    }

    #[test]
    fn dfe_sr_known_codes() {
        assert_eq!(dfe_sr_khz(1), Some(4000));
        assert_eq!(dfe_sr_khz(4), Some(1000));
        assert_eq!(dfe_sr_khz(10), Some(400));
        assert_eq!(dfe_sr_khz(7), None);
        assert_eq!(dfe_sr_khz(0), None);
    }

    #[test]
    fn edd_us_examples() {
        // df=0 disables averaging.
        assert_eq!(edd_us(0, 0), 0);
        // dtb=0 -> 2 µs, df=10 -> 20 µs total.
        assert_eq!(edd_us(0, 10), 20);
        // dtb=3 -> 128 µs, df=4 -> 512 µs total.
        assert_eq!(edd_us(3, 4), 512);
    }

    #[test]
    fn agcc_avg_table() {
        assert_eq!(agcc_avg_samples(0), 8);
        assert_eq!(agcc_avg_samples(1), 16);
        assert_eq!(agcc_avg_samples(2), 32);
        assert_eq!(agcc_avg_samples(3), 64);
    }

    #[test]
    fn agcs_target_table() {
        assert_eq!(agcs_target_dbfs(0), -21);
        assert_eq!(agcs_target_dbfs(7), -42);
    }
}
