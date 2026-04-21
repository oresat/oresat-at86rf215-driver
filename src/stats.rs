//! Runtime radio statistics for Object Dictionary / telemetry export.
//!
//! `RadioStats` aggregates counters and signal-quality metrics that the
//! daemon maintains across its lifetime.  It is designed to be cheaply
//! serialisable (CBOR or TOML) for export to a system Object Dictionary,
//! a monitoring socket, or a TUI dashboard.
//!
//! All fields are plain integers/floats so the struct is `Copy`.

use serde::{Deserialize, Serialize};

/// Aggregated radio statistics.
///
/// The daemon updates these on every TX/RX event and periodic telemetry
/// tick.  A consumer (Object Dictionary agent, TUI, monitoring endpoint)
/// can snapshot the struct at any time.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RadioStats {
    // ── packet counters ────────────────────────────────────────────────
    /// Total frames transmitted.
    pub tx_count: u64,
    /// Total frames received (CRC valid).
    pub rx_count: u64,
    /// Frames dropped due to invalid CRC.
    pub rx_crc_fail: u64,
    /// TX errors (SPI failures, FIFO overflows, etc.).
    pub tx_errors: u64,

    // ── RSSI tracking ──────────────────────────────────────────────────
    /// Most recent RSSI reading (dBm, 127 = invalid).
    pub rssi_last: i8,
    /// Minimum RSSI seen since reset.
    pub rssi_min: i8,
    /// Maximum RSSI seen since reset.
    pub rssi_max: i8,
    /// Running sum for mean calculation (divide by `rssi_samples`).
    pub rssi_sum: i64,
    /// Number of RSSI samples accumulated.
    pub rssi_samples: u64,

    // ── uptime ─────────────────────────────────────────────────────────
    /// Telemetry ticks since daemon start.
    pub ticks: u64,
}

impl RadioStats {
    /// Create a zeroed stats snapshot.
    pub fn new() -> Self {
        Self {
            tx_count: 0,
            rx_count: 0,
            rx_crc_fail: 0,
            tx_errors: 0,
            rssi_last: 127, // invalid
            rssi_min: 127,
            rssi_max: -128,
            rssi_sum: 0,
            rssi_samples: 0,
            ticks: 0,
        }
    }

    /// Record a successful transmission.
    pub fn record_tx(&mut self) {
        self.tx_count += 1;
    }

    /// Record a TX error.
    pub fn record_tx_error(&mut self) {
        self.tx_errors += 1;
    }

    /// Record a successfully received frame with its RSSI.
    pub fn record_rx(&mut self, rssi: i8) {
        self.rx_count += 1;
        self.update_rssi(rssi);
    }

    /// Record a CRC failure (frame received but dropped).
    pub fn record_crc_fail(&mut self) {
        self.rx_crc_fail += 1;
    }

    /// Update RSSI tracking from a periodic status read or RX event.
    pub fn update_rssi(&mut self, rssi: i8) {
        if rssi == 127 {
            return; // invalid reading
        }
        self.rssi_last = rssi;
        if rssi < self.rssi_min {
            self.rssi_min = rssi;
        }
        if rssi > self.rssi_max {
            self.rssi_max = rssi;
        }
        self.rssi_sum += rssi as i64;
        self.rssi_samples += 1;
    }

    /// Increment the tick counter (called once per telemetry interval).
    pub fn tick(&mut self) {
        self.ticks += 1;
    }

    /// Mean RSSI in dBm, or `None` if no samples.
    pub fn rssi_mean(&self) -> Option<f64> {
        if self.rssi_samples == 0 {
            None
        } else {
            Some(self.rssi_sum as f64 / self.rssi_samples as f64)
        }
    }
}

impl Default for RadioStats {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_stats_are_zeroed() {
        let s = RadioStats::new();
        assert_eq!(s.tx_count, 0);
        assert_eq!(s.rx_count, 0);
        assert_eq!(s.rssi_last, 127);
        assert_eq!(s.rssi_mean(), None);
    }

    #[test]
    fn record_tx_increments() {
        let mut s = RadioStats::new();
        s.record_tx();
        s.record_tx();
        assert_eq!(s.tx_count, 2);
    }

    #[test]
    fn record_rx_tracks_rssi() {
        let mut s = RadioStats::new();
        s.record_rx(-80);
        s.record_rx(-60);
        s.record_rx(-70);
        assert_eq!(s.rx_count, 3);
        assert_eq!(s.rssi_last, -70);
        assert_eq!(s.rssi_min, -80);
        assert_eq!(s.rssi_max, -60);
        assert!((s.rssi_mean().unwrap() - (-70.0)).abs() < 0.01);
    }

    #[test]
    fn invalid_rssi_ignored() {
        let mut s = RadioStats::new();
        s.update_rssi(127); // invalid
        assert_eq!(s.rssi_samples, 0);
        assert_eq!(s.rssi_last, 127);
        assert_eq!(s.rssi_mean(), None);
    }

    #[test]
    fn crc_fail_separate_from_rx() {
        let mut s = RadioStats::new();
        s.record_rx(-50);
        s.record_crc_fail();
        s.record_crc_fail();
        assert_eq!(s.rx_count, 1);
        assert_eq!(s.rx_crc_fail, 2);
    }

    #[test]
    fn cbor_roundtrip() {
        let mut s = RadioStats::new();
        s.record_tx();
        s.record_rx(-42);
        s.tick();
        let mut buf = Vec::new();
        ciborium::ser::into_writer(&s, &mut buf).unwrap();
        let decoded: RadioStats = ciborium::de::from_reader(&buf[..]).unwrap();
        assert_eq!(s, decoded);
    }
}
