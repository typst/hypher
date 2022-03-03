use std::collections::HashMap;

/// Builds a trie from patterns.
pub struct TrieBuilder {
    pub root: usize,
    pub nodes: Vec<Node>,
    pub levels: Vec<(u8, u8)>,
}

/// A node in the trie.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct Node {
    pub trans: Vec<u8>,
    pub targets: Vec<usize>,
    pub levels: Option<(usize, usize)>,
}

impl TrieBuilder {
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
        let mut start = 0;
        while start < self.levels.len() && !self.levels[start ..].starts_with(&levels) {
            start += 1;
        }

        // If there was no matching "substring", we must store new levels.
        if start == self.levels.len() {
            self.levels.extend(&levels);
        }

        // Add levels for the final node.
        self.nodes[state].levels = Some((start, levels.len()));
    }

    /// Perform suffix compression on the trie.
    pub fn compress(&mut self) {
        let mut map = HashMap::new();
        let mut new = vec![];
        self.root = self.compress_node(0, &mut map, &mut new);
        self.nodes = new;
    }

    /// Recursively compress a node.
    fn compress_node(
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

    /// Encode the tree.
    pub fn encode(&self) -> Vec<u8> {
        let mut addr = 4 + 2 * self.levels.len();
        let mut addrs = vec![];
        for node in &self.nodes {
            addrs.push(u32::try_from(addr).expect("too high address"));
            addr += 1;
            if node.levels.is_some() {
                addr += 2;
            }
            addr += 5 * node.trans.len();
        }

        let mut data = addrs[self.root].to_be_bytes().to_vec();
        data.extend(self.levels.iter().flat_map(|(d, v)| [d, v]));

        for node in &self.nodes {
            assert!(node.trans.len() < 128);
            let has_levels = node.levels.is_some() as u8;
            let count = node.trans.len() as u8;
            data.push(has_levels << 7 | count);

            if let Some((start, len)) = node.levels {
                assert!(start < 4096, "too high level start");
                assert!(len < 16, "too high level count");

                let start_hi = (start >> 4) as u8;
                let start_lo = ((start & 15) << 4) as u8;
                let len = len as u8;

                data.push(start_hi);
                data.push(start_lo | len);
            }

            data.extend(&node.trans);
            data.extend(node.targets.iter().flat_map(|&idx| addrs[idx].to_be_bytes()));
        }

        data
    }
}

/// A state in a trie traversal.
#[derive(Copy, Clone)]
pub struct State<'a> {
    data: &'a [u8],
    levels: &'a [u8],
    trans: &'a [u8],
    targets: &'a [u8],
}

impl<'a> State<'a> {
    /// Create a new state at the root node.
    pub fn root(data: &'a [u8]) -> Self {
        let bytes = data[.. 4].try_into().unwrap();
        let addr = u32::from_be_bytes(bytes) as usize;
        Self::at(data, addr)
    }

    /// Create a new state at the given node address.
    pub fn at(data: &'a [u8], addr: usize) -> Self {
        let node = &data[addr ..];
        let mut pos = 0;

        // Decode whether the state has levels and the transition count.
        let has_levels = node[pos] >> 7 != 0;
        let count = usize::from(node[pos] & 127);
        pos += 1;

        // Decode the levels.
        let mut levels: &[u8] = &[];
        if has_levels {
            let start = usize::from(node[pos]) << 4 | usize::from(node[pos + 1]) >> 4;
            let len = usize::from(node[pos + 1] & 15);
            let offset = 4 + 2 * start;
            levels = &data[offset .. offset + 2 * len];
            pos += 2;
        }

        // Decode the transitions.
        let trans = &node[pos .. pos + count];
        pos += count;

        // Decode the targets.
        let targets = &node[pos .. pos + 4 * count];

        Self { data, levels, trans, targets }
    }

    /// Return the state reached by following the transition labelled `b`.
    /// Returns `None` if there is no such state.
    pub fn transition(self, b: u8) -> Option<Self> {
        self.trans.iter().position(|&x| x == b).map(|idx| {
            let offset = 4 * idx;
            let bytes = self.targets[offset .. offset + 4].try_into().unwrap();
            let next = u32::from_be_bytes(bytes) as usize;
            Self::at(self.data, next)
        })
    }

    /// Returns the levels contained in the state.
    pub fn levels(self) -> impl Iterator<Item = (usize, u8)> + 'a {
        let mut offset = 0;
        self.levels.chunks_exact(2).map(move |chunk| {
            offset += usize::from(chunk[0]);
            (offset, chunk[1])
        })
    }
}
