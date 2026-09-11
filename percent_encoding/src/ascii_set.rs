// Copyright 2013-2016 The rust-url developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use core::ops;

/// Represents a set of characters or bytes in the ASCII range.
///
/// This is used in [`percent_encode`] and [`utf8_percent_encode`].
/// This is similar to [percent-encode sets](https://url.spec.whatwg.org/#percent-encoded-bytes).
///
/// Use the `add` method of an existing set to define a new set. For example:
///
/// [`percent_encode`]: crate::percent_encode
/// [`utf8_percent_encode`]: crate::utf8_percent_encode
///
/// ```
/// use percent_encoding::{AsciiSet, CONTROLS};
///
/// /// https://url.spec.whatwg.org/#fragment-percent-encode-set
/// const FRAGMENT: &AsciiSet = &CONTROLS.add(b' ').add(b'"').add(b'<').add(b'>').add(b'`');
/// ```
#[derive(Clone, PartialEq, Eq)]
pub struct AsciiSet {
    /// A direct 256-entry lookup table: `should[b]` is `true` when byte `b`
    /// must be percent-encoded. Entries for the non-ASCII range (`0x80..=0xFF`)
    /// are always `true`, since non-ASCII bytes are always encoded. Folding the
    /// non-ASCII check into the table lets the hot per-byte scan in
    /// `PercentEncode` be a single indexed load with no branch or bit-shift.
    should: [bool; 256],
}

impl AsciiSet {
    /// An empty set.
    pub const EMPTY: Self = {
        let mut should = [false; 256];
        // Non-ASCII bytes are always percent-encoded.
        let mut i = 0x80;
        while i < 256 {
            should[i] = true;
            i += 1;
        }
        Self { should }
    };

    /// Called with UTF-8 bytes rather than code points.
    /// Not used for non-ASCII bytes.
    pub(crate) const fn contains(&self, byte: u8) -> bool {
        self.should[byte as usize]
    }

    pub(crate) fn should_percent_encode(&self, byte: u8) -> bool {
        self.contains(byte)
    }

    pub const fn add(&self, byte: u8) -> Self {
        let mut should = self.should;
        should[byte as usize] = true;
        Self { should }
    }

    pub const fn remove(&self, byte: u8) -> Self {
        let mut should = self.should;
        // Non-ASCII bytes are always encoded and cannot be removed from the set.
        if (byte as usize) < 0x80 {
            should[byte as usize] = false;
        }
        Self { should }
    }

    /// Return the union of two sets.
    pub const fn union(&self, other: Self) -> Self {
        let mut should = self.should;
        let mut i = 0;
        while i < 256 {
            should[i] |= other.should[i];
            i += 1;
        }
        Self { should }
    }

    /// Return the negation of the set.
    pub const fn complement(&self) -> Self {
        let mut should = self.should;
        // Only the ASCII range is flipped; non-ASCII bytes stay encoded.
        let mut i = 0;
        while i < 0x80 {
            should[i] = !should[i];
            i += 1;
        }
        Self { should }
    }
}

impl core::fmt::Debug for AsciiSet {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_set()
            .entries((0u16..0x80).filter(|&b| self.should[b as usize]))
            .finish()
    }
}

impl ops::Add for AsciiSet {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        self.union(other)
    }
}

impl ops::Not for AsciiSet {
    type Output = Self;

    fn not(self) -> Self {
        self.complement()
    }
}

/// The set of 0x00 to 0x1F (C0 controls), and 0x7F (DEL).
///
/// Note that this includes the newline and tab characters, but not the space 0x20.
///
/// <https://url.spec.whatwg.org/#c0-control-percent-encode-set>
pub const CONTROLS: &AsciiSet = &{
    let mut set = AsciiSet::EMPTY;
    // C0 controls: 0x00 to 0x1F
    let mut i = 0u8;
    while i < 0x20 {
        set = set.add(i);
        i += 1;
    }
    // DEL: 0x7F
    set.add(0x7F)
};

macro_rules! static_assert {
    ($( $bool: expr, )+) => {
        fn _static_assert() {
            $(
                let _ = core::mem::transmute::<[u8; $bool as usize], u8>;
            )+
        }
    }
}

static_assert! {
    CONTROLS.contains(0x00),
    CONTROLS.contains(0x1F),
    !CONTROLS.contains(0x20),
    !CONTROLS.contains(0x7E),
    CONTROLS.contains(0x7F),
}

/// Everything that is not an ASCII letter or digit.
///
/// This is probably more eager than necessary in any context.
pub const NON_ALPHANUMERIC: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'!')
    .add(b'"')
    .add(b'#')
    .add(b'$')
    .add(b'%')
    .add(b'&')
    .add(b'\'')
    .add(b'(')
    .add(b')')
    .add(b'*')
    .add(b'+')
    .add(b',')
    .add(b'-')
    .add(b'.')
    .add(b'/')
    .add(b':')
    .add(b';')
    .add(b'<')
    .add(b'=')
    .add(b'>')
    .add(b'?')
    .add(b'@')
    .add(b'[')
    .add(b'\\')
    .add(b']')
    .add(b'^')
    .add(b'_')
    .add(b'`')
    .add(b'{')
    .add(b'|')
    .add(b'}')
    .add(b'~');

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_op() {
        let left = AsciiSet::EMPTY.add(b'A');
        let right = AsciiSet::EMPTY.add(b'B');
        let expected = AsciiSet::EMPTY.add(b'A').add(b'B');
        assert_eq!(left + right, expected);
    }

    #[test]
    fn not_op() {
        let set = AsciiSet::EMPTY.add(b'A').add(b'B');
        let not_set = !set;
        assert!(!not_set.contains(b'A'));
        assert!(not_set.contains(b'C'));
    }

    /// This test ensures that we can get the union of two sets as a constant value, which is
    /// useful for defining sets in a modular way.
    #[test]
    fn union() {
        const A: AsciiSet = AsciiSet::EMPTY.add(b'A');
        const B: AsciiSet = AsciiSet::EMPTY.add(b'B');
        const UNION: AsciiSet = A.union(B);
        const EXPECTED: AsciiSet = AsciiSet::EMPTY.add(b'A').add(b'B');
        assert_eq!(UNION, EXPECTED);
    }

    /// This test ensures that we can get the complement of a set as a constant value, which is
    /// useful for defining sets in a modular way.
    #[test]
    fn complement() {
        const BOTH: AsciiSet = AsciiSet::EMPTY.add(b'A').add(b'B');
        const COMPLEMENT: AsciiSet = BOTH.complement();
        assert!(!COMPLEMENT.contains(b'A'));
        assert!(!COMPLEMENT.contains(b'B'));
        assert!(COMPLEMENT.contains(b'C'));
    }

    /// Non-ASCII bytes are always encoded, regardless of set operations.
    #[test]
    fn non_ascii_always_encoded() {
        assert!(AsciiSet::EMPTY.contains(0x80));
        assert!(AsciiSet::EMPTY.contains(0xFF));
        // `remove` and `complement` must never clear a non-ASCII entry.
        assert!(AsciiSet::EMPTY.remove(0x80).contains(0x80));
        assert!((!AsciiSet::EMPTY).contains(0x80));
        assert!(NON_ALPHANUMERIC.contains(0xC3));
    }
}
