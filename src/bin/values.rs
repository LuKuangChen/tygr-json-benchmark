//! JSON pare throughput of serde_json, tygr, winnow and nom; see the README.

#[path = "../harness.rs"]
mod harness;
#[path = "../nom_json.rs"]
mod nom_json;
#[path = "../tygr_json.rs"]
mod tygr_json;
#[path = "../value.rs"]
mod value;
#[path = "../winnow_json.rs"]
mod winnow_json;

use std::hint::black_box;

use harness::Parser;

const PARSERS: [Parser; 4] = [
    (
        "serde_json",
        |s| black_box(serde_value(s)).is_some(),
        |s| serde_value(s).map(Into::into),
    ),
    (
        "tygr",
        |s| black_box(tygr_json::parse(s)).is_some(),
        tygr_json::parse,
    ),
    (
        "winnow",
        |s| black_box(winnow_json::parse(s)).is_some(),
        winnow_json::parse,
    ),
    (
        "nom",
        |s| black_box(nom_json::parse(s)).is_some(),
        nom_json::parse,
    ),
];

fn serde_value(input: &str) -> Option<serde_json::Value> {
    serde_json::from_str(input).ok()
}

/// Checked against serde_json along with the files.
const SAMPLES: [&str; 9] = [
    "0",
    r#"{"a": 1, "b": 2, "a": 3}"#,
    " [ ] ",
    r#"{"a": [1, -2.5e+3, true, false, null], "b": {}}"#,
    r#""\"\\\/\b\f\n\r\t\u00e9 é""#,
    r#""\uD83D\uDE00""#,
    r#""a\u00e9b\uD83D\uDE00c\n""#,
    "[[[[]]]]",
    "-0.0E-0",
];

fn main() {
    harness::run(&PARSERS, &SAMPLES);
}
