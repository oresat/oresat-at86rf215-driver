#!/usr/bin/env python3
"""
Useful script for converting non-serializable bitfield structs in registers.rs into serializable config structs

Usage:
    python3 gen_config_structs.py <path to registers.rs> [output.rs]

If no output file is given, the generated code is printed to stdout.
"""
import re
import sys

class FieldDef:
    def __init__(self, name: str, rust_type: str, is_ro: bool):
        self.name = name
        self.rust_type = rust_type
        self.is_ro = is_ro

class StructDef:
    def __init__(self, name: str):
        self.name = name
        self.fields: list[FieldDef] = []

# Matches the opening of a bitfield struct:
#   #[bitfield(u8)]
#   pub struct RfCfg {
STRUCT_OPEN_RE = re.compile(
    r'#\[bitfield\([^\)]+\)\]\s*(?:#\[allow\([^\)]*\)\]\s*)?pub\s+struct\s+(\w+)\s*\{'
)

# Matches a field line inside the struct body:
#   #[bits(2)]
#   pub drv: u8,
#
#   #[bits(2, access = RO)]
#   pub frzs: bool,
#
#   #[bits(2, access = RO, from = SomeEnum::from_bits)]
#   pub state: SomeEnum,
FIELD_BITS_RE = re.compile(r'#\[bits\((\d+)([^\)]*)\)\]')
FIELD_DECL_RE = re.compile(r'pub\s+(\w+)\s*:\s*(\S+),')
ACCESS_RO_RE  = re.compile(r'access\s*=\s*RO')


def parse_structs(source: str) -> list[StructDef]:
    """
    Parses source and extracts all bitfield struct definitions.
    Returns a list of StructDef objects containing only writable, non-padding fields.
    """
    structs: list[StructDef] = []
    lines = source.splitlines()
    i = 0

    while i < len(lines):
        # Try to match the start of a bitfield struct (may span two lines due to
        # optional #[allow(...)] between the bitfield attr and `pub struct`)
        window = "\n".join(lines[i : i + 4])
        m = STRUCT_OPEN_RE.search(window)
        if not m:
            i += 1
            continue

        struct_name = m.group(1)
        struct_def = StructDef(name=struct_name)

        # Advance past the opening brace
        brace_line = lines[i]
        while "{" not in brace_line and i < len(lines) - 1:
            i += 1
            brace_line = lines[i]
        i += 1  # now inside the struct body

        # Parse fields until the closing brace
        pending_bits_attrs: list[str] = []  # accumulate #[bits(...)] lines

        while i < len(lines):
            line = lines[i].strip()

            if line == "}" or line == "},":
                # End of struct
                break

            bits_m = FIELD_BITS_RE.search(line)
            if bits_m:
                pending_bits_attrs.append(line)
                i += 1
                continue

            field_m = FIELD_DECL_RE.search(line)
            if field_m and pending_bits_attrs:
                field_name = field_m.group(1)
                field_type = field_m.group(2).rstrip(",")

                # Determine read-only from the most recent #[bits(...)] attr
                last_bits_line = pending_bits_attrs[-1]
                is_ro = bool(ACCESS_RO_RE.search(last_bits_line))

                # Skip padding fields and purely read-only fields
                if field_name != "__" and not is_ro:
                    # Strip trailing comma/semicolon from type if any
                    struct_def.fields.append(
                        FieldDef(
                            name=field_name,
                            rust_type=field_type,
                            is_ro=is_ro,
                        )
                    )

                pending_bits_attrs = []

            i += 1

        structs.append(struct_def)
        i += 1

    return structs

# Code generator

def generate_config(s: StructDef) -> str:
    """
    Produce three Rust items for each struct
      1. FooConfig struct
      2. impl From<&Foo> for FooConfig
      3. impl From<&FooConfig> for Foo
    """
    config_name = f"{s.name}Config"
    orig_name   = s.name

    if not s.fields:
        return ""

    lines: list[str] = []

    # Struct dec
    lines.append(f"#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]")
    lines.append(f"pub struct {config_name} {{")
    for f in s.fields:
        lines.append(f"    #[serde(skip_serializing_if = \"Option::is_none\")]")
        lines.append(f"    pub {f.name}: Option<{f.rust_type}>,")
    lines.append("}")

    lines.append("")

    # in case a non-None default impl is ever of use
#     lines.append(f"impl Default for {config_name} {{")
#     lines.append(f"    fn default() -> Self {{")
#     lines.append(f"        let default = {orig_name}::new();")
#     lines.append(f"        Self {{")
#     for f in s.fields:
#
#         lines.append(f"            {f.name}: default.{f.name}(),")
#     lines.append(f"        }}")
#     lines.append(f"    }}")
#     lines.append(f"}}")

#    lines.append("")

    # impl From<&Foo> for FooConfig
    lines.append(f"impl From<&{orig_name}> for {config_name} {{")
    lines.append(f"    fn from(r: &{orig_name}) -> Self {{")
    lines.append(f"        let default = {orig_name}::new();")
    lines.append(f"        Self {{")
    for f in s.fields:
        lines.append(f"            {f.name}: (r.{f.name}() != default.{f.name}()).then(|| r.{f.name}()),")
    lines.append(f"        }}")
    lines.append(f"    }}")
    lines.append(f"}}")

    lines.append("")

    # impl From<&FooConfig> for Foo
    lines.append(f"impl From<&{config_name}> for {orig_name} {{")
    lines.append(f"    fn from(c: &{config_name}) -> Self {{")
    lines.append(f"        let default = {orig_name}::new();")
    lines.append(f"        {orig_name}::new()")
    for f in s.fields:
        lines.append(f"            .with_{f.name}(c.{f.name}.unwrap_or_else(|| default.{f.name}()))")
        # Note this isn't a struct initialization, but rather it's calling each fields various setter functions chained together
    lines.append(f"    }}")
    lines.append(f"}}")

    return "\n".join(lines)


def generate_all(structs: list[StructDef]) -> str:
    header = '''use serde::{Deserialize, Serialize};
use crate::registers::*;

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
    #[serde(skip_serializing_if = "is_default")] pub rf_cfg:               RfCfgConfig,
    #[serde(skip_serializing_if = "is_default")] pub rf_clko:              RfClkoConfig,
    #[serde(skip_serializing_if = "is_default")] pub rf_bmdvc:             RfBmdvcConfig,
    #[serde(skip_serializing_if = "is_default")] pub rf_xoc:               RfXocConfig,
    #[serde(skip_serializing_if = "is_default")] pub rf_iqifc0:            RfIqifc0Config,
    #[serde(skip_serializing_if = "is_default")] pub rf_iqifc1:            RfIqifc1Config,

    // Transceiver Auxiliary Settings
    #[serde(skip_serializing_if = "is_default")] pub rf09_auxs:            RfnAuxsConfig,

    // Channel configuration
    #[serde(skip_serializing_if = "is_default")] pub rf09_cs:              RfnCsConfig,
    #[serde(skip_serializing_if = "is_default")] pub rf09_ccf0:            RfnCcf0Config,
    #[serde(skip_serializing_if = "is_default")] pub rf09_cn:              RfnCnConfig,

    // Receiver configuration
    #[serde(skip_serializing_if = "is_default")] pub rf09_rxbwc:           RfnRxbwcConfig,
    #[serde(skip_serializing_if = "is_default")] pub rf09_rxdfe:           RfnRxdfeConfig,
    #[serde(skip_serializing_if = "is_default")] pub rf09_agcc:            RfnAgccConfig,
    #[serde(skip_serializing_if = "is_default")] pub rf09_agcs:            RfnAgcsConfig,

    // Transmitter configuration
    #[serde(skip_serializing_if = "is_default")] pub rf09_txcutc:          RfnTxcutcConfig,
    #[serde(skip_serializing_if = "is_default")] pub rf09_txdfe:           RfnTxdfeConfig,
    #[serde(skip_serializing_if = "is_default")] pub rf09_pac:             RfnPacConfig,
    #[serde(skip_serializing_if = "is_default")] pub rf09_padfe:           RfnPadfeConfig,

    // PLL configuration
    #[serde(skip_serializing_if = "is_default")] pub rf09_pll:             RfnPllConfig,
    #[serde(skip_serializing_if = "is_default")] pub rf24_auxs:            RfnAuxsConfig,

    // Channel configuration
    #[serde(skip_serializing_if = "is_default")] pub rf24_cs:              RfnCsConfig,
    #[serde(skip_serializing_if = "is_default")] pub rf24_ccf0:            RfnCcf0Config,
    #[serde(skip_serializing_if = "is_default")] pub rf24_cn:              RfnCnConfig,

    // Receiver configuration
    #[serde(skip_serializing_if = "is_default")] pub rf24_rxbwc:           RfnRxbwcConfig,
    #[serde(skip_serializing_if = "is_default")] pub rf24_rxdfe:           RfnRxdfeConfig,
    #[serde(skip_serializing_if = "is_default")] pub rf24_agcc:            RfnAgccConfig,
    #[serde(skip_serializing_if = "is_default")] pub rf24_agcs:            RfnAgcsConfig,

    // Transmitter configuration
    #[serde(skip_serializing_if = "is_default")] pub rf24_txcutc:          RfnTxcutcConfig,
    #[serde(skip_serializing_if = "is_default")] pub rf24_txdfe:           RfnTxdfeConfig,
    #[serde(skip_serializing_if = "is_default")] pub rf24_pac:             RfnPacConfig,
    #[serde(skip_serializing_if = "is_default")] pub rf24_padfe:           RfnPadfeConfig,

    // PLL configuration
    #[serde(skip_serializing_if = "is_default")] pub rf24_pll:             RfnPllConfig,

    // BBC0 OFDM PHY configuration
    #[serde(skip_serializing_if = "is_default")] pub bbc0_ofdmphrtx:       BbcnOfdmphrtxConfig,
    #[serde(skip_serializing_if = "is_default")] pub bbc0_ofdmc:           BbcnOfdmcConfig,
    #[serde(skip_serializing_if = "is_default")] pub bbc0_ofdmsw:          BbcnOfdmswConfig,

    // BBC0 O-QPSK PHY configuration
    #[serde(skip_serializing_if = "is_default")] pub bbc0_oqpskc0:         BbcnOqpskc0Config,
    #[serde(skip_serializing_if = "is_default")] pub bbc0_oqpskc1:         BbcnOqpskc1Config,
    #[serde(skip_serializing_if = "is_default")] pub bbc0_oqpskc2:         BbcnOqpskc2Config,
    #[serde(skip_serializing_if = "is_default")] pub bbc0_oqpskc3:         BbcnOqpskc3Config,
    #[serde(skip_serializing_if = "is_default")] pub bbc0_oqpskphrtx:      BbcnOqpskphrtxConfig,

    // BBC0 Address filter configuration
    #[serde(skip_serializing_if = "is_default")] pub bbc0_afc0:            BbcnAfc0Config,
    #[serde(skip_serializing_if = "is_default")] pub bbc0_afc1:            BbcnAfc1Config,
    #[serde(skip_serializing_if = "is_default")] pub bbc0_afftm:           BbcnAfftmConfig,
    #[serde(skip_serializing_if = "is_default")] pub bbc0_affvm:           BbcnAffvmConfig,

    // BBC0 Auto mode configuration
    #[serde(skip_serializing_if = "is_default")] pub bbc0_amcs:            BbcnAmcsConfig,
    #[serde(skip_serializing_if = "is_default")] pub bbc0_amedt:           BbcnAmedtConfig,
    #[serde(skip_serializing_if = "is_default")] pub bbc0_amaackpd:        BbcnAmaackpdConfig,
    #[serde(skip_serializing_if = "is_default")] pub bbc0_amaackt:         BbcnAmaacktConfig,

    // BBC0 FSK PHY configuration
    #[serde(skip_serializing_if = "is_default")] pub bbc0_fskc0:           BbcnFskc0Config,
    #[serde(skip_serializing_if = "is_default")] pub bbc0_fskc1:           BbcnFskc1Config,
    #[serde(skip_serializing_if = "is_default")] pub bbc0_fskc2:           BbcnFskc2Config,
    #[serde(skip_serializing_if = "is_default")] pub bbc0_fskc3:           BbcnFskc3Config,
    #[serde(skip_serializing_if = "is_default")] pub bbc0_fskc4:           BbcnFskc4Config,
    #[serde(skip_serializing_if = "is_default")] pub bbc0_fskpll:          BbcnFskpllConfig,
    #[serde(skip_serializing_if = "is_default")] pub bbc0_fsksfd0:         BbcnFsksfdConfig,
    #[serde(skip_serializing_if = "is_default")] pub bbc0_fsksfd1:         BbcnFsksfdConfig,
    #[serde(skip_serializing_if = "is_default")] pub bbc0_fskphrtx:        BbcnFskphrtxConfig,
    #[serde(skip_serializing_if = "is_default")] pub bbc0_fskrpc:          BbcnFskrpcConfig,
    #[serde(skip_serializing_if = "is_default")] pub bbc0_fskrpcont:       BbcnFskrpcontConfig,
    #[serde(skip_serializing_if = "is_default")] pub bbc0_fskrpcofft:      BbcnFskrpcofftConfig,
    #[serde(skip_serializing_if = "is_default")] pub bbc0_fskdm:           BbcnFskdmConfig,
    #[serde(skip_serializing_if = "is_default")] pub bbc0_fskpe0:          BbcnFskpeConfig,
    #[serde(skip_serializing_if = "is_default")] pub bbc0_fskpe1:          BbcnFskpeConfig,
    #[serde(skip_serializing_if = "is_default")] pub bbc0_fskpe2:          BbcnFskpeConfig,

    // BBC1 OFDM PHY configuration
    #[serde(skip_serializing_if = "is_default")] pub bbc1_ofdmphrtx:       BbcnOfdmphrtxConfig,
    #[serde(skip_serializing_if = "is_default")] pub bbc1_ofdmc:           BbcnOfdmcConfig,
    #[serde(skip_serializing_if = "is_default")] pub bbc1_ofdmsw:          BbcnOfdmswConfig,

    // BBC1 O-QPSK PHY configuration
    #[serde(skip_serializing_if = "is_default")] pub bbc1_oqpskc0:         BbcnOqpskc0Config,
    #[serde(skip_serializing_if = "is_default")] pub bbc1_oqpskc1:         BbcnOqpskc1Config,
    #[serde(skip_serializing_if = "is_default")] pub bbc1_oqpskc2:         BbcnOqpskc2Config,
    #[serde(skip_serializing_if = "is_default")] pub bbc1_oqpskc3:         BbcnOqpskc3Config,
    #[serde(skip_serializing_if = "is_default")] pub bbc1_oqpskphrtx:      BbcnOqpskphrtxConfig,

    // BBC1 Address filter configuration
    #[serde(skip_serializing_if = "is_default")] pub bbc1_afc0:            BbcnAfc0Config,
    #[serde(skip_serializing_if = "is_default")] pub bbc1_afc1:            BbcnAfc1Config,
    #[serde(skip_serializing_if = "is_default")] pub bbc1_afftm:           BbcnAfftmConfig,
    #[serde(skip_serializing_if = "is_default")] pub bbc1_affvm:           BbcnAffvmConfig,

    // BBC1 Auto mode configuration
    #[serde(skip_serializing_if = "is_default")] pub bbc1_amcs:            BbcnAmcsConfig,
    #[serde(skip_serializing_if = "is_default")] pub bbc1_amedt:           BbcnAmedtConfig,
    #[serde(skip_serializing_if = "is_default")] pub bbc1_amaackpd:        BbcnAmaackpdConfig,
    #[serde(skip_serializing_if = "is_default")] pub bbc1_amaackt:         BbcnAmaacktConfig,

    // BBC1 FSK PHY configuration
    #[serde(skip_serializing_if = "is_default")] pub bbc1_fskc0:           BbcnFskc0Config,
    #[serde(skip_serializing_if = "is_default")] pub bbc1_fskc1:           BbcnFskc1Config,
    #[serde(skip_serializing_if = "is_default")] pub bbc1_fskc2:           BbcnFskc2Config,
    #[serde(skip_serializing_if = "is_default")] pub bbc1_fskc3:           BbcnFskc3Config,
    #[serde(skip_serializing_if = "is_default")] pub bbc1_fskc4:           BbcnFskc4Config,
    #[serde(skip_serializing_if = "is_default")] pub bbc1_fskpll:          BbcnFskpllConfig,
    #[serde(skip_serializing_if = "is_default")] pub bbc1_fsksfd0:         BbcnFsksfdConfig,
    #[serde(skip_serializing_if = "is_default")] pub bbc1_fsksfd1:         BbcnFsksfdConfig,
    #[serde(skip_serializing_if = "is_default")] pub bbc1_fskphrtx:        BbcnFskphrtxConfig,
    #[serde(skip_serializing_if = "is_default")] pub bbc1_fskrpc:          BbcnFskrpcConfig,
    #[serde(skip_serializing_if = "is_default")] pub bbc1_fskrpcont:       BbcnFskrpcontConfig,
    #[serde(skip_serializing_if = "is_default")] pub bbc1_fskrpcofft:      BbcnFskrpcofftConfig,
    #[serde(skip_serializing_if = "is_default")] pub bbc1_fskdm:           BbcnFskdmConfig,
    #[serde(skip_serializing_if = "is_default")] pub bbc1_fskpe0:          BbcnFskpeConfig,
    #[serde(skip_serializing_if = "is_default")] pub bbc1_fskpe1:          BbcnFskpeConfig,
    #[serde(skip_serializing_if = "is_default")] pub bbc1_fskpe2:          BbcnFskpeConfig,
}


// Used for reducing empty members in radioConfig struct
fn is_default<T: Default + PartialEq>(v: &T) -> bool {
    v == &T::default()
}'''
    blocks = [generate_config(s) for s in structs]
    return header + "\n\n".join(blocks)

def main() -> None:
    if len(sys.argv) < 2:
        print("Error: requires path to registers.rs")
        sys.exit(1)

    input_path = sys.argv[1]
    output_path = sys.argv[2] if len(sys.argv) > 2 else None

    with open(input_path, "r", encoding="utf-8") as fh:
        source = fh.read()

    structs = parse_structs(source)
    print(f"Found {len(structs)} bitfield struct(s):", file=sys.stderr)
    for s in structs:
        writable = [f.name for f in s.fields]
        print(f"  {s.name}: {writable}", file=sys.stderr)

    output = generate_all(structs)

    if output_path:
        with open(output_path, "w", encoding="utf-8") as fh:
            fh.write(output)
        print(f"Written to {output_path}", file=sys.stderr)
    else:
        print(output)


if __name__ == "__main__":
    main()
