//! Object Dictionary surface for radio statistics.
//!
//! ## Example
//!
//! ```
//! use oresat_at86rf215_driver::od::{radio_stats_entry, sub, OdValue, RADIO_STATS_INDEX};
//! use oresat_at86rf215_driver::stats::RadioStats;
//!
//! let mut stats = RadioStats::new();
//! stats.record_tx();
//! stats.record_rx(-42);
//!
//! // Subindex 0 always reports the record's highest subindex.
//! let count = radio_stats_entry(&stats, RADIO_STATS_INDEX, sub::COUNT).unwrap();
//! assert_eq!(count.value, OdValue::U8(sub::HIGHEST));
//!
//! let tx = radio_stats_entry(&stats, RADIO_STATS_INDEX, sub::TX_COUNT).unwrap();
//! assert_eq!(tx.value, OdValue::U64(1));
//!
//! // Unknown (index, sub) returns None - the SDO server replies with the
//! // appropriate abort code.
//! assert!(radio_stats_entry(&stats, 0xFFFF, 0).is_none());
//! ```

use crate::stats::RadioStats;

/// CANopen manufacturer-specific index hosting the radio-stats record.
pub const RADIO_STATS_INDEX: u16 = 0x6000;

/// Subindex constants for the radio-stats record. Pinning them in one place
/// means a future OD agent, a CANopen EDS generator, and any downstream
/// consumer can all agree on the layout without re-deriving it.
#[allow(non_snake_case)]
pub mod sub {
    /// CANopen convention: sub 0 of a record returns the highest valid sub.
    pub const COUNT: u8 = 0;
    pub const TX_COUNT: u8 = 1;
    pub const RX_COUNT: u8 = 2;
    pub const RX_CRC_FAIL: u8 = 3;
    pub const TX_ERRORS: u8 = 4;
    pub const RSSI_LAST: u8 = 5;
    pub const RSSI_MIN: u8 = 6;
    pub const RSSI_MAX: u8 = 7;
    pub const RSSI_SUM: u8 = 8;
    pub const RSSI_SAMPLES: u8 = 9;
    pub const RSSI_MEAN: u8 = 10;
    pub const TICKS: u8 = 11;

    /// Highest valid subindex for the radio-stats record.
    pub const HIGHEST: u8 = TICKS;
}

/// Access mode for an OD entry. Mirrors the CANopen spec.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    /// Readable via SDO upload; writes abort.
    Ro,
    /// Writable via SDO download; reads abort.
    Wo,
    /// Both readable and writable.
    Rw,
}

/// Value variant carried by an OD entry. Covers the integer/float widths
/// the stats export actually uses; extend as new fields land.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OdValue {
    I8(i8),
    U8(u8),
    U16(u16),
    U32(u32),
    I64(i64),
    U64(u64),
    /// RSSI mean carried as f64. OD servers that can't encode floats may
    /// round to an integer dBm before publishing - the full precision is
    /// preserved here so the decision is theirs.
    F64(f64),
    /// RSSI-style sentinel: 127 means "no valid reading". Exposed as a
    /// distinct variant so a CANopen-side mapping can represent the NA
    /// state without overloading an integer.
    RssiOrInvalid(i8),
}

/// A single OD dictionary entry - what an SDO upload returns.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OdEntry {
    pub index: u16,
    pub subindex: u8,
    pub name: &'static str,
    pub value: OdValue,
    pub access: Access,
}

/// Look up a radio-stats entry by `(index, subindex)`. Returns `None` for
/// any unknown address - the caller (SDO server) maps that to a CANopen
/// abort code (`0x0602_0000` "object does not exist").
pub fn radio_stats_entry(stats: &RadioStats, index: u16, sub: u8) -> Option<OdEntry> {
    if index != RADIO_STATS_INDEX {
        return None;
    }
    let (name, value) = match sub {
        sub::COUNT => ("radio_stats.count", OdValue::U8(sub::HIGHEST)),
        sub::TX_COUNT => ("tx_count", OdValue::U64(stats.tx_count)),
        sub::RX_COUNT => ("rx_count", OdValue::U64(stats.rx_count)),
        sub::RX_CRC_FAIL => ("rx_crc_fail", OdValue::U64(stats.rx_crc_fail)),
        sub::TX_ERRORS => ("tx_errors", OdValue::U64(stats.tx_errors)),
        sub::RSSI_LAST => ("rssi_last_dbm", OdValue::RssiOrInvalid(stats.rssi_last)),
        sub::RSSI_MIN => ("rssi_min_dbm", OdValue::RssiOrInvalid(stats.rssi_min)),
        sub::RSSI_MAX => ("rssi_max_dbm", OdValue::I8(stats.rssi_max)),
        sub::RSSI_SUM => ("rssi_sum", OdValue::I64(stats.rssi_sum)),
        sub::RSSI_SAMPLES => ("rssi_samples", OdValue::U64(stats.rssi_samples)),
        sub::RSSI_MEAN => (
            "rssi_mean_dbm",
            OdValue::F64(stats.rssi_mean().unwrap_or(f64::NAN)),
        ),
        sub::TICKS => ("ticks", OdValue::U64(stats.ticks)),
        _ => return None,
    };
    Some(OdEntry {
        index,
        subindex: sub,
        name,
        value,
        access: Access::Ro,
    })
}

/// Iterate every defined entry in order (sub 0 first, then 1..=HIGHEST).
pub fn radio_stats_entries(stats: &RadioStats) -> impl Iterator<Item = OdEntry> + '_ {
    (0..=sub::HIGHEST).filter_map(move |s| radio_stats_entry(stats, RADIO_STATS_INDEX, s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subindex_zero_returns_highest_as_u8() {
        let stats = RadioStats::new();
        let e = radio_stats_entry(&stats, RADIO_STATS_INDEX, sub::COUNT).unwrap();
        assert_eq!(e.value, OdValue::U8(sub::HIGHEST));
        assert_eq!(e.access, Access::Ro);
        assert_eq!(e.index, RADIO_STATS_INDEX);
    }

    #[test]
    fn unknown_index_returns_none() {
        let stats = RadioStats::new();
        assert!(radio_stats_entry(&stats, 0x1000, 0).is_none());
        assert!(radio_stats_entry(&stats, 0x6001, 0).is_none());
    }

    #[test]
    fn unknown_subindex_returns_none() {
        let stats = RadioStats::new();
        assert!(radio_stats_entry(&stats, RADIO_STATS_INDEX, sub::HIGHEST + 1).is_none());
        assert!(radio_stats_entry(&stats, RADIO_STATS_INDEX, 0xFF).is_none());
    }

    #[test]
    fn stats_fields_round_trip_through_od() {
        let mut stats = RadioStats::new();
        stats.record_tx();
        stats.record_tx();
        stats.record_rx(-42);
        stats.record_rx(-60);
        stats.record_crc_fail();
        stats.tick();

        let tx = radio_stats_entry(&stats, RADIO_STATS_INDEX, sub::TX_COUNT).unwrap();
        assert_eq!(tx.value, OdValue::U64(2));

        let rx = radio_stats_entry(&stats, RADIO_STATS_INDEX, sub::RX_COUNT).unwrap();
        assert_eq!(rx.value, OdValue::U64(2));

        let crc = radio_stats_entry(&stats, RADIO_STATS_INDEX, sub::RX_CRC_FAIL).unwrap();
        assert_eq!(crc.value, OdValue::U64(1));

        let last = radio_stats_entry(&stats, RADIO_STATS_INDEX, sub::RSSI_LAST).unwrap();
        assert_eq!(last.value, OdValue::RssiOrInvalid(-60));

        let samples = radio_stats_entry(&stats, RADIO_STATS_INDEX, sub::RSSI_SAMPLES).unwrap();
        assert_eq!(samples.value, OdValue::U64(2));

        let mean = radio_stats_entry(&stats, RADIO_STATS_INDEX, sub::RSSI_MEAN).unwrap();
        match mean.value {
            OdValue::F64(v) => assert!((v - (-51.0)).abs() < 0.01, "mean was {v}"),
            other => panic!("expected F64, got {other:?}"),
        }

        let ticks = radio_stats_entry(&stats, RADIO_STATS_INDEX, sub::TICKS).unwrap();
        assert_eq!(ticks.value, OdValue::U64(1));
    }

    #[test]
    fn rssi_mean_is_nan_when_no_samples() {
        let stats = RadioStats::new();
        let mean = radio_stats_entry(&stats, RADIO_STATS_INDEX, sub::RSSI_MEAN).unwrap();
        match mean.value {
            OdValue::F64(v) => assert!(v.is_nan()),
            other => panic!("expected NaN F64, got {other:?}"),
        }
    }

    #[test]
    fn iterate_enumerates_count_plus_all_fields() {
        let stats = RadioStats::new();
        let entries: Vec<_> = radio_stats_entries(&stats).collect();
        // sub 0 (count) + sub 1..=HIGHEST
        assert_eq!(entries.len(), usize::from(sub::HIGHEST) + 1);
        // Subindices are contiguous 0..=HIGHEST.
        for (i, e) in entries.iter().enumerate() {
            assert_eq!(e.subindex as usize, i);
            assert_eq!(e.index, RADIO_STATS_INDEX);
            assert_eq!(e.access, Access::Ro);
        }
    }

    #[test]
    fn every_subindex_constant_has_a_matching_entry() {
        let stats = RadioStats::new();
        for s in [
            sub::COUNT,
            sub::TX_COUNT,
            sub::RX_COUNT,
            sub::RX_CRC_FAIL,
            sub::TX_ERRORS,
            sub::RSSI_LAST,
            sub::RSSI_MIN,
            sub::RSSI_MAX,
            sub::RSSI_SUM,
            sub::RSSI_SAMPLES,
            sub::RSSI_MEAN,
            sub::TICKS,
        ] {
            assert!(
                radio_stats_entry(&stats, RADIO_STATS_INDEX, s).is_some(),
                "subindex {s} is declared but not resolved"
            );
        }
    }
}
