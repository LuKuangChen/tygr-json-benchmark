# tygr-json-benchmark

JSON throughput of [tygr](https://github.com/intersystems-community/tygr)
against other Rust parsers on the
[serde-rs/json-benchmark](https://github.com/serde-rs/json-benchmark) corpus.
serde_json is the baseline: its parser is written by hand, as fast parsers
usually are, so the inputs are the ones serde_json's own benchmark uses:
- [canada.json](https://raw.githubusercontent.com/serde-rs/json-benchmark/master/data/canada.json)
- [citm_catalog.json](https://raw.githubusercontent.com/serde-rs/json-benchmark/master/data/citm_catalog.json)
- [twitter.json](https://raw.githubusercontent.com/serde-rs/json-benchmark/master/data/twitter.json)

The benchmark compares the lowest-cost error-tracking mode each parser exposes,
while still checking that the parsed values match a shared JSON shape.

Competitors build a JSON value with escapes decoded and numbers as correctly
rounded `f64`:
- serde_json builds its own `serde_json::Value`, whose enum definition is in
  [serde-rs/json](https://github.com/serde-rs/json/blob/8c535a260ce561eaab4111aafc29038eb9f77ff3/src/value/mod.rs#L116), with `float_roundtrip` enabled to build correctly rounded floats
- tygr, winnow, and nom build [the shared `JsonValue` enum](src/value.rs) used by the benchmark,
  equivalent to nom's `JsonValue` in
  [rust-bakery/nom](https://github.com/rust-bakery/nom/blob/51c3c4e44fa78a8a09b413419372b97b2cc2a787/benchmarks/benches/json.rs#L22)
  and winnow's `JsonValue` in
  [winnow-rs/winnow](https://github.com/winnow-rs/winnow/blob/21b3ab2535a675768a170a851efda83253b9eb21/examples/json/json.rs#L4)

The parser definitions used are:
- serde_json: upstream [serde-rs/json](https://github.com/serde-rs/json), with conversion checks in [`src/value.rs`](src/value.rs)
- tygr: [`src/tygr_json.rs`](src/tygr_json.rs)
- winnow: [`src/winnow_json.rs`](src/winnow_json.rs), adapted from the upstream parser in
  [winnow-rs/winnow](https://github.com/winnow-rs/winnow/blob/main/examples/json/parser_dispatch.rs)
- nom: [`src/nom_json.rs`](src/nom_json.rs), adapted from the upstream parser in
  [rust-bakery/nom](https://github.com/rust-bakery/nom/blob/master/benchmarks/benches/json.rs)

Minimal error-tracking setup:
- serde_json: default parsing path; detailed diagnostics are only produced on failure
- tygr: default build (no error-tracking feature flags)
- winnow: `EmptyError`
- nom: `Check`

```sh
cargo run --release              # values
cargo run --release --bin trees  # syntax trees
```

Each parser runs 100 times per file, interleaved with the others so drift
affects them all alike. The tables give the median and a 95% confidence
interval for it: the 40th and 61st of the sorted times, which assumes no
distribution.

## Results

Apple M2, rustc 1.98, LTO, one codegen unit; tygr at its `optimization`
branch (`5a3bd8f`), nom 8.0.0, winnow 1.0.5, serde_json 1.0.154.

| MB/s, median (95% CI) | canada | citm_catalog | twitter |
|---|---:|---:|---:|
| serde_json | `358 (358–359)` | `683 (680–689)` | `502 (501–503)` |
| tygr | `427 (426–428)` | `445 (444–446)` | `262 (261–262)` |
| winnow | `395 (394–396)` | `439 (437–440)` | `220 (219–220)` |
| nom | `318 (317–318)` | `333 (332–333)` | `199 (198–200)` |

| vs serde_json, medians | canada | citm_catalog | twitter |
|---|---:|---:|---:|
| serde_json | `1.00×` | `1.00×` | `1.00×` |
| tygr | `1.19×` | `0.65×` | `0.52×` |
| winnow | `1.10×` | `0.64×` | `0.44×` |
| nom | `0.89×` | `0.49×` | `0.40×` |

tygr is faster than winnow (1.01–1.19×) and nom (1.32–1.34×).

## Syntax trees

Parsers like tree-sitter and pest build no value: they build a syntax tree
whose nodes are byte ranges into the input, leaving atoms undecoded.
The second benchmark compares them with a sibling tygr grammar that does
the same:
- tree-sitter: [tree-sitter-json](https://github.com/tree-sitter/tree-sitter-json)'s
  grammar. Every node, punctuation included, has a range, and strings have
  content and escape nodes. Its numbers take no `+` in the exponent
  (none of the files has one), and a tree with error nodes counts as a
  failure.
- tygr: [`src/tygr_tree.rs`](src/tygr_tree.rs), `tygr_json`'s tokens, with
  objects and arrays kept as `Ranged`, and atoms as
  `Ranged<PhantomData<_>>`: checked, not decoded.
- pest: [pest_grammars](https://github.com/pest-parser/pest/blob/master/grammars/src/grammars/json.pest)'
  JSON grammar, whose `Pairs` give each rule's span. Its error tracking
  can't be turned off.

Each tree is checked by converting it to the shared `JsonValue`, with
atoms decoded by serde_json, outside the timing. The benchmark is its own
binary, so tygr's two grammars share no compiled code.

tree-sitter 0.27.2 with tree-sitter-json 0.24.8, pest 2.9.6:

| MB/s, median (95% CI) | canada | citm_catalog | twitter |
|---|---:|---:|---:|
| tree-sitter | `29 (29–29)` | `48 (48–48)` | `37 (37–37)` |
| tygr | `531 (526–533)` | `938 (934–939)` | `849 (845–853)` |
| pest | `82 (81–82)` | `95 (94–96)` | `75 (74–75)` |

| vs tree-sitter, medians | canada | citm_catalog | twitter |
|---|---:|---:|---:|
| tree-sitter | `1.00×` | `1.00×` | `1.00×` |
| tygr | `18.14×` | `19.46×` | `22.70×` |
| pest | `2.79×` | `1.98×` | `1.99×` |

tygr is faster than tree-sitter (18–23×) and pest (6.5–11×).
