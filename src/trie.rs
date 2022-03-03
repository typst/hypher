use std::collections::HashMap;

/// A trie over bytes.
pub struct Trie {
    pub root: usize,
    pub nodes: Vec<Node>,
    pub levels: Vec<(u8, u8)>,
}

/// A node in the trie.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct Node {
    pub trans: Vec<u8>,
    pub targets: Vec<usize>,
    pub levels: Option<(u16, u8)>,
}

impl Trie {
    /// Create a new trie with just the root node.
    pub fn new() -> Self {
        Self {
            root: 0,
            nodes: vec![Node::default()],
            levels: vec![],
        }
    }

    /// Insert a pattern like `.a1bc2d` into the trie.
    pub fn insert(&mut self, pattern: &str) {
        let mut state = 0;
        let mut dist = 0;
        let mut levels = vec![];

        // Follow the existing transitions / add new ones.
        for b in pattern.bytes() {
            if matches!(b, b'0' ..= b'9') {
                let d = u8::try_from(dist).expect("too high distance");
                let v = b - b'0';
                levels.push((d, v));
                dist = 0;
            } else {
                let len = self.nodes.len();
                let node = &mut self.nodes[state];
                if let Some(i) = node.trans.iter().position(|&x| x == b) {
                    state = node.targets[i];
                } else {
                    node.trans.push(b);
                    node.targets.push(len);
                    state = len;
                    self.nodes.push(Node::default());
                }
                dist += 1;
            }
        }

        // Try to reuse existing levels.
        let mut offset = 0;
        while offset < self.levels.len() && !self.levels[offset ..].starts_with(&levels) {
            offset += 1;
        }

        // If there was no matching "substring", we must store new levels.
        if offset == self.levels.len() {
            self.levels.extend(&levels);
        }

        // Add levels for the final node.
        let offset = u16::try_from(offset).expect("too high offset");
        let len = u8::try_from(levels.len()).expect("too many levels");
        self.nodes[state].levels = Some((offset, len));
    }

    /// Perform suffix compression on the trie.
    pub fn compress(&mut self) {
        let mut map = HashMap::new();
        let mut new = vec![];
        self.root = self.compress_node(0, &mut map, &mut new);
        self.nodes = new;
    }

    /// Recursively compress a node.
    pub fn compress_node(
        &self,
        node: usize,
        map: &mut HashMap<Node, usize>,
        new: &mut Vec<Node>,
    ) -> usize {
        let mut x = self.nodes[node].clone();
        for target in x.targets.iter_mut() {
            *target = self.compress_node(*target, map, new);
        }
        *map.entry(x.clone()).or_insert_with(|| {
            let idx = new.len();
            new.push(x);
            idx
        })
    }
}

#[derive(Copy, Clone)]
pub struct State<'a> {
    trie: &'a Trie,
    idx: usize,
}

impl<'a> State<'a> {
    pub fn root(trie: &'a Trie) -> Self {
        Self { trie, idx: trie.root }
    }

    /// Return the state reached by following the transition labelled `b`.
    /// Returns `None` if there is no such state.
    pub fn transition(self, b: u8) -> Option<Self> {
        let node = &self.trie.nodes[self.idx];
        node.trans
            .iter()
            .position(|&x| x == b)
            .map(|i| Self { trie: self.trie, idx: node.targets[i] })
    }

    /// Returns the levels contained in the state.
    pub fn levels(self) -> impl Iterator<Item = (usize, u8)> + 'a {
        let mut offset = 0;
        let node = &self.trie.nodes[self.idx];
        node.levels
            .iter()
            .flat_map(|&(offset, len)| {
                let start = usize::from(offset);
                let end = start + usize::from(len);
                &self.trie.levels[start .. end]
            })
            .map(move |&(d, v)| {
                offset += usize::from(d);
                (offset, v)
            })
    }
}
