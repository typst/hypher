/// A trie over bytes.
pub struct Trie {
    nodes: Vec<Node>,
}

/// A node in the trie.
struct Node {
    trans: Vec<(u8, usize)>,
    levels: Option<Vec<(usize, u8)>>,
}

impl Trie {
    /// Create a new trie with just the root node.
    pub fn new() -> Self {
        Self {
            nodes: vec![Node { trans: vec![], levels: None }],
        }
    }

    /// Insert a pattern like `.a1bc2d` into the trie.
    pub fn insert(&mut self, pattern: &str) {
        let mut state = 0;
        let mut count = 0;
        let mut levels = vec![];

        // Follow the existing transitions / add new ones.
        for b in pattern.bytes() {
            if matches!(b, b'0' ..= b'9') {
                levels.push((count, b - b'0'));
            } else {
                if let Some(&(_, target)) =
                    self.nodes[state].trans.iter().find(|&&(x, _)| x == b)
                {
                    state = target;
                } else {
                    let new = self.nodes.len();
                    self.nodes[state].trans.push((b, new));
                    self.nodes.push(Node { trans: vec![], levels: None });
                    state = new;
                }
                count += 1;
            }
        }

        // Mark the final address as terminating.
        self.nodes[state].levels = Some(levels);
    }
}

#[derive(Copy, Clone)]
pub struct State<'a> {
    trie: &'a Trie,
    idx: usize,
}

impl<'a> State<'a> {
    pub fn root(trie: &'a Trie) -> Self {
        Self { trie, idx: 0 }
    }

    /// Return the state reached by following the transition labelled `b`.
    /// Returns `None` if there is no such state.
    pub fn transition(self, b: u8) -> Option<Self> {
        let node = &self.trie.nodes[self.idx];
        node.trans
            .iter()
            .find(|&&(x, _)| x == b)
            .map(|&(_, target)| Self { trie: self.trie, idx: target })
    }

    /// Returns the levels contained in the state.
    pub fn levels(self) -> impl Iterator<Item = (usize, u8)> + 'a {
        let node = &self.trie.nodes[self.idx];
        node.levels.iter().flat_map(|levels| levels).copied()
    }
}
