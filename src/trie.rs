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
    pub levels: Option<(u16, u8)>,
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
            addr += 4 + 5 * node.trans.len();
        }

        let mut data = addrs[self.root].to_be_bytes().to_vec();
        data.extend(self.levels.iter().flat_map(|(d, v)| [d, v]));

        for node in &self.nodes {
            data.push(node.trans.len() as u8);

            if let Some((offset, len)) = node.levels {
                data.extend(offset.to_be_bytes());
                data.push(len);
            } else {
                data.extend([0, 0, 0]);
            }

            data.extend(&node.trans);
            data.extend(node.targets.iter().flat_map(|&idx| addrs[idx].to_be_bytes()));
        }

        data
    }
}

#[derive(Copy, Clone)]
pub struct State<'a> {
    data: &'a [u8],
    addr: usize,
}

impl<'a> State<'a> {
    pub fn start(data: &'a [u8]) -> Self {
        let bytes = data[.. 4].try_into().unwrap();
        let addr = u32::from_be_bytes(bytes) as usize;
        Self { data, addr }
    }

    /// Return the state reached by following the transition labelled `b`.
    /// Returns `None` if there is no such state.
    pub fn transition(self, b: u8) -> Option<Self> {
        let node = &self.data[self.addr ..];
        let count = usize::from(node[0]);
        node[4 .. 4 + count].iter().position(|&x| x == b).map(|idx| {
            let offset = 4 + count + 4 * idx;
            let bytes = node[offset .. offset + 4].try_into().unwrap();
            let next = u32::from_be_bytes(bytes) as usize;
            Self { data: self.data, addr: next }
        })
    }

    /// Returns the levels contained in the state.
    pub fn levels(self) -> impl Iterator<Item = (usize, u8)> + 'a {
        let node = &self.data[self.addr ..];
        let bytes = node[1 .. 3].try_into().unwrap();
        let offset = 4 + 2 * usize::from(u16::from_be_bytes(bytes));
        let len = 2 * usize::from(node[3]);
        let levels = &self.data[offset .. offset + len];
        let mut seen = 0;
        levels.chunks_exact(2).map(move |chunk| {
            seen += usize::from(chunk[0]);
            (seen, chunk[1])
        })
    }
}
