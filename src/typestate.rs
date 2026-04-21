//! Type-state encoding of the AT86RF215 transceiver state machine (datasheet fig. 5-5).
//!
//! The `Radio` struct is the full register snapshot; it has no opinion on which
//! commands are legal when. This module layers a typed state machine on top so
//! illegal transitions (e.g. `Tx` from `Sleep`) fail to compile.
//!
//! Each method consumes `self` and returns a new `Transceiver<NewState>` - the
//! previous token is moved away so stale state cannot be used. Commands are
//! staged into the underlying `Radio` by writing the matching `TransceiverCmd`
//! to `rf09_cmd` / `rf24_cmd`; the caller still owns the byte-array generation
//! and SPI transport.
//!
//! Only RF09 and RF24 are currently wrapped. Baseband (BBC) registers are
//! unaffected.

use core::marker::PhantomData;

use crate::radio::Radio;
use crate::registers::{RfnCmd, RfnState, TransceiverCmd, TransceiverState};

/// Which transceiver a given `Transceiver<S>` talks to.
pub trait Band: sealed::Sealed {
    /// Write `cmd` to the transceiver's RFn_CMD register on `radio`.
    fn write_cmd(radio: &mut Radio, cmd: TransceiverCmd);

    /// Read the in-memory snapshot of the transceiver's RFn_STATE register.
    /// Caller is responsible for refreshing the snapshot from real SPI first.
    fn read_state(radio: &Radio) -> TransceiverState;

    /// Overwrite the in-memory snapshot of RFn_STATE. Tests use this; production
    /// code typically updates the snapshot via SPI read.
    fn write_state(radio: &mut Radio, state: TransceiverState);
}

/// RF09 sub-1 GHz marker.
pub struct Rf09;
/// RF24 2.4 GHz marker.
pub struct Rf24;

impl Band for Rf09 {
    fn write_cmd(radio: &mut Radio, cmd: TransceiverCmd) {
        radio.rf09_cmd.value = RfnCmd::new().with_cmd(cmd);
    }
    fn read_state(radio: &Radio) -> TransceiverState {
        radio.rf09_state.value.state()
    }
    fn write_state(radio: &mut Radio, state: TransceiverState) {
        radio.rf09_state.value = RfnState::from(state.into_bits());
    }
}
impl Band for Rf24 {
    fn write_cmd(radio: &mut Radio, cmd: TransceiverCmd) {
        radio.rf24_cmd.value = RfnCmd::new().with_cmd(cmd);
    }
    fn read_state(radio: &Radio) -> TransceiverState {
        radio.rf24_state.value.state()
    }
    fn write_state(radio: &mut Radio, state: TransceiverState) {
        radio.rf24_state.value = RfnState::from(state.into_bits());
    }
}

/// State markers - zero-sized, used only as a type parameter.
pub struct Sleep;
pub struct TrxOff;
pub struct TxPrep;
pub struct Tx;
pub struct Rx;

/// A typed handle that bundles the full `Radio` snapshot with a compile-time
/// transceiver state.
pub struct Transceiver<B: Band, S> {
    pub radio: Radio,
    _band: PhantomData<B>,
    _state: PhantomData<S>,
}

impl<B: Band, S> Transceiver<B, S> {
    fn transition<N>(mut self, cmd: TransceiverCmd) -> Transceiver<B, N> {
        B::write_cmd(&mut self.radio, cmd);
        Transceiver { radio: self.radio, _band: PhantomData, _state: PhantomData }
    }

    /// Drop the typed wrapper and return the underlying `Radio`.
    pub fn into_radio(self) -> Radio {
        self.radio
    }

    /// Re-tag this handle based on what the in-memory `RFn_STATE` snapshot
    /// actually says - useful after the chip has autonomously transitioned
    /// (`Tx -> TxPrep` at end-of-frame, `trxerr -> TrxOff`, etc.). The caller is
    /// responsible for refreshing the snapshot from a real SPI read first.
    pub fn reconcile(self) -> AnyTransceiver<B> {
        AnyTransceiver::from_radio(self.radio)
    }
}

/// Result of [`Transceiver::reconcile`] / [`AnyTransceiver::from_radio`].
///
/// Sleep is not directly observable: the chip reports both `Reset` and `Sleep`
/// as the `Reset` state register code, and software must remember which one it
/// commanded. The runtime API surfaces that ambiguity as the `Reset` variant.
pub enum AnyTransceiver<B: Band> {
    TrxOff(Transceiver<B, TrxOff>),
    TxPrep(Transceiver<B, TxPrep>),
    Tx(Transceiver<B, Tx>),
    Rx(Transceiver<B, Rx>),
    /// Chip is mid-transition. Caller should refresh the snapshot and try again.
    Transition(Radio),
    /// Chip is in Reset or Sleep. Software, not the state register, knows which.
    Reset(Radio),
}

impl<B: Band> AnyTransceiver<B> {
    /// Read the in-memory `RFn_STATE` snapshot and tag the radio accordingly.
    pub fn from_radio(radio: Radio) -> Self {
        match B::read_state(&radio) {
            TransceiverState::TrxOff => Self::TrxOff(typed(radio)),
            TransceiverState::TxPrep => Self::TxPrep(typed(radio)),
            TransceiverState::Tx => Self::Tx(typed(radio)),
            TransceiverState::Rx => Self::Rx(typed(radio)),
            TransceiverState::Transition => Self::Transition(radio),
            TransceiverState::Reset => Self::Reset(radio),
        }
    }
}

fn typed<B: Band, S>(radio: Radio) -> Transceiver<B, S> {
    Transceiver { radio, _band: PhantomData, _state: PhantomData }
}

impl<B: Band> Transceiver<B, TrxOff> {
    /// Wrap a fresh `Radio` in `TrxOff`. The chip powers up into this state
    /// after reset, so it is the only valid entry point.
    pub fn new(radio: Radio) -> Self {
        Self { radio, _band: PhantomData, _state: PhantomData }
    }

    pub fn sleep(self) -> Transceiver<B, Sleep> {
        self.transition(TransceiverCmd::Sleep)
    }
    pub fn tx_prep(self) -> Transceiver<B, TxPrep> {
        self.transition(TransceiverCmd::TxPrep)
    }
}

impl<B: Band> Transceiver<B, Sleep> {
    pub fn trx_off(self) -> Transceiver<B, TrxOff> {
        self.transition(TransceiverCmd::TrxOff)
    }
}

impl<B: Band> Transceiver<B, TxPrep> {
    pub fn trx_off(self) -> Transceiver<B, TrxOff> {
        self.transition(TransceiverCmd::TrxOff)
    }
    pub fn tx(self) -> Transceiver<B, Tx> {
        self.transition(TransceiverCmd::Tx)
    }
    pub fn rx(self) -> Transceiver<B, Rx> {
        self.transition(TransceiverCmd::Rx)
    }
}

impl<B: Band> Transceiver<B, Tx> {
    /// The datasheet routes `TX -> TX_PREP` automatically when the frame ends;
    /// this is the software-side acknowledgement of that transition.
    pub fn tx_prep(self) -> Transceiver<B, TxPrep> {
        self.transition(TransceiverCmd::TxPrep)
    }
    pub fn trx_off(self) -> Transceiver<B, TrxOff> {
        self.transition(TransceiverCmd::TrxOff)
    }
}

impl<B: Band> Transceiver<B, Rx> {
    pub fn tx_prep(self) -> Transceiver<B, TxPrep> {
        self.transition(TransceiverCmd::TxPrep)
    }
    pub fn trx_off(self) -> Transceiver<B, TrxOff> {
        self.transition(TransceiverCmd::TrxOff)
    }
}

// ── borrowing variant ──────────────────────────────────────────────────────
//
// `TransceiverRef<'r, B, S>` is the borrowing sibling of `Transceiver<B, S>`:
// it holds `&'r mut Radio` instead of taking ownership, so callers can run a
// typed state transition on a `Radio` field of some larger struct without
// moving it out and back.

/// Borrowing variant of [`Transceiver`] - the `Radio` stays with the caller.
pub struct TransceiverRef<'r, B: Band, S> {
    pub radio: &'r mut Radio,
    _band: PhantomData<B>,
    _state: PhantomData<S>,
}

impl<'r, B: Band, S> TransceiverRef<'r, B, S> {
    fn transition<N>(self, cmd: TransceiverCmd) -> TransceiverRef<'r, B, N> {
        B::write_cmd(self.radio, cmd);
        TransceiverRef { radio: self.radio, _band: PhantomData, _state: PhantomData }
    }

    /// Re-tag this handle based on the in-memory `RFn_STATE` snapshot.
    /// Caller must refresh the snapshot from SPI first.
    pub fn reconcile(self) -> AnyTransceiverRef<'r, B> {
        AnyTransceiverRef::from_radio(self.radio)
    }
}

/// Result of [`TransceiverRef::reconcile`] / [`AnyTransceiverRef::from_radio`].
pub enum AnyTransceiverRef<'r, B: Band> {
    TrxOff(TransceiverRef<'r, B, TrxOff>),
    TxPrep(TransceiverRef<'r, B, TxPrep>),
    Tx(TransceiverRef<'r, B, Tx>),
    Rx(TransceiverRef<'r, B, Rx>),
    /// Chip is mid-transition - caller should refresh the snapshot and retry.
    Transition(&'r mut Radio),
    /// Chip is in Reset or Sleep - software, not the state register, knows which.
    Reset(&'r mut Radio),
}

impl<'r, B: Band> AnyTransceiverRef<'r, B> {
    pub fn from_radio(radio: &'r mut Radio) -> Self {
        match B::read_state(radio) {
            TransceiverState::TrxOff => Self::TrxOff(typed_ref(radio)),
            TransceiverState::TxPrep => Self::TxPrep(typed_ref(radio)),
            TransceiverState::Tx => Self::Tx(typed_ref(radio)),
            TransceiverState::Rx => Self::Rx(typed_ref(radio)),
            TransceiverState::Transition => Self::Transition(radio),
            TransceiverState::Reset => Self::Reset(radio),
        }
    }
}

fn typed_ref<B: Band, S>(radio: &mut Radio) -> TransceiverRef<'_, B, S> {
    TransceiverRef { radio, _band: PhantomData, _state: PhantomData }
}

impl<'r, B: Band> TransceiverRef<'r, B, TrxOff> {
    /// Borrow a `Radio` as a typed `TrxOff` handle. The chip powers up into
    /// this state after reset, so it is the only safe entry point.
    pub fn new(radio: &'r mut Radio) -> Self {
        Self { radio, _band: PhantomData, _state: PhantomData }
    }

    pub fn sleep(self) -> TransceiverRef<'r, B, Sleep> {
        self.transition(TransceiverCmd::Sleep)
    }
    pub fn tx_prep(self) -> TransceiverRef<'r, B, TxPrep> {
        self.transition(TransceiverCmd::TxPrep)
    }
}

impl<'r, B: Band> TransceiverRef<'r, B, Sleep> {
    pub fn trx_off(self) -> TransceiverRef<'r, B, TrxOff> {
        self.transition(TransceiverCmd::TrxOff)
    }
}

impl<'r, B: Band> TransceiverRef<'r, B, TxPrep> {
    pub fn trx_off(self) -> TransceiverRef<'r, B, TrxOff> {
        self.transition(TransceiverCmd::TrxOff)
    }
    pub fn tx(self) -> TransceiverRef<'r, B, Tx> {
        self.transition(TransceiverCmd::Tx)
    }
    pub fn rx(self) -> TransceiverRef<'r, B, Rx> {
        self.transition(TransceiverCmd::Rx)
    }
}

impl<'r, B: Band> TransceiverRef<'r, B, Tx> {
    pub fn tx_prep(self) -> TransceiverRef<'r, B, TxPrep> {
        self.transition(TransceiverCmd::TxPrep)
    }
    pub fn trx_off(self) -> TransceiverRef<'r, B, TrxOff> {
        self.transition(TransceiverCmd::TrxOff)
    }
}

impl<'r, B: Band> TransceiverRef<'r, B, Rx> {
    pub fn tx_prep(self) -> TransceiverRef<'r, B, TxPrep> {
        self.transition(TransceiverCmd::TxPrep)
    }
    pub fn trx_off(self) -> TransceiverRef<'r, B, TrxOff> {
        self.transition(TransceiverCmd::TrxOff)
    }
}

mod sealed {
    pub trait Sealed {}
    impl Sealed for super::Rf09 {}
    impl Sealed for super::Rf24 {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trxoff_to_tx_to_trxoff_stages_commands() {
        let trx: Transceiver<Rf09, TrxOff> = Transceiver::new(Radio::new());
        let trx = trx.tx_prep().tx().tx_prep().trx_off();
        // Last command staged is TrxOff.
        assert_eq!(trx.radio.rf09_cmd.value.cmd(), TransceiverCmd::TrxOff);
    }

    #[test]
    fn rf24_sleep_roundtrip() {
        let trx: Transceiver<Rf24, TrxOff> = Transceiver::new(Radio::new());
        let trx = trx.sleep().trx_off();
        assert_eq!(trx.radio.rf24_cmd.value.cmd(), TransceiverCmd::TrxOff);
    }

    /// After the chip autonomously moves `Tx -> TxPrep` at end-of-frame, a
    /// snapshot read should let `reconcile()` re-tag a `Transceiver<_, Tx>`
    /// handle as a `Transceiver<_, TxPrep>` handle.
    #[test]
    fn reconcile_tx_to_txprep_after_frame_end() {
        let mut radio = Radio::new();
        Rf09::write_state(&mut radio, TransceiverState::TxPrep);
        let trx: Transceiver<Rf09, Tx> = typed(radio);
        match trx.reconcile() {
            AnyTransceiver::TxPrep(_) => {}
            _ => panic!("expected TxPrep"),
        }
    }

    /// `trxerr` drops the chip to TrxOff. Reconciliation should surface that.
    #[test]
    fn reconcile_rx_to_trxoff_after_trxerr() {
        let mut radio = Radio::new();
        Rf24::write_state(&mut radio, TransceiverState::TrxOff);
        let trx: Transceiver<Rf24, Rx> = typed(radio);
        match trx.reconcile() {
            AnyTransceiver::TrxOff(_) => {}
            _ => panic!("expected TrxOff"),
        }
    }

    /// Mid-transition reads should surface the `Transition` variant so the
    /// caller knows to retry.
    #[test]
    fn reconcile_transition_returns_radio() {
        let mut radio = Radio::new();
        Rf09::write_state(&mut radio, TransceiverState::Transition);
        match AnyTransceiver::<Rf09>::from_radio(radio) {
            AnyTransceiver::Transition(_) => {}
            _ => panic!("expected Transition"),
        }
    }

    /// Borrowing variant: the `Radio` stays with the caller after the typed
    /// transitions complete.
    #[test]
    fn transceiver_ref_does_not_move_radio() {
        let mut radio = Radio::new();
        {
            let trx: TransceiverRef<Rf09, TrxOff> = TransceiverRef::new(&mut radio);
            let _ = trx.tx_prep().rx().trx_off();
        }
        // Radio is still usable - no move happened.
        assert_eq!(radio.rf09_cmd.value.cmd(), TransceiverCmd::TrxOff);
    }

    /// `TransceiverRef::reconcile` re-tags the borrowed handle from the
    /// in-memory state snapshot, just like the owning variant.
    #[test]
    fn transceiver_ref_reconcile_tags_from_state() {
        let mut radio = Radio::new();
        Rf24::write_state(&mut radio, TransceiverState::Rx);
        let trx: TransceiverRef<Rf24, TxPrep> = typed_ref(&mut radio);
        match trx.reconcile() {
            AnyTransceiverRef::Rx(_) => {}
            _ => panic!("expected Rx"),
        }
    }
}
