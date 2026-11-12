//! How each parser departs from json.org's grammar, as Markdown:
//! `cargo run --release --bin conformance > CONFORMANCE.md`.

#[path = "../nom_json.rs"]
mod nom_json;
#[allow(dead_code)] // Only `parse` is used here.
#[path = "../tygr_json.rs"]
mod tygr_json;
#[allow(dead_code)] // Only `parse` is used here.
#[path = "../tygr_tree.rs"]
mod tygr_tree;
#[path = "../value.rs"]
mod value;
#[path = "../winnow_json.rs"]
mod winnow_json;

use pest::Parser as _;
use pest_grammars::json::{JsonParser, Rule};

/// An input, how to show it, and whether json.org's grammar allows it.
struct Case {
    shown: String,
    input: String,
    valid: bool,
}

/// An input shown as itself.
fn plain(input: &str, valid: bool) -> Case {
    named(&format!("`{input}`"), input, valid)
}

/// An input shown by a description.
fn named(shown: &str, input: impl Into<String>, valid: bool) -> Case {
    Case {
        shown: shown.to_owned(),
        input: input.into(),
        valid,
    }
}

fn cases() -> Vec<Case> {
    let nested = |depth: usize| format!("{}{}", "[".repeat(depth), "]".repeat(depth));
    vec![
        plain("-0", true),
        plain("0.0e+0", true),
        plain("1E400", true),
        plain("1e-400", true),
        named("a 400-digit integer", "1".repeat(400), true),
        plain(r#""\u0000""#, true),
        plain(r#""\uD800""#, true),
        plain(r#""\uDC00""#, true),
        plain(r#""\uDFFF\uD800""#, true),
        plain(r#""é""#, true),
        plain(r#""\/""#, true),
        named("a raw DEL (U+007F) in a string", "\"\u{7f}\"", true),
        plain(r#"{"":0}"#, true),
        named("all four whitespace characters", "[\r\n\t ]", true),
        named("arrays nested 128 deep", nested(128), true),
        plain("+1", false),
        plain("01", false),
        plain("1.", false),
        plain(".5", false),
        plain("-", false),
        plain("1e", false),
        plain("1e+", false),
        plain("0x10", false),
        plain("inf", false),
        plain("-inf", false),
        plain("nan", false),
        plain("Infinity", false),
        plain("TRUE", false),
        plain("nul", false),
        named("a raw tab in a string", "\"a\tb\"", false),
        named("a raw newline in a string", "\"a\nb\"", false),
        plain(r#""\u+123""#, false),
        plain(r#""\u12""#, false),
        plain(r#""\x41""#, false),
        plain(r#""\'""#, false),
        plain("'a'", false),
        plain("[1,]", false),
        plain(r#"{"a":1,}"#, false),
        plain("[1 2]", false),
        plain(r#"{"a" 1}"#, false),
        plain("{a:1}", false),
        named("a `//` comment", "// c\n1", false),
        named("a `/* */` comment", "/* c */1", false),
        named("two values (`1 2`)", "1 2", false),
        named("empty input", "", false),
        plain("[1] x", false),
        named("a form feed as whitespace", " \u{c}1", false),
        named("a vertical tab as whitespace", " \u{b}1", false),
        named("a no-break space as whitespace", "\u{a0}1", false),
        named("a byte-order mark", "\u{feff}1", false),
    ]
}

fn tree_sitter(input: &str) -> bool {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_json::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(input, None);
    tree.is_some_and(|tree| !tree.root_node().has_error())
}

const PARSERS: [(&str, fn(&str) -> bool); 7] = [
    ("serde_json", |s| {
        serde_json::from_str::<serde_json::Value>(s).is_ok()
    }),
    ("tygr", |s| tygr_json::parse(s).is_some()),
    ("winnow", |s| winnow_json::parse(s).is_some()),
    ("nom", |s| nom_json::parse(s).is_some()),
    ("tygr (tree)", |s| tygr_tree::parse(s).is_some()),
    ("tree-sitter", tree_sitter),
    ("pest", |s| JsonParser::parse(Rule::json, s).is_ok()),
];

fn main() {
    let cases = cases();
    println!("# Conformance\n");
    println!("How each parser departs from [json.org](https://www.json.org)'s grammar, on");
    println!("the {} inputs in", cases.len());
    println!("[`src/bin/conformance.rs`](src/bin/conformance.rs). The corpus uses none of");
    println!("them. Regenerate this file with");
    println!("`cargo run --release --bin conformance > CONFORMANCE.md`.\n");
    println!("| parser | accepts, though not JSON | rejects, though JSON |");
    println!("|---|---|---|");
    for (name, accepts) in PARSERS {
        // The cases of one kind that it gets wrong.
        let wrong = |valid: bool| {
            let shown: Vec<&str> = cases
                .iter()
                .filter(|case| case.valid == valid && accepts(&case.input) != valid)
                .map(|case| case.shown.as_str())
                .collect();
            if shown.is_empty() {
                "none".to_owned()
            } else {
                shown.join(", ")
            }
        };
        println!("| {name} | {} | {} |", wrong(false), wrong(true));
    }
}
