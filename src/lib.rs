//! `hypher` separates words into syllables.
//!
//! # Features
//! - All-inclusive: Hyphenation patterns are embedded into the binary as
//!   efficiently encoded finite automata at build time.
//! - Zero startup time: Hyphenation automata operate directly over the embedded
//!   binary data with no up-front decoding.
//! - No allocations unless when hyphenating very long words (> 40 bytes).
//! - Support for many languages.
//!
//! # Example
//! ```
//! use hypher::{hyphenate, Lang};
//!
//! let syllables = hyphenate("extensive", Lang::English);
//! let joined = syllables.collect::<Vec<_>>().join("-");
//! assert_eq!(joined, "ex-ten-sive");
//! ```

/// Segment a word into syllables.
pub fn hyphenate(word: &str, lang: Lang) -> impl Iterator<Item = &str> {
    // The level between each two inner bytes of the word.
    let len = word.len().saturating_sub(1);
    let mut levels = vec![0; len];

    // Start pattern matching at each character boundary.
    let dotted = format!(".{}.", word.to_ascii_lowercase());
    for (start, _) in dotted.char_indices() {
        let mut state = lang.root();
        for b in dotted[start ..].bytes() {
            if let Some(next) = state.transition(b) {
                state = next;
                for (offset, level) in state.levels() {
                    let split = start + offset;
                    if split > 2 && split < dotted.len() - 2 {
                        let slot = &mut levels[split - 2];
                        *slot = (*slot).max(level);
                    }
                }
            } else {
                break;
            }
        }
    }

    // Break into segments at odd levels.
    // TODO: Left and right min hyphen
    let mut start = 0;
    levels
        .into_iter()
        .take(len)
        .enumerate()
        .filter_map(|(i, lvl)| (lvl % 2 == 1).then(|| 1 + i))
        .chain(std::iter::once(word.len()))
        .map(move |end| {
            let seg = &word[start .. end];
            start = end;
            seg
        })
}

// Include language data.
include!(concat!(env!("OUT_DIR"), "/langs.rs"));

/// A state in a trie traversal.
#[derive(Copy, Clone)]
struct State<'a> {
    data: &'a [u8],
    addr: usize,
    stride: usize,
    levels: &'a [u8],
    trans: &'a [u8],
    targets: &'a [u8],
}

impl<'a> State<'a> {
    /// Create a new state at the root node.
    fn root(data: &'a [u8]) -> Self {
        let bytes = data[.. 4].try_into().unwrap();
        let addr = u32::from_be_bytes(bytes) as usize;
        Self::at(data, addr)
    }

    /// Create a new state at the given node address.
    fn at(data: &'a [u8], addr: usize) -> Self {
        let node = &data[addr ..];
        let mut pos = 0;

        // Decode whether the state has levels and the transition count.
        let has_levels = node[pos] >> 7 != 0;
        let stride = usize::from((node[pos] >> 5) & 3);
        let mut count = usize::from(node[pos] & 31);
        pos += 1;

        // Possibly decode high transition count.
        if count == 31 {
            count = usize::from(node[pos]);
            pos += 1;
        }

        // Decode the levels.
        let mut levels: &[u8] = &[];
        if has_levels {
            let offset_hi = usize::from(node[pos]) << 4;
            let offset_lo = usize::from(node[pos + 1]) >> 4;
            let offset = offset_hi | offset_lo;
            let len = usize::from(node[pos + 1] & 15);
            levels = &data[offset .. offset + len];
            pos += 2;
        }

        // Decode the transitions.
        let trans = &node[pos .. pos + count];
        pos += count;

        // Decode the targets.
        let targets = &node[pos .. pos + stride * count];

        Self {
            data,
            addr,
            stride,
            levels,
            trans,
            targets,
        }
    }

    /// Return the state reached by following the transition labelled `b`.
    /// Returns `None` if there is no such state.
    fn transition(self, b: u8) -> Option<Self> {
        self.trans.iter().position(|&x| x == b).map(|idx| {
            let offset = self.stride * idx;
            let delta = from_be_bytes(&self.targets[offset .. offset + self.stride]);
            let next = (self.addr as isize + delta) as usize;
            Self::at(self.data, next)
        })
    }

    /// Returns the levels contained in the state.
    fn levels(self) -> impl Iterator<Item = (usize, u8)> + 'a {
        let mut offset = 0;
        self.levels.iter().map(move |&packed| {
            let dist = usize::from(packed / 10);
            let level = packed % 10;
            offset += dist;
            (offset, level)
        })
    }
}

/// Decode a signed number with 1, 2 or 3 bytes.
fn from_be_bytes(buf: &[u8]) -> isize {
    if let Ok(array) = buf.try_into() {
        i8::from_be_bytes(array) as isize
    } else if let Ok(array) = buf.try_into() {
        i16::from_be_bytes(array) as isize
    } else if buf.len() == 3 {
        let first = usize::from(buf[0]) << 16;
        let second = usize::from(buf[1]) << 8;
        let third = usize::from(buf[2]);
        let unsigned = first | second | third;
        unsigned as isize - (1 << 23)
    } else {
        panic!("invalid stride");
    }
}

#[cfg(test)]
mod tests {
    use super::{hyphenate, Lang};
    use Lang::*;

    fn test(lang: Lang, hyphenated: &str) {
        let word = hyphenated.replace('-', "");
        let parts = hyphenate(&word, lang).collect::<Vec<_>>();
        let joined = parts.join("-");
        println!("{joined}");
        assert_eq!(joined, hyphenated);
    }

    #[test]
    fn test_english() {
        test(English, "hi");
        test(English, "hel-lo");
        test(English, "wel-come");
        test(English, "walk-ing");
        test(English, "cap-tiVe");
        test(English, "pur-sue");
        test(English, "wHaT-eV-eR");
        test(English, "bro-ken");
        test(English, "ex-ten-sive");
        test(English, "Prob-a-bil-i-ty");
        test(English, "col-or");
        test(English, "rec-og-nize");
    }

    #[test]
    fn test_german() {
        test(German, "Baum");
        test(German, "ge-hen");
        test(German, "Ap-fel");
        test(German, "To-ma-te");
        test(German, "Ein-ga-be-auf-for-de-rung");
        test(German, "Fort-pflan-zungs-lem-ma");
        test(German, "stra-te-gie-er-hal-ten-den");
        test(German, "hübsch");
        test(German, "häss-lich");
        test(German, "über-zeu-gen-der");
    }

    #[test]
    fn test_greek() {
        test(Greek, "δια-με-ρί-σμα-τα");
        test(Greek, "λα-τρευ-τός");
        test(Greek, "κά-τοι-κος");
    }

    #[test]
    fn test_georgian() {
        test(Georgian, "თა-რო");
        test(Georgian, "შეყ-ვა-ნა");
        test(Georgian, "კარ-ტო-ფი-ლი");
    }
}
