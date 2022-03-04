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
//! assert_eq!(syllables.join("-"), "ex-ten-sive");
//! ```

// Include language data.
include!(concat!(env!("OUT_DIR"), "/langs.rs"));

/// Segment a word into syllables.
///
/// Returns an iterator over the syllables.
/// ```
/// # use hypher::{hyphenate, Lang};
/// let mut syllables = hyphenate("extensive", Lang::English);
/// assert_eq!(syllables.next(), Some("ex"));
/// assert_eq!(syllables.next(), Some("ten"));
/// assert_eq!(syllables.next(), Some("sive"));
/// assert_eq!(syllables.next(), None);
/// # assert_eq!(syllables.next(), None);
/// ```
///
/// This uses the default bounds for the language.
pub fn hyphenate(word: &str, lang: Lang) -> Syllables<'_> {
    let (left_min, right_min) = lang.bounds();
    hyphenate_with_bounds(word, lang, left_min, right_min)
}

/// Segment a word into syllables, but forbid breaking betwen the given number
/// of chars to each side.
pub fn hyphenate_with_bounds(
    word: &str,
    lang: Lang,
    left_min: usize,
    right_min: usize,
) -> Syllables<'_> {
    // Initialize the trie state for the language.
    let root = lang.root();

    // It makes no sense to split outside the word.
    let left_min = left_min.max(1);
    let right_min = right_min.max(1);

    // Convert from chars to byte indices in the dotted word.
    let min_idx = 1 + word.chars().take(left_min).map(char::len_utf8).sum::<usize>();
    let max_idx = 1 + word.len()
        - word.chars().rev().take(right_min).map(char::len_utf8).sum::<usize>();

    // Add a dot to each side to enable patterns that match based on whether
    // they are at the edges of the word.
    let dotted = format!(".{}.", word.to_ascii_lowercase());

    // The level between each two inner bytes of the word.
    let len = word.len().saturating_sub(1);
    let mut levels = vec![0; len];

    // Start pattern matching at each character boundary.
    for (start, _) in dotted.char_indices() {
        let mut state = root;
        for b in dotted[start ..].bytes() {
            if let Some(next) = state.transition(b) {
                state = next;
                for (offset, level) in state.levels() {
                    let split = start + offset;

                    // Example
                    //
                    // Dotted: . h e l l o .
                    // Levels:    0 2 3 0
                    if split >= min_idx && split <= max_idx {
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
    Syllables {
        word,
        cursor: 0,
        levels: levels.into_iter(),
    }
}

/// An iterator over the syllables of a word.
///
/// This struct is created by [`hyphenate`].
#[derive(Debug, Clone)]
pub struct Syllables<'a> {
    word: &'a str,
    cursor: usize,
    levels: std::vec::IntoIter<u8>,
}

impl Syllables<'_> {
    /// Join the syllables with a separator like a hyphen or soft hyphen.
    ///
    /// # Example
    /// Adding soft hyphens at every opportunity.
    /// ```
    /// # use hypher::{hyphenate, Lang};
    /// # let joined =
    /// hyphenate("wonderful", Lang::English).join("\u{ad}");
    /// # assert_eq!(joined, "won\u{ad}der\u{ad}ful")
    /// ```
    pub fn join(mut self, sep: &str) -> String {
        let extra = self.splits() * sep.len();
        let mut s = String::with_capacity(self.word.len() + extra);
        s.extend(self.next());
        for syllable in self {
            s.push_str(sep);
            s.push_str(syllable);
        }
        s
    }

    /// The remaining number of splits in the word.
    fn splits(&self) -> usize {
        self.levels.as_slice().iter().filter(|&lvl| lvl % 2 == 1).count()
    }
}

impl<'a> Iterator for Syllables<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let found = self.levels.any(|lvl| lvl % 2 == 1);
        let start = self.cursor;
        let end = self.word.len() - self.levels.len() - found as usize;
        self.cursor = end;
        (start < end).then(|| &self.word[start .. end])
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = if self.word.is_empty() { 0 } else { 1 + self.splits() };
        (len, Some(len))
    }
}

impl ExactSizeIterator for Syllables<'_> {}

impl std::iter::FusedIterator for Syllables<'_> {}

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
        let syllables = hyphenate(&word, lang);
        assert_eq!(syllables.join("-"), hyphenated);
    }

    #[test]
    fn test_empty() {
        let mut syllables = hyphenate("", Lang::English);
        assert_eq!(syllables.next(), None);
    }

    #[test]
    fn test_exact() {
        assert_eq!(hyphenate("", Lang::English).len(), 0);
        assert_eq!(hyphenate("hello", Lang::English).len(), 1);
        assert_eq!(hyphenate("extensive", Lang::English).len(), 3);
    }

    #[test]
    fn test_english() {
        test(English, "");
        test(English, "hi");
        test(English, "wel-come");
        test(English, "walk-ing");
        test(English, "cap-tiVe");
        test(English, "pur-sue");
        test(English, "wHaT-eVeR");
        test(English, "bro-ken");
        test(English, "ex-ten-sive");
        test(English, "Prob-a-bil-ity");
        test(English, "rec-og-nize");
    }

    #[test]
    fn test_german() {
        test(German, "");
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
