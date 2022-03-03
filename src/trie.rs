use std::collections::HashMap;

/// Builds a trie from patterns.
pub struct TrieBuilder {
    pub root: usize,
    pub nodes: Vec<Node>,
    pub levels: Vec<(usize, u8)>,
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
                levels.push((dist, b - b'0'));
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

        // If there was no matching level "substring", we must append the new
        // levels at the end.
        if offset == self.levels.len() {
            self.levels.extend(&levels);
        }

        // Add levels for the final node.
        self.nodes[state].levels = Some((offset, levels.len()));
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
        let start = 4 + self.levels.len();

        // Compute an address estimate for each node. We can't know the final
        // addresses yet because the addresses depend on the stride of each
        // target list and that stride of the target lists depends on the
        // addresses.
        let mut addr = start;
        let mut estimates = vec![];
        for node in &self.nodes {
            estimates.push(addr);
            addr += 1
                + ((node.trans.len() >= 31) as usize)
                + 2 * (node.levels.is_some() as usize)
                + (1 + 3) * node.trans.len();
        }

        // Use the address estimates to determine how many bytes to use for each
        // state and compute the final addresses.
        let mut addr = start;
        let mut addrs = vec![];
        let mut strides = vec![];
        for (i, node) in self.nodes.iter().enumerate() {
            let stride = node
                .targets
                .iter()
                .map(|&t| how_many_bytes(estimates[t] as isize - estimates[i] as isize))
                .max()
                .unwrap_or(1);

            addrs.push(addr);
            strides.push(stride);
            addr += 1
                + ((node.trans.len() >= 31) as usize)
                + 2 * (node.levels.is_some() as usize)
                + (1 + stride) * node.trans.len();
        }

        let mut data = vec![];

        // Encode the root address.
        data.extend(u32::try_from(addrs[self.root] as u32).unwrap().to_be_bytes());

        // Encode the levels.
        for &(dist, level) in &self.levels {
            assert!(dist <= 24, "too high level distance");
            assert!(level < 10, "too high level");
            data.push(dist as u8 * 10 + level);
        }

        // Encode the nodes.
        for ((node, &addr), stride) in self.nodes.iter().zip(&addrs).zip(strides) {
            data.push(
                (node.levels.is_some() as u8) << 7
                    | (stride as u8) << 5
                    | (node.trans.len().min(31) as u8),
            );

            if node.trans.len() >= 31 {
                data.push(u8::try_from(node.trans.len()).expect("too many transitions"));
            }

            if let Some((offset, len)) = node.levels {
                let offset = 4 + offset;

                assert!(offset < 4096, "too high level offset");
                assert!(len < 16, "too high level count");

                let offset_hi = (offset >> 4) as u8;
                let offset_lo = ((offset & 15) << 4) as u8;
                let len = len as u8;

                data.push(offset_hi);
                data.push(offset_lo | len);
            }

            data.extend(&node.trans);

            for &target in &node.targets {
                let delta = addrs[target] as isize - addr as isize;
                to_be_bytes(&mut data, delta, stride);
            }
        }

        data
    }
}

/// A state in a trie traversal.
#[derive(Copy, Clone)]
pub struct State<'a> {
    data: &'a [u8],
    addr: usize,
    stride: usize,
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
    pub fn transition(self, b: u8) -> Option<Self> {
        self.trans.iter().position(|&x| x == b).map(|idx| {
            let offset = self.stride * idx;
            let delta = from_be_bytes(&self.targets[offset .. offset + self.stride]);
            let next = (self.addr as isize + delta) as usize;
            Self::at(self.data, next)
        })
    }

    /// Returns the levels contained in the state.
    pub fn levels(self) -> impl Iterator<Item = (usize, u8)> + 'a {
        let mut offset = 0;
        self.levels.iter().map(move |&packed| {
            let dist = usize::from(packed / 10);
            let level = packed % 10;
            offset += dist;
            (offset, level)
        })
    }
}

/// How many bytes are needed to encode a signed number.
fn how_many_bytes(num: isize) -> usize {
    if i8::try_from(num).is_ok() {
        1
    } else if i16::try_from(num).is_ok() {
        2
    } else if -(1 << 23) <= num && num < (1 << 23) {
        3
    } else {
        panic!("too large number");
    }
}

/// Encode a signed number with 1, 2 or 3 bytes.
fn to_be_bytes(buf: &mut Vec<u8>, num: isize, stride: usize) {
    if stride == 1 {
        buf.extend(i8::try_from(num).unwrap().to_be_bytes());
    } else if stride == 2 {
        buf.extend(i16::try_from(num).unwrap().to_be_bytes());
    } else if stride == 3 {
        let unsigned = (num + (1 << 23)) as usize;
        buf.push((unsigned >> 16) as u8);
        buf.push((unsigned >> 8) as u8);
        buf.push(unsigned as u8);
    } else {
        panic!("invalid stride");
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
