use core::marker::PhantomData;
use core::ops::BitOr;

/// A typed bit-mask marking a single RW bit on register `R`.
///
/// `BitOr` composes these into a value of `R`. The macro only emits a
/// `RwFlag<R>` constant for fields declared `rw`, so read-only fields can't
/// participate in composition.
pub struct RwFlag<R> {
    /// Raw bit mask within the register's backing byte.
    pub mask: u8,
    _phantom: PhantomData<R>,
}

impl<R> Clone for RwFlag<R> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<R> Copy for RwFlag<R> {}

impl<R> RwFlag<R> {
    /// Build a flag with the given bit mask. Called by the macro expansion.
    pub const fn new(mask: u8) -> Self {
        Self {
            mask,
            _phantom: PhantomData,
        }
    }
}

/// Shared interface implemented by every macro-generated register type.
pub trait DsReg: Copy {
    /// Register address on the SPI bus (datasheet ch. 8).
    const ADDRESS: u16;
    /// Register width in bytes.
    const SIZE: usize;
    /// Wrap a raw byte as the register type.
    fn from_raw(raw: u8) -> Self;
    /// Unwrap to the raw byte.
    fn raw(self) -> u8;
}

/// `RwFlag<R> | RwFlag<R> -> R` - blanket composition for any macro-defined
/// u8 register. The macro additionally emits `R | RwFlag<R> -> R` per-type so
/// you can chain any number of flags.
impl<R: DsReg> BitOr<RwFlag<R>> for RwFlag<R> {
    type Output = R;
    fn bitor(self, rhs: RwFlag<R>) -> R {
        R::from_raw(self.mask | rhs.mask)
    }
}

/// Syntax:
/// ```text
/// datasheet_register! {
///     RegName: u8 @ 0xADDR = {
///         NAME [rw|ro, bit|lsb..=msb, flag|u8],
///         ...
///     }
/// }
/// ```
///
/// For every field:
///   * `rw` emits a `pub const NAME: RwFlag<Self>` that composes via `|`.
///   * `ro` emits no constant - composition with an RO field is a compile
///     error (the identifier doesn't exist in the associated namespace).
///   * Both kinds emit a lowercase getter (`.name() -> bool` for `flag`,
///     `.name() -> u8` for integer fields) for reading back the value.
///
/// The macro is u8-only by design - all flag-heavy registers on the RF215
/// are single-byte. Multi-byte registers continue to use `bitfield-struct`.
#[macro_export]
macro_rules! datasheet_register {
    (
        $(#[$outer:meta])*
        $name:ident : u8 @ $addr:literal = {
            $($body:tt)*
        }
    ) => {
        $(#[$outer])*
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
        pub struct $name(u8);

        impl $name {
            /// Register address (datasheet ch. 8).
            pub const ADDRESS: u16 = $addr;
            /// Register width in bytes.
            pub const SIZE: usize = 1;

            /// Fresh zero-valued register.
            pub const fn new() -> Self { Self(0) }
            /// Wrap a raw byte.
            pub const fn from_raw(raw: u8) -> Self { Self(raw) }
            /// Unwrap to the raw byte.
            pub const fn raw(self) -> u8 { self.0 }
        }

        impl $crate::ds::DsReg for $name {
            const ADDRESS: u16 = $addr;
            const SIZE: usize = 1;
            fn from_raw(raw: u8) -> Self { Self(raw) }
            fn raw(self) -> u8 { self.0 }
        }

        impl core::ops::BitOr<$crate::ds::RwFlag<$name>> for $name {
            type Output = $name;
            fn bitor(self, rhs: $crate::ds::RwFlag<$name>) -> $name {
                $name(self.0 | rhs.mask)
            }
        }

        $crate::datasheet_register!(@fields $name; $($body)*);
    };

    // ── recursive field parser ────────────────────────────────────────────
    // Each field emits its own `impl $name { ... }` block.

    (@fields $reg:ident;) => {};
    (@fields $reg:ident; $(,)+) => {};

    // rw flag: const + lowercase getter
    (@fields $reg:ident; $field:ident [rw, $bit:literal, flag] $(, $($rest:tt)*)?) => {
        impl $reg {
            $crate::ds::paste::paste! {
                #[allow(non_upper_case_globals)]
                pub const $field: $crate::ds::RwFlag<$reg> =
                    $crate::ds::RwFlag::new(1u8 << $bit);
                pub const fn [<$field:lower>](self) -> bool {
                    (self.0 >> $bit) & 1 != 0
                }
            }
        }
        $crate::datasheet_register!(@fields $reg; $($($rest)*)?);
    };

    // ro flag: lowercase getter only
    (@fields $reg:ident; $field:ident [ro, $bit:literal, flag] $(, $($rest:tt)*)?) => {
        impl $reg {
            $crate::ds::paste::paste! {
                pub const fn [<$field:lower>](self) -> bool {
                    (self.0 >> $bit) & 1 != 0
                }
            }
        }
        $crate::datasheet_register!(@fields $reg; $($($rest)*)?);
    };

    // ro integer on an lsb..=msb range
    (@fields $reg:ident; $field:ident [ro, $lsb:literal..=$msb:literal, $ty:ident] $(, $($rest:tt)*)?) => {
        impl $reg {
            $crate::ds::paste::paste! {
                pub const fn [<$field:lower>](self) -> u8 {
                    let width = $msb - $lsb + 1;
                    let mask = (1u8 << width) - 1;
                    (self.0 >> $lsb) & mask
                }
            }
        }
        $crate::datasheet_register!(@fields $reg; $($($rest)*)?);
    };

    // rw integer on an lsb..=msb range (no BitOr const - multi-bit writes go
    // through a setter rather than flag composition)
    (@fields $reg:ident; $field:ident [rw, $lsb:literal..=$msb:literal, $ty:ident] $(, $($rest:tt)*)?) => {
        impl $reg {
            $crate::ds::paste::paste! {
                pub const fn [<$field:lower>](self) -> u8 {
                    let width = $msb - $lsb + 1;
                    let mask = (1u8 << width) - 1;
                    (self.0 >> $lsb) & mask
                }
                pub const fn [<with_ $field:lower>](self, value: u8) -> Self {
                    let width = $msb - $lsb + 1;
                    let mask = ((1u8 << width) - 1) << $lsb;
                    Self((self.0 & !mask) | ((value << $lsb) & mask))
                }
            }
        }
        $crate::datasheet_register!(@fields $reg; $($($rest)*)?);
    };
}

#[doc(hidden)]
pub use paste;

// PMUC register
datasheet_register! {
    /// `BBCn_PMUC` - Phase Measurement Unit control.
    PmucDs: u8 @ 0x0380 = {
        CCFTS [rw, 7, flag],
        IQSEL [rw, 6, flag],
        FED   [rw, 5, flag],
        SYNC  [ro, 2..=4, u3],
        AVG   [rw, 1, flag],
        EN    [rw, 0, flag],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn address_and_size_constants_match_datasheet() {
        assert_eq!(PmucDs::ADDRESS, 0x0380);
        assert_eq!(PmucDs::SIZE, 1);
        // DsReg trait constants must agree with inherent constants.
        assert_eq!(<PmucDs as DsReg>::ADDRESS, 0x0380);
        assert_eq!(<PmucDs as DsReg>::SIZE, 1);
    }

    #[test]
    fn rw_flags_compose_via_bitor() {
        let pmuc = PmucDs::CCFTS | PmucDs::FED | PmucDs::EN;
        // Bits 7, 5, 0 set.
        assert_eq!(pmuc.raw(), 0b1010_0001);
        assert!(pmuc.ccfts());
        assert!(pmuc.fed());
        assert!(pmuc.en());
        assert!(!pmuc.iqsel());
        assert!(!pmuc.avg());
    }

    #[test]
    fn bitor_is_order_independent() {
        let a = PmucDs::CCFTS | PmucDs::EN;
        let b = PmucDs::EN | PmucDs::CCFTS;
        assert_eq!(a.raw(), b.raw());
    }

    #[test]
    fn ro_integer_field_decodes_correctly() {
        // bits 4..=2 = 0b110 -> SYNC = 6
        #[allow(clippy::unusual_byte_groupings)]
        let pmuc = PmucDs::from_raw(0b000_110_00);
        assert_eq!(pmuc.sync(), 6);
        // Other fields should be zero.
        assert!(!pmuc.ccfts());
        assert!(!pmuc.en());
    }

    #[test]
    fn sync_reads_independently_of_rw_flags() {
        // All flags + SYNC = 0b111 simultaneously.
        let raw = 0b1110_0111; // CCFTS=1, IQSEL=1, FED=1, SYNC=001, AVG=1, EN=1
        let pmuc = PmucDs::from_raw(raw);
        assert!(pmuc.ccfts());
        assert!(pmuc.iqsel());
        assert!(pmuc.fed());
        assert_eq!(pmuc.sync(), 0b001);
        assert!(pmuc.avg());
        assert!(pmuc.en());
    }

    #[test]
    fn from_raw_round_trips_through_raw() {
        for raw in [0x00, 0x55, 0xAA, 0xFF] {
            assert_eq!(PmucDs::from_raw(raw).raw(), raw);
        }
    }

    #[test]
    fn default_is_zero() {
        assert_eq!(PmucDs::default().raw(), 0);
        assert_eq!(PmucDs::new().raw(), 0);
    }
}
