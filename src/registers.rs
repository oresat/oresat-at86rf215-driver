//! AT86RF215 Register Definitions

use bitfield_struct::bitfield;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadOnly<T, const ADDR: u16, const SIZE: usize> {
    pub value: T,
}

impl<T, const ADDR: u16, const SIZE: usize> ReadOnly<T, ADDR, SIZE> {
    pub const fn new(value: T) -> Self {
        Self { value }
    }

    pub const fn address(&self) -> u16 {
        ADDR
    }

    pub const fn size(&self) -> usize {
        SIZE
    }

    pub fn read_command(&self) -> Vec<u8> {
        let header = generate_read_header(ADDR);
        let mut cmd = Vec::with_capacity(2 + SIZE);
        cmd.push(header[0]);
        cmd.push(header[1]);
        cmd.resize(2 + SIZE, 0x00); // Add SIZE dummy bytes
        cmd
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WriteOnly<T, const ADDR: u16, const SIZE: usize> {
    pub value: T,
}

impl<T, const ADDR: u16, const SIZE: usize> WriteOnly<T, ADDR, SIZE> {
    pub const fn new(value: T) -> Self {
        Self { value }
    }

    pub const fn address(&self) -> u16 {
        ADDR
    }

    pub const fn size(&self) -> usize {
        SIZE
    }
}

impl<T: Copy + Into<u8>, const ADDR: u16, const SIZE: usize> WriteOnly<T, ADDR, SIZE> {
    pub fn write_command(&self) -> Vec<u8> {
        let bits: u8 = self.value.into();
        build_write_command(generate_write_header(ADDR), bits.to_le_vec())
    }
}

impl<T: Copy + Into<u16>, const ADDR: u16, const SIZE: usize> WriteOnly<T, ADDR, SIZE> {
    pub fn write_command_u16(&self) -> Vec<u8> {
        let bits: u16 = self.value.into();
        build_write_command(generate_write_header(ADDR), bits.to_le_vec())
    }
}

impl<T: Copy + Into<u32>, const ADDR: u16, const SIZE: usize> WriteOnly<T, ADDR, SIZE> {
    pub fn write_command_u32(&self) -> Vec<u8> {
        let bits: u32 = self.value.into();
        build_write_command(generate_write_header(ADDR), bits.to_le_vec())
    }
}

impl<T: Copy + Into<u64>, const ADDR: u16, const SIZE: usize> WriteOnly<T, ADDR, SIZE> {
    pub fn write_command_u64(&self) -> Vec<u8> {
        let bits: u64 = self.value.into();
        build_write_command(generate_write_header(ADDR), bits.to_le_vec())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadWrite<T, const ADDR: u16, const SIZE: usize> {
    pub value: T,
}

impl<T, const ADDR: u16, const SIZE: usize> ReadWrite<T, ADDR, SIZE> {
    pub const fn new(value: T) -> Self {
        Self { value }
    }

    pub const fn address(&self) -> u16 {
        ADDR
    }

    pub const fn size(&self) -> usize {
        SIZE
    }

    pub fn read_command(&self) -> Vec<u8> {
        let header = generate_read_header(ADDR);
        let mut cmd = Vec::with_capacity(2 + SIZE);
        cmd.push(header[0]);
        cmd.push(header[1]);
        cmd.resize(2 + SIZE, 0x00); // Add SIZE dummy bytes
        cmd
    }
}

impl<T: Copy + Into<u8>, const ADDR: u16, const SIZE: usize> ReadWrite<T, ADDR, SIZE> {
    pub fn write_command(&self) -> Vec<u8> {
        let bits: u8 = self.value.into();
        build_write_command(generate_write_header(ADDR), bits.to_le_vec())
    }
}

impl<T: Copy + Into<u16>, const ADDR: u16, const SIZE: usize> ReadWrite<T, ADDR, SIZE> {
    pub fn write_command_u16(&self) -> Vec<u8> {
        let bits: u16 = self.value.into();
        build_write_command(generate_write_header(ADDR), bits.to_le_vec())
    }
}

impl<T: Copy + Into<u32>, const ADDR: u16, const SIZE: usize> ReadWrite<T, ADDR, SIZE> {
    pub fn write_command_u32(&self) -> Vec<u8> {
        let bits: u32 = self.value.into();
        build_write_command(generate_write_header(ADDR), bits.to_le_vec())
    }
}

impl<T: Copy + Into<u64>, const ADDR: u16, const SIZE: usize> ReadWrite<T, ADDR, SIZE> {
    pub fn write_command_u64(&self) -> Vec<u8> {
        let bits: u64 = self.value.into();
        build_write_command(generate_write_header(ADDR), bits.to_le_vec())
    }
}

pub const fn generate_read_header(addr: u16) -> [u8; 2] {
    let cmd = addr & 0x3FFF;
    [(cmd >> 8) as u8, (cmd & 0xFF) as u8]
}

pub const fn generate_write_header(addr: u16) -> [u8; 2] {
    let addr = addr & 0x3FFF;
    let cmd: u16 = (0b10 << 14) | addr;
    [(cmd >> 8) as u8, (cmd & 0xFF) as u8]
}

trait ToLeVec {
    fn to_le_vec(self) -> Vec<u8>;
}

impl ToLeVec for u8 {
    #[inline]
    fn to_le_vec(self) -> Vec<u8> {
        vec![self]
    }
}

impl ToLeVec for u16 {
    #[inline]
    fn to_le_vec(self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
}

impl ToLeVec for u32 {
    #[inline]
    fn to_le_vec(self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
}

impl ToLeVec for u64 {
    #[inline]
    fn to_le_vec(self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
}

#[inline]
fn build_write_command(header: [u8; 2], value_bytes: Vec<u8>) -> Vec<u8> {
    let mut cmd = Vec::with_capacity(2 + value_bytes.len());
    cmd.push(header[0]);
    cmd.push(header[1]);
    cmd.extend(value_bytes);
    cmd
}

// =============================================================================
// Common Chip Register Definitions
// =============================================================================

/// RF_PN - Device Part Number (Read Only)
///
/// Contains the part number identifying the device variant.
#[bitfield(u8)]
pub struct RfPn {
    /// Device Part Number
    /// - 0x34: AT86RF215
    /// - 0x35: AT86RF215IQ
    /// - 0x36: AT86RF215M
    #[bits(8, access = RO, from = DevicePartNumber::from_bits)]
    pub pn: DevicePartNumber,
}

/// RF_VN - Device Version Number (Read Only)
///
/// Contains the version number of the device.
/// - v1: 0x01
/// - v3: 0x03
#[bitfield(u8)]
pub struct RfVn {
    /// Version Number
    #[bits(8, access = RO)]
    pub vn: u8,
}

/// RF_RST - Chip Reset Command
/// Writing 0x7 to CMD triggers a complete chip reset.
#[bitfield(u8)]
pub struct RfRst {
    /// Chip Reset Command
    /// - 0x7: Trigger chip reset
    /// - Other values: No operation
    #[bits(3, from = ChipResetCmd::from_bits)]
    pub cmd: ChipResetCmd,

    #[bits(5)]
    __: u8,
}

/// RF_CFG - IRQ Configuration
///
/// Configures IRQ behavior and pad driver strength.
#[bitfield(u8)]
pub struct RfCfg {
    /// Output Driver Strength of Pads (IRQ, MISO, frontend control pins)
    /// - 0: 2mA
    /// - 1: 4mA
    /// - 2: 6mA
    /// - 3: 8mA
    #[bits(2)]
    pub drv: u8,

    /// IRQ Polarity
    /// - 0: Active high
    /// - 1: Active low
    #[bits(1)]
    pub irqp: bool,

    /// IRQ Mask Mode
    /// - 0: Masked IRQ reasons do not appear in IRQS register
    /// - 1: Masked IRQ reasons do appear in IRQS register
    #[bits(1)]
    pub irqmm: bool,

    #[bits(4)]
    __: u8,
}

/// RF_CLKO - Clock Output Configuration
///
/// Controls the CLKO pin output signal frequency and driver strength.
#[bitfield(u8)]
pub struct RfClko {
    /// Clock Output Selection
    /// - 0: OFF
    /// - 1: 26MHz
    /// - 2: 32MHz
    /// - 3: 16MHz
    /// - 4: 8MHz
    /// - 5: 4MHz
    /// - 6: 2MHz
    /// - 7: 1MHz
    #[bits(3)]
    pub os: u8,

    /// Output Driver Strength CLKO
    /// - 0: 2mA
    /// - 1: 4mA
    /// - 2: 6mA
    /// - 3: 8mA
    #[bits(2)]
    pub drv: u8,

    #[bits(3)]
    __: u8,
}

/// RF_BMDVC - Battery Monitor and Voltage Configuration
///
/// Controls battery monitor and voltage regulator settings.
#[bitfield(u8)]
pub struct RfBmdvc {
    /// Battery Monitor Threshold
    /// Configures the battery monitor voltage threshold
    #[bits(4)]
    pub bmth: u8,

    /// Battery Monitor Range
    /// - 0: Low range (1.7V - 3.0V)
    /// - 1: High range (2.0V - 3.6V)
    #[bits(1)]
    pub bmr: bool,

    /// Battery Monitor Enable
    /// - 0: Battery monitor disabled
    /// - 1: Battery monitor enabled
    #[bits(1)]
    pub bmen: bool,

    #[bits(2)]
    __: u8,
}

/// RF_XOC - Crystal Oscillator Configuration
///
/// Controls crystal oscillator settings and trim.
#[bitfield(u8)]
pub struct RfXoc {
    /// Crystal Oscillator Load Capacitance Trim
    /// Fine-tunes the crystal oscillator load capacitance
    #[bits(4)]
    pub trim: u8,

    /// Crystal Oscillator Frequency Select
    /// - 0: 26MHz
    /// - 1: Reserved
    #[bits(1)]
    pub fs: u8,

    #[bits(3)]
    __: u8,
}

/// RF_IQIFC0 - I/Q Data Interface Configuration 0
///
/// Configures I/Q data interface parameters.
#[bitfield(u8)]
pub struct RfIqifc0 {
    /// I/Q IF Enable Embedded TX Start Control
    /// - 0: Embedded control not enabled
    /// - 1: Embedded control enabled
    #[bits(1)]
    pub eec: bool,

    /// I/Q IF Common Mode Voltage Compliant to IEEE Std 1596
    /// - 0: CMV sub-register value is valid
    /// - 1: Common mode of 1.2V is set
    #[bits(1)]
    pub cmv1v2: bool,

    /// I/Q IF Common Mode Voltage
    /// - 0: 150mV
    /// - 1: 200mV
    /// - 2: 250mV
    /// - 3: 300mV
    #[bits(2)]
    pub cmv: u8,

    /// I/Q IF Driver Output Current
    /// - 0: 1mA
    /// - 1: 2mA
    /// - 2: 3mA
    /// - 3: 4mA
    #[bits(2)]
    pub drv: u8,

    /// I/Q IF Synchronization Failure (Read Only)
    /// - 0: No synchronization failure
    /// - 1: Synchronization failure
    #[bits(1, access = RO)]
    pub sf: bool,

    /// I/Q IF External Loopback
    /// - 0: External loopback disabled
    /// - 1: External loopback enabled
    #[bits(1)]
    pub extlb: bool,
}

/// RF_IQIFC1 - I/Q Data Interface Configuration 1
///
/// Configures chip mode and I/Q interface timing.
#[bitfield(u8)]
pub struct RfIqifc1 {
    /// Skew alignment I/Q IF driver
    /// - 0: 1.906ns
    /// - 1: 2.906ns
    /// - 2: 3.906ns (default)
    /// - 3: 4.906ns
    #[bits(2)]
    pub skewdrv: u8,

    #[bits(2)]
    __: u8,

    /// Chip Mode
    /// - 0: RF enabled, baseband (BBC0, BBC1) enabled, I/Q IF disabled
    /// - 1: RF enabled, baseband (BBC0, BBC1) disabled, I/Q IF enabled
    /// - 4: RF enabled, baseband (BBC0) disabled and (BBC1) enabled, I/Q IF for sub-GHz enabled
    /// - 5: RF enabled, baseband (BBC1) disabled and (BBC0) enabled, I/Q IF for 2.4GHz enabled
    #[bits(3, from = ChipMode::from_bits)]
    pub chpm: ChipMode,

    /// I/Q IF Receiver Failsafe Status (Read Only)
    /// - 0: Normal operation
    /// - 1: Receiver in failsafe mode (not driven by LVDS driver)
    #[bits(1, access = RO)]
    pub failsf: bool,
}

/// RF_IQIFC2 - I/Q Data Interface Configuration 2
///
/// Contains I/Q interface deserializer status.
#[bitfield(u8)]
pub struct RfIqifc2 {
    #[bits(7)]
    __: u8,

    /// I/Q IF Deserializer Synchronization Status (Read Only)
    /// - 0: Not synchronized
    /// - 1: Synchronized to incoming I/Q data stream
    #[bits(1, access = RO)]
    pub sync: bool,
}

// =============================================================================
// Radio Register Definitions (Common for RF09 and RF24)
// =============================================================================

/// RFn_IRQS - Radio IRQ Status (Read to Clear)
///
/// Interrupt status register. Reading this register clears all status bits.
#[bitfield(u8)]
pub struct RfnIrqs {
    /// Wake-up / Reset Interrupt
    /// Set to 1 when wake-up from SLEEP/DEEP_SLEEP or reset procedure is completed.
    #[bits(1, access = RO)]
    pub wakeup: bool,

    /// Transceiver Ready Interrupt
    /// Set to 1 when TXPREP state is reached or PLL settles after frequency change.
    #[bits(1, access = RO)]
    pub trxrdy: bool,

    /// Energy Detection Completion Interrupt
    /// Set to 1 when energy measurement is completed.
    #[bits(1, access = RO)]
    pub edc: bool,

    /// Battery Low Interrupt
    /// Set to 1 when battery monitor detects voltage below threshold.
    #[bits(1, access = RO)]
    pub batlow: bool,

    /// Transceiver Error Interrupt
    /// Set to 1 when PLL lock error occurs.
    #[bits(1, access = RO)]
    pub trxerr: bool,

    /// I/Q IF Synchronization Failure Interrupt
    /// Set to 1 when I/Q data interface synchronization fails.
    #[bits(1, access = RO)]
    pub iqifsf: bool,

    #[bits(2)]
    __: u8,
}

/// RFn_IRQM - Radio IRQ Mask
///
/// IRQ mask register. A bit set to 1 enables the corresponding IRQ.
#[bitfield(u8)]
pub struct RfnIrqm {
    /// Wake-up / Reset Interrupt Mask
    #[bits(1)]
    pub wakeup: bool,

    /// Transceiver Ready Interrupt Mask
    #[bits(1)]
    pub trxrdy: bool,

    /// Energy Detection Completion Interrupt Mask
    #[bits(1)]
    pub edc: bool,

    /// Battery Low Interrupt Mask
    #[bits(1)]
    pub batlow: bool,

    /// Transceiver Error Interrupt Mask
    #[bits(1)]
    pub trxerr: bool,

    /// I/Q IF Synchronization Failure Interrupt Mask
    #[bits(1)]
    pub iqifsf: bool,

    #[bits(2)]
    __: u8,
}

/// RFn_AUXS - Transceiver Auxiliary Settings
///
/// Controls auxiliary transceiver settings including PA voltage and external LNA.
#[bitfield(u8)]
pub struct RfnAuxs {
    /// Power Amplifier Voltage Control
    /// - 0: 2.0V
    /// - 1: 2.2V
    /// - 2: 2.4V (default)
    #[bits(2)]
    pub pavc: u8,

    /// Analog Voltage External
    /// - 0: Internal analog supply from regulator
    /// - 1: External analog supply
    #[bits(1)]
    pub ave: bool,

    /// Analog Voltage Enable in TRXOFF
    /// - 0: AVDD powered off in TRXOFF (default)
    /// - 1: AVDD remains on in TRXOFF
    #[bits(1)]
    pub aven: bool,

    /// AGC Input Freeze
    /// - 0: Use current AGC input
    /// - 1: Freeze AGC input
    #[bits(1)]
    pub agcmap: bool,

    /// External LNA Bypass Available
    /// - 0: External LNA cannot be bypassed
    /// - 1: External LNA can be bypassed
    #[bits(1)]
    pub extlnabyp: bool,

    #[bits(2)]
    __: u8,
}

/// RFn_STATE - Transceiver State (Read Only)
///
/// Indicates the current transceiver state.
#[bitfield(u8)]
pub struct RfnState {
    /// Transceiver State
    /// - 2: TRXOFF (Transceiver off, SPI active)
    /// - 3: TXPREP (Transmit preparation)
    /// - 4: TX (Transmit)
    /// - 5: RX (Receive)
    /// - 6: TRANSITION (State transition in progress)
    /// - 7: RESET (Transceiver in RESET or SLEEP)
    #[bits(3, access = RO, from = TransceiverState::from_bits)]
    pub state: TransceiverState,

    #[bits(5)]
    __: u8,
}

/// RFn_CMD - Transceiver Command
///
/// Writing to this register triggers a state transition.
#[bitfield(u8)]
pub struct RfnCmd {
    /// Transceiver Command
    /// - 0: NOP (No operation)
    /// - 1: SLEEP
    /// - 2: TRXOFF
    /// - 3: TXPREP
    /// - 4: TX
    /// - 5: RX
    /// - 7: RESET (Transceiver reset, ends in TRXOFF state)
    #[bits(3, access = WO, from = TransceiverCmd::from_bits)]
    pub cmd: TransceiverCmd,

    #[bits(5)]
    __: u8,
}

/// RFn_CS - Channel Spacing
///
/// Channel spacing with 25kHz resolution. Must write CNM last to apply.
#[bitfield(u8)]
pub struct RfnCs {
    /// Channel Spacing (units of 25kHz)
    /// Example: 0x08 = 8 × 25kHz = 200kHz spacing
    #[bits(8)]
    pub cs: u8,
}

/// RFn_CCF0 - Channel Center Frequency F0
///
/// 16-bit channel center frequency with 25kHz resolution.
#[bitfield(u16)]
pub struct RfnCcf0 {
    /// Channel Center Frequency F0 (16-bit)
    #[bits(16)]
    pub ccf0: u16,
}

/// RFn_CN - Channel Number and Mode
///
/// 16-bit channel number (9-bit) and mode configuration.
#[bitfield(u16)]
pub struct RfnCn {
    /// Channel Number (9-bit: 0-511)
    #[bits(9)]
    pub cn: u16,

    #[bits(5)]
    __: u16,

    /// Channel Setting Mode
    /// - 0: IEEE compliant (CCF0+CN*CS)*25kHz + offset
    /// - 1: Fine resolution 389.5-510MHz with ~99Hz stepping
    /// - 2: Fine resolution 779-1020MHz with ~198Hz stepping
    /// - 3: Fine resolution 2400-2483.5MHz with ~397Hz stepping
    #[bits(2)]
    pub cm: u8,
}

/// RFn_RXBWC - Receiver Filter Bandwidth Control
///
/// Configures receiver filter bandwidth and IF settings.
#[bitfield(u8)]
pub struct RfnRxbwc {
    /// Receiver Bandwidth
    /// - 0: 160kHz @ 250kHz IF
    /// - 1: 200kHz @ 250kHz IF
    /// - 2: 250kHz @ 250kHz IF
    /// - 3: 320kHz @ 500kHz IF
    /// - 4: 400kHz @ 500kHz IF
    /// - 5: 500kHz @ 500kHz IF
    /// - 6: 630kHz @ 1000kHz IF
    /// - 7: 800kHz @ 1000kHz IF
    /// - 8: 1000kHz @ 1000kHz IF
    /// - 9: 1250kHz @ 2000kHz IF
    /// - 10: 1600kHz @ 2000kHz IF
    /// - 11: 2000kHz @ 2000kHz IF
    #[bits(4)]
    pub bw: u8,

    /// IF Shift - Multiplies IF by 1.25
    /// - 0: Normal IF
    /// - 1: IF shifted by factor 1.25
    #[bits(1)]
    pub ifs: bool,

    /// IF Inversion
    /// - 0: Normal (default)
    /// - 1: Inverted sign IF frequency
    #[bits(1)]
    pub ifi: bool,

    #[bits(2)]
    __: u8,
}

/// RFn_RXDFE - Receiver Digital Frontend
///
/// Configures receiver sampling rate and filter.
#[bitfield(u8)]
pub struct RfnRxdfe {
    /// RX Sample Rate
    /// - 1: 4000kHz
    /// - 2: 2000kHz
    /// - 3: 4000/3 kHz
    /// - 4: 1000kHz
    /// - 5: 800kHz
    /// - 6: 2000/3 kHz
    /// - 8: 500kHz
    /// - 10: 400kHz
    #[bits(4)]
    pub sr: u8,

    #[bits(1)]
    __: u8,

    /// RX Filter Relative Cut-off Frequency
    /// - 0: 0.25 × fS/2
    /// - 1: 0.375 × fS/2
    /// - 2: 0.5 × fS/2
    /// - 3: 0.75 × fS/2
    /// - 4: 1.0 × fS/2 (bypass)
    #[bits(3)]
    pub rcut: u8,
}

/// RFn_AGCC - AGC Control
///
/// Configures automatic gain control behavior.
#[bitfield(u8)]
pub struct RfnAgcc {
    /// AGC Enable
    /// - 0: Manual gain control via AGCS.GCW
    /// - 1: Automatic gain control enabled
    #[bits(1)]
    pub en: bool,

    /// AGC Freeze Control
    /// - 0: AGC released
    /// - 1: AGC frozen to current value
    #[bits(1)]
    pub frzc: bool,

    /// AGC Freeze Status (Read Only)
    /// - 0: AGC not frozen
    /// - 1: AGC is frozen
    #[bits(1, access = RO)]
    pub frzs: bool,

    /// AGC Reset
    /// - 1: Reset AGC and set maximum receiver gain (auto-clears)
    #[bits(1)]
    pub rst: bool,

    /// AGC Average Time in Number of Samples
    /// - 0: 8 samples
    /// - 1: 16 samples
    /// - 2: 32 samples
    /// - 3: 64 samples
    #[bits(2)]
    pub avgs: u8,

    /// AGC Input
    /// - 0: Use filtered signal (after channel filter)
    /// - 1: Use unfiltered signal (faster settling)
    #[bits(1)]
    pub agci: bool,

    #[bits(1)]
    __: u8,
}

/// RFn_AGCS - AGC Settings
///
/// AGC target level and gain control word.
#[bitfield(u8)]
pub struct RfnAgcs {
    /// RX Gain Control Word
    /// If AGCC.EN=1: Read-only, indicates current receiver gain
    /// If AGCC.EN=0: Read-write, manually set receiver gain
    /// Value 23 = max gain, each step = 3dB
    #[bits(5)]
    pub gcw: u8,

    /// AGC Target Level (relative to ADC full scale)
    /// - 0: -21dB
    /// - 1: -24dB
    /// - 2: -27dB
    /// - 3: -30dB
    /// - 4: -33dB
    /// - 5: -36dB
    /// - 6: -39dB
    /// - 7: -42dB
    #[bits(3)]
    pub tgt: u8,
}

/// RFn_RSSI - Received Signal Strength Indicator (Read Only)
///
/// Current RSSI value in dBm. Valid range: -127 to +4 dBm.
/// Value 127 indicates invalid RSSI.
#[bitfield(u8)]
pub struct RfnRssi {
    /// RSSI Value (signed, two's complement)
    #[bits(8, access = RO)]
    pub rssi: i8,
}

/// RFn_EDC - Energy Detection Configuration
///
/// Configures energy detection measurement mode.
#[bitfield(u8)]
pub struct RfnEdc {
    /// Energy Detection Mode
    /// - 0: EDAUTO - Automatic trigger when AGC held
    /// - 1: EDSINGLE - Single measurement
    /// - 2: EDCONT - Continuous measurements
    /// - 3: EDOFF - Disabled
    #[bits(2, from = EnergyDetectionMode::from_bits)]
    pub edm: EnergyDetectionMode,

    #[bits(6)]
    __: u8,
}

/// RFn_EDD - Energy Detection Duration
///
/// Configures averaging duration for energy detection.
#[bitfield(u8)]
pub struct RfnEdd {
    /// Duration Time Basis
    /// - 0: 2µs
    /// - 1: 8µs
    /// - 2: 32µs
    /// - 3: 128µs
    #[bits(2)]
    pub dtb: u8,

    /// Duration Factor
    /// Averaging time = DF × DTB
    #[bits(6)]
    pub df: u8,
}

/// RFn_EDV - Energy Detection Value (Read Only)
///
/// Averaged energy detection result in dBm.
#[bitfield(u8)]
pub struct RfnEdv {
    /// Energy Detection Value (signed, two's complement)
    /// Valid range: -127 to +4 dBm
    #[bits(8, access = RO)]
    pub edv: i8,
}

/// RFn_PLL - PLL Control
///
/// PLL configuration and status.
#[bitfield(u8)]
pub struct RfnPll {
    #[bits(1)]
    __: u8,

    /// PLL Lock Status (Read Only)
    /// - 0: PLL not locked
    /// - 1: PLL locked
    #[bits(1, access = RO)]
    pub ls: bool,

    #[bits(2)]
    __: u8,

    /// Loop Bandwidth (Sub-1GHz only)
    /// - 0: Default
    /// - 1: 15% smaller
    /// - 2: 15% larger
    #[bits(2)]
    pub lbw: u8,

    #[bits(2)]
    __: u8,
}

/// RFn_PLLCF - PLL Center Frequency Value (Read Only)
///
/// Center frequency calibration value for current channel.
#[bitfield(u8)]
pub struct RfnPllcf {
    /// PLL Center Frequency Value
    #[bits(6, access = RO)]
    pub cf: u8,

    #[bits(2)]
    __: u8,
}

/// RFn_TXCUTC - Transmitter Filter Cutoff Control and PA Ramp Time
///
/// Configures TX filter and power amplifier ramp.
#[bitfield(u8)]
pub struct RfnTxcutc {
    /// Transmitter Cut-off Frequency
    /// - 0: 80kHz
    /// - 1: 100kHz
    /// - 2: 125kHz
    /// - 3: 160kHz
    /// - 4: 200kHz
    /// - 5: 250kHz
    /// - 6: 315kHz
    /// - 7: 400kHz
    /// - 8: 500kHz
    /// - 9: 625kHz
    /// - 10: 800kHz
    /// - 11: 1000kHz
    #[bits(4)]
    pub lpfcut: u8,

    #[bits(2)]
    __: u8,

    /// Power Amplifier Ramp Time
    /// - 0: 4µs
    /// - 1: 8µs
    /// - 2: 16µs
    /// - 3: 32µs
    #[bits(2)]
    pub paramp: u8,
}

/// RFn_TXDFE - Transmitter Digital Frontend
///
/// Configures transmitter sampling rate and filter.
#[bitfield(u8)]
pub struct RfnTxdfe {
    /// TX Sample Rate
    /// - 1: 4000kHz
    /// - 2: 2000kHz
    /// - 3: 4000/3 kHz
    /// - 4: 1000kHz
    /// - 5: 800kHz
    /// - 6: 2000/3 kHz
    /// - 8: 500kHz
    /// - 10: 400kHz
    #[bits(4)]
    pub sr: u8,

    /// Direct Modulation
    /// - 0: Direct modulation disabled
    /// - 1: Direct modulation enabled (for FSK/OQPSK)
    #[bits(1)]
    pub dm: bool,

    /// TX Filter Relative Cut-off Frequency
    /// - 0: 0.25 × fS/2
    /// - 1: 0.375 × fS/2
    /// - 2: 0.5 × fS/2
    /// - 3: 0.75 × fS/2
    /// - 4: 1.0 × fS/2 (bypass)
    #[bits(3)]
    pub rcut: u8,
}

/// RFn_PAC - Power Amplifier Control
///
/// Controls transmit power and PA bias current.
#[bitfield(u8)]
pub struct RfnPac {
    /// Transmitter Output Power
    /// - 0: Minimum output power
    /// - 31: Maximum output power
    /// Approximately 1dB steps
    #[bits(5)]
    pub txpwr: u8,

    /// Power Amplifier Current Control
    /// - 0: ~22mA reduction (3dB gain reduction)
    /// - 1: ~18mA reduction (2dB gain reduction)
    /// - 2: ~11mA reduction (1dB gain reduction)
    /// - 3: No reduction (max gain)
    #[bits(2)]
    pub pacur: u8,

    #[bits(1)]
    __: u8,
}

/// RFn_RNDV - Random Value (Read Only)
///
/// Random number value for random number generation.
#[bitfield(u8)]
pub struct RfnRndv {
    /// Random Value
    #[bits(8, access = RO)]
    pub rndv: u8,
}

/// RFn_PADFE - Power Amplifier Digital Frontend Enhancement
///
/// PA frontend enhancement configuration.
#[bitfield(u8)]
pub struct RfnPadfe {
    #[bits(7)]
    __: u8,

    /// PA Digital Frontend Enhancement
    #[bits(1)]
    pub padfe: bool,
}

/// RFn_TXCI - Transmitter I DC Offset Calibration
///
/// DC offset calibration for I channel.
#[bitfield(u8)]
pub struct RfnTxci {
    /// DC Offset I
    #[bits(6)]
    pub dcoi: u8,

    #[bits(2)]
    __: u8,
}

/// RFn_TXCQ - Transmitter Q DC Offset Calibration
///
/// DC offset calibration for Q channel.
#[bitfield(u8)]
pub struct RfnTxcq {
    /// DC Offset Q
    #[bits(6)]
    pub dcoq: u8,

    #[bits(2)]
    __: u8,
}

/// RFn_TXDACI - Transmitter I DAC Value
///
/// DAC value for I channel with enable bit.
#[bitfield(u8)]
pub struct RfnTxdaci {
    /// TX DAC I Value
    #[bits(7)]
    pub txdacid: u8,

    /// Enable TX DAC I Direct Mode
    #[bits(1)]
    pub entxdacid: bool,
}

/// RFn_TXDACQ - Transmitter Q DAC Value
///
/// DAC value for Q channel with enable bit.
#[bitfield(u8)]
pub struct RfnTxdacq {
    /// TX DAC Q Value
    #[bits(7)]
    pub txdacqd: u8,

    /// Enable TX DAC Q Direct Mode
    #[bits(1)]
    pub entxdacqd: bool,
}

// =============================================================================
// Baseband Register Definitions
// =============================================================================

/// BBCn_PC - PHY Control
///
/// Main baseband control register for PHY configuration.
#[bitfield(u8)]
pub struct BbcnPc {
    /// PHY Type
    /// - 0: FSK
    /// - 1: OFDM
    /// - 2: OQPSK
    /// - 3: Reserved/Legacy O-QPSK
    #[bits(2)]
    pub pt: u8,

    /// Baseband Enable
    /// - 0: Baseband disabled
    /// - 1: Baseband enabled
    #[bits(1)]
    pub bben: bool,

    /// Frame Checksum Type
    /// - 0: 16-bit CRC (FCS)
    /// - 1: 32-bit CRC (FCS)
    #[bits(1)]
    pub fcst: bool,

    /// Automatic FCS Transmission
    /// - 0: FCS not appended automatically
    /// - 1: FCS appended to TX frame
    #[bits(1)]
    pub txafcs: bool,

    /// FCS OK
    /// - 0: Last received frame FCS invalid (Read Only)
    /// - 1: Last received frame FCS valid (Read Only)
    #[bits(1, access = RO)]
    pub fcsok: bool,

    /// FCS Filter Enable
    /// - 0: Accept frames regardless of FCS
    /// - 1: Filter out frames with invalid FCS
    #[bits(1)]
    pub fcsfe: bool,

    /// Continuous TX Mode
    /// - 0: Normal TX mode
    /// - 1: Continuous TX (for testing)
    #[bits(1)]
    pub ctx: bool,
}

/// BBCn_PS - PHY Status
///
/// PHY status flags.
#[bitfield(u8)]
pub struct BbcnPs {
    /// TX Underrun
    /// - 0: No underrun
    /// - 1: TX underrun occurred
    #[bits(1, access = RO)]
    pub txur: bool,

    #[bits(7)]
    __: u8,
}

/// BBCn_RXFL - RX Frame Length (Read Only)
///
/// 11-bit received frame length (0-2047 bytes).
#[bitfield(u16)]
pub struct BbcnRxfl {
    /// RX Frame Length (11-bit)
    #[bits(11, access = RO)]
    pub rxfl: u16,

    #[bits(5)]
    __: u16,
}

/// BBCn_TXFL - TX Frame Length
///
/// 11-bit transmit frame length (0-2047 bytes).
#[bitfield(u16)]
pub struct BbcnTxfl {
    /// TX Frame Length (11-bit)
    #[bits(11)]
    pub txfl: u16,

    #[bits(5)]
    __: u16,
}

/// BBCn_FBL - Frame Buffer Level (Read Only)
///
/// 11-bit current frame buffer level.
#[bitfield(u16)]
pub struct BbcnFbl {
    /// Frame Buffer Level (11-bit)
    #[bits(11, access = RO)]
    pub fbl: u16,

    #[bits(5)]
    __: u16,
}

/// BBCn_FBLI - Frame Buffer Level Interrupt Threshold
///
/// 11-bit threshold for frame buffer level interrupt.
#[bitfield(u16)]
pub struct BbcnFbli {
    /// Frame Buffer Level Interrupt Threshold (11-bit)
    #[bits(11)]
    pub fbli: u16,

    #[bits(5)]
    __: u16,
}

/// BBCn_OFDMPHRTX - OFDM PHR Transmit Configuration
///
/// OFDM PHY header transmit configuration.
#[bitfield(u8)]
pub struct BbcnOfdmphrtx {
    /// Modulation and Coding Scheme
    #[bits(3)]
    pub mcs: u8,

    #[bits(1)]
    __: u8,

    /// Reserved Bit 5
    #[bits(1)]
    pub rb5: bool,

    /// Reserved Bit 17
    #[bits(1)]
    pub rb17: bool,

    /// Reserved Bit 18
    #[bits(1)]
    pub rb18: bool,

    /// Reserved Bit 21
    #[bits(1)]
    pub rb21: bool,
}

/// BBCn_OFDMPHRRX - OFDM PHR Receive Configuration (Read Only)
///
/// OFDM PHY header received configuration.
#[bitfield(u8)]
pub struct BbcnOfdmphrrx {
    /// Modulation and Coding Scheme (Read Only)
    #[bits(3, access = RO)]
    pub mcs: u8,

    /// Scrambler/Interleaver Puncturing Configuration (Read Only)
    #[bits(1, access = RO)]
    pub spc: bool,

    /// Reserved Bit 5 (Read Only)
    #[bits(1, access = RO)]
    pub rb5: bool,

    /// Reserved Bit 17 (Read Only)
    #[bits(1, access = RO)]
    pub rb17: bool,

    /// Reserved Bit 18 (Read Only)
    #[bits(1, access = RO)]
    pub rb18: bool,

    /// Reserved Bit 21 (Read Only)
    #[bits(1, access = RO)]
    pub rb21: bool,
}

/// BBCn_OFDMC - OFDM Configuration
///
/// Main OFDM configuration register.
#[bitfield(u8)]
pub struct BbcnOfdmc {
    /// OFDM Option
    /// - 0: Option 1 (BPSK, QPSK, 16-QAM, 64-QAM)
    /// - 1: Option 2 (BPSK, QPSK, 16-QAM)
    /// - 2: Option 3 (QPSK, 16-QAM)
    /// - 3: Option 4 (BPSK)
    #[bits(2)]
    pub opt: u8,

    /// Pilot Interleaving
    /// - 0: Disabled
    /// - 1: Enabled
    #[bits(1)]
    pub poi: bool,

    /// Legacy OFDM mode
    #[bits(1)]
    pub lfo: bool,

    /// Interleaver Scrambler Seed TX
    #[bits(2)]
    pub sstx: u8,

    /// Interleaver Scrambler Seed RX
    #[bits(2)]
    pub ssrx: u8,
}

/// BBCn_OFDMSW - OFDM Switch
///
/// OFDM receiver configuration.
#[bitfield(u8)]
pub struct BbcnOfdmsw {
    #[bits(4)]
    __: u8,

    /// RX Override
    #[bits(2)]
    pub rxo: u8,

    /// Preamble Detection Threshold
    #[bits(2)]
    pub pdt: u8,
}

/// BBCn_OQPSKC0 - O-QPSK Configuration 0
///
/// O-QPSK main configuration.
#[bitfield(u8)]
#[allow(non_upper_case_globals, non_snake_case)]
pub struct BbcnOqpskc0 {
    /// Chip Frequency
    /// - 0: 100kchip/s
    /// - 1: 200kchip/s
    /// - 2: 1000kchip/s
    /// - 3: 2000kchip/s
    #[bits(2)]
    pub fchip: u8,

    #[bits(1)]
    __: u8,

    /// Modulation (MOD field) - impulse response of shaping filter
    /// - 0: RC-0.8 (raised cosine, roll-off = 0.8)
    /// - 1: RRC-0.8 (root raised cosine, roll-off = 0.8)
    #[bits(1)]
    #[allow(non_snake_case)]
    pub mod_: bool,

    /// Direct Modulation
    #[bits(1)]
    pub dm: bool,

    #[bits(3)]
    __: u8,
}

/// BBCn_OQPSKC1 - O-QPSK Configuration 1
///
/// O-QPSK preamble and detection configuration.
#[bitfield(u8)]
pub struct BbcnOqpskc1 {
    /// Preamble Detection Threshold 0
    #[bits(4)]
    pub pdt0: u8,

    /// Preamble Detection Threshold 1
    #[bits(2)]
    pub pdt1: u8,

    /// RX Override Legacy
    #[bits(1)]
    pub rxoleg: bool,

    /// RX Override
    #[bits(1)]
    pub rxo: bool,
}

/// BBCn_OQPSKC2 - O-QPSK Configuration 2
///
/// O-QPSK receiver configuration.
#[bitfield(u8)]
pub struct BbcnOqpskc2 {
    /// RX Mode
    #[bits(2)]
    pub rxm: u8,

    /// FCS Type Legacy
    #[bits(1)]
    pub fcstleg: bool,

    /// Enable Proprietary Mode
    #[bits(1)]
    pub enprop: bool,

    /// Rate/Power Control
    #[bits(1)]
    pub rpc: bool,

    /// Scrambler/Puncturing Configuration
    #[bits(1)]
    pub spc: bool,

    #[bits(2)]
    __: u8,
}

/// BBCn_OQPSKC3 - O-QPSK Configuration 3
///
/// O-QPSK SFD configuration.
#[bitfield(u8)]
pub struct BbcnOqpskc3 {
    #[bits(2)]
    __: u8,

    /// Non-standard SFD
    #[bits(2)]
    pub nsfd: u8,

    #[bits(1)]
    __: u8,

    /// High Rate Legacy
    #[bits(1)]
    pub hrleg: bool,

    #[bits(2)]
    __: u8,
}

/// BBCn_OQPSKPHRTX - O-QPSK PHR Transmit
///
/// O-QPSK PHY header transmit configuration.
#[bitfield(u8)]
pub struct BbcnOqpskphrtx {
    /// Legacy Mode
    #[bits(1)]
    pub leg: bool,

    /// Rate Mode (MOD field)
    #[bits(3)]
    #[allow(non_snake_case)]
    pub mod_: u8,

    /// Reserved Bit 0
    #[bits(1)]
    pub rb0: bool,

    /// PPDU Type
    #[bits(1)]
    pub ppdut: bool,

    #[bits(2)]
    __: u8,
}

/// BBCn_OQPSKPHRRX - O-QPSK PHR Receive (Read Only)
///
/// O-QPSK PHY header received.
#[bitfield(u8)]
pub struct BbcnOqpskphrrx {
    /// Legacy Mode (Read Only)
    #[bits(1, access = RO)]
    pub leg: bool,

    /// Rate Mode (Read Only, MOD field)
    #[bits(3, access = RO)]
    #[allow(non_snake_case)]
    pub mod_: u8,

    /// Reserved Bit 0 (Read Only)
    #[bits(1, access = RO)]
    pub rb0: bool,

    /// PPDU Type (Read Only)
    #[bits(1, access = RO)]
    pub ppdut: bool,

    #[bits(2)]
    __: u8,
}

// =============================================================================
// MAC Address Filtering Registers
// =============================================================================

/// BBCn_AFC0 - Automatic Frame Filtering Configuration 0
///
/// Frame filtering enable bits.
#[bitfield(u8)]
pub struct BbcnAfc0 {
    /// Address Filter Enable 0
    #[bits(1)]
    pub afen0: bool,

    /// Address Filter Enable 1
    #[bits(1)]
    pub afen1: bool,

    /// Address Filter Enable 2
    #[bits(1)]
    pub afen2: bool,

    /// Address Filter Enable 3
    #[bits(1)]
    pub afen3: bool,

    /// Promiscuous Mode
    /// - 0: Normal filtering
    /// - 1: Accept all frames
    #[bits(1)]
    pub pm: bool,

    #[bits(3)]
    __: u8,
}

/// BBCn_AFC1 - Automatic Frame Filtering Configuration 1
///
/// PAN coordinator and MAC reserved frame type filtering.
#[bitfield(u8)]
pub struct BbcnAfc1 {
    /// PAN Coordinator
    #[bits(4)]
    pub panc: u8,

    /// MAC Reserved Frame Type
    #[bits(4)]
    pub mrft: u8,
}

/// BBCn_AFFTM - Automatic Frame Filtering Type Mask
///
/// Frame type filter mask.
#[bitfield(u8)]
pub struct BbcnAfftm {
    #[bits(8)]
    pub afftm: u8,
}

/// BBCn_AFFVM - Automatic Frame Filtering Version Mask
///
/// Frame version filter mask.
#[bitfield(u8)]
pub struct BbcnAffvm {
    /// Address Filter Frame Version Mask
    #[bits(4)]
    pub affvm: u8,

    #[bits(4)]
    __: u8,
}

/// BBCn_AFS - Automatic Frame Filtering Status (Read Only)
///
/// Address match status.
#[bitfield(u8)]
pub struct BbcnAfs {
    /// Address Match 0
    #[bits(1, access = RO)]
    pub am0: bool,

    /// Address Match 1
    #[bits(1, access = RO)]
    pub am1: bool,

    /// Address Match 2
    #[bits(1, access = RO)]
    pub am2: bool,

    /// Address Match 3
    #[bits(1, access = RO)]
    pub am3: bool,

    /// Extended Address Match
    #[bits(1, access = RO)]
    pub em: bool,

    #[bits(3)]
    __: u8,
}

/// BBCn_MACEA - MAC Extended Address
///
/// 64-bit extended MAC address.
#[bitfield(u64)]
pub struct BbcnMacea {
    /// MAC Extended Address (64-bit)
    #[bits(64)]
    pub macea: u64,
}

/// BBCn_MACPID - MAC PAN ID
///
/// 16-bit PAN ID for address filtering.
#[bitfield(u16)]
pub struct BbcnMacpid {
    /// PAN ID (16-bit)
    #[bits(16)]
    pub macpid: u16,
}

/// BBCn_MACSHA - MAC Short Address
///
/// 16-bit short address for filtering.
#[bitfield(u16)]
pub struct BbcnMacsha {
    /// Short Address (16-bit)
    #[bits(16)]
    pub macsha: u16,
}

/// BBCn_AMCS - Automatic MAC ACK Control & Settings
///
/// Automatic acknowledgment configuration.
#[bitfield(u8)]
pub struct BbcnAmcs {
    /// TX to RX Switch
    #[bits(1)]
    pub tx2rx: bool,

    /// CCA before TX
    #[bits(1)]
    pub ccatx: bool,

    /// CCA Energy Detection
    #[bits(1)]
    pub ccaed: bool,

    /// Auto ACK Enable
    #[bits(1)]
    pub aack: bool,

    /// Auto ACK Status
    #[bits(1)]
    pub aacks: bool,

    /// Auto ACK Data Rate
    #[bits(1)]
    pub aackdr: bool,

    /// Auto ACK Frame Pending Address
    #[bits(1)]
    pub aackfa: bool,

    /// Auto ACK Frame Pending Type
    #[bits(1)]
    pub aackft: bool,
}

/// BBCn_AMEDT - Automatic MAC ACK ED Threshold
///
/// Energy detection threshold for auto-ACK.
#[bitfield(u8)]
pub struct BbcnAmedt {
    #[bits(8)]
    pub amedt: u8,
}

/// BBCn_AMAACKPD - Automatic MAC ACK Pending Bit
///
/// Frame pending bit configuration for auto-ACK.
#[bitfield(u8)]
pub struct BbcnAmaackpd {
    /// Pending Data 0
    #[bits(1)]
    pub pd0: bool,

    /// Pending Data 1
    #[bits(1)]
    pub pd1: bool,

    /// Pending Data 2
    #[bits(1)]
    pub pd2: bool,

    /// Pending Data 3
    #[bits(1)]
    pub pd3: bool,

    #[bits(4)]
    __: u8,
}

/// BBCn_AMAACKT - Automatic MAC ACK Turnaround Time
///
/// ACK turnaround time in microseconds (11-bit value).
#[bitfield(u16)]
pub struct BbcnAmaackt {
    /// ACK Turnaround Time in µs (11-bit)
    #[bits(11)]
    pub amaackt: u16,

    #[bits(5)]
    __: u16,
}

// =============================================================================
// FSK PHY Registers
// =============================================================================

/// BBCn_FSKC0 - FSK Configuration 0
///
/// FSK modulation order and index configuration.
#[bitfield(u8)]
pub struct BbcnFskc0 {
    /// Modulation Order
    /// - 0: 2FSK
    /// - 1: 4FSK
    #[bits(1)]
    pub mord: bool,

    /// Modulation Index
    #[bits(3)]
    pub midx: u8,

    /// Modulation Index Scale
    #[bits(2)]
    pub midxs: u8,

    /// BT Product (Gaussian Filter)
    #[bits(2)]
    pub bt: u8,
}

/// BBCn_FSKC1 - FSK Configuration 1
///
/// FSK symbol rate and preamble configuration.
#[bitfield(u8)]
pub struct BbcnFskc1 {
    /// Symbol Rate
    #[bits(4)]
    pub srate: u8,

    #[bits(1)]
    __: u8,

    /// Channel Filter Index
    #[bits(1)]
    pub fi: bool,

    /// Preamble Length
    #[bits(2)]
    pub fskplh: u8,
}

/// BBCn_FSKC2 - FSK Configuration 2
///
/// FSK receiver configuration.
#[bitfield(u8)]
pub struct BbcnFskc2 {
    /// FEC Inner Code Enable
    #[bits(1)]
    pub fecie: bool,

    /// FEC Scheme
    #[bits(1)]
    pub fecs: bool,

    /// Priority Mode for RX Interframe Spacing
    #[bits(1)]
    pub pri: bool,

    /// Mode Switch Enable
    #[bits(1)]
    pub mse: bool,

    /// RX PHR Timeout
    #[bits(1)]
    pub rxpto: bool,

    /// RX Override
    #[bits(2)]
    pub rxo: u8,

    /// Preamble Detection Threshold Mode
    #[bits(1)]
    pub pdtm: bool,
}

/// BBCn_FSKC3 - FSK Configuration 3
///
/// FSK preamble and SFD configuration.
#[bitfield(u8)]
pub struct BbcnFskc3 {
    /// Preamble Detection Threshold
    #[bits(5)]
    pub pdt: u8,

    /// SFD Detection Threshold
    #[bits(3)]
    pub sfdt: u8,
}

/// BBCn_FSKC4 - FSK Configuration 4
///
/// FSK SFD and raw bit configuration.
#[bitfield(u8)]
pub struct BbcnFskc4 {
    /// Custom SFD 0
    #[bits(2)]
    pub csfd0: u8,

    /// Custom SFD 1
    #[bits(2)]
    pub csfd1: u8,

    /// Raw RX Bit Mode
    #[bits(1)]
    pub rawrbit: bool,

    /// SFD 32-bit Mode
    #[bits(1)]
    pub sfd32: bool,

    /// SFD Qualifier
    #[bits(1)]
    pub sfdq: bool,

    #[bits(1)]
    __: u8,
}

/// BBCn_FSKPLL - FSK PLL Configuration
///
/// FSK PLL bandwidth configuration.
#[bitfield(u8)]
pub struct BbcnFskpll {
    #[bits(8)]
    pub fskpll: u8,
}

/// BBCn_FSKSFD - FSK SFD Pattern
///
/// 16-bit custom SFD pattern for FSK.
#[bitfield(u16)]
pub struct BbcnFsksfd {
    /// SFD Pattern (16-bit)
    #[bits(16)]
    pub fsksfd: u16,
}

/// BBCn_FSKPHRTX - FSK PHR Transmit
///
/// FSK PHY header transmit configuration.
#[bitfield(u8)]
pub struct BbcnFskphrtx {
    /// Reserved Bit 1
    #[bits(1)]
    pub rb1: bool,

    /// Reserved Bit 2
    #[bits(1)]
    pub rb2: bool,

    /// Data Whitening
    #[bits(1)]
    pub dw: bool,

    /// SFD Identifier
    #[bits(1)]
    pub sfd: bool,

    #[bits(4)]
    __: u8,
}

/// BBCn_FSKPHRRX - FSK PHR Receive (Read Only)
///
/// FSK PHY header received.
#[bitfield(u8)]
pub struct BbcnFskphrrx {
    /// Reserved Bit 1 (Read Only)
    #[bits(1, access = RO)]
    pub rb1: bool,

    /// Reserved Bit 2 (Read Only)
    #[bits(1, access = RO)]
    pub rb2: bool,

    /// Data Whitening (Read Only)
    #[bits(1, access = RO)]
    pub dw: bool,

    /// SFD Identifier (Read Only)
    #[bits(1, access = RO)]
    pub sfd: bool,

    #[bits(2)]
    __: u8,

    /// Mode Switch (Read Only)
    #[bits(1, access = RO)]
    pub ms: bool,

    /// FCS Type (Read Only)
    #[bits(1, access = RO)]
    pub fcst: bool,
}

/// BBCn_FSKRPC - FSK Raw Pattern Configuration
///
/// Raw pattern transmit mode configuration.
#[bitfield(u8)]
pub struct BbcnFskrpc {
    /// Base Time
    #[bits(4)]
    pub baset: u8,

    /// Enable
    #[bits(1)]
    pub en: bool,

    #[bits(3)]
    __: u8,
}

/// BBCn_FSKRPCONT - FSK Raw Pattern Continuous
///
/// Continuous raw pattern value.
#[bitfield(u8)]
pub struct BbcnFskrpcont {
    #[bits(8)]
    pub fskrpcont: u8,
}

/// BBCn_FSKRPCOFFT - FSK Raw Pattern Offset
///
/// Raw pattern offset.
#[bitfield(u8)]
pub struct BbcnFskrpcofft {
    #[bits(8)]
    pub fskrpcofft: u8,
}

/// BBCn_FSKRRXFL - FSK Raw RX Frame Length (Read Only)
///
/// 11-bit raw mode RX frame length.
#[bitfield(u16)]
pub struct BbcnFskrrxfl {
    /// FSK Raw RX Frame Length (11-bit)
    #[bits(11, access = RO)]
    pub fskrrxfl: u16,

    #[bits(5)]
    __: u16,
}

/// BBCn_FSKDM - FSK Direct Modulation
///
/// Direct modulation configuration.
#[bitfield(u8)]
pub struct BbcnFskdm {
    /// Enable Direct Modulation
    #[bits(1)]
    pub en: bool,

    /// Preamble Extension
    #[bits(1)]
    pub pe: bool,

    #[bits(6)]
    __: u8,
}

/// BBCn_FSKPEn - FSK Preamble Extension n
///
/// Preamble extension pattern bytes.
#[bitfield(u8)]
pub struct BbcnFskpe {
    #[bits(8)]
    pub fskpe: u8,
}

// =============================================================================
// Counter Registers
// =============================================================================

/// BBCn_CNTC - Counter Control
///
/// Counter enable and control.
#[bitfield(u8)]
pub struct BbcnCntc {
    /// Enable Counter
    #[bits(1)]
    pub en: bool,

    /// Reset RX Symbol Counter
    #[bits(1)]
    pub rstrxs: bool,

    /// Reset TX Symbol Counter
    #[bits(1)]
    pub rsttxs: bool,

    /// Capture RX Symbol Counter
    #[bits(1)]
    pub caprxs: bool,

    /// Capture TX Symbol Counter
    #[bits(1)]
    pub captxs: bool,

    #[bits(3)]
    __: u8,
}

/// BBCn_CNT - Counter Value (Read Only)
///
/// 32-bit counter value.
#[bitfield(u32)]
pub struct BbcnCnt {
    /// Counter Value (32-bit)
    #[bits(32, access = RO)]
    pub cnt: u32,
}

// =============================================================================
// Baseband Register Definitions
// =============================================================================

/// BBCn_IRQS - Baseband IRQ Status (Read to Clear)
///
/// Baseband interrupt status. Reading clears all bits.
#[bitfield(u8)]
pub struct BbcnIrqs {
    /// Receiver Frame Start Interrupt
    /// Set when valid frame start (PHR) detected
    #[bits(1, access = RO)]
    pub rxfs: bool,

    /// Receiver Frame End Interrupt
    /// Set when frame end detected
    #[bits(1, access = RO)]
    pub rxfe: bool,

    /// Receiver Address Match Interrupt
    /// Set when received frame matches 3rd level filter
    #[bits(1, access = RO)]
    pub rxam: bool,

    /// Receiver Extended Match Interrupt
    /// Set when received frame matches extended filter
    #[bits(1, access = RO)]
    pub rxem: bool,

    /// Transmitter Frame End Interrupt
    /// Set when frame transmission completed
    #[bits(1, access = RO)]
    pub txfe: bool,

    /// AGC Hold Interrupt
    /// Set when baseband holds AGC
    #[bits(1, access = RO)]
    pub agch: bool,

    /// AGC Release Interrupt
    /// Set when baseband releases AGC
    #[bits(1, access = RO)]
    pub agcr: bool,

    /// Frame Buffer Level Indication Interrupt
    /// Set when RX bytes exceed frame buffer level
    #[bits(1, access = RO)]
    pub fbli: bool,
}

/// BBCn_IRQM - Baseband IRQ Mask
///
/// Baseband interrupt mask. Bit set to 1 enables corresponding IRQ.
#[bitfield(u8)]
pub struct BbcnIrqm {
    /// Receiver Frame Start Interrupt Mask
    #[bits(1)]
    pub rxfs: bool,

    /// Receiver Frame End Interrupt Mask
    #[bits(1)]
    pub rxfe: bool,

    /// Receiver Address Match Interrupt Mask
    #[bits(1)]
    pub rxam: bool,

    /// Receiver Extended Match Interrupt Mask
    #[bits(1)]
    pub rxem: bool,

    /// Transmitter Frame End Interrupt Mask
    #[bits(1)]
    pub txfe: bool,

    /// AGC Hold Interrupt Mask
    #[bits(1)]
    pub agch: bool,

    /// AGC Release Interrupt Mask
    #[bits(1)]
    pub agcr: bool,

    /// Frame Buffer Level Indication Interrupt Mask
    #[bits(1)]
    pub fbli: bool,
}

// =============================================================================
// Phase Measurement Unit (PMU) Register Definitions
// =============================================================================

/// BBCn_PMUC - PMU Control
///
/// Main control register for Phase Measurement Unit (PMU) functionality.
/// The PMU allows register-based monitoring of phase and signal parameters
/// from the RX Digital Frontend with a constant 8µs period.
#[bitfield(u8)]
pub struct BbcnPmuc {
    /// PMU Enable
    /// - 0: PMU disabled
    /// - 1: PMU enabled - periodically measures phase and signal parameters
    #[bits(1)]
    pub en: bool,

    /// I/Q Averaging Enable
    /// - 0: I/Q sampling at end of PMU period
    /// - 1: I/Q averaging over PMU period (8µs)
    #[bits(1)]
    pub avg: bool,

    /// PMU Synchronisation (Read Only)
    /// 1MHz counter running from 0 to 7 during 8µs PMU period.
    /// All output registers update at transition from 7 to 0.
    #[bits(3, access = RO)]
    pub sync: u8,

    /// Frequency Error Detection
    /// - 0: PMUQF acts as unsigned quality factor [0-255]
    /// - 1: PMUQF returns frequency error estimate (requires 1MHz sample rate)
    ///      FO[Hz] = 500kHz * PMUQF/256 (signed two's complement)
    #[bits(1)]
    pub fed: bool,

    /// IQ Output Selector
    /// - 0: Normalized I/Q: (PMUI + i*PMUQ) ≈ 63 * exp(i * π * PMUVAL/128)
    /// - 1: I/Q without normalization: arg(PMUI + i*PMUQ) = π * PMUVAL/128
    #[bits(1)]
    pub iqsel: bool,

    /// Channel Center Frequency Time Synchronization
    /// - 0: Frequency changes applied directly per CNM register
    /// - 1: Frequency changes time-aligned to 8µs PMU period
    #[bits(1)]
    pub ccfts: bool,
}

/// BBCn_PMUVAL - PMU Phase Value (Read Only)
///
/// 8-bit phase value covering angles 0 to 2π in 256 steps.
/// - 0 = 0°
/// - 255 = (360/256) × 255°
#[bitfield(u8)]
pub struct BbcnPmuval {
    /// PMU Phase Value
    /// Represents phase in range [0, 2π) with 256 discrete steps
    #[bits(8, access = RO)]
    pub pmuval: u8,
}

/// BBCn_PMUQF - PMU Quality Factor (Read Only)
///
/// Dual-purpose register depending on PMUC.FED setting:
/// - FED=0: Quality factor (0-255, higher is better quality)
/// - FED=1: Frequency offset (signed two's complement)
#[bitfield(u8)]
pub struct BbcnPmuqf {
    /// PMU Quality Factor / Frequency Offset
    ///
    /// When PMUC.FED=0: Unsigned quality factor
    /// - 255 = best quality
    /// - Related to average phase drift during PMU interval
    ///
    /// When PMUC.FED=1: Signed frequency offset (two's complement)
    /// - FO[Hz] = 500kHz * PMUQF/256
    /// - Valid range: -128 to 127
    /// - Requires RXDFE_SR=4 (1MHz sample rate)
    #[bits(8, access = RO)]
    pub pmuqf: i8,
}

/// BBCn_PMUI - PMU I/Q Value, Real Part (Read Only)
///
/// Real (I) component of PMU measurement value.
#[bitfield(u8)]
pub struct BbcnPmui {
    /// PMU I/Q Value, Real Part (signed two's complement)
    ///
    /// Interpretation depends on PMUC.IQSEL:
    /// - IQSEL=0: Normalized, range ≈ [-63, 63]
    /// - IQSEL=1: Non-normalized I component
    #[bits(8, access = RO)]
    pub pmui: i8,
}

/// BBCn_PMUQ - PMU I/Q Value, Imaginary Part (Read Only)
///
/// Imaginary (Q) component of PMU measurement value.
#[bitfield(u8)]
pub struct BbcnPmuq {
    /// PMU I/Q Value, Imaginary Part (signed two's complement)
    ///
    /// Interpretation depends on PMUC.IQSEL:
    /// - IQSEL=0: Normalized, range ≈ [-63, 63]
    /// - IQSEL=1: Non-normalized Q component
    #[bits(8, access = RO)]
    pub pmuq: i8,
}

// =============================================================================
// Enumerations for Register Field Values
// =============================================================================

/// Device Part Number Values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DevicePartNumber {
    AT86RF215 = 0x34,
    AT86RF215IQ = 0x35,
    AT86RF215M = 0x36,
}

impl DevicePartNumber {
    pub const fn into_bits(self) -> u8 {
        self as _
    }

    pub const fn from_bits(value: u8) -> Self {
        match value {
            0x34 => Self::AT86RF215,
            0x35 => Self::AT86RF215IQ,
            0x36 => Self::AT86RF215M,
            _ => Self::AT86RF215, // Default fallback
        }
    }
}

/// Chip Reset Commands
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ChipResetCmd {
    Reset = 0x07,
}

impl ChipResetCmd {
    pub const fn into_bits(self) -> u8 {
        self as _
    }

    pub const fn from_bits(value: u8) -> Self {
        match value {
            0x07 => Self::Reset,
            _ => Self::Reset,
            // TODO check if Chip reset needs other enum fields
            // I think it needs another entry for other to not match to Self::Reset
        }
    }
}

/// Transceiver States
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TransceiverState {
    TrxOff = 2,
    TxPrep = 3,
    Tx = 4,
    Rx = 5,
    Transition = 6,
    Reset = 7,
}

impl TransceiverState {
    pub const fn into_bits(self) -> u8 {
        self as _
    }

    pub const fn from_bits(value: u8) -> Self {
        match value {
            2 => Self::TrxOff,
            3 => Self::TxPrep,
            4 => Self::Tx,
            5 => Self::Rx,
            6 => Self::Transition,
            7 => Self::Reset,
            _ => Self::TrxOff, // Default fallback
        }
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TransceiverCmd {
    Nop = 0,
    Sleep = 1,
    TrxOff = 2,
    TxPrep = 3,
    Tx = 4,
    Rx = 5,
    Reset = 7,
}

impl TransceiverCmd {
    pub const fn into_bits(self) -> u8 {
        self as _
    }

    pub const fn from_bits(value: u8) -> Self {
        match value {
            0 => Self::Nop,
            1 => Self::Sleep,
            2 => Self::TrxOff,
            3 => Self::TxPrep,
            4 => Self::Tx,
            5 => Self::Rx,
            7 => Self::Reset,
            _ => Self::Nop, // Default fallback
        }
    }
}

/// Chip Operating Modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ChipMode {
    /// RF enabled, both basebands enabled, I/Q IF disabled
    BasebandMode = 0,
    /// RF enabled, both basebands disabled, I/Q IF enabled
    IqRadioMode = 1,
    /// RF09 I/Q mode, RF24 baseband mode
    Rf09IqRf24Bb = 4,
    /// RF09 baseband mode, RF24 I/Q mode
    Rf09BbRf24Iq = 5,
}

impl ChipMode {
    pub const fn into_bits(self) -> u8 {
        self as _
    }

    pub const fn from_bits(value: u8) -> Self {
        match value {
            0 => Self::BasebandMode,
            1 => Self::IqRadioMode,
            4 => Self::Rf09IqRf24Bb,
            5 => Self::Rf09BbRf24Iq,
            _ => Self::BasebandMode, // Default fallback
        }
    }
}

/// Energy Detection Modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EnergyDetectionMode {
    /// Automatic trigger when AGC held
    Auto = 0,
    /// Single measurement
    Single = 1,
    /// Continuous measurements
    Continuous = 2,
    /// Disabled
    Off = 3,
}

impl EnergyDetectionMode {
    pub const fn into_bits(self) -> u8 {
        self as _
    }

    pub const fn from_bits(value: u8) -> Self {
        match value {
            0 => Self::Auto,
            1 => Self::Single,
            2 => Self::Continuous,
            3 => Self::Off,
            _ => Self::Off, // Default fallback
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bbcn_pmuc_read_only_field() {
        let mut pmuc = BbcnPmuc::new()
            .with_en(true)
            .with_avg(false);

        // Verify we can read the sync field (it's read-only)
        let sync_value = pmuc.sync();
        assert_eq!(sync_value, 0);

        // Verify we can set writable fields
        assert_eq!(pmuc.en(), true);
        pmuc.set_en(false);
        assert_eq!(pmuc.en(), false);

        assert_eq!(pmuc.avg(), false);
        pmuc.set_avg(true);
        assert_eq!(pmuc.avg(), true);

        // The sync field should remain unchanged
        assert_eq!(pmuc.sync(), 0);
    }

    #[test]
    fn test_bbcn_pmuc_writable_fields() {
        let pmuc = BbcnPmuc::new()
            .with_en(true)
            .with_avg(true)
            .with_fed(true)
            .with_iqsel(false)
            .with_ccfts(true);

        assert_eq!(pmuc.en(), true);
        assert_eq!(pmuc.avg(), true);
        assert_eq!(pmuc.fed(), true);
        assert_eq!(pmuc.iqsel(), false);
        assert_eq!(pmuc.ccfts(), true);
    }


}
