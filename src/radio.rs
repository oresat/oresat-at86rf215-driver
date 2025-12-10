use crate::registers::*;

#[derive(Debug, Clone)]
pub struct Radio {
    // =========================================================================
    // Common Chip Registers (0x0000-0x000E)
    // =========================================================================

    // Read-only chip identification
    pub rf_pn: ReadOnly<RfPn, 0x000D, 1>,
    pub rf_vn: ReadOnly<RfVn, 0x000E, 1>,

    // Chip reset command
    pub rf_rst: WriteOnly<RfRst, 0x0005, 1>,

    // Configuration registers
    pub rf_cfg: ReadWrite<RfCfg, 0x0006, 1>,
    pub rf_clko: ReadWrite<RfClko, 0x0007, 1>,
    pub rf_bmdvc: ReadWrite<RfBmdvc, 0x0008, 1>,
    pub rf_xoc: ReadWrite<RfXoc, 0x0009, 1>,
    pub rf_iqifc0: ReadWrite<RfIqifc0, 0x000A, 1>,
    pub rf_iqifc1: ReadWrite<RfIqifc1, 0x000B, 1>,
    pub rf_iqifc2: ReadOnly<RfIqifc2, 0x000C, 1>,

    // =========================================================================
    // RF09 Sub-1GHz Radio Registers (0x0100-0x0128)
    // =========================================================================

    // Status and interrupts
    pub rf09_irqs: ReadOnly<RfnIrqs, 0x0100, 1>,
    pub rf09_irqm: ReadWrite<RfnIrqm, 0x0100, 1>,
    pub rf09_state: ReadOnly<RfnState, 0x0102, 1>,

    // Commands and control
    pub rf09_cmd: WriteOnly<RfnCmd, 0x0103, 1>,
    pub rf09_auxs: ReadWrite<RfnAuxs, 0x0101, 1>,

    // Channel configuration
    pub rf09_cs: ReadWrite<RfnCs, 0x0104, 1>,
    pub rf09_ccf0: ReadWrite<RfnCcf0, 0x0105, 2>,
    pub rf09_cn: ReadWrite<RfnCn, 0x0107, 2>,

    // Receiver configuration
    pub rf09_rxbwc: ReadWrite<RfnRxbwc, 0x0109, 1>,
    pub rf09_rxdfe: ReadWrite<RfnRxdfe, 0x010A, 1>,
    pub rf09_agcc: ReadWrite<RfnAgcc, 0x010B, 1>,
    pub rf09_agcs: ReadWrite<RfnAgcs, 0x010C, 1>,

    // Energy detection
    pub rf09_edc: ReadWrite<RfnEdc, 0x010E, 1>,
    pub rf09_edd: ReadWrite<RfnEdd, 0x010F, 1>,
    pub rf09_edv: ReadOnly<RfnEdv, 0x0110, 1>,

    // Random number
    pub rf09_rndv: ReadOnly<RfnRndv, 0x0111, 1>,

    // Transmitter configuration
    pub rf09_txcutc: ReadWrite<RfnTxcutc, 0x0112, 1>,
    pub rf09_txdfe: ReadWrite<RfnTxdfe, 0x0113, 1>,
    pub rf09_pac: ReadWrite<RfnPac, 0x0114, 1>,
    pub rf09_padfe: ReadWrite<RfnPadfe, 0x0116, 1>,

    // PLL configuration
    pub rf09_pll: ReadWrite<RfnPll, 0x0121, 1>,
    pub rf09_pllcf: ReadOnly<RfnPllcf, 0x0122, 1>,

    // TX calibration
    pub rf09_txci: ReadWrite<RfnTxci, 0x0125, 1>,
    pub rf09_txcq: ReadWrite<RfnTxcq, 0x0126, 1>,
    pub rf09_txdaci: ReadWrite<RfnTxdaci, 0x0127, 1>,
    pub rf09_txdacq: ReadWrite<RfnTxdacq, 0x0128, 1>,

    // =========================================================================
    // RF24 2.4GHz Radio Registers (0x0200-0x0228)
    // =========================================================================

    // Status and interrupts
    pub rf24_irqs: ReadOnly<RfnIrqs, 0x0200, 1>,
    pub rf24_irqm: ReadWrite<RfnIrqm, 0x0200, 1>,
    pub rf24_state: ReadOnly<RfnState, 0x0202, 1>,

    // Commands and control
    pub rf24_cmd: WriteOnly<RfnCmd, 0x0203, 1>,
    pub rf24_auxs: ReadWrite<RfnAuxs, 0x0201, 1>,

    // Channel configuration
    pub rf24_cs: ReadWrite<RfnCs, 0x0204, 1>,
    pub rf24_ccf0: ReadWrite<RfnCcf0, 0x0205, 2>,
    pub rf24_cn: ReadWrite<RfnCn, 0x0207, 2>,

    // Receiver configuration
    pub rf24_rxbwc: ReadWrite<RfnRxbwc, 0x0209, 1>,
    pub rf24_rxdfe: ReadWrite<RfnRxdfe, 0x020A, 1>,
    pub rf24_agcc: ReadWrite<RfnAgcc, 0x020B, 1>,
    pub rf24_agcs: ReadWrite<RfnAgcs, 0x020C, 1>,

    // Energy detection
    pub rf24_edc: ReadWrite<RfnEdc, 0x020E, 1>,
    pub rf24_edd: ReadWrite<RfnEdd, 0x020F, 1>,
    pub rf24_edv: ReadOnly<RfnEdv, 0x0210, 1>,

    // Random number
    pub rf24_rndv: ReadOnly<RfnRndv, 0x0211, 1>,

    // Transmitter configuration
    pub rf24_txcutc: ReadWrite<RfnTxcutc, 0x0212, 1>,
    pub rf24_txdfe: ReadWrite<RfnTxdfe, 0x0213, 1>,
    pub rf24_pac: ReadWrite<RfnPac, 0x0214, 1>,
    pub rf24_padfe: ReadWrite<RfnPadfe, 0x0216, 1>,

    // PLL configuration
    pub rf24_pll: ReadWrite<RfnPll, 0x0221, 1>,
    pub rf24_pllcf: ReadOnly<RfnPllcf, 0x0222, 1>,

    // TX calibration
    pub rf24_txci: ReadWrite<RfnTxci, 0x0225, 1>,
    pub rf24_txcq: ReadWrite<RfnTxcq, 0x0226, 1>,
    pub rf24_txdaci: ReadWrite<RfnTxdaci, 0x0227, 1>,
    pub rf24_txdacq: ReadWrite<RfnTxdacq, 0x0228, 1>,

    // =========================================================================
    // BBC0 Sub-1GHz Baseband Registers (0x0300-0x0394)
    // =========================================================================

    // Status and interrupts
    pub bbc0_irqs: ReadOnly<BbcnIrqs, 0x0302, 1>,
    pub bbc0_irqm: ReadWrite<BbcnIrqm, 0x0300, 1>,
    pub bbc0_ps: ReadOnly<BbcnPs, 0x0302, 1>,

    // PHY control
    pub bbc0_pc: ReadWrite<BbcnPc, 0x0301, 1>,

    // Frame length registers
    pub bbc0_rxfl: ReadOnly<BbcnRxfl, 0x0304, 2>,
    pub bbc0_txfl: ReadWrite<BbcnTxfl, 0x0306, 2>,
    pub bbc0_fbl: ReadOnly<BbcnFbl, 0x0308, 2>,
    pub bbc0_fbli: ReadWrite<BbcnFbli, 0x030A, 2>,

    // OFDM PHY configuration
    pub bbc0_ofdmphrtx: ReadWrite<BbcnOfdmphrtx, 0x030C, 1>,
    pub bbc0_ofdmphrrx: ReadOnly<BbcnOfdmphrrx, 0x030D, 1>,
    pub bbc0_ofdmc: ReadWrite<BbcnOfdmc, 0x030E, 1>,
    pub bbc0_ofdmsw: ReadWrite<BbcnOfdmsw, 0x030F, 1>,

    // O-QPSK PHY configuration
    pub bbc0_oqpskc0: ReadWrite<BbcnOqpskc0, 0x0310, 1>,
    pub bbc0_oqpskc1: ReadWrite<BbcnOqpskc1, 0x0311, 1>,
    pub bbc0_oqpskc2: ReadWrite<BbcnOqpskc2, 0x0312, 1>,
    pub bbc0_oqpskc3: ReadWrite<BbcnOqpskc3, 0x0313, 1>,
    pub bbc0_oqpskphrtx: ReadWrite<BbcnOqpskphrtx, 0x0314, 1>,
    pub bbc0_oqpskphrrx: ReadOnly<BbcnOqpskphrrx, 0x0315, 1>,

    // Address filter configuration
    pub bbc0_afc0: ReadWrite<BbcnAfc0, 0x0320, 1>,
    pub bbc0_afc1: ReadWrite<BbcnAfc1, 0x0321, 1>,
    pub bbc0_afftm: ReadWrite<BbcnAfftm, 0x0322, 1>,
    pub bbc0_affvm: ReadWrite<BbcnAffvm, 0x0323, 1>,
    pub bbc0_afs: ReadOnly<BbcnAfs, 0x0324, 1>,

    // MAC extended address (8 bytes)
    pub bbc0_macea: ReadWrite<BbcnMacea, 0x0325, 8>,

    // MAC PAN ID and short address for filter 0-3
    pub bbc0_macpidf0: ReadWrite<BbcnMacpid, 0x032D, 2>,
    pub bbc0_macshaf0: ReadWrite<BbcnMacsha, 0x032F, 2>,

    pub bbc0_macpidf1: ReadWrite<BbcnMacpid, 0x0331, 2>,
    pub bbc0_macshaf1: ReadWrite<BbcnMacsha, 0x0333, 2>,

    pub bbc0_macpidf2: ReadWrite<BbcnMacpid, 0x0335, 2>,
    pub bbc0_macshaf2: ReadWrite<BbcnMacsha, 0x0337, 2>,

    pub bbc0_macpidf3: ReadWrite<BbcnMacpid, 0x0339, 2>,
    pub bbc0_macshaf3: ReadWrite<BbcnMacsha, 0x033B, 2>,

    // Auto mode configuration
    pub bbc0_amcs: ReadWrite<BbcnAmcs, 0x0340, 1>,
    pub bbc0_amedt: ReadWrite<BbcnAmedt, 0x0341, 1>,
    pub bbc0_amaackpd: ReadWrite<BbcnAmaackpd, 0x0342, 1>,
    pub bbc0_amaackt: ReadWrite<BbcnAmaackt, 0x0343, 2>,

    // FSK PHY configuration
    pub bbc0_fskc0: ReadWrite<BbcnFskc0, 0x0360, 1>,
    pub bbc0_fskc1: ReadWrite<BbcnFskc1, 0x0361, 1>,
    pub bbc0_fskc2: ReadWrite<BbcnFskc2, 0x0362, 1>,
    pub bbc0_fskc3: ReadWrite<BbcnFskc3, 0x0363, 1>,
    pub bbc0_fskc4: ReadWrite<BbcnFskc4, 0x0364, 1>,
    pub bbc0_fskpll: ReadWrite<BbcnFskpll, 0x0365, 1>,
    pub bbc0_fsksfd0: ReadWrite<BbcnFsksfd, 0x0366, 2>,
    pub bbc0_fsksfd1: ReadWrite<BbcnFsksfd, 0x0368, 2>,
    pub bbc0_fskphrtx: ReadWrite<BbcnFskphrtx, 0x036A, 1>,
    pub bbc0_fskphrrx: ReadOnly<BbcnFskphrrx, 0x036B, 1>,
    pub bbc0_fskrpc: ReadWrite<BbcnFskrpc, 0x036C, 1>,
    pub bbc0_fskrpcont: ReadWrite<BbcnFskrpcont, 0x036D, 1>,
    pub bbc0_fskrpcofft: ReadWrite<BbcnFskrpcofft, 0x036E, 1>,
    pub bbc0_fskrrxfl: ReadOnly<BbcnFskrrxfl, 0x0370, 2>,
    pub bbc0_fskdm: ReadWrite<BbcnFskdm, 0x0372, 1>,
    pub bbc0_fskpe0: ReadWrite<BbcnFskpe, 0x0373, 1>,
    pub bbc0_fskpe1: ReadWrite<BbcnFskpe, 0x0374, 1>,
    pub bbc0_fskpe2: ReadWrite<BbcnFskpe, 0x0375, 1>,

    // Phase measurement unit
    pub bbc0_pmuc: ReadWrite<BbcnPmuc, 0x0380, 1>,
    pub bbc0_pmuval: ReadOnly<BbcnPmuval, 0x0381, 1>,
    pub bbc0_pmuqf: ReadOnly<BbcnPmuqf, 0x0382, 1>,
    pub bbc0_pmui: ReadOnly<BbcnPmui, 0x0383, 1>,
    pub bbc0_pmuq: ReadOnly<BbcnPmuq, 0x0384, 1>,

    // Timestamp counter
    pub bbc0_cntc: ReadWrite<BbcnCntc, 0x0390, 1>,
    pub bbc0_cnt: ReadOnly<BbcnCnt, 0x0391, 4>,

    // =========================================================================
    // BBC1 2.4GHz Baseband Registers (0x0400-0x0494)
    // =========================================================================

    // Status and interrupts
    pub bbc1_irqs: ReadOnly<BbcnIrqs, 0x0402, 1>,
    pub bbc1_irqm: ReadWrite<BbcnIrqm, 0x0400, 1>,
    pub bbc1_ps: ReadOnly<BbcnPs, 0x0402, 1>,

    // PHY control
    pub bbc1_pc: ReadWrite<BbcnPc, 0x0401, 1>,

    // Frame length registers
    pub bbc1_rxfl: ReadOnly<BbcnRxfl, 0x0404, 2>,
    pub bbc1_txfl: ReadWrite<BbcnTxfl, 0x0406, 2>,
    pub bbc1_fbl: ReadOnly<BbcnFbl, 0x0408, 2>,
    pub bbc1_fbli: ReadWrite<BbcnFbli, 0x040A, 2>,

    // OFDM PHY configuration
    pub bbc1_ofdmphrtx: ReadWrite<BbcnOfdmphrtx, 0x040C, 1>,
    pub bbc1_ofdmphrrx: ReadOnly<BbcnOfdmphrrx, 0x040D, 1>,
    pub bbc1_ofdmc: ReadWrite<BbcnOfdmc, 0x040E, 1>,
    pub bbc1_ofdmsw: ReadWrite<BbcnOfdmsw, 0x040F, 1>,

    // O-QPSK PHY configuration
    pub bbc1_oqpskc0: ReadWrite<BbcnOqpskc0, 0x0410, 1>,
    pub bbc1_oqpskc1: ReadWrite<BbcnOqpskc1, 0x0411, 1>,
    pub bbc1_oqpskc2: ReadWrite<BbcnOqpskc2, 0x0412, 1>,
    pub bbc1_oqpskc3: ReadWrite<BbcnOqpskc3, 0x0413, 1>,
    pub bbc1_oqpskphrtx: ReadWrite<BbcnOqpskphrtx, 0x0414, 1>,
    pub bbc1_oqpskphrrx: ReadOnly<BbcnOqpskphrrx, 0x0415, 1>,

    // Address filter configuration
    pub bbc1_afc0: ReadWrite<BbcnAfc0, 0x0420, 1>,
    pub bbc1_afc1: ReadWrite<BbcnAfc1, 0x0421, 1>,
    pub bbc1_afftm: ReadWrite<BbcnAfftm, 0x0422, 1>,
    pub bbc1_affvm: ReadWrite<BbcnAffvm, 0x0423, 1>,
    pub bbc1_afs: ReadOnly<BbcnAfs, 0x0424, 1>,

    // MAC extended address (8 bytes)
    pub bbc1_macea: ReadWrite<BbcnMacea, 0x0425, 8>,

    // MAC PAN ID and short address for filter 0-3
    pub bbc1_macpidf0: ReadWrite<BbcnMacpid, 0x042D, 2>,
    pub bbc1_macshaf0: ReadWrite<BbcnMacsha, 0x042F, 2>,

    pub bbc1_macpidf1: ReadWrite<BbcnMacpid, 0x0431, 2>,
    pub bbc1_macshaf1: ReadWrite<BbcnMacsha, 0x0433, 2>,

    pub bbc1_macpidf2: ReadWrite<BbcnMacpid, 0x0435, 2>,
    pub bbc1_macshaf2: ReadWrite<BbcnMacsha, 0x0437, 2>,

    pub bbc1_macpidf3: ReadWrite<BbcnMacpid, 0x0439, 2>,
    pub bbc1_macshaf3: ReadWrite<BbcnMacsha, 0x043B, 2>,

    // Auto mode configuration
    pub bbc1_amcs: ReadWrite<BbcnAmcs, 0x0440, 1>,
    pub bbc1_amedt: ReadWrite<BbcnAmedt, 0x0441, 1>,
    pub bbc1_amaackpd: ReadWrite<BbcnAmaackpd, 0x0442, 1>,
    pub bbc1_amaackt: ReadWrite<BbcnAmaackt, 0x0443, 2>,

    // FSK PHY configuration
    pub bbc1_fskc0: ReadWrite<BbcnFskc0, 0x0460, 1>,
    pub bbc1_fskc1: ReadWrite<BbcnFskc1, 0x0461, 1>,
    pub bbc1_fskc2: ReadWrite<BbcnFskc2, 0x0462, 1>,
    pub bbc1_fskc3: ReadWrite<BbcnFskc3, 0x0463, 1>,
    pub bbc1_fskc4: ReadWrite<BbcnFskc4, 0x0464, 1>,
    pub bbc1_fskpll: ReadWrite<BbcnFskpll, 0x0465, 1>,
    pub bbc1_fsksfd0: ReadWrite<BbcnFsksfd, 0x0466, 2>,
    pub bbc1_fsksfd1: ReadWrite<BbcnFsksfd, 0x0468, 2>,
    pub bbc1_fskphrtx: ReadWrite<BbcnFskphrtx, 0x046A, 1>,
    pub bbc1_fskphrrx: ReadOnly<BbcnFskphrrx, 0x046B, 1>,
    pub bbc1_fskrpc: ReadWrite<BbcnFskrpc, 0x046C, 1>,
    pub bbc1_fskrpcont: ReadWrite<BbcnFskrpcont, 0x046D, 1>,
    pub bbc1_fskrpcofft: ReadWrite<BbcnFskrpcofft, 0x046E, 1>,
    pub bbc1_fskrrxfl: ReadOnly<BbcnFskrrxfl, 0x0470, 2>,
    pub bbc1_fskdm: ReadWrite<BbcnFskdm, 0x0472, 1>,
    pub bbc1_fskpe0: ReadWrite<BbcnFskpe, 0x0473, 1>,
    pub bbc1_fskpe1: ReadWrite<BbcnFskpe, 0x0474, 1>,
    pub bbc1_fskpe2: ReadWrite<BbcnFskpe, 0x0475, 1>,

    // Phase measurement unit
    pub bbc1_pmuc: ReadWrite<BbcnPmuc, 0x0480, 1>,
    pub bbc1_pmuval: ReadOnly<BbcnPmuval, 0x0481, 1>,
    pub bbc1_pmuqf: ReadOnly<BbcnPmuqf, 0x0482, 1>,
    pub bbc1_pmui: ReadOnly<BbcnPmui, 0x0483, 1>,
    pub bbc1_pmuq: ReadOnly<BbcnPmuq, 0x0484, 1>,

    // Timestamp counter
    pub bbc1_cntc: ReadWrite<BbcnCntc, 0x0490, 1>,
    pub bbc1_cnt: ReadOnly<BbcnCnt, 0x0491, 4>,
}

impl Default for Radio {
    fn default() -> Self {
        Self::new()
    }
}

impl Radio {
    /// Create a new Radio with all registers initialized to default values
    pub fn new() -> Self {
        Self {
            // Common chip registers
            rf_pn: ReadOnly::new(RfPn::new()),
            rf_vn: ReadOnly::new(RfVn::new()),
            rf_rst: WriteOnly::new(RfRst::new()),
            rf_cfg: ReadWrite::new(RfCfg::new()),
            rf_clko: ReadWrite::new(RfClko::new()),
            rf_bmdvc: ReadWrite::new(RfBmdvc::new()),
            rf_xoc: ReadWrite::new(RfXoc::new()),
            rf_iqifc0: ReadWrite::new(RfIqifc0::new()),
            rf_iqifc1: ReadWrite::new(RfIqifc1::new()),
            rf_iqifc2: ReadOnly::new(RfIqifc2::new()),

            // RF09 registers
            rf09_irqs: ReadOnly::new(RfnIrqs::new()),
            rf09_irqm: ReadWrite::new(RfnIrqm::new()),
            rf09_state: ReadOnly::new(RfnState::new()),
            rf09_cmd: WriteOnly::new(RfnCmd::new()),
            rf09_auxs: ReadWrite::new(RfnAuxs::new()),
            rf09_cs: ReadWrite::new(RfnCs::new()),
            rf09_ccf0: ReadWrite::new(RfnCcf0::new()),
            rf09_cn: ReadWrite::new(RfnCn::new()),
            rf09_rxbwc: ReadWrite::new(RfnRxbwc::new()),
            rf09_rxdfe: ReadWrite::new(RfnRxdfe::new()),
            rf09_agcc: ReadWrite::new(RfnAgcc::new()),
            rf09_agcs: ReadWrite::new(RfnAgcs::new()),
            rf09_edc: ReadWrite::new(RfnEdc::new()),
            rf09_edd: ReadWrite::new(RfnEdd::new()),
            rf09_edv: ReadOnly::new(RfnEdv::new()),
            rf09_rndv: ReadOnly::new(RfnRndv::new()),
            rf09_txcutc: ReadWrite::new(RfnTxcutc::new()),
            rf09_txdfe: ReadWrite::new(RfnTxdfe::new()),
            rf09_pac: ReadWrite::new(RfnPac::new()),
            rf09_padfe: ReadWrite::new(RfnPadfe::new()),
            rf09_pll: ReadWrite::new(RfnPll::new()),
            rf09_pllcf: ReadOnly::new(RfnPllcf::new()),
            rf09_txci: ReadWrite::new(RfnTxci::new()),
            rf09_txcq: ReadWrite::new(RfnTxcq::new()),
            rf09_txdaci: ReadWrite::new(RfnTxdaci::new()),
            rf09_txdacq: ReadWrite::new(RfnTxdacq::new()),

            // RF24 registers
            rf24_irqs: ReadOnly::new(RfnIrqs::new()),
            rf24_irqm: ReadWrite::new(RfnIrqm::new()),
            rf24_state: ReadOnly::new(RfnState::new()),
            rf24_cmd: WriteOnly::new(RfnCmd::new()),
            rf24_auxs: ReadWrite::new(RfnAuxs::new()),
            rf24_cs: ReadWrite::new(RfnCs::new()),
            rf24_ccf0: ReadWrite::new(RfnCcf0::new()),
            rf24_cn: ReadWrite::new(RfnCn::new()),
            rf24_rxbwc: ReadWrite::new(RfnRxbwc::new()),
            rf24_rxdfe: ReadWrite::new(RfnRxdfe::new()),
            rf24_agcc: ReadWrite::new(RfnAgcc::new()),
            rf24_agcs: ReadWrite::new(RfnAgcs::new()),
            rf24_edc: ReadWrite::new(RfnEdc::new()),
            rf24_edd: ReadWrite::new(RfnEdd::new()),
            rf24_edv: ReadOnly::new(RfnEdv::new()),
            rf24_rndv: ReadOnly::new(RfnRndv::new()),
            rf24_txcutc: ReadWrite::new(RfnTxcutc::new()),
            rf24_txdfe: ReadWrite::new(RfnTxdfe::new()),
            rf24_pac: ReadWrite::new(RfnPac::new()),
            rf24_padfe: ReadWrite::new(RfnPadfe::new()),
            rf24_pll: ReadWrite::new(RfnPll::new()),
            rf24_pllcf: ReadOnly::new(RfnPllcf::new()),
            rf24_txci: ReadWrite::new(RfnTxci::new()),
            rf24_txcq: ReadWrite::new(RfnTxcq::new()),
            rf24_txdaci: ReadWrite::new(RfnTxdaci::new()),
            rf24_txdacq: ReadWrite::new(RfnTxdacq::new()),

            // BBC0 registers
            bbc0_irqs: ReadOnly::new(BbcnIrqs::new()),
            bbc0_irqm: ReadWrite::new(BbcnIrqm::new()),
            bbc0_ps: ReadOnly::new(BbcnPs::new()),
            bbc0_pc: ReadWrite::new(BbcnPc::new()),
            bbc0_rxfl: ReadOnly::new(BbcnRxfl::new()),
            bbc0_txfl: ReadWrite::new(BbcnTxfl::new()),
            bbc0_fbl: ReadOnly::new(BbcnFbl::new()),
            bbc0_fbli: ReadWrite::new(BbcnFbli::new()),
            bbc0_ofdmphrtx: ReadWrite::new(BbcnOfdmphrtx::new()),
            bbc0_ofdmphrrx: ReadOnly::new(BbcnOfdmphrrx::new()),
            bbc0_ofdmc: ReadWrite::new(BbcnOfdmc::new()),
            bbc0_ofdmsw: ReadWrite::new(BbcnOfdmsw::new()),
            bbc0_oqpskc0: ReadWrite::new(BbcnOqpskc0::new()),
            bbc0_oqpskc1: ReadWrite::new(BbcnOqpskc1::new()),
            bbc0_oqpskc2: ReadWrite::new(BbcnOqpskc2::new()),
            bbc0_oqpskc3: ReadWrite::new(BbcnOqpskc3::new()),
            bbc0_oqpskphrtx: ReadWrite::new(BbcnOqpskphrtx::new()),
            bbc0_oqpskphrrx: ReadOnly::new(BbcnOqpskphrrx::new()),
            bbc0_afc0: ReadWrite::new(BbcnAfc0::new()),
            bbc0_afc1: ReadWrite::new(BbcnAfc1::new()),
            bbc0_afftm: ReadWrite::new(BbcnAfftm::new()),
            bbc0_affvm: ReadWrite::new(BbcnAffvm::new()),
            bbc0_afs: ReadOnly::new(BbcnAfs::new()),
            bbc0_macea: ReadWrite::new(BbcnMacea::new()),
            bbc0_macpidf0: ReadWrite::new(BbcnMacpid::new()),
            bbc0_macshaf0: ReadWrite::new(BbcnMacsha::new()),
            bbc0_macpidf1: ReadWrite::new(BbcnMacpid::new()),
            bbc0_macshaf1: ReadWrite::new(BbcnMacsha::new()),
            bbc0_macpidf2: ReadWrite::new(BbcnMacpid::new()),
            bbc0_macshaf2: ReadWrite::new(BbcnMacsha::new()),
            bbc0_macpidf3: ReadWrite::new(BbcnMacpid::new()),
            bbc0_macshaf3: ReadWrite::new(BbcnMacsha::new()),
            bbc0_amcs: ReadWrite::new(BbcnAmcs::new()),
            bbc0_amedt: ReadWrite::new(BbcnAmedt::new()),
            bbc0_amaackpd: ReadWrite::new(BbcnAmaackpd::new()),
            bbc0_amaackt: ReadWrite::new(BbcnAmaackt::new()),
            bbc0_fskc0: ReadWrite::new(BbcnFskc0::new()),
            bbc0_fskc1: ReadWrite::new(BbcnFskc1::new()),
            bbc0_fskc2: ReadWrite::new(BbcnFskc2::new()),
            bbc0_fskc3: ReadWrite::new(BbcnFskc3::new()),
            bbc0_fskc4: ReadWrite::new(BbcnFskc4::new()),
            bbc0_fskpll: ReadWrite::new(BbcnFskpll::new()),
            bbc0_fsksfd0: ReadWrite::new(BbcnFsksfd::new()),
            bbc0_fsksfd1: ReadWrite::new(BbcnFsksfd::new()),
            bbc0_fskphrtx: ReadWrite::new(BbcnFskphrtx::new()),
            bbc0_fskphrrx: ReadOnly::new(BbcnFskphrrx::new()),
            bbc0_fskrpc: ReadWrite::new(BbcnFskrpc::new()),
            bbc0_fskrpcont: ReadWrite::new(BbcnFskrpcont::new()),
            bbc0_fskrpcofft: ReadWrite::new(BbcnFskrpcofft::new()),
            bbc0_fskrrxfl: ReadOnly::new(BbcnFskrrxfl::new()),
            bbc0_fskdm: ReadWrite::new(BbcnFskdm::new()),
            bbc0_fskpe0: ReadWrite::new(BbcnFskpe::new()),
            bbc0_fskpe1: ReadWrite::new(BbcnFskpe::new()),
            bbc0_fskpe2: ReadWrite::new(BbcnFskpe::new()),
            bbc0_pmuc: ReadWrite::new(BbcnPmuc::new()),
            bbc0_pmuval: ReadOnly::new(BbcnPmuval::new()),
            bbc0_pmuqf: ReadOnly::new(BbcnPmuqf::new()),
            bbc0_pmui: ReadOnly::new(BbcnPmui::new()),
            bbc0_pmuq: ReadOnly::new(BbcnPmuq::new()),
            bbc0_cntc: ReadWrite::new(BbcnCntc::new()),
            bbc0_cnt: ReadOnly::new(BbcnCnt::new()),

            // BBC1 registers
            bbc1_irqs: ReadOnly::new(BbcnIrqs::new()),
            bbc1_irqm: ReadWrite::new(BbcnIrqm::new()),
            bbc1_ps: ReadOnly::new(BbcnPs::new()),
            bbc1_pc: ReadWrite::new(BbcnPc::new()),
            bbc1_rxfl: ReadOnly::new(BbcnRxfl::new()),
            bbc1_txfl: ReadWrite::new(BbcnTxfl::new()),
            bbc1_fbl: ReadOnly::new(BbcnFbl::new()),
            bbc1_fbli: ReadWrite::new(BbcnFbli::new()),
            bbc1_ofdmphrtx: ReadWrite::new(BbcnOfdmphrtx::new()),
            bbc1_ofdmphrrx: ReadOnly::new(BbcnOfdmphrrx::new()),
            bbc1_ofdmc: ReadWrite::new(BbcnOfdmc::new()),
            bbc1_ofdmsw: ReadWrite::new(BbcnOfdmsw::new()),
            bbc1_oqpskc0: ReadWrite::new(BbcnOqpskc0::new()),
            bbc1_oqpskc1: ReadWrite::new(BbcnOqpskc1::new()),
            bbc1_oqpskc2: ReadWrite::new(BbcnOqpskc2::new()),
            bbc1_oqpskc3: ReadWrite::new(BbcnOqpskc3::new()),
            bbc1_oqpskphrtx: ReadWrite::new(BbcnOqpskphrtx::new()),
            bbc1_oqpskphrrx: ReadOnly::new(BbcnOqpskphrrx::new()),
            bbc1_afc0: ReadWrite::new(BbcnAfc0::new()),
            bbc1_afc1: ReadWrite::new(BbcnAfc1::new()),
            bbc1_afftm: ReadWrite::new(BbcnAfftm::new()),
            bbc1_affvm: ReadWrite::new(BbcnAffvm::new()),
            bbc1_afs: ReadOnly::new(BbcnAfs::new()),
            bbc1_macea: ReadWrite::new(BbcnMacea::new()),
            bbc1_macpidf0: ReadWrite::new(BbcnMacpid::new()),
            bbc1_macshaf0: ReadWrite::new(BbcnMacsha::new()),
            bbc1_macpidf1: ReadWrite::new(BbcnMacpid::new()),
            bbc1_macshaf1: ReadWrite::new(BbcnMacsha::new()),
            bbc1_macpidf2: ReadWrite::new(BbcnMacpid::new()),
            bbc1_macshaf2: ReadWrite::new(BbcnMacsha::new()),
            bbc1_macpidf3: ReadWrite::new(BbcnMacpid::new()),
            bbc1_macshaf3: ReadWrite::new(BbcnMacsha::new()),
            bbc1_amcs: ReadWrite::new(BbcnAmcs::new()),
            bbc1_amedt: ReadWrite::new(BbcnAmedt::new()),
            bbc1_amaackpd: ReadWrite::new(BbcnAmaackpd::new()),
            bbc1_amaackt: ReadWrite::new(BbcnAmaackt::new()),
            bbc1_fskc0: ReadWrite::new(BbcnFskc0::new()),
            bbc1_fskc1: ReadWrite::new(BbcnFskc1::new()),
            bbc1_fskc2: ReadWrite::new(BbcnFskc2::new()),
            bbc1_fskc3: ReadWrite::new(BbcnFskc3::new()),
            bbc1_fskc4: ReadWrite::new(BbcnFskc4::new()),
            bbc1_fskpll: ReadWrite::new(BbcnFskpll::new()),
            bbc1_fsksfd0: ReadWrite::new(BbcnFsksfd::new()),
            bbc1_fsksfd1: ReadWrite::new(BbcnFsksfd::new()),
            bbc1_fskphrtx: ReadWrite::new(BbcnFskphrtx::new()),
            bbc1_fskphrrx: ReadOnly::new(BbcnFskphrrx::new()),
            bbc1_fskrpc: ReadWrite::new(BbcnFskrpc::new()),
            bbc1_fskrpcont: ReadWrite::new(BbcnFskrpcont::new()),
            bbc1_fskrpcofft: ReadWrite::new(BbcnFskrpcofft::new()),
            bbc1_fskrrxfl: ReadOnly::new(BbcnFskrrxfl::new()),
            bbc1_fskdm: ReadWrite::new(BbcnFskdm::new()),
            bbc1_fskpe0: ReadWrite::new(BbcnFskpe::new()),
            bbc1_fskpe1: ReadWrite::new(BbcnFskpe::new()),
            bbc1_fskpe2: ReadWrite::new(BbcnFskpe::new()),
            bbc1_pmuc: ReadWrite::new(BbcnPmuc::new()),
            bbc1_pmuval: ReadOnly::new(BbcnPmuval::new()),
            bbc1_pmuqf: ReadOnly::new(BbcnPmuqf::new()),
            bbc1_pmui: ReadOnly::new(BbcnPmui::new()),
            bbc1_pmuq: ReadOnly::new(BbcnPmuq::new()),
            bbc1_cntc: ReadWrite::new(BbcnCntc::new()),
            bbc1_cnt: ReadOnly::new(BbcnCnt::new()),
        }
    }
}

// =============================================================================
// Frame Buffer Addresses
// =============================================================================

/// TX Frame Buffer Start (Sub-1GHz)
pub const BBC0_FBTXS: u16 = 0x2000;

/// RX Frame Buffer Start (Sub-1GHz)
pub const BBC0_FBRXS: u16 = 0x3000;

/// TX Frame Buffer Start (2.4GHz)
pub const BBC1_FBTXS: u16 = 0x2800;

/// RX Frame Buffer Start (2.4GHz)
pub const BBC1_FBRXS: u16 = 0x3800;

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rf_cfg_write_command() {
        let mut radio = Radio::new();

        radio.rf_cfg.value = radio.rf_cfg.value.with_drv(3).with_irqmm(true);
        assert_eq!(radio.rf_cfg.value.into_bits(), 0b00001011);
        radio.rf_cfg.value.set_irqp(true);
        assert_eq!(radio.rf_cfg.value.into_bits(), 0b00001111);

        let cmd = radio.rf_cfg.write_command();

        // Should be [header_low, header_high, data]
        assert_eq!(cmd.len(), 3);

        // Verify header for address 0x0006 with write bit
        // Write header: 0x8000 | 0x0006 = 0x8006
        assert_eq!(cmd[0], 0x80);
        assert_eq!(cmd[1], 0x06);
        assert_eq!(cmd[2], 0b00001111); // 0x0F
    }

    #[test]
    fn test_rf_cfg_read_command() {
        let radio = Radio::new();

        let cmd = radio.rf_cfg.read_command();

        // Should be [header_low, header_high, dummy-byte]
        assert_eq!(cmd.len(), 3);

        // Verify header for address 0x0006 with read bit
        // Read header: 0x0000 | 0x0006 = 0x0006
        assert_eq!(cmd[0], 0x00);
        assert_eq!(cmd[1], 0x06);
        assert_eq!(cmd[2], 0x00); // dummy-byte
    }

    #[test]
    fn test_rf09_cmd_write_command() {
        let mut radio = Radio::new();

        assert_eq!(radio.rf09_cmd.value.cmd(), TransceiverCmd::Nop);

        radio.rf09_cmd.value.set_cmd(TransceiverCmd::Sleep);
        assert_eq!(radio.rf09_cmd.value.cmd(), TransceiverCmd::Sleep);

        let cmd = radio.rf09_cmd.write_command();

        // Should be [header_low, header_high, data]
        assert_eq!(cmd.len(), 3);

        // Verify header for address 0x0103 with write bit
        // Write header: 0x8000 | 0x0103 = 0x8103
        assert_eq!(cmd[0], 0x81);
        assert_eq!(cmd[1], 0x03);
        assert_eq!(cmd[2], 0x01); // TransceiverCmd::Sleep value (1)
    }

    #[test]
    fn test_bbc0_cnt_read_command() {
        let radio = Radio::new();

        let cmd = radio.bbc0_cnt.read_command();

        // Should be [header_low, header_high, dummy-0..4]
        assert_eq!(cmd.len(), 6);

        // Verify header for address 0x0391 with read bit
        // Read header: 0x0000 | 0x0391 = 0x0391
        assert_eq!(cmd[0], 0x03);
        assert_eq!(cmd[1], 0x91);

        // Verify all dummy bytes are 0x00
        assert_eq!(&cmd[2..6], &[0x00, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn test_rf09_ccf0_write_command_u16() {
        let mut radio = Radio::new();

        // Set a 16-bit channel center frequency value
        radio.rf09_ccf0.value.set_ccf0(0x1234);

        let cmd = radio.rf09_ccf0.write_command();

        // Should be [header_low, header_high, data_low, data_high]
        assert_eq!(cmd.len(), 4);

        // Verify header for address 0x0105 with write bit
        // Write header: 0x8000 | 0x0105 = 0x8105
        assert_eq!(cmd[0], 0x81);
        assert_eq!(cmd[1], 0x05);
        // data:
        assert_eq!(cmd[2], 0x34); // LSB
        assert_eq!(cmd[3], 0x12); // MSB
    }
}
