use std::path::PathBuf;

use crate::registers::*;
use serde::{Deserialize, Serialize};

/*
 * This file contains non-bitfield versions of the radio configuration registers
 * that can be serialized into and loaded from toml files
 *
 * Every register with at least one writable non-padding field has a Config struct
 * So to add a register to the serialized config, just add it to
 * 1. The RadioConfig struct
 * 2. apply_config in Radio
 * 3. to_config in Radio
 */

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)] // allows for serialization from incomplete toml files
pub struct RadioConfig {
    // General Config
    #[serde(skip_serializing_if = "is_default")]
    pub rf_cfg: RfCfgConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub rf_clko: RfClkoConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub rf_bmdvc: RfBmdvcConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub rf_xoc: RfXocConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub rf_iqifc0: RfIqifc0Config,
    #[serde(skip_serializing_if = "is_default")]
    pub rf_iqifc1: RfIqifc1Config,

    // Transceiver Auxiliary Settings
    #[serde(skip_serializing_if = "is_default")]
    pub rf09_auxs: RfnAuxsConfig,

    // Channel configuration
    #[serde(skip_serializing_if = "is_default")]
    pub rf09_cs: RfnCsConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub rf09_ccf0: RfnCcf0Config,
    #[serde(skip_serializing_if = "is_default")]
    pub rf09_cn: RfnCnConfig,

    // Receiver configuration
    #[serde(skip_serializing_if = "is_default")]
    pub rf09_rxbwc: RfnRxbwcConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub rf09_rxdfe: RfnRxdfeConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub rf09_agcc: RfnAgccConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub rf09_agcs: RfnAgcsConfig,

    // Transmitter configuration
    #[serde(skip_serializing_if = "is_default")]
    pub rf09_txcutc: RfnTxcutcConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub rf09_txdfe: RfnTxdfeConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub rf09_pac: RfnPacConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub rf09_padfe: RfnPadfeConfig,

    // PLL configuration
    #[serde(skip_serializing_if = "is_default")]
    pub rf09_pll: RfnPllConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub rf24_auxs: RfnAuxsConfig,

    // Channel configuration
    #[serde(skip_serializing_if = "is_default")]
    pub rf24_cs: RfnCsConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub rf24_ccf0: RfnCcf0Config,
    #[serde(skip_serializing_if = "is_default")]
    pub rf24_cn: RfnCnConfig,

    // Receiver configuration
    #[serde(skip_serializing_if = "is_default")]
    pub rf24_rxbwc: RfnRxbwcConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub rf24_rxdfe: RfnRxdfeConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub rf24_agcc: RfnAgccConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub rf24_agcs: RfnAgcsConfig,

    // Transmitter configuration
    #[serde(skip_serializing_if = "is_default")]
    pub rf24_txcutc: RfnTxcutcConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub rf24_txdfe: RfnTxdfeConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub rf24_pac: RfnPacConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub rf24_padfe: RfnPadfeConfig,

    // PLL configuration
    #[serde(skip_serializing_if = "is_default")]
    pub rf24_pll: RfnPllConfig,

    // BBC0 PHY control (PHY type, baseband enable, FCS mode)
    #[serde(skip_serializing_if = "is_default")]
    pub bbc0_pc: BbcnPcConfig,

    // BBC0 OFDM PHY configuration
    #[serde(skip_serializing_if = "is_default")]
    pub bbc0_ofdmphrtx: BbcnOfdmphrtxConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc0_ofdmc: BbcnOfdmcConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc0_ofdmsw: BbcnOfdmswConfig,

    // BBC0 O-QPSK PHY configuration
    #[serde(skip_serializing_if = "is_default")]
    pub bbc0_oqpskc0: BbcnOqpskc0Config,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc0_oqpskc1: BbcnOqpskc1Config,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc0_oqpskc2: BbcnOqpskc2Config,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc0_oqpskc3: BbcnOqpskc3Config,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc0_oqpskphrtx: BbcnOqpskphrtxConfig,

    // BBC0 Address filter configuration
    #[serde(skip_serializing_if = "is_default")]
    pub bbc0_afc0: BbcnAfc0Config,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc0_afc1: BbcnAfc1Config,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc0_afftm: BbcnAfftmConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc0_affvm: BbcnAffvmConfig,

    // BBC0 Auto mode configuration
    #[serde(skip_serializing_if = "is_default")]
    pub bbc0_amcs: BbcnAmcsConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc0_amedt: BbcnAmedtConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc0_amaackpd: BbcnAmaackpdConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc0_amaackt: BbcnAmaacktConfig,

    // BBC0 FSK PHY configuration
    #[serde(skip_serializing_if = "is_default")]
    pub bbc0_fskc0: BbcnFskc0Config,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc0_fskc1: BbcnFskc1Config,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc0_fskc2: BbcnFskc2Config,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc0_fskc3: BbcnFskc3Config,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc0_fskc4: BbcnFskc4Config,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc0_fskpll: BbcnFskpllConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc0_fsksfd0: BbcnFsksfdConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc0_fsksfd1: BbcnFsksfdConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc0_fskphrtx: BbcnFskphrtxConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc0_fskrpc: BbcnFskrpcConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc0_fskrpcont: BbcnFskrpcontConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc0_fskrpcofft: BbcnFskrpcofftConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc0_fskdm: BbcnFskdmConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc0_fskpe0: BbcnFskpeConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc0_fskpe1: BbcnFskpeConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc0_fskpe2: BbcnFskpeConfig,

    // BBC1 PHY control (PHY type, baseband enable, FCS mode)
    #[serde(skip_serializing_if = "is_default")]
    pub bbc1_pc: BbcnPcConfig,

    // BBC1 OFDM PHY configuration
    #[serde(skip_serializing_if = "is_default")]
    pub bbc1_ofdmphrtx: BbcnOfdmphrtxConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc1_ofdmc: BbcnOfdmcConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc1_ofdmsw: BbcnOfdmswConfig,

    // BBC1 O-QPSK PHY configuration
    #[serde(skip_serializing_if = "is_default")]
    pub bbc1_oqpskc0: BbcnOqpskc0Config,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc1_oqpskc1: BbcnOqpskc1Config,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc1_oqpskc2: BbcnOqpskc2Config,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc1_oqpskc3: BbcnOqpskc3Config,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc1_oqpskphrtx: BbcnOqpskphrtxConfig,

    // BBC1 Address filter configuration
    #[serde(skip_serializing_if = "is_default")]
    pub bbc1_afc0: BbcnAfc0Config,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc1_afc1: BbcnAfc1Config,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc1_afftm: BbcnAfftmConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc1_affvm: BbcnAffvmConfig,

    // BBC1 Auto mode configuration
    #[serde(skip_serializing_if = "is_default")]
    pub bbc1_amcs: BbcnAmcsConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc1_amedt: BbcnAmedtConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc1_amaackpd: BbcnAmaackpdConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc1_amaackt: BbcnAmaacktConfig,

    // BBC1 FSK PHY configuration
    #[serde(skip_serializing_if = "is_default")]
    pub bbc1_fskc0: BbcnFskc0Config,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc1_fskc1: BbcnFskc1Config,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc1_fskc2: BbcnFskc2Config,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc1_fskc3: BbcnFskc3Config,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc1_fskc4: BbcnFskc4Config,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc1_fskpll: BbcnFskpllConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc1_fsksfd0: BbcnFsksfdConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc1_fsksfd1: BbcnFsksfdConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc1_fskphrtx: BbcnFskphrtxConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc1_fskrpc: BbcnFskrpcConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc1_fskrpcont: BbcnFskrpcontConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc1_fskrpcofft: BbcnFskrpcofftConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc1_fskdm: BbcnFskdmConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc1_fskpe0: BbcnFskpeConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc1_fskpe1: BbcnFskpeConfig,
    #[serde(skip_serializing_if = "is_default")]
    pub bbc1_fskpe2: BbcnFskpeConfig,
}

// Used for reducing empty members in radioConfig struct
fn is_default<T: Default + PartialEq>(v: &T) -> bool {
    v == &T::default()
}

/// Every top-level table name a `--config` TOML may contain: the register
/// tables of [`RadioConfig`] plus the `[beacon]`/`[rssi]`/`[spi]`/`[gpio]`
/// daemon tables of [`NetConfig`]. Used by [`check_known_tables`] to reject typos.
pub const KNOWN_TABLES: &[&str] = &[
    // NetConfig
    "beacon", "rssi", "spi", "gpio",
    // RadioConfig - general
    "rf_cfg", "rf_clko", "rf_bmdvc", "rf_xoc", "rf_iqifc0", "rf_iqifc1",
    // RF09
    "rf09_auxs", "rf09_cs", "rf09_ccf0", "rf09_cn", "rf09_rxbwc", "rf09_rxdfe",
    "rf09_agcc", "rf09_agcs", "rf09_txcutc", "rf09_txdfe", "rf09_pac",
    "rf09_padfe", "rf09_pll",
    // RF24
    "rf24_auxs", "rf24_cs", "rf24_ccf0", "rf24_cn", "rf24_rxbwc", "rf24_rxdfe",
    "rf24_agcc", "rf24_agcs", "rf24_txcutc", "rf24_txdfe", "rf24_pac",
    "rf24_padfe", "rf24_pll",
    // BBC0
    "bbc0_pc", "bbc0_ofdmphrtx", "bbc0_ofdmc", "bbc0_ofdmsw", "bbc0_oqpskc0",
    "bbc0_oqpskc1", "bbc0_oqpskc2", "bbc0_oqpskc3", "bbc0_oqpskphrtx",
    "bbc0_afc0", "bbc0_afc1", "bbc0_afftm", "bbc0_affvm", "bbc0_amcs",
    "bbc0_amedt", "bbc0_amaackpd", "bbc0_amaackt", "bbc0_fskc0", "bbc0_fskc1",
    "bbc0_fskc2", "bbc0_fskc3", "bbc0_fskc4", "bbc0_fskpll", "bbc0_fsksfd0",
    "bbc0_fsksfd1", "bbc0_fskphrtx", "bbc0_fskrpc", "bbc0_fskrpcont",
    "bbc0_fskrpcofft", "bbc0_fskdm", "bbc0_fskpe0", "bbc0_fskpe1", "bbc0_fskpe2",
    // BBC1
    "bbc1_pc", "bbc1_ofdmphrtx", "bbc1_ofdmc", "bbc1_ofdmsw", "bbc1_oqpskc0",
    "bbc1_oqpskc1", "bbc1_oqpskc2", "bbc1_oqpskc3", "bbc1_oqpskphrtx",
    "bbc1_afc0", "bbc1_afc1", "bbc1_afftm", "bbc1_affvm", "bbc1_amcs",
    "bbc1_amedt", "bbc1_amaackpd", "bbc1_amaackt", "bbc1_fskc0", "bbc1_fskc1",
    "bbc1_fskc2", "bbc1_fskc3", "bbc1_fskc4", "bbc1_fskpll", "bbc1_fsksfd0",
    "bbc1_fsksfd1", "bbc1_fskphrtx", "bbc1_fskrpc", "bbc1_fskrpcont",
    "bbc1_fskrpcofft", "bbc1_fskdm", "bbc1_fskpe0", "bbc1_fskpe1", "bbc1_fskpe2",
];

/// Reject a config TOML that contains an unknown top-level table - almost always
/// a typo (e.g. `[rf09_rxdfee]`) that serde would otherwise silently ignore,
/// leaving the corresponding register at its (often deaf) reset default. Returns
/// a single error listing every unrecognised table.
pub fn check_known_tables(contents: &str) -> Result<(), String> {
    let table: toml::Table =
        contents.parse().map_err(|e| format!("config is not valid TOML: {e}"))?;
    let unknown: Vec<&str> = table
        .keys()
        .map(|k| k.as_str())
        .filter(|k| !KNOWN_TABLES.contains(k))
        .collect();
    if unknown.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "config has unknown table(s): [{}] - check for typos (known tables are register \
             names like rf09_rxdfe, plus beacon/rssi/spi/gpio)",
            unknown.join("], ["),
        ))
    }
}

/// Daemon socket settings, loaded from the SAME `--config` TOML as the register
/// config via a second `toml::from_str` pass. Because the crate sets no
/// `deny_unknown_fields`, the register pass ignores the `[beacon]`/`[rssi]`
/// tables and this pass ignores all register tables, so register-only
/// configs keep working (every field stays `None` -> built-in default).
///
/// Every field is optional so a CLI flag can override it; the daemon resolves
/// the effective value as CLI flag > TOML > default.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct NetConfig {
    pub beacon: BeaconConfig,
    pub rssi: RssiConfig,
    pub spi: SpiConfig,
    pub gpio: GpioConfig,
}

/// `[beacon]` table: the dedicated TX-input socket to send the beacon frames to.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct BeaconConfig {
    /// Bind the beacon socket at all (default `true`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// UDP port to bind on 127.0.0.1 (default `10015`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    /// Full UDP bind address (e.g. "0.0.0.0:10015"); overrides `port`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bind: Option<String>,
    /// Unix-domain datagram path; overrides `port`/`bind`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uds: Option<PathBuf>,
}

/// `[rssi]` table: the RSSI push to a UDP port the satellite C3 reads (one raw
/// int8 dBm byte sent after every received frame). An old `interval_ms` key
/// is tolerated (serde ignores unknown fields here) but no longer read: the
/// push is per-frame, not periodic.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct RssiConfig {
    /// Enable the push (default: on when `peer` is set).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// UDP destination "host:port" to push RSSI to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peer: Option<String>,
}

/// `[spi]` table: which spidev character device drives the radio, and at what
/// clock. Both fields are optional so a CLI flag can override them; the daemon
/// resolves the effective value as CLI flag > TOML > default.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct SpiConfig {
    /// spidev device path (default "/dev/spidev0.0").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dev: Option<String>,
    /// SPI clock rate in Hz (default 1_000_000, see `spi::DEFAULT_SPI_HZ`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hz: Option<u32>,
}

/// `[gpio]` table: the GPIO line the radio's IRQ is wired to. Both fields are
/// optional so a CLI flag can override them; the daemon resolves the effective
/// value as CLI flag > TOML > default.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct GpioConfig {
    /// GPIO chip character device path (default "/dev/gpiochip0").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chip: Option<String>,
    /// GPIO line/offset for the radio IRQ, rising edge (default 25).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct RfRstConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cmd: Option<ChipResetCmd>,
}

impl From<&RfRst> for RfRstConfig {
    fn from(r: &RfRst) -> Self {
        let default = RfRst::new();
        Self {
            cmd: (r.cmd() != default.cmd()).then(|| r.cmd()),
        }
    }
}

impl From<&RfRstConfig> for RfRst {
    fn from(c: &RfRstConfig) -> Self {
        let default = RfRst::new();
        RfRst::new().with_cmd(c.cmd.unwrap_or_else(|| default.cmd()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct RfCfgConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drv: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub irqp: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub irqmm: Option<bool>,
}

impl From<&RfCfg> for RfCfgConfig {
    fn from(r: &RfCfg) -> Self {
        let default = RfCfg::new();
        Self {
            drv: (r.drv() != default.drv()).then(|| r.drv()),
            irqp: (r.irqp() != default.irqp()).then(|| r.irqp()),
            irqmm: (r.irqmm() != default.irqmm()).then(|| r.irqmm()),
        }
    }
}

impl From<&RfCfgConfig> for RfCfg {
    fn from(c: &RfCfgConfig) -> Self {
        let default = RfCfg::new();
        RfCfg::new()
            .with_drv(c.drv.unwrap_or_else(|| default.drv()))
            .with_irqp(c.irqp.unwrap_or_else(|| default.irqp()))
            .with_irqmm(c.irqmm.unwrap_or_else(|| default.irqmm()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct RfClkoConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drv: Option<u8>,
}

impl From<&RfClko> for RfClkoConfig {
    fn from(r: &RfClko) -> Self {
        let default = RfClko::new();
        Self {
            os: (r.os() != default.os()).then(|| r.os()),
            drv: (r.drv() != default.drv()).then(|| r.drv()),
        }
    }
}

impl From<&RfClkoConfig> for RfClko {
    fn from(c: &RfClkoConfig) -> Self {
        let default = RfClko::new();
        RfClko::new()
            .with_os(c.os.unwrap_or_else(|| default.os()))
            .with_drv(c.drv.unwrap_or_else(|| default.drv()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct RfBmdvcConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bmth: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bmr: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bmen: Option<bool>,
}

impl From<&RfBmdvc> for RfBmdvcConfig {
    fn from(r: &RfBmdvc) -> Self {
        let default = RfBmdvc::new();
        Self {
            bmth: (r.bmth() != default.bmth()).then(|| r.bmth()),
            bmr: (r.bmr() != default.bmr()).then(|| r.bmr()),
            bmen: (r.bmen() != default.bmen()).then(|| r.bmen()),
        }
    }
}

impl From<&RfBmdvcConfig> for RfBmdvc {
    fn from(c: &RfBmdvcConfig) -> Self {
        let default = RfBmdvc::new();
        RfBmdvc::new()
            .with_bmth(c.bmth.unwrap_or_else(|| default.bmth()))
            .with_bmr(c.bmr.unwrap_or_else(|| default.bmr()))
            .with_bmen(c.bmen.unwrap_or_else(|| default.bmen()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct RfXocConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trim: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fs: Option<u8>,
}

impl From<&RfXoc> for RfXocConfig {
    fn from(r: &RfXoc) -> Self {
        let default = RfXoc::new();
        Self {
            trim: (r.trim() != default.trim()).then(|| r.trim()),
            fs: (r.fs() != default.fs()).then(|| r.fs()),
        }
    }
}

impl From<&RfXocConfig> for RfXoc {
    fn from(c: &RfXocConfig) -> Self {
        let default = RfXoc::new();
        RfXoc::new()
            .with_trim(c.trim.unwrap_or_else(|| default.trim()))
            .with_fs(c.fs.unwrap_or_else(|| default.fs()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct RfIqifc0Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eec: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cmv1v2: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cmv: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drv: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extlb: Option<bool>,
}

impl From<&RfIqifc0> for RfIqifc0Config {
    fn from(r: &RfIqifc0) -> Self {
        let default = RfIqifc0::new();
        Self {
            eec: (r.eec() != default.eec()).then(|| r.eec()),
            cmv1v2: (r.cmv1v2() != default.cmv1v2()).then(|| r.cmv1v2()),
            cmv: (r.cmv() != default.cmv()).then(|| r.cmv()),
            drv: (r.drv() != default.drv()).then(|| r.drv()),
            extlb: (r.extlb() != default.extlb()).then(|| r.extlb()),
        }
    }
}

impl From<&RfIqifc0Config> for RfIqifc0 {
    fn from(c: &RfIqifc0Config) -> Self {
        let default = RfIqifc0::new();
        RfIqifc0::new()
            .with_eec(c.eec.unwrap_or_else(|| default.eec()))
            .with_cmv1v2(c.cmv1v2.unwrap_or_else(|| default.cmv1v2()))
            .with_cmv(c.cmv.unwrap_or_else(|| default.cmv()))
            .with_drv(c.drv.unwrap_or_else(|| default.drv()))
            .with_extlb(c.extlb.unwrap_or_else(|| default.extlb()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct RfIqifc1Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skewdrv: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chpm: Option<ChipMode>,
}

impl From<&RfIqifc1> for RfIqifc1Config {
    fn from(r: &RfIqifc1) -> Self {
        let default = RfIqifc1::new();
        Self {
            skewdrv: (r.skewdrv() != default.skewdrv()).then(|| r.skewdrv()),
            chpm: (r.chpm() != default.chpm()).then(|| r.chpm()),
        }
    }
}

impl From<&RfIqifc1Config> for RfIqifc1 {
    fn from(c: &RfIqifc1Config) -> Self {
        let default = RfIqifc1::new();
        RfIqifc1::new()
            .with_skewdrv(c.skewdrv.unwrap_or_else(|| default.skewdrv()))
            .with_chpm(c.chpm.unwrap_or_else(|| default.chpm()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct RfnIrqmConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wakeup: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trxrdy: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edc: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batlow: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trxerr: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iqifsf: Option<bool>,
}

impl From<&RfnIrqm> for RfnIrqmConfig {
    fn from(r: &RfnIrqm) -> Self {
        let default = RfnIrqm::new();
        Self {
            wakeup: (r.wakeup() != default.wakeup()).then(|| r.wakeup()),
            trxrdy: (r.trxrdy() != default.trxrdy()).then(|| r.trxrdy()),
            edc: (r.edc() != default.edc()).then(|| r.edc()),
            batlow: (r.batlow() != default.batlow()).then(|| r.batlow()),
            trxerr: (r.trxerr() != default.trxerr()).then(|| r.trxerr()),
            iqifsf: (r.iqifsf() != default.iqifsf()).then(|| r.iqifsf()),
        }
    }
}

impl From<&RfnIrqmConfig> for RfnIrqm {
    fn from(c: &RfnIrqmConfig) -> Self {
        let default = RfnIrqm::new();
        RfnIrqm::new()
            .with_wakeup(c.wakeup.unwrap_or_else(|| default.wakeup()))
            .with_trxrdy(c.trxrdy.unwrap_or_else(|| default.trxrdy()))
            .with_edc(c.edc.unwrap_or_else(|| default.edc()))
            .with_batlow(c.batlow.unwrap_or_else(|| default.batlow()))
            .with_trxerr(c.trxerr.unwrap_or_else(|| default.trxerr()))
            .with_iqifsf(c.iqifsf.unwrap_or_else(|| default.iqifsf()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct RfnAuxsConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pavc: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ave: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aven: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agcmap: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extlnabyp: Option<bool>,
}

impl From<&RfnAuxs> for RfnAuxsConfig {
    fn from(r: &RfnAuxs) -> Self {
        let default = RfnAuxs::new();
        Self {
            pavc: (r.pavc() != default.pavc()).then(|| r.pavc()),
            ave: (r.ave() != default.ave()).then(|| r.ave()),
            aven: (r.aven() != default.aven()).then(|| r.aven()),
            agcmap: (r.agcmap() != default.agcmap()).then(|| r.agcmap()),
            extlnabyp: (r.extlnabyp() != default.extlnabyp()).then(|| r.extlnabyp()),
        }
    }
}

impl From<&RfnAuxsConfig> for RfnAuxs {
    fn from(c: &RfnAuxsConfig) -> Self {
        let default = RfnAuxs::new();
        RfnAuxs::new()
            .with_pavc(c.pavc.unwrap_or_else(|| default.pavc()))
            .with_ave(c.ave.unwrap_or_else(|| default.ave()))
            .with_aven(c.aven.unwrap_or_else(|| default.aven()))
            .with_agcmap(c.agcmap.unwrap_or_else(|| default.agcmap()))
            .with_extlnabyp(c.extlnabyp.unwrap_or_else(|| default.extlnabyp()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct RfnCmdConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cmd: Option<TransceiverCmd>,
}

impl From<&RfnCmd> for RfnCmdConfig {
    fn from(r: &RfnCmd) -> Self {
        let default = RfnCmd::new();
        Self {
            cmd: (r.cmd() != default.cmd()).then(|| r.cmd()),
        }
    }
}

impl From<&RfnCmdConfig> for RfnCmd {
    fn from(c: &RfnCmdConfig) -> Self {
        let default = RfnCmd::new();
        RfnCmd::new().with_cmd(c.cmd.unwrap_or_else(|| default.cmd()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct RfnCsConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cs: Option<u8>,
}

impl From<&RfnCs> for RfnCsConfig {
    fn from(r: &RfnCs) -> Self {
        let default = RfnCs::new();
        Self {
            cs: (r.cs() != default.cs()).then(|| r.cs()),
        }
    }
}

impl From<&RfnCsConfig> for RfnCs {
    fn from(c: &RfnCsConfig) -> Self {
        let default = RfnCs::new();
        RfnCs::new().with_cs(c.cs.unwrap_or_else(|| default.cs()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct RfnCcf0Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ccf0: Option<u16>,
}

impl From<&RfnCcf0> for RfnCcf0Config {
    fn from(r: &RfnCcf0) -> Self {
        let default = RfnCcf0::new();
        Self {
            ccf0: (r.ccf0() != default.ccf0()).then(|| r.ccf0()),
        }
    }
}

impl From<&RfnCcf0Config> for RfnCcf0 {
    fn from(c: &RfnCcf0Config) -> Self {
        let default = RfnCcf0::new();
        RfnCcf0::new().with_ccf0(c.ccf0.unwrap_or_else(|| default.ccf0()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct RfnCnConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cn: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cm: Option<u8>,
}

impl From<&RfnCn> for RfnCnConfig {
    fn from(r: &RfnCn) -> Self {
        let default = RfnCn::new();
        Self {
            cn: (r.cn() != default.cn()).then(|| r.cn()),
            cm: (r.cm() != default.cm()).then(|| r.cm()),
        }
    }
}

impl From<&RfnCnConfig> for RfnCn {
    fn from(c: &RfnCnConfig) -> Self {
        let default = RfnCn::new();
        RfnCn::new()
            .with_cn(c.cn.unwrap_or_else(|| default.cn()))
            .with_cm(c.cm.unwrap_or_else(|| default.cm()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct RfnRxbwcConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bw: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ifs: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ifi: Option<bool>,
}

impl From<&RfnRxbwc> for RfnRxbwcConfig {
    fn from(r: &RfnRxbwc) -> Self {
        let default = RfnRxbwc::new();
        Self {
            bw: (r.bw() != default.bw()).then(|| r.bw()),
            ifs: (r.ifs() != default.ifs()).then(|| r.ifs()),
            ifi: (r.ifi() != default.ifi()).then(|| r.ifi()),
        }
    }
}

impl From<&RfnRxbwcConfig> for RfnRxbwc {
    fn from(c: &RfnRxbwcConfig) -> Self {
        let default = RfnRxbwc::new();
        RfnRxbwc::new()
            .with_bw(c.bw.unwrap_or_else(|| default.bw()))
            .with_ifs(c.ifs.unwrap_or_else(|| default.ifs()))
            .with_ifi(c.ifi.unwrap_or_else(|| default.ifi()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct RfnRxdfeConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sr: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rcut: Option<u8>,
}

impl From<&RfnRxdfe> for RfnRxdfeConfig {
    fn from(r: &RfnRxdfe) -> Self {
        let default = RfnRxdfe::new();
        Self {
            sr: (r.sr() != default.sr()).then(|| r.sr()),
            rcut: (r.rcut() != default.rcut()).then(|| r.rcut()),
        }
    }
}

impl From<&RfnRxdfeConfig> for RfnRxdfe {
    fn from(c: &RfnRxdfeConfig) -> Self {
        let default = RfnRxdfe::new();
        RfnRxdfe::new()
            .with_sr(c.sr.unwrap_or_else(|| default.sr()))
            .with_rcut(c.rcut.unwrap_or_else(|| default.rcut()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct RfnAgccConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub en: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frzc: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rst: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avgs: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agci: Option<bool>,
}

impl From<&RfnAgcc> for RfnAgccConfig {
    fn from(r: &RfnAgcc) -> Self {
        let default = RfnAgcc::new();
        Self {
            en: (r.en() != default.en()).then(|| r.en()),
            frzc: (r.frzc() != default.frzc()).then(|| r.frzc()),
            rst: (r.rst() != default.rst()).then(|| r.rst()),
            avgs: (r.avgs() != default.avgs()).then(|| r.avgs()),
            agci: (r.agci() != default.agci()).then(|| r.agci()),
        }
    }
}

impl From<&RfnAgccConfig> for RfnAgcc {
    fn from(c: &RfnAgccConfig) -> Self {
        let default = RfnAgcc::new();
        RfnAgcc::new()
            .with_en(c.en.unwrap_or_else(|| default.en()))
            .with_frzc(c.frzc.unwrap_or_else(|| default.frzc()))
            .with_rst(c.rst.unwrap_or_else(|| default.rst()))
            .with_avgs(c.avgs.unwrap_or_else(|| default.avgs()))
            .with_agci(c.agci.unwrap_or_else(|| default.agci()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct RfnAgcsConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gcw: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tgt: Option<u8>,
}

impl From<&RfnAgcs> for RfnAgcsConfig {
    fn from(r: &RfnAgcs) -> Self {
        let default = RfnAgcs::new();
        Self {
            gcw: (r.gcw() != default.gcw()).then(|| r.gcw()),
            tgt: (r.tgt() != default.tgt()).then(|| r.tgt()),
        }
    }
}

impl From<&RfnAgcsConfig> for RfnAgcs {
    fn from(c: &RfnAgcsConfig) -> Self {
        let default = RfnAgcs::new();
        RfnAgcs::new()
            .with_gcw(c.gcw.unwrap_or_else(|| default.gcw()))
            .with_tgt(c.tgt.unwrap_or_else(|| default.tgt()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct RfnEdcConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edm: Option<EnergyDetectionMode>,
}

impl From<&RfnEdc> for RfnEdcConfig {
    fn from(r: &RfnEdc) -> Self {
        let default = RfnEdc::new();
        Self {
            edm: (r.edm() != default.edm()).then(|| r.edm()),
        }
    }
}

impl From<&RfnEdcConfig> for RfnEdc {
    fn from(c: &RfnEdcConfig) -> Self {
        let default = RfnEdc::new();
        RfnEdc::new().with_edm(c.edm.unwrap_or_else(|| default.edm()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct RfnEddConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dtb: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub df: Option<u8>,
}

impl From<&RfnEdd> for RfnEddConfig {
    fn from(r: &RfnEdd) -> Self {
        let default = RfnEdd::new();
        Self {
            dtb: (r.dtb() != default.dtb()).then(|| r.dtb()),
            df: (r.df() != default.df()).then(|| r.df()),
        }
    }
}

impl From<&RfnEddConfig> for RfnEdd {
    fn from(c: &RfnEddConfig) -> Self {
        let default = RfnEdd::new();
        RfnEdd::new()
            .with_dtb(c.dtb.unwrap_or_else(|| default.dtb()))
            .with_df(c.df.unwrap_or_else(|| default.df()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct RfnPllConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lbw: Option<u8>,
}

impl From<&RfnPll> for RfnPllConfig {
    fn from(r: &RfnPll) -> Self {
        let default = RfnPll::new();
        Self {
            lbw: (r.lbw() != default.lbw()).then(|| r.lbw()),
        }
    }
}

impl From<&RfnPllConfig> for RfnPll {
    fn from(c: &RfnPllConfig) -> Self {
        let default = RfnPll::new();
        RfnPll::new().with_lbw(c.lbw.unwrap_or_else(|| default.lbw()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct RfnTxcutcConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lpfcut: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paramp: Option<u8>,
}

impl From<&RfnTxcutc> for RfnTxcutcConfig {
    fn from(r: &RfnTxcutc) -> Self {
        let default = RfnTxcutc::new();
        Self {
            lpfcut: (r.lpfcut() != default.lpfcut()).then(|| r.lpfcut()),
            paramp: (r.paramp() != default.paramp()).then(|| r.paramp()),
        }
    }
}

impl From<&RfnTxcutcConfig> for RfnTxcutc {
    fn from(c: &RfnTxcutcConfig) -> Self {
        let default = RfnTxcutc::new();
        RfnTxcutc::new()
            .with_lpfcut(c.lpfcut.unwrap_or_else(|| default.lpfcut()))
            .with_paramp(c.paramp.unwrap_or_else(|| default.paramp()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct RfnTxdfeConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sr: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dm: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rcut: Option<u8>,
}

impl From<&RfnTxdfe> for RfnTxdfeConfig {
    fn from(r: &RfnTxdfe) -> Self {
        let default = RfnTxdfe::new();
        Self {
            sr: (r.sr() != default.sr()).then(|| r.sr()),
            dm: (r.dm() != default.dm()).then(|| r.dm()),
            rcut: (r.rcut() != default.rcut()).then(|| r.rcut()),
        }
    }
}

impl From<&RfnTxdfeConfig> for RfnTxdfe {
    fn from(c: &RfnTxdfeConfig) -> Self {
        let default = RfnTxdfe::new();
        RfnTxdfe::new()
            .with_sr(c.sr.unwrap_or_else(|| default.sr()))
            .with_dm(c.dm.unwrap_or_else(|| default.dm()))
            .with_rcut(c.rcut.unwrap_or_else(|| default.rcut()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct RfnPacConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub txpwr: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pacur: Option<u8>,
}

impl From<&RfnPac> for RfnPacConfig {
    fn from(r: &RfnPac) -> Self {
        let default = RfnPac::new();
        Self {
            txpwr: (r.txpwr() != default.txpwr()).then(|| r.txpwr()),
            pacur: (r.pacur() != default.pacur()).then(|| r.pacur()),
        }
    }
}

impl From<&RfnPacConfig> for RfnPac {
    fn from(c: &RfnPacConfig) -> Self {
        let default = RfnPac::new();
        RfnPac::new()
            .with_txpwr(c.txpwr.unwrap_or_else(|| default.txpwr()))
            .with_pacur(c.pacur.unwrap_or_else(|| default.pacur()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct RfnPadfeConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padfe: Option<u8>,
}

impl From<&RfnPadfe> for RfnPadfeConfig {
    fn from(r: &RfnPadfe) -> Self {
        let default = RfnPadfe::new();
        Self {
            padfe: (r.padfe() != default.padfe()).then(|| r.padfe()),
        }
    }
}

impl From<&RfnPadfeConfig> for RfnPadfe {
    fn from(c: &RfnPadfeConfig) -> Self {
        let default = RfnPadfe::new();
        RfnPadfe::new().with_padfe(c.padfe.unwrap_or_else(|| default.padfe()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct RfnTxciConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dcoi: Option<u8>,
}

impl From<&RfnTxci> for RfnTxciConfig {
    fn from(r: &RfnTxci) -> Self {
        let default = RfnTxci::new();
        Self {
            dcoi: (r.dcoi() != default.dcoi()).then(|| r.dcoi()),
        }
    }
}

impl From<&RfnTxciConfig> for RfnTxci {
    fn from(c: &RfnTxciConfig) -> Self {
        let default = RfnTxci::new();
        RfnTxci::new().with_dcoi(c.dcoi.unwrap_or_else(|| default.dcoi()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct RfnTxcqConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dcoq: Option<u8>,
}

impl From<&RfnTxcq> for RfnTxcqConfig {
    fn from(r: &RfnTxcq) -> Self {
        let default = RfnTxcq::new();
        Self {
            dcoq: (r.dcoq() != default.dcoq()).then(|| r.dcoq()),
        }
    }
}

impl From<&RfnTxcqConfig> for RfnTxcq {
    fn from(c: &RfnTxcqConfig) -> Self {
        let default = RfnTxcq::new();
        RfnTxcq::new().with_dcoq(c.dcoq.unwrap_or_else(|| default.dcoq()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct RfnTxdaciConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub txdacid: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entxdacid: Option<bool>,
}

impl From<&RfnTxdaci> for RfnTxdaciConfig {
    fn from(r: &RfnTxdaci) -> Self {
        let default = RfnTxdaci::new();
        Self {
            txdacid: (r.txdacid() != default.txdacid()).then(|| r.txdacid()),
            entxdacid: (r.entxdacid() != default.entxdacid()).then(|| r.entxdacid()),
        }
    }
}

impl From<&RfnTxdaciConfig> for RfnTxdaci {
    fn from(c: &RfnTxdaciConfig) -> Self {
        let default = RfnTxdaci::new();
        RfnTxdaci::new()
            .with_txdacid(c.txdacid.unwrap_or_else(|| default.txdacid()))
            .with_entxdacid(c.entxdacid.unwrap_or_else(|| default.entxdacid()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct RfnTxdacqConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub txdacqd: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entxdacqd: Option<bool>,
}

impl From<&RfnTxdacq> for RfnTxdacqConfig {
    fn from(r: &RfnTxdacq) -> Self {
        let default = RfnTxdacq::new();
        Self {
            txdacqd: (r.txdacqd() != default.txdacqd()).then(|| r.txdacqd()),
            entxdacqd: (r.entxdacqd() != default.entxdacqd()).then(|| r.entxdacqd()),
        }
    }
}

impl From<&RfnTxdacqConfig> for RfnTxdacq {
    fn from(c: &RfnTxdacqConfig) -> Self {
        let default = RfnTxdacq::new();
        RfnTxdacq::new()
            .with_txdacqd(c.txdacqd.unwrap_or_else(|| default.txdacqd()))
            .with_entxdacqd(c.entxdacqd.unwrap_or_else(|| default.entxdacqd()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BbcnPcConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pt: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bben: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fcst: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub txafcs: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fcsfe: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ctx: Option<bool>,
}

impl From<&BbcnPc> for BbcnPcConfig {
    fn from(r: &BbcnPc) -> Self {
        let default = BbcnPc::new();
        Self {
            pt: (r.pt() != default.pt()).then(|| r.pt()),
            bben: (r.bben() != default.bben()).then(|| r.bben()),
            fcst: (r.fcst() != default.fcst()).then(|| r.fcst()),
            txafcs: (r.txafcs() != default.txafcs()).then(|| r.txafcs()),
            fcsfe: (r.fcsfe() != default.fcsfe()).then(|| r.fcsfe()),
            ctx: (r.ctx() != default.ctx()).then(|| r.ctx()),
        }
    }
}

impl From<&BbcnPcConfig> for BbcnPc {
    fn from(c: &BbcnPcConfig) -> Self {
        let default = BbcnPc::new();
        BbcnPc::new()
            .with_pt(c.pt.unwrap_or_else(|| default.pt()))
            .with_bben(c.bben.unwrap_or_else(|| default.bben()))
            .with_fcst(c.fcst.unwrap_or_else(|| default.fcst()))
            .with_txafcs(c.txafcs.unwrap_or_else(|| default.txafcs()))
            .with_fcsfe(c.fcsfe.unwrap_or_else(|| default.fcsfe()))
            .with_ctx(c.ctx.unwrap_or_else(|| default.ctx()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BbcnTxflConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub txfl: Option<u16>,
}

impl From<&BbcnTxfl> for BbcnTxflConfig {
    fn from(r: &BbcnTxfl) -> Self {
        let default = BbcnTxfl::new();
        Self {
            txfl: (r.txfl() != default.txfl()).then(|| r.txfl()),
        }
    }
}

impl From<&BbcnTxflConfig> for BbcnTxfl {
    fn from(c: &BbcnTxflConfig) -> Self {
        let default = BbcnTxfl::new();
        BbcnTxfl::new().with_txfl(c.txfl.unwrap_or_else(|| default.txfl()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BbcnFbliConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fbli: Option<u16>,
}

impl From<&BbcnFbli> for BbcnFbliConfig {
    fn from(r: &BbcnFbli) -> Self {
        let default = BbcnFbli::new();
        Self {
            fbli: (r.fbli() != default.fbli()).then(|| r.fbli()),
        }
    }
}

impl From<&BbcnFbliConfig> for BbcnFbli {
    fn from(c: &BbcnFbliConfig) -> Self {
        let default = BbcnFbli::new();
        BbcnFbli::new().with_fbli(c.fbli.unwrap_or_else(|| default.fbli()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BbcnOfdmphrtxConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcs: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rb5: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rb17: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rb18: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rb21: Option<bool>,
}

impl From<&BbcnOfdmphrtx> for BbcnOfdmphrtxConfig {
    fn from(r: &BbcnOfdmphrtx) -> Self {
        let default = BbcnOfdmphrtx::new();
        Self {
            mcs: (r.mcs() != default.mcs()).then(|| r.mcs()),
            rb5: (r.rb5() != default.rb5()).then(|| r.rb5()),
            rb17: (r.rb17() != default.rb17()).then(|| r.rb17()),
            rb18: (r.rb18() != default.rb18()).then(|| r.rb18()),
            rb21: (r.rb21() != default.rb21()).then(|| r.rb21()),
        }
    }
}

impl From<&BbcnOfdmphrtxConfig> for BbcnOfdmphrtx {
    fn from(c: &BbcnOfdmphrtxConfig) -> Self {
        let default = BbcnOfdmphrtx::new();
        BbcnOfdmphrtx::new()
            .with_mcs(c.mcs.unwrap_or_else(|| default.mcs()))
            .with_rb5(c.rb5.unwrap_or_else(|| default.rb5()))
            .with_rb17(c.rb17.unwrap_or_else(|| default.rb17()))
            .with_rb18(c.rb18.unwrap_or_else(|| default.rb18()))
            .with_rb21(c.rb21.unwrap_or_else(|| default.rb21()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BbcnOfdmcConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opt: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poi: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lfo: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sstx: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssrx: Option<u8>,
}

impl From<&BbcnOfdmc> for BbcnOfdmcConfig {
    fn from(r: &BbcnOfdmc) -> Self {
        let default = BbcnOfdmc::new();
        Self {
            opt: (r.opt() != default.opt()).then(|| r.opt()),
            poi: (r.poi() != default.poi()).then(|| r.poi()),
            lfo: (r.lfo() != default.lfo()).then(|| r.lfo()),
            sstx: (r.sstx() != default.sstx()).then(|| r.sstx()),
            ssrx: (r.ssrx() != default.ssrx()).then(|| r.ssrx()),
        }
    }
}

impl From<&BbcnOfdmcConfig> for BbcnOfdmc {
    fn from(c: &BbcnOfdmcConfig) -> Self {
        let default = BbcnOfdmc::new();
        BbcnOfdmc::new()
            .with_opt(c.opt.unwrap_or_else(|| default.opt()))
            .with_poi(c.poi.unwrap_or_else(|| default.poi()))
            .with_lfo(c.lfo.unwrap_or_else(|| default.lfo()))
            .with_sstx(c.sstx.unwrap_or_else(|| default.sstx()))
            .with_ssrx(c.ssrx.unwrap_or_else(|| default.ssrx()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BbcnOfdmswConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rxo: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdt: Option<u8>,
}

impl From<&BbcnOfdmsw> for BbcnOfdmswConfig {
    fn from(r: &BbcnOfdmsw) -> Self {
        let default = BbcnOfdmsw::new();
        Self {
            rxo: (r.rxo() != default.rxo()).then(|| r.rxo()),
            pdt: (r.pdt() != default.pdt()).then(|| r.pdt()),
        }
    }
}

impl From<&BbcnOfdmswConfig> for BbcnOfdmsw {
    fn from(c: &BbcnOfdmswConfig) -> Self {
        let default = BbcnOfdmsw::new();
        BbcnOfdmsw::new()
            .with_rxo(c.rxo.unwrap_or_else(|| default.rxo()))
            .with_pdt(c.pdt.unwrap_or_else(|| default.pdt()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BbcnOqpskc0Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fchip: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mod_: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dm: Option<bool>,
}

impl From<&BbcnOqpskc0> for BbcnOqpskc0Config {
    fn from(r: &BbcnOqpskc0) -> Self {
        let default = BbcnOqpskc0::new();
        Self {
            fchip: (r.fchip() != default.fchip()).then(|| r.fchip()),
            mod_: (r.mod_() != default.mod_()).then(|| r.mod_()),
            dm: (r.dm() != default.dm()).then(|| r.dm()),
        }
    }
}

impl From<&BbcnOqpskc0Config> for BbcnOqpskc0 {
    fn from(c: &BbcnOqpskc0Config) -> Self {
        let default = BbcnOqpskc0::new();
        BbcnOqpskc0::new()
            .with_fchip(c.fchip.unwrap_or_else(|| default.fchip()))
            .with_mod_(c.mod_.unwrap_or_else(|| default.mod_()))
            .with_dm(c.dm.unwrap_or_else(|| default.dm()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BbcnOqpskc1Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdt0: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdt1: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rxoleg: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rxo: Option<bool>,
}

impl From<&BbcnOqpskc1> for BbcnOqpskc1Config {
    fn from(r: &BbcnOqpskc1) -> Self {
        let default = BbcnOqpskc1::new();
        Self {
            pdt0: (r.pdt0() != default.pdt0()).then(|| r.pdt0()),
            pdt1: (r.pdt1() != default.pdt1()).then(|| r.pdt1()),
            rxoleg: (r.rxoleg() != default.rxoleg()).then(|| r.rxoleg()),
            rxo: (r.rxo() != default.rxo()).then(|| r.rxo()),
        }
    }
}

impl From<&BbcnOqpskc1Config> for BbcnOqpskc1 {
    fn from(c: &BbcnOqpskc1Config) -> Self {
        let default = BbcnOqpskc1::new();
        BbcnOqpskc1::new()
            .with_pdt0(c.pdt0.unwrap_or_else(|| default.pdt0()))
            .with_pdt1(c.pdt1.unwrap_or_else(|| default.pdt1()))
            .with_rxoleg(c.rxoleg.unwrap_or_else(|| default.rxoleg()))
            .with_rxo(c.rxo.unwrap_or_else(|| default.rxo()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BbcnOqpskc2Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rxm: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fcstleg: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enprop: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rpc: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spc: Option<bool>,
}

impl From<&BbcnOqpskc2> for BbcnOqpskc2Config {
    fn from(r: &BbcnOqpskc2) -> Self {
        let default = BbcnOqpskc2::new();
        Self {
            rxm: (r.rxm() != default.rxm()).then(|| r.rxm()),
            fcstleg: (r.fcstleg() != default.fcstleg()).then(|| r.fcstleg()),
            enprop: (r.enprop() != default.enprop()).then(|| r.enprop()),
            rpc: (r.rpc() != default.rpc()).then(|| r.rpc()),
            spc: (r.spc() != default.spc()).then(|| r.spc()),
        }
    }
}

impl From<&BbcnOqpskc2Config> for BbcnOqpskc2 {
    fn from(c: &BbcnOqpskc2Config) -> Self {
        let default = BbcnOqpskc2::new();
        BbcnOqpskc2::new()
            .with_rxm(c.rxm.unwrap_or_else(|| default.rxm()))
            .with_fcstleg(c.fcstleg.unwrap_or_else(|| default.fcstleg()))
            .with_enprop(c.enprop.unwrap_or_else(|| default.enprop()))
            .with_rpc(c.rpc.unwrap_or_else(|| default.rpc()))
            .with_spc(c.spc.unwrap_or_else(|| default.spc()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BbcnOqpskc3Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nsfd: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hrleg: Option<bool>,
}

impl From<&BbcnOqpskc3> for BbcnOqpskc3Config {
    fn from(r: &BbcnOqpskc3) -> Self {
        let default = BbcnOqpskc3::new();
        Self {
            nsfd: (r.nsfd() != default.nsfd()).then(|| r.nsfd()),
            hrleg: (r.hrleg() != default.hrleg()).then(|| r.hrleg()),
        }
    }
}

impl From<&BbcnOqpskc3Config> for BbcnOqpskc3 {
    fn from(c: &BbcnOqpskc3Config) -> Self {
        let default = BbcnOqpskc3::new();
        BbcnOqpskc3::new()
            .with_nsfd(c.nsfd.unwrap_or_else(|| default.nsfd()))
            .with_hrleg(c.hrleg.unwrap_or_else(|| default.hrleg()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BbcnOqpskphrtxConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leg: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mod_: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rb0: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ppdut: Option<bool>,
}

impl From<&BbcnOqpskphrtx> for BbcnOqpskphrtxConfig {
    fn from(r: &BbcnOqpskphrtx) -> Self {
        let default = BbcnOqpskphrtx::new();
        Self {
            leg: (r.leg() != default.leg()).then(|| r.leg()),
            mod_: (r.mod_() != default.mod_()).then(|| r.mod_()),
            rb0: (r.rb0() != default.rb0()).then(|| r.rb0()),
            ppdut: (r.ppdut() != default.ppdut()).then(|| r.ppdut()),
        }
    }
}

impl From<&BbcnOqpskphrtxConfig> for BbcnOqpskphrtx {
    fn from(c: &BbcnOqpskphrtxConfig) -> Self {
        let default = BbcnOqpskphrtx::new();
        BbcnOqpskphrtx::new()
            .with_leg(c.leg.unwrap_or_else(|| default.leg()))
            .with_mod_(c.mod_.unwrap_or_else(|| default.mod_()))
            .with_rb0(c.rb0.unwrap_or_else(|| default.rb0()))
            .with_ppdut(c.ppdut.unwrap_or_else(|| default.ppdut()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BbcnAfc0Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub afen0: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub afen1: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub afen2: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub afen3: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pm: Option<bool>,
}

impl From<&BbcnAfc0> for BbcnAfc0Config {
    fn from(r: &BbcnAfc0) -> Self {
        let default = BbcnAfc0::new();
        Self {
            afen0: (r.afen0() != default.afen0()).then(|| r.afen0()),
            afen1: (r.afen1() != default.afen1()).then(|| r.afen1()),
            afen2: (r.afen2() != default.afen2()).then(|| r.afen2()),
            afen3: (r.afen3() != default.afen3()).then(|| r.afen3()),
            pm: (r.pm() != default.pm()).then(|| r.pm()),
        }
    }
}

impl From<&BbcnAfc0Config> for BbcnAfc0 {
    fn from(c: &BbcnAfc0Config) -> Self {
        let default = BbcnAfc0::new();
        BbcnAfc0::new()
            .with_afen0(c.afen0.unwrap_or_else(|| default.afen0()))
            .with_afen1(c.afen1.unwrap_or_else(|| default.afen1()))
            .with_afen2(c.afen2.unwrap_or_else(|| default.afen2()))
            .with_afen3(c.afen3.unwrap_or_else(|| default.afen3()))
            .with_pm(c.pm.unwrap_or_else(|| default.pm()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BbcnAfc1Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub panc: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mrft: Option<u8>,
}

impl From<&BbcnAfc1> for BbcnAfc1Config {
    fn from(r: &BbcnAfc1) -> Self {
        let default = BbcnAfc1::new();
        Self {
            panc: (r.panc() != default.panc()).then(|| r.panc()),
            mrft: (r.mrft() != default.mrft()).then(|| r.mrft()),
        }
    }
}

impl From<&BbcnAfc1Config> for BbcnAfc1 {
    fn from(c: &BbcnAfc1Config) -> Self {
        let default = BbcnAfc1::new();
        BbcnAfc1::new()
            .with_panc(c.panc.unwrap_or_else(|| default.panc()))
            .with_mrft(c.mrft.unwrap_or_else(|| default.mrft()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BbcnAfftmConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub afftm: Option<u8>,
}

impl From<&BbcnAfftm> for BbcnAfftmConfig {
    fn from(r: &BbcnAfftm) -> Self {
        let default = BbcnAfftm::new();
        Self {
            afftm: (r.afftm() != default.afftm()).then(|| r.afftm()),
        }
    }
}

impl From<&BbcnAfftmConfig> for BbcnAfftm {
    fn from(c: &BbcnAfftmConfig) -> Self {
        let default = BbcnAfftm::new();
        BbcnAfftm::new().with_afftm(c.afftm.unwrap_or_else(|| default.afftm()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BbcnAffvmConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub affvm: Option<u8>,
}

impl From<&BbcnAffvm> for BbcnAffvmConfig {
    fn from(r: &BbcnAffvm) -> Self {
        let default = BbcnAffvm::new();
        Self {
            affvm: (r.affvm() != default.affvm()).then(|| r.affvm()),
        }
    }
}

impl From<&BbcnAffvmConfig> for BbcnAffvm {
    fn from(c: &BbcnAffvmConfig) -> Self {
        let default = BbcnAffvm::new();
        BbcnAffvm::new().with_affvm(c.affvm.unwrap_or_else(|| default.affvm()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BbcnMaceaConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub macea: Option<u64>,
}

impl From<&BbcnMacea> for BbcnMaceaConfig {
    fn from(r: &BbcnMacea) -> Self {
        let default = BbcnMacea::new();
        Self {
            macea: (r.macea() != default.macea()).then(|| r.macea()),
        }
    }
}

impl From<&BbcnMaceaConfig> for BbcnMacea {
    fn from(c: &BbcnMaceaConfig) -> Self {
        let default = BbcnMacea::new();
        BbcnMacea::new().with_macea(c.macea.unwrap_or_else(|| default.macea()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BbcnMacpidConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub macpid: Option<u16>,
}

impl From<&BbcnMacpid> for BbcnMacpidConfig {
    fn from(r: &BbcnMacpid) -> Self {
        let default = BbcnMacpid::new();
        Self {
            macpid: (r.macpid() != default.macpid()).then(|| r.macpid()),
        }
    }
}

impl From<&BbcnMacpidConfig> for BbcnMacpid {
    fn from(c: &BbcnMacpidConfig) -> Self {
        let default = BbcnMacpid::new();
        BbcnMacpid::new().with_macpid(c.macpid.unwrap_or_else(|| default.macpid()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BbcnMacshaConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub macsha: Option<u16>,
}

impl From<&BbcnMacsha> for BbcnMacshaConfig {
    fn from(r: &BbcnMacsha) -> Self {
        let default = BbcnMacsha::new();
        Self {
            macsha: (r.macsha() != default.macsha()).then(|| r.macsha()),
        }
    }
}

impl From<&BbcnMacshaConfig> for BbcnMacsha {
    fn from(c: &BbcnMacshaConfig) -> Self {
        let default = BbcnMacsha::new();
        BbcnMacsha::new().with_macsha(c.macsha.unwrap_or_else(|| default.macsha()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BbcnAmcsConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx2rx: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ccatx: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ccaed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aack: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aacks: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aackdr: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aackfa: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aackft: Option<bool>,
}

impl From<&BbcnAmcs> for BbcnAmcsConfig {
    fn from(r: &BbcnAmcs) -> Self {
        let default = BbcnAmcs::new();
        Self {
            tx2rx: (r.tx2rx() != default.tx2rx()).then(|| r.tx2rx()),
            ccatx: (r.ccatx() != default.ccatx()).then(|| r.ccatx()),
            ccaed: (r.ccaed() != default.ccaed()).then(|| r.ccaed()),
            aack: (r.aack() != default.aack()).then(|| r.aack()),
            aacks: (r.aacks() != default.aacks()).then(|| r.aacks()),
            aackdr: (r.aackdr() != default.aackdr()).then(|| r.aackdr()),
            aackfa: (r.aackfa() != default.aackfa()).then(|| r.aackfa()),
            aackft: (r.aackft() != default.aackft()).then(|| r.aackft()),
        }
    }
}

impl From<&BbcnAmcsConfig> for BbcnAmcs {
    fn from(c: &BbcnAmcsConfig) -> Self {
        let default = BbcnAmcs::new();
        BbcnAmcs::new()
            .with_tx2rx(c.tx2rx.unwrap_or_else(|| default.tx2rx()))
            .with_ccatx(c.ccatx.unwrap_or_else(|| default.ccatx()))
            .with_ccaed(c.ccaed.unwrap_or_else(|| default.ccaed()))
            .with_aack(c.aack.unwrap_or_else(|| default.aack()))
            .with_aacks(c.aacks.unwrap_or_else(|| default.aacks()))
            .with_aackdr(c.aackdr.unwrap_or_else(|| default.aackdr()))
            .with_aackfa(c.aackfa.unwrap_or_else(|| default.aackfa()))
            .with_aackft(c.aackft.unwrap_or_else(|| default.aackft()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BbcnAmedtConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amedt: Option<u8>,
}

impl From<&BbcnAmedt> for BbcnAmedtConfig {
    fn from(r: &BbcnAmedt) -> Self {
        let default = BbcnAmedt::new();
        Self {
            amedt: (r.amedt() != default.amedt()).then(|| r.amedt()),
        }
    }
}

impl From<&BbcnAmedtConfig> for BbcnAmedt {
    fn from(c: &BbcnAmedtConfig) -> Self {
        let default = BbcnAmedt::new();
        BbcnAmedt::new().with_amedt(c.amedt.unwrap_or_else(|| default.amedt()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BbcnAmaackpdConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pd0: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pd1: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pd2: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pd3: Option<bool>,
}

impl From<&BbcnAmaackpd> for BbcnAmaackpdConfig {
    fn from(r: &BbcnAmaackpd) -> Self {
        let default = BbcnAmaackpd::new();
        Self {
            pd0: (r.pd0() != default.pd0()).then(|| r.pd0()),
            pd1: (r.pd1() != default.pd1()).then(|| r.pd1()),
            pd2: (r.pd2() != default.pd2()).then(|| r.pd2()),
            pd3: (r.pd3() != default.pd3()).then(|| r.pd3()),
        }
    }
}

impl From<&BbcnAmaackpdConfig> for BbcnAmaackpd {
    fn from(c: &BbcnAmaackpdConfig) -> Self {
        let default = BbcnAmaackpd::new();
        BbcnAmaackpd::new()
            .with_pd0(c.pd0.unwrap_or_else(|| default.pd0()))
            .with_pd1(c.pd1.unwrap_or_else(|| default.pd1()))
            .with_pd2(c.pd2.unwrap_or_else(|| default.pd2()))
            .with_pd3(c.pd3.unwrap_or_else(|| default.pd3()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BbcnAmaacktConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amaackt: Option<u16>,
}

impl From<&BbcnAmaackt> for BbcnAmaacktConfig {
    fn from(r: &BbcnAmaackt) -> Self {
        let default = BbcnAmaackt::new();
        Self {
            amaackt: (r.amaackt() != default.amaackt()).then(|| r.amaackt()),
        }
    }
}

impl From<&BbcnAmaacktConfig> for BbcnAmaackt {
    fn from(c: &BbcnAmaacktConfig) -> Self {
        let default = BbcnAmaackt::new();
        BbcnAmaackt::new().with_amaackt(c.amaackt.unwrap_or_else(|| default.amaackt()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BbcnFskc0Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mord: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub midx: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub midxs: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bt: Option<u8>,
}

impl From<&BbcnFskc0> for BbcnFskc0Config {
    fn from(r: &BbcnFskc0) -> Self {
        let default = BbcnFskc0::new();
        Self {
            mord: (r.mord() != default.mord()).then(|| r.mord()),
            midx: (r.midx() != default.midx()).then(|| r.midx()),
            midxs: (r.midxs() != default.midxs()).then(|| r.midxs()),
            bt: (r.bt() != default.bt()).then(|| r.bt()),
        }
    }
}

impl From<&BbcnFskc0Config> for BbcnFskc0 {
    fn from(c: &BbcnFskc0Config) -> Self {
        let default = BbcnFskc0::new();
        BbcnFskc0::new()
            .with_mord(c.mord.unwrap_or_else(|| default.mord()))
            .with_midx(c.midx.unwrap_or_else(|| default.midx()))
            .with_midxs(c.midxs.unwrap_or_else(|| default.midxs()))
            .with_bt(c.bt.unwrap_or_else(|| default.bt()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BbcnFskc1Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub srate: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fi: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fskplh: Option<u8>,
}

impl From<&BbcnFskc1> for BbcnFskc1Config {
    fn from(r: &BbcnFskc1) -> Self {
        let default = BbcnFskc1::new();
        Self {
            srate: (r.srate() != default.srate()).then(|| r.srate()),
            fi: (r.fi() != default.fi()).then(|| r.fi()),
            fskplh: (r.fskplh() != default.fskplh()).then(|| r.fskplh()),
        }
    }
}

impl From<&BbcnFskc1Config> for BbcnFskc1 {
    fn from(c: &BbcnFskc1Config) -> Self {
        let default = BbcnFskc1::new();
        BbcnFskc1::new()
            .with_srate(c.srate.unwrap_or_else(|| default.srate()))
            .with_fi(c.fi.unwrap_or_else(|| default.fi()))
            .with_fskplh(c.fskplh.unwrap_or_else(|| default.fskplh()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BbcnFskc2Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fecie: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fecs: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pri: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mse: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rxpto: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rxo: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdtm: Option<bool>,
}

impl From<&BbcnFskc2> for BbcnFskc2Config {
    fn from(r: &BbcnFskc2) -> Self {
        let default = BbcnFskc2::new();
        Self {
            fecie: (r.fecie() != default.fecie()).then(|| r.fecie()),
            fecs: (r.fecs() != default.fecs()).then(|| r.fecs()),
            pri: (r.pri() != default.pri()).then(|| r.pri()),
            mse: (r.mse() != default.mse()).then(|| r.mse()),
            rxpto: (r.rxpto() != default.rxpto()).then(|| r.rxpto()),
            rxo: (r.rxo() != default.rxo()).then(|| r.rxo()),
            pdtm: (r.pdtm() != default.pdtm()).then(|| r.pdtm()),
        }
    }
}

impl From<&BbcnFskc2Config> for BbcnFskc2 {
    fn from(c: &BbcnFskc2Config) -> Self {
        let default = BbcnFskc2::new();
        BbcnFskc2::new()
            .with_fecie(c.fecie.unwrap_or_else(|| default.fecie()))
            .with_fecs(c.fecs.unwrap_or_else(|| default.fecs()))
            .with_pri(c.pri.unwrap_or_else(|| default.pri()))
            .with_mse(c.mse.unwrap_or_else(|| default.mse()))
            .with_rxpto(c.rxpto.unwrap_or_else(|| default.rxpto()))
            .with_rxo(c.rxo.unwrap_or_else(|| default.rxo()))
            .with_pdtm(c.pdtm.unwrap_or_else(|| default.pdtm()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BbcnFskc3Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdt: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sfdt: Option<u8>,
}

impl From<&BbcnFskc3> for BbcnFskc3Config {
    fn from(r: &BbcnFskc3) -> Self {
        let default = BbcnFskc3::new();
        Self {
            pdt: (r.pdt() != default.pdt()).then(|| r.pdt()),
            sfdt: (r.sfdt() != default.sfdt()).then(|| r.sfdt()),
        }
    }
}

impl From<&BbcnFskc3Config> for BbcnFskc3 {
    fn from(c: &BbcnFskc3Config) -> Self {
        let default = BbcnFskc3::new();
        BbcnFskc3::new()
            .with_pdt(c.pdt.unwrap_or_else(|| default.pdt()))
            .with_sfdt(c.sfdt.unwrap_or_else(|| default.sfdt()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BbcnFskc4Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub csfd0: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub csfd1: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rawrbit: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sfd32: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sfdq: Option<bool>,
}

impl From<&BbcnFskc4> for BbcnFskc4Config {
    fn from(r: &BbcnFskc4) -> Self {
        let default = BbcnFskc4::new();
        Self {
            csfd0: (r.csfd0() != default.csfd0()).then(|| r.csfd0()),
            csfd1: (r.csfd1() != default.csfd1()).then(|| r.csfd1()),
            rawrbit: (r.rawrbit() != default.rawrbit()).then(|| r.rawrbit()),
            sfd32: (r.sfd32() != default.sfd32()).then(|| r.sfd32()),
            sfdq: (r.sfdq() != default.sfdq()).then(|| r.sfdq()),
        }
    }
}

impl From<&BbcnFskc4Config> for BbcnFskc4 {
    fn from(c: &BbcnFskc4Config) -> Self {
        let default = BbcnFskc4::new();
        BbcnFskc4::new()
            .with_csfd0(c.csfd0.unwrap_or_else(|| default.csfd0()))
            .with_csfd1(c.csfd1.unwrap_or_else(|| default.csfd1()))
            .with_rawrbit(c.rawrbit.unwrap_or_else(|| default.rawrbit()))
            .with_sfd32(c.sfd32.unwrap_or_else(|| default.sfd32()))
            .with_sfdq(c.sfdq.unwrap_or_else(|| default.sfdq()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BbcnFskpllConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fskpll: Option<u8>,
}

impl From<&BbcnFskpll> for BbcnFskpllConfig {
    fn from(r: &BbcnFskpll) -> Self {
        let default = BbcnFskpll::new();
        Self {
            fskpll: (r.fskpll() != default.fskpll()).then(|| r.fskpll()),
        }
    }
}

impl From<&BbcnFskpllConfig> for BbcnFskpll {
    fn from(c: &BbcnFskpllConfig) -> Self {
        let default = BbcnFskpll::new();
        BbcnFskpll::new().with_fskpll(c.fskpll.unwrap_or_else(|| default.fskpll()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BbcnFsksfdConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fsksfd: Option<u16>,
}

impl From<&BbcnFsksfd> for BbcnFsksfdConfig {
    fn from(r: &BbcnFsksfd) -> Self {
        let default = BbcnFsksfd::new();
        Self {
            fsksfd: (r.fsksfd() != default.fsksfd()).then(|| r.fsksfd()),
        }
    }
}

impl From<&BbcnFsksfdConfig> for BbcnFsksfd {
    fn from(c: &BbcnFsksfdConfig) -> Self {
        let default = BbcnFsksfd::new();
        BbcnFsksfd::new().with_fsksfd(c.fsksfd.unwrap_or_else(|| default.fsksfd()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BbcnFskphrtxConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rb1: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rb2: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dw: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sfd: Option<bool>,
}

impl From<&BbcnFskphrtx> for BbcnFskphrtxConfig {
    fn from(r: &BbcnFskphrtx) -> Self {
        let default = BbcnFskphrtx::new();
        Self {
            rb1: (r.rb1() != default.rb1()).then(|| r.rb1()),
            rb2: (r.rb2() != default.rb2()).then(|| r.rb2()),
            dw: (r.dw() != default.dw()).then(|| r.dw()),
            sfd: (r.sfd() != default.sfd()).then(|| r.sfd()),
        }
    }
}

impl From<&BbcnFskphrtxConfig> for BbcnFskphrtx {
    fn from(c: &BbcnFskphrtxConfig) -> Self {
        let default = BbcnFskphrtx::new();
        BbcnFskphrtx::new()
            .with_rb1(c.rb1.unwrap_or_else(|| default.rb1()))
            .with_rb2(c.rb2.unwrap_or_else(|| default.rb2()))
            .with_dw(c.dw.unwrap_or_else(|| default.dw()))
            .with_sfd(c.sfd.unwrap_or_else(|| default.sfd()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BbcnFskrpcConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub baset: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub en: Option<bool>,
}

impl From<&BbcnFskrpc> for BbcnFskrpcConfig {
    fn from(r: &BbcnFskrpc) -> Self {
        let default = BbcnFskrpc::new();
        Self {
            baset: (r.baset() != default.baset()).then(|| r.baset()),
            en: (r.en() != default.en()).then(|| r.en()),
        }
    }
}

impl From<&BbcnFskrpcConfig> for BbcnFskrpc {
    fn from(c: &BbcnFskrpcConfig) -> Self {
        let default = BbcnFskrpc::new();
        BbcnFskrpc::new()
            .with_baset(c.baset.unwrap_or_else(|| default.baset()))
            .with_en(c.en.unwrap_or_else(|| default.en()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BbcnFskrpcontConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fskrpcont: Option<u8>,
}

impl From<&BbcnFskrpcont> for BbcnFskrpcontConfig {
    fn from(r: &BbcnFskrpcont) -> Self {
        let default = BbcnFskrpcont::new();
        Self {
            fskrpcont: (r.fskrpcont() != default.fskrpcont()).then(|| r.fskrpcont()),
        }
    }
}

impl From<&BbcnFskrpcontConfig> for BbcnFskrpcont {
    fn from(c: &BbcnFskrpcontConfig) -> Self {
        let default = BbcnFskrpcont::new();
        BbcnFskrpcont::new().with_fskrpcont(c.fskrpcont.unwrap_or_else(|| default.fskrpcont()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BbcnFskrpcofftConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fskrpcofft: Option<u8>,
}

impl From<&BbcnFskrpcofft> for BbcnFskrpcofftConfig {
    fn from(r: &BbcnFskrpcofft) -> Self {
        let default = BbcnFskrpcofft::new();
        Self {
            fskrpcofft: (r.fskrpcofft() != default.fskrpcofft()).then(|| r.fskrpcofft()),
        }
    }
}

impl From<&BbcnFskrpcofftConfig> for BbcnFskrpcofft {
    fn from(c: &BbcnFskrpcofftConfig) -> Self {
        let default = BbcnFskrpcofft::new();
        BbcnFskrpcofft::new().with_fskrpcofft(c.fskrpcofft.unwrap_or_else(|| default.fskrpcofft()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BbcnFskdmConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub en: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pe: Option<bool>,
}

impl From<&BbcnFskdm> for BbcnFskdmConfig {
    fn from(r: &BbcnFskdm) -> Self {
        let default = BbcnFskdm::new();
        Self {
            en: (r.en() != default.en()).then(|| r.en()),
            pe: (r.pe() != default.pe()).then(|| r.pe()),
        }
    }
}

impl From<&BbcnFskdmConfig> for BbcnFskdm {
    fn from(c: &BbcnFskdmConfig) -> Self {
        let default = BbcnFskdm::new();
        BbcnFskdm::new()
            .with_en(c.en.unwrap_or_else(|| default.en()))
            .with_pe(c.pe.unwrap_or_else(|| default.pe()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BbcnFskpeConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fskpe: Option<u8>,
}

impl From<&BbcnFskpe> for BbcnFskpeConfig {
    fn from(r: &BbcnFskpe) -> Self {
        let default = BbcnFskpe::new();
        Self {
            fskpe: (r.fskpe() != default.fskpe()).then(|| r.fskpe()),
        }
    }
}

impl From<&BbcnFskpeConfig> for BbcnFskpe {
    fn from(c: &BbcnFskpeConfig) -> Self {
        let default = BbcnFskpe::new();
        BbcnFskpe::new().with_fskpe(c.fskpe.unwrap_or_else(|| default.fskpe()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BbcnCntcConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub en: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rstrxs: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rsttxs: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caprxs: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub captxs: Option<bool>,
}

impl From<&BbcnCntc> for BbcnCntcConfig {
    fn from(r: &BbcnCntc) -> Self {
        let default = BbcnCntc::new();
        Self {
            en: (r.en() != default.en()).then(|| r.en()),
            rstrxs: (r.rstrxs() != default.rstrxs()).then(|| r.rstrxs()),
            rsttxs: (r.rsttxs() != default.rsttxs()).then(|| r.rsttxs()),
            caprxs: (r.caprxs() != default.caprxs()).then(|| r.caprxs()),
            captxs: (r.captxs() != default.captxs()).then(|| r.captxs()),
        }
    }
}

impl From<&BbcnCntcConfig> for BbcnCntc {
    fn from(c: &BbcnCntcConfig) -> Self {
        let default = BbcnCntc::new();
        BbcnCntc::new()
            .with_en(c.en.unwrap_or_else(|| default.en()))
            .with_rstrxs(c.rstrxs.unwrap_or_else(|| default.rstrxs()))
            .with_rsttxs(c.rsttxs.unwrap_or_else(|| default.rsttxs()))
            .with_caprxs(c.caprxs.unwrap_or_else(|| default.caprxs()))
            .with_captxs(c.captxs.unwrap_or_else(|| default.captxs()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BbcnIrqmConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rxfs: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rxfe: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rxam: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rxem: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub txfe: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agch: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agcr: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fbli: Option<bool>,
}

impl From<&BbcnIrqm> for BbcnIrqmConfig {
    fn from(r: &BbcnIrqm) -> Self {
        let default = BbcnIrqm::new();
        Self {
            rxfs: (r.rxfs() != default.rxfs()).then(|| r.rxfs()),
            rxfe: (r.rxfe() != default.rxfe()).then(|| r.rxfe()),
            rxam: (r.rxam() != default.rxam()).then(|| r.rxam()),
            rxem: (r.rxem() != default.rxem()).then(|| r.rxem()),
            txfe: (r.txfe() != default.txfe()).then(|| r.txfe()),
            agch: (r.agch() != default.agch()).then(|| r.agch()),
            agcr: (r.agcr() != default.agcr()).then(|| r.agcr()),
            fbli: (r.fbli() != default.fbli()).then(|| r.fbli()),
        }
    }
}

impl From<&BbcnIrqmConfig> for BbcnIrqm {
    fn from(c: &BbcnIrqmConfig) -> Self {
        let default = BbcnIrqm::new();
        BbcnIrqm::new()
            .with_rxfs(c.rxfs.unwrap_or_else(|| default.rxfs()))
            .with_rxfe(c.rxfe.unwrap_or_else(|| default.rxfe()))
            .with_rxam(c.rxam.unwrap_or_else(|| default.rxam()))
            .with_rxem(c.rxem.unwrap_or_else(|| default.rxem()))
            .with_txfe(c.txfe.unwrap_or_else(|| default.txfe()))
            .with_agch(c.agch.unwrap_or_else(|| default.agch()))
            .with_agcr(c.agcr.unwrap_or_else(|| default.agcr()))
            .with_fbli(c.fbli.unwrap_or_else(|| default.fbli()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BbcnPmucConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub en: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iqsel: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ccfts: Option<bool>,
}

impl From<&BbcnPmuc> for BbcnPmucConfig {
    fn from(r: &BbcnPmuc) -> Self {
        let default = BbcnPmuc::new();
        Self {
            en: (r.en() != default.en()).then(|| r.en()),
            avg: (r.avg() != default.avg()).then(|| r.avg()),
            fed: (r.fed() != default.fed()).then(|| r.fed()),
            iqsel: (r.iqsel() != default.iqsel()).then(|| r.iqsel()),
            ccfts: (r.ccfts() != default.ccfts()).then(|| r.ccfts()),
        }
    }
}

impl From<&BbcnPmucConfig> for BbcnPmuc {
    fn from(c: &BbcnPmucConfig) -> Self {
        let default = BbcnPmuc::new();
        BbcnPmuc::new()
            .with_en(c.en.unwrap_or_else(|| default.en()))
            .with_avg(c.avg.unwrap_or_else(|| default.avg()))
            .with_fed(c.fed.unwrap_or_else(|| default.fed()))
            .with_iqsel(c.iqsel.unwrap_or_else(|| default.iqsel()))
            .with_ccfts(c.ccfts.unwrap_or_else(|| default.ccfts()))
    }
}

#[cfg(test)]
mod net_config_tests {
    use super::*;

    // A config file that mixes register tables (consumed by RadioConfig) with
    // the daemon's [beacon]/[rssi] tables (consumed by NetConfig). The two
    // deserialize passes must each ignore the other's tables.
    const MIXED: &str = r#"
        [bbc0_pc]
        pt = 1

        [rf09_rxdfe]
        sr = 10

        [beacon]
        port = 10015
        enabled = true

        [rssi]
        peer = "127.0.0.1:10030"
        # removed from RssiConfig - must still parse (ignored)
        interval_ms = 500

        [spi]
        dev = "/dev/spidev1.0"
        hz = 5000000

        [gpio]
        chip = "/dev/gpiochip3"
        line = 30
    "#;

    #[test]
    fn net_pass_reads_beacon_and_rssi_ignoring_register_tables() {
        let net: NetConfig = toml::from_str(MIXED).unwrap();
        assert_eq!(net.beacon.port, Some(10015));
        assert_eq!(net.beacon.enabled, Some(true));
        assert_eq!(net.beacon.bind, None);
        assert_eq!(net.beacon.uds, None);
        assert_eq!(net.rssi.peer.as_deref(), Some("127.0.0.1:10030"));
        assert_eq!(net.rssi.enabled, None);
        assert_eq!(net.spi.dev.as_deref(), Some("/dev/spidev1.0"));
        assert_eq!(net.spi.hz, Some(5_000_000));
        assert_eq!(net.gpio.chip.as_deref(), Some("/dev/gpiochip3"));
        assert_eq!(net.gpio.line, Some(30));
    }

    #[test]
    fn register_pass_ignores_beacon_and_rssi_tables() {
        // The register pass on the same string must not error on [beacon]/[rssi]
        // and must still pick up the register fields.
        let radio: RadioConfig = toml::from_str(MIXED).unwrap();
        assert_eq!(radio.bbc0_pc.pt, Some(1));
        assert_eq!(radio.rf09_rxdfe.sr, Some(10));
    }

    #[test]
    fn register_only_config_yields_default_net() {
        let net: NetConfig = toml::from_str("[bbc0_pc]\npt = 1\n").unwrap();
        assert_eq!(net, NetConfig::default());
    }

    #[test]
    fn check_known_tables_accepts_a_mixed_config() {
        check_known_tables(MIXED).unwrap();
    }

    #[test]
    fn check_known_tables_rejects_a_typo() {
        // A single-letter typo in a register table name is the silent
        // deaf-radio failure this guards against.
        let err = check_known_tables("[rf09_rxdfee]\nsr = 10\n").unwrap_err();
        assert!(err.contains("rf09_rxdfee"), "message was: {err}");
    }

    #[test]
    fn known_tables_have_no_duplicates() {
        let mut seen = std::collections::HashSet::new();
        for t in KNOWN_TABLES {
            assert!(seen.insert(*t), "duplicate table in KNOWN_TABLES: {t}");
        }
    }

    // Every register table in KNOWN_TABLES must be a real field that
    // RadioConfig actually deserializes (and the two net tables must parse into
    // NetConfig).
    #[test]
    fn every_known_table_round_trips_into_its_config_struct() {
        for t in KNOWN_TABLES {
            let doc = format!("[{t}]\n");
            check_known_tables(&doc)
                .unwrap_or_else(|e| panic!("KNOWN_TABLES entry [{t}] rejected itself: {e}"));
            if matches!(*t, "beacon" | "rssi" | "spi" | "gpio") {
                toml::from_str::<NetConfig>(&doc)
                    .unwrap_or_else(|e| panic!("[{t}] is not a NetConfig table: {e}"));
            } else {
                // RadioConfig has no deny_unknown_fields, so an empty known table
                // always parses; this still proves the name is spelled the same
                // here as in the struct via the serialize round-trip below.
                toml::from_str::<RadioConfig>(&doc)
                    .unwrap_or_else(|e| panic!("[{t}] is not a RadioConfig table: {e}"));
            }
        }
        // Reverse direction: a fully-populated RadioConfig serializes to ONLY
        // table names present in KNOWN_TABLES (catches a register added to the
        // struct but forgotten here).
        let toml_str = toml::to_string(&RadioConfig::default()).unwrap();
        // Default serializes nothing (all skip), so also exercise to_config of a
        // touched radio via a representative non-default set.
        let touched = "[rf09_rxdfe]\nsr = 10\n[bbc0_pc]\npt = 1\n";
        let cfg: RadioConfig = toml::from_str(touched).unwrap();
        let ser = toml::to_string(&cfg).unwrap();
        for line in ser.lines().chain(toml_str.lines()) {
            let line = line.trim();
            if let Some(name) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                assert!(
                    KNOWN_TABLES.contains(&name),
                    "RadioConfig serializes table [{name}] missing from KNOWN_TABLES"
                );
            }
        }
    }
}
