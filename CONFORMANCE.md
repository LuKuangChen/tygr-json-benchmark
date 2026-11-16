# Conformance

How each parser departs from [json.org](https://www.json.org)'s grammar, on
the 50 inputs in
[`src/bin/conformance.rs`](src/bin/conformance.rs). The corpus uses none of
them. Regenerate this file with
`cargo run --release --bin conformance > CONFORMANCE.md`.

| parser | accepts, though not JSON | rejects, though JSON |
|---|---|---|
| serde_json | none | `1E400`, a 400-digit integer, `"\uD800"`, `"\uDC00"`, `"\uDFFF\uD800"`, arrays nested 128 deep |
| tygr | none | none |
| winnow | `+1`, `01`, `1.`, `-inf`, a raw tab in a string, a raw newline in a string, `"\u+123"` | `"\uD800"`, `"\uDC00"`, `"\uDFFF\uD800"` |
| nom | `+1`, `01`, `1.`, `.5`, `inf`, `nan`, `Infinity`, a raw tab in a string, a raw newline in a string, `"\u+123"` | `"\uD800"`, `"\uDC00"`, `"\uDFFF\uD800"` |
| chumsky | a raw tab in a string, a raw newline in a string, a form feed as whitespace, a vertical tab as whitespace, a no-break space as whitespace | `"\uD800"`, `"\uDC00"`, `"\uDFFF\uD800"` |
| tygr (tree) | none | none |
| tree-sitter | `01`, `1.`, a raw tab in a string, `"\u+123"`, `"\u12"`, a `//` comment, a `/* */` comment, two values (`1 2`), empty input, a form feed as whitespace, a vertical tab as whitespace, a byte-order mark | `0.0e+0` |
| pest | none | none |
