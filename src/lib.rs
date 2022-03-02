mod tex;
mod trie;

/// Segment a word into syllables.
pub fn hyphenate(word: &str) -> impl Iterator<Item = &str> {
    let mut trie = trie::Trie::new();
    tex::parse(include_str!("../patterns/hyph-en-us.tex"), |pat| {
        trie.insert(pat);
    });
    trie.compress();

    // The level between each two inner bytes of the word.
    let len = word.len().saturating_sub(1);
    let mut levels = vec![0; len];

    // Start pattern matching at each character boundary.
    let dotted = format!(".{}.", word.to_ascii_lowercase());
    for (start, _) in dotted.char_indices() {
        let mut state = trie::State::root(&trie);
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

#[cfg(test)]
mod tests {
    use super::hyphenate;

    fn test(hyphenated: &str) {
        let word = hyphenated.replace('-', "");
        let parts = hyphenate(&word).collect::<Vec<_>>();
        let joined = parts.join("-");
        assert_eq!(joined, hyphenated);
    }

    #[test]
    fn test_hyphenate() {
        test("hi");
        test("hel-lo");
        test("wel-come");
        test("walk-ing");
        test("cap-tiVe");
        test("pur-sue");
        test("wHaT-eV-eR");
        test("bro-ken");
        test("ex-ten-sive");
        test("Prob-a-bil-i-ty");
    }
}
