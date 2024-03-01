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
- No allocations unless when hyphenating very long words (> 41 bytes). You can
  disable the `alloc` feature, but then overly long words lead to a panic.
- Support for many languages.
- No unsafe code, no dependencies, no std.

## Example
```rust
use hypher::{hyphenate, Lang};

let syllables = hyphenate("extensive", Lang::English);
assert_eq!(syllables.join("-"), "ex-ten-sive");
```

## Languages
By default, this crate supports hyphenating more than 30 languages. Embedding
automata for all these languages will add ~1.1 MiB to your binary.
Alternatively, you can disable support for all languages and manually choose
which ones get added:

```toml
[dependencies]
hypher = { version = "0.1", default-features = false, features = ["english", "greek"] }
```

Each language added individually contributes:

| Language   | Space   |
|------------|---------|
| Afrikaans  | 60 KiB  |
| Albanian   | 1.4 KiB |
| Belarusian | 3.9 KiB |
| Bulgarian  | 13 KiB  |
| Catalan    | 1.7 KiB |
| Croatian   | 2.0 KiB |
| Czech      | 40 KiB  |
| Danish     | 5.7 KiB |
| Dutch      | 63 KiB  |
| English    | 27 KiB  |
| Estonian   | 19 KiB  |
| Finnish    | 1.3 KiB |
| French     | 6.9 KiB |
| Georgian   | 11 KiB  |
| German     | 192 KiB |
| Greek      | 2.0 KiB |
| Hungarian  | 346 KiB |
| Icelandic  | 21 KiB  |
| Italian    | 1.6 KiB |
| Kurmanji   | 1.4 KiB |
| Latin      | 1003 B  |
| Lithuanian | 6.5 KiB |
| Mongolian  | 4.9 KiB |
| Norwegian  | 153 KiB |
| Polish     | 16 KiB  |
| Portuguese | 343 B   |
| Russian    | 33 KiB  |
| Serbian    | 13 KiB  |
| Slovak     | 13 KiB  |
| Slovenian  | 5.5 KiB |
| Spanish    | 14 KiB  |
| Swedish    | 24 KiB  |
| Turkish    | 526 B   |
| Turkmen    | 1.4 KiB |
| Ukrainian  | 21 KiB  |

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
