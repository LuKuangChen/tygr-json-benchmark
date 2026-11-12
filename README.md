# tygr-json-benchmark

JSON throughput of [tygr](https://github.com/intersystems-community/tygr)
against other Rust parsers, in two benchmarks: building a JSON value, and
building a tree of source ranges. Both run on the
[serde-rs/json-benchmark](https://github.com/serde-rs/json-benchmark) corpus.
serde_json's parser is written by hand, as fast parsers usually are, so it is
the baseline for values, and the inputs are the ones its own benchmark uses:
- [canada.json](https://raw.githubusercontent.com/serde-rs/json-benchmark/master/data/canada.json)
- [citm_catalog.json](https://raw.githubusercontent.com/serde-rs/json-benchmark/master/data/citm_catalog.json)
- [twitter.json](https://raw.githubusercontent.com/serde-rs/json-benchmark/master/data/twitter.json)

```sh
cargo run --release --bin values
cargo run --release --bin trees
```

Each benchmark is its own binary, so no parser shares compiled code with one
it isn't compared against. Both first check every parser's output against
serde_json's value, on the files and on a few samples. Then each parser runs
100 times per file, interleaved with the others so drift affects them all
alike. The tables give the median and a 95% confidence interval for it: the
40th and 61st of the sorted times, which assumes no distribution.

Layout:
- [`src/bin/values.rs`](src/bin/values.rs), [`src/bin/trees.rs`](src/bin/trees.rs): the two benchmarks
- [`src/harness.rs`](src/harness.rs): the check, the timing and the tables they share
- [`src/value.rs`](src/value.rs): the shared `JsonValue`, also tygr's value grammar
- [`src/tygr_json.rs`](src/tygr_json.rs), [`src/tygr_tree.rs`](src/tygr_tree.rs): tygr's two grammars
- [`src/winnow_json.rs`](src/winnow_json.rs), [`src/nom_json.rs`](src/nom_json.rs): the upstream winnow and nom parsers

## Values

This benchmark compares the lowest-cost error-tracking mode each parser exposes,
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

tygr ties winnow on citm_catalog and is faster on canada and twitter
(1.08×, 1.19×). It is faster than nom on all three (1.32–1.34×).

## Trees of Ranges

Parsers like tree-sitter and pest don't build a value ready for use on
their own. Rather, they build a tree of byte ranges into the input: rich in
structure, but not decoded. This benchmark compares them with a sibling tygr
grammar that does the same:
- tygr: [`src/tygr_tree.rs`](src/tygr_tree.rs), `tygr_json`'s tokens, each
  kept as `Ranged<PhantomData<_>>`: checked, not decoded.
- tree-sitter: [tree-sitter-json](https://github.com/tree-sitter/tree-sitter-json)'s
  grammar, unchanged. It differs from JSON in ways the corpus doesn't
  exercise: it accepts comments, any number of top-level values and any
  whitespace, and rejects `+` in exponents, so its samples use `-2.5e3`.
- pest: [pest_grammars](https://github.com/pest-parser/pest/blob/master/grammars/src/grammars/json.pest)'
  JSON grammar, whose `Pairs` give each rule's span.

tygr's and tree-sitter's trees have the same shape: a range for every
bracket, comma, colon and quote, strings split into content runs and escapes,
and each pair's `key` and `value` named. The one difference: tree-sitter
splits `\u` from its hex digits, where tygr keeps `\uXXXX` as one escape.

pest's tree keeps less: no delimiters, each string or number as one node
(the grammar makes them atomic), and a pair's key and value by position, not
name. It adds a `value` node around every value, and an index of line starts,
so each span can give its line and column.

Interpret these numbers with more caution than the values benchmark:
- Error handling differs:
  - tygr and pest reject ill-formed input, whereas tree-sitter recovers from
    it with error nodes;
  - pest always records where it failed. tygr does so with its default
    `trace_one_node` feature, measured in a second run below
    (`cargo run --release --bin trees --features trace_one_node`).
- tree-sitter keeps more, for incremental reparsing: rows and columns, and
  what it needs to reuse subtrees. Reparsing after an edit, its strength,
  isn't measured here.

Each tree is checked by converting it to the shared `JsonValue`, with atoms
decoded by serde_json, and tygr's delimiter and string-part ranges checked to
tile the input.

tree-sitter 0.27.2 with tree-sitter-json 0.24.8, pest 2.9.6. The
`trace_one_node` row is from a second run, with that feature:

| MB/s, median (95% CI) | canada | citm_catalog | twitter |
|---|---:|---:|---:|
| tygr | `373 (370–375)` | `515 (511–520)` | `390 (385–393)` |
| tygr (trace_one_node) | `272 (270–274)` | `392 (390–396)` | `310 (309–312)` |
| tree-sitter | `29 (29–29)` | `48 (48–48)` | `37 (37–37)` |
| pest | `81 (81–82)` | `94 (94–95)` | `74 (74–74)` |
