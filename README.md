# tygr-json-benchmark

JSON throughput of [tygr](https://github.com/intersystems-community/tygr)
against other Rust parsers on the
[serde-rs/json-benchmark](https://github.com/serde-rs/json-benchmark) corpus.
The benchmark uses these three inputs:
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
cargo run --release
```

Each parser runs 100 times per file, interleaved with the others so drift
affects them all alike. The tables give the median and a 95% confidence
interval for it: the 40th and 61st of the sorted times, which assumes no
distribution.

## Results

Apple M2, rustc 1.98, LTO, one codegen unit; tygr at its `optimization`
branch, nom 8.0.0, winnow 1.0.5, serde_json 1.0.154.

| MB/s, median (95% CI) | canada | citm_catalog | twitter |
|---|---:|---:|---:|
| serde_json | `353 (352–354)` | `671 (666–675)` | `495 (493–497)` |
| tygr | `444 (442–447)` | `454 (452–457)` | `263 (262–263)` |
| winnow | `389 (386–391)` | `433 (432–434)` | `215 (213–217)` |
| nom | `310 (310–311)` | `322 (321–325)` | `198 (198–199)` |

| vs serde_json, medians | canada | citm_catalog | twitter |
|---|---:|---:|---:|
| serde_json | `1.00×` | `1.00×` | `1.00×` |
| tygr | `1.26×` | `0.68×` | `0.53×` |
| winnow | `1.10×` | `0.64×` | `0.43×` |
| nom | `0.88×` | `0.48×` | `0.40×` |

tygr is faster than winnow (1.05–1.22×) and nom (1.33–1.43×).
