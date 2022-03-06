# hypher
[![Crates.io](https://img.shields.io/crates/v/hypher.svg)](https://crates.io/crates/hypher)
[![Documentation](https://docs.rs/hypher/badge.svg)](https://docs.rs/hypher)

_hypher_ separates words into syllables.

```toml
[dependencies]
hypher = "0.1"
```

## Features
- All-inclusive: Hyphenation patterns are embedded into the binary as
  efficiently encoded finite automata at build time.
- Zero load time: Hyphenation automata operate directly over the embedded
  binary data with no up-front decoding.
- No allocations unless when hyphenating very long words (>= 39 bytes).
- Support for many languages.
- No unsafe code, no dependencies.

## Example
```rust
use hypher::{hyphenate, Lang};

let syllables = hyphenate("extensive", Lang::English);
assert_eq!(syllables.join("-"), "ex-ten-sive");
```

## Languages
By default, this crate supports hyphenating more than 30 languages. Embedding
automata for all these languages will add ~1.1 MB to your binary. Alternatively,
you can disable support for all languages other than English. Then, only
27 KB will be added to your binary.

```toml
[dependencies]
hypher = { version = "0.1", default-features = false, features = ["english"] }
```

## Benchmarks
| Task                               | `hypher`  | [`hyphenation`] |
|------------------------------------|----------:|----------------:|
| Hyphenating `extensive` (english)  | **356ns** |           698ns |
| Hyphenating `διαμερίσματα` (greek) | **503ns** |          1121ns |
| Loading the english patterns       |   **0us** |           151us |
| Loading the greek patterns         |   **0us** |         0.826us |

Benchmarks were executed on ARM, Apple M1.

## License
The code of this crate is dual-licensed under the MIT and Apache 2.0 licenses.

The files in `patterns/` are subject to the individual licenses stated therein.
The patterns are processed at build time and then embedded (i.e. statically
linked) into your binary. However, _hypher_ includes only patterns that are
available under permissive licenses. Patterns licenses include the LPPL, MPL,
MIT, BSD-3.

[`hyphenation`]: https://github.com/tapeinosyne/hyphenation
