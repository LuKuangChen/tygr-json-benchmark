# tygr-json-benchmark

JSON throughput of [tygr](https://github.com/intersystems-community/tygr)
against other parsers usable from Rust, in two benchmarks: building a JSON
value, and building a tree of byte ranges. serde_json's parser is written by
hand, as fast parsers usually are, so it is the baseline for values, and
both benchmarks use its corpus, from
[serde-rs/json-benchmark](https://github.com/serde-rs/json-benchmark):
- [canada.json](https://raw.githubusercontent.com/serde-rs/json-benchmark/master/data/canada.json)
- [citm_catalog.json](https://raw.githubusercontent.com/serde-rs/json-benchmark/master/data/citm_catalog.json)
- [twitter.json](https://raw.githubusercontent.com/serde-rs/json-benchmark/master/data/twitter.json)

```sh
cargo run --release --bin values
cargo run --release --bin trees
```

Each benchmark is a separate binary, so the parsers in one can't change how
the other's are compiled. Each first checks every parser's output against
serde_json's, on the files and on a few small samples. Then every parser
parses each file 100 times, taking turns with the others so that any drift
in speed affects them all alike. The tables give the median speed and, in
parentheses, a 95% confidence interval for it: the 40th and 61st of the
sorted speeds. This interval comes from the sign test, which assumes nothing
about how speeds are distributed. [`src/harness.rs`](src/harness.rs) has the
check, the timing and the tables. All numbers are from an Apple M2, with
rustc 1.97.1, LTO and one codegen unit, and tygr 0.9.0.

## Values

This benchmark, [`src/bin/values.rs`](src/bin/values.rs), runs each parser
with the least error tracking it offers, and checks that they all build the
same JSON values.

Each parser builds a JSON value with escapes decoded and numbers as
correctly rounded `f64`:
- serde_json builds its own
  [`serde_json::Value`](https://github.com/serde-rs/json/blob/8c535a260ce561eaab4111aafc29038eb9f77ff3/src/value/mod.rs#L116),
  with `float_roundtrip` on so that floats are correctly rounded.
- tygr, winnow and nom build the shared [`JsonValue`](src/value.rs), the
  same as nom's
  [`JsonValue`](https://github.com/rust-bakery/nom/blob/51c3c4e44fa78a8a09b413419372b97b2cc2a787/benchmarks/benches/json.rs#L22)
  and winnow's
  [`JsonValue`](https://github.com/winnow-rs/winnow/blob/21b3ab2535a675768a170a851efda83253b9eb21/examples/json/json.rs#L4).

The parsers:
- serde_json: [serde-rs/json](https://github.com/serde-rs/json) itself; its
  value is converted for the check in [`src/value.rs`](src/value.rs).
- tygr: [`src/tygr_json.rs`](src/tygr_json.rs).
- winnow: [`src/winnow_json.rs`](src/winnow_json.rs), adapted from
  [winnow's example](https://github.com/winnow-rs/winnow/blob/21b3ab2535a675768a170a851efda83253b9eb21/examples/json/parser_dispatch.rs).
- nom: [`src/nom_json.rs`](src/nom_json.rs), adapted from
  [nom's benchmark](https://github.com/rust-bakery/nom/blob/51c3c4e44fa78a8a09b413419372b97b2cc2a787/benchmarks/benches/json.rs).

Their error tracking, set to the least each offers:
- serde_json: the default; it builds detailed errors only on failure.
- tygr: `level::Min`, which traces nothing.
- winnow: `EmptyError`.
- nom: `Check`.

tygr follows [json.org](https://www.json.org)'s grammar exactly. The others
depart from it in ways the corpus doesn't use, all listed in
[`CONFORMANCE.md`](CONFORMANCE.md). Some change how much a parser checks:
- serde_json checks more: it limits nesting to 127 levels, which guards the
  stack (tygr, winnow and nom have no such guard), and rejects numbers
  beyond `f64`'s range.
- winnow and nom check less: they let raw control characters into strings.
  Their number parsers also accept forms JSON doesn't, such as `+1` and
  `01`.

serde_json 1.0.154, winnow 1.0.5, nom 8.0.0:

| MB/s, median (95% CI) | canada | citm_catalog | twitter |
|---|---:|---:|---:|
| serde_json | `439 (438–440)` | `910 (904–913)` | `636 (633–638)` |
| tygr | `443 (442–446)` | `563 (561–565)` | `333 (332–335)` |
| winnow | `536 (534–539)` | `550 (548–551)` | `275 (275–276)` |
| nom | `407 (406–408)` | `403 (402–404)` | `256 (256–257)` |

| vs serde_json, medians | canada | citm_catalog | twitter |
|---|---:|---:|---:|
| serde_json | `1.00×` | `1.00×` | `1.00×` |
| tygr | `1.01×` | `0.62×` | `0.52×` |
| winnow | `1.22×` | `0.60×` | `0.43×` |
| nom | `0.93×` | `0.44×` | `0.40×` |

Neither tygr nor winnow is faster on every file: winnow is faster on canada,
tygr on citm_catalog and twitter. Both are faster than nom on all three
files.

## Trees of Ranges

Some parser generators output a tree of byte ranges into the input. The tree
holds the structure, but to read a string or number you need the input too.
This benchmark, [`src/bin/trees.rs`](src/bin/trees.rs), has each system
build such a tree from a JSON grammar:
- tygr: [`src/tygr_tree.rs`](src/tygr_tree.rs), which reuses `tygr_json`'s
  tokens. Its tree is typed: an enum for each value, a struct for each pair
  or string, and a `Ranged<PhantomData<_>>` for each token. That keeps the
  token's byte range: the token is checked but not decoded.
- tree-sitter: [tree-sitter-json](https://github.com/tree-sitter/tree-sitter-json)'s
  grammar, unchanged. Its nodes all share one type, `Node`: each has a kind
  and a range, in bytes and in rows and columns.
- pest: [pest_grammars](https://github.com/pest-parser/pest/blob/master/grammars/src/grammars/json.pest)'
  JSON grammar. Its `Pairs` give each rule it matched and its byte range,
  nested like the rules. Delimiters, and the parts inside strings and
  numbers, get no pairs of their own. A pair's key and value are found by
  position, not by name. Each parse also records where every line starts,
  so any span's line and column can be looked up.

Read these numbers with more care than the values benchmark's:
- Outputs differ, as described above, and also:
  - tree-sitter keeps what it needs to reuse parts of the tree after an
    edit. That is what it is designed for, and it isn't measured here.
  - tree-sitter splits `\u` from its hex digits, while tygr keeps `\uXXXX`
    as one escape, as [json.org](https://www.json.org)'s grammar does.
- Error handling differs:
  - tygr and pest reject bad input, while tree-sitter recovers from it with
    error nodes.
  - pest always records where parsing failed. tygr does this at
    `level::Max`, its default and the first tygr row; the second row,
    `level::Min`, traces nothing.
- Safety differs: pest checks the remaining stack on every rule, so input
  nested too deeply fails with an error. tygr has no such check, and input
  nested deeply enough overflows the stack. pest's check comes with its
  default `std` feature, and pest_grammars turns it on too; built without
  `std`, pest's JSON grammar ran about 50% faster here.
- Grammars differ: tygr and pest follow json.org's grammar exactly.
  tree-sitter-json's accepts more, such as comments between any two tokens,
  which its lexer has to look for. [`CONFORMANCE.md`](CONFORMANCE.md) lists
  every difference; the corpus uses none of them.

Each tree is checked by turning it into the shared
[`JsonValue`](src/value.rs), with serde_json decoding its strings, numbers,
`true`, `false` and `null`. For tygr, the check also confirms each
delimiter's text, and that each string's parts cover it end to end, with no
gaps.

tree-sitter 0.27.2 with tree-sitter-json 0.24.8, pest and pest_grammars
2.9.6:

| MB/s, median (95% CI) | canada | citm_catalog | twitter |
|---|---:|---:|---:|
| tygr (Min) | `522 (514–527)` | `706 (698–710)` | `589 (583–591)` |
| tygr (Max) | `506 (503–511)` | `660 (657–666)` | `538 (534–541)` |
| pest | `92 (91–93)` | `106 (105–106)` | `80 (80–81)` |
| tree-sitter | `34 (34–34)` | `60 (59–61)` | `45 (45–46)` |

pest has slowed down over its releases: on this machine, with the same
grammar, pest 2.1.3 parsed the corpus at 395–663 MB/s.
