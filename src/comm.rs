//! Socket <-> radio bridge and telemetry types.
//!
//! The daemon accepts packets on UDP sockets and stages them for TX,
//! and forwards RX packets back out to a socket.  Telemetry (register
//! snapshots, RSSI, etc.) is CBOR-encoded to a separate socket for
//! the TUI viewer.

use std::net::UdpSocket;

use serde::{Deserialize, Serialize};

use crate::stats::RadioStats;

/// Telemetry envelope - CBOR-encoded and sent to the viewer socket.
///
/// Mirrors the ax5043 `CommState` pattern: each variant is a discrete
/// telemetry datum so the viewer can update its widgets independently.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum CommState {
    /// A frame received from the radio.
    Rx(RxPacket),
    /// A frame queued or sent via the radio.
    Tx(TxPacket),
    /// Periodic RF09 status snapshot.
    Rf09Status(RfStatus),
    /// Periodic RF24 status snapshot.
    Rf24Status(RfStatus),
    /// BBC0 status snapshot.
    Bbc0Status(BbcStatus),
    /// BBC1 status snapshot.
    Bbc1Status(BbcStatus),
    /// Aggregated radio statistics (packet counts, RSSI tracking, uptime).
    Stats(RadioStats),
}

/// A received frame with metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RxPacket {
    /// Raw frame bytes (excluding FCS if the chip stripped it).
    pub data: Vec<u8>,
    /// RSSI at time of reception (dBm, 127 = invalid).
    pub rssi: i8,
    /// Energy detector value at time of reception (dBm).
    pub edv: i8,
}

/// A transmitted frame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TxPacket {
    /// Raw frame bytes as sent.
    pub data: Vec<u8>,
}

/// Snapshot of one RF transceiver's key status registers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RfStatus {
    /// Current state machine state.
    pub state: u8,
    /// RSSI reading (dBm, 127 = invalid).
    pub rssi: i8,
    /// Energy detector value (dBm).
    pub edv: i8,
    /// AGC gain control word.
    pub agc_gcw: u8,
}

/// Snapshot of one baseband core's status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BbcStatus {
    /// PHY type (0=FSK, 1=OFDM, 2=OQPSK, 3=Legacy).
    pub phy_type: u8,
    /// RX frame length register.
    pub rxfl: u16,
    /// TX frame length register.
    pub txfl: u16,
    /// Symbol counter.
    pub cnt: u32,
}

impl CommState {
    /// CBOR-encode and send to a UDP socket. Returns the number of bytes sent.
    pub fn send(&self, socket: &UdpSocket) -> std::io::Result<usize> {
        let mut buf = Vec::new();
        ciborium::ser::into_writer(self, &mut buf)
            .map_err(std::io::Error::other)?;
        socket.send(&buf)
    }

    /// CBOR-encode into a `Vec<u8>` (no socket needed).
    pub fn encode(&self) -> Result<Vec<u8>, ciborium::ser::Error<std::io::Error>> {
        let mut buf = Vec::new();
        ciborium::ser::into_writer(self, &mut buf)?;
        Ok(buf)
    }

    /// Decode a CBOR-encoded CommState from a byte slice.
    pub fn decode(buf: &[u8]) -> Result<Self, ciborium::de::Error<std::io::Error>> {
        ciborium::de::from_reader(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rx_packet_cbor_roundtrip() {
        let orig = CommState::Rx(RxPacket {
            data: vec![0xDE, 0xAD, 0xBE, 0xEF],
            rssi: -42,
            edv: -55,
        });
        let bytes = orig.encode().unwrap();
        let decoded = CommState::decode(&bytes).unwrap();
        assert_eq!(orig, decoded);
    }

    #[test]
    fn tx_packet_cbor_roundtrip() {
        let orig = CommState::Tx(TxPacket {
            data: vec![1, 2, 3],
        });
        let bytes = orig.encode().unwrap();
        let decoded = CommState::decode(&bytes).unwrap();
        assert_eq!(orig, decoded);
    }

    #[test]
    fn rf_status_cbor_roundtrip() {
        let orig = CommState::Rf09Status(RfStatus {
            state: 0x04, // Rx
            rssi: -80,
            edv: -75,
            agc_gcw: 12,
        });
        let bytes = orig.encode().unwrap();
        let decoded = CommState::decode(&bytes).unwrap();
        assert_eq!(orig, decoded);
    }

    #[test]
    fn bbc_status_cbor_roundtrip() {
        let orig = CommState::Bbc0Status(BbcStatus {
            phy_type: 1, // OFDM
            rxfl: 128,
            txfl: 256,
            cnt: 1_000_000,
        });
        let bytes = orig.encode().unwrap();
        let decoded = CommState::decode(&bytes).unwrap();
        assert_eq!(orig, decoded);
    }

    #[test]
    fn empty_frame_roundtrip() {
        let orig = CommState::Rx(RxPacket {
            data: vec![],
            rssi: 127, // invalid
            edv: 0,
        });
        let bytes = orig.encode().unwrap();
        let decoded = CommState::decode(&bytes).unwrap();
        assert_eq!(orig, decoded);
    }
}
