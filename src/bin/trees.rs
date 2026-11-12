//! Syntax-tree throughput of tygr, tree-sitter and pest; see the README.
//! Its own binary, so tygr's two grammars share no compiled code.

#[path = "../harness.rs"]
mod harness;
#[allow(dead_code)] // Only its tokens are used here.
#[path = "../tygr_json.rs"]
mod tygr_json;
#[path = "../tygr_tree.rs"]
mod tygr_tree;
#[path = "../value.rs"]
mod value;

use std::cell::RefCell;
use std::hint::black_box;

use harness::Parser;
use pest::Parser as _;
use pest::iterators::Pair;
use pest_grammars::json::{JsonParser, Rule};
use tygr::Wrap;
use tygr_json::Ws;
use tygr_tree::{Comma, Leaf};
use value::JsonValue;

const PARSERS: [Parser; 3] = [
    (
        if cfg!(feature = "trace_one_node") {
            "tygr (trace one node)"
        } else {
            "tygr (no trace)"
        },
        |s| black_box(tygr_tree::parse(s)).is_some(),
        |s| tygr_tree::parse(s).map(|node| tygr_value(&node, s)),
    ),
    (
        "tree-sitter",
        |s| black_box(tree_sitter_parse(s)).is_some(),
        |s| tree_sitter_parse(s).map(|tree| tree_sitter_value(tree.root_node(), s)),
    ),
    (
        "pest",
        |s| black_box(pest_parse(s)).is_some(),
        |s| pest_parse(s).map(|mut pairs| pest_value(pairs.next().unwrap())),
    ),
];

/// As in the values benchmark, but tree-sitter-json's numbers take no `+`
/// in the exponent: `-2.5e+3` is `-2.5e3` here.
const SAMPLES: [&str; 8] = [
    "0",
    " [ ] ",
    r#"{"a": [1, -2.5e3, true, false, null], "b": {}}"#,
    r#""\"\\\/\b\f\n\r\t\u00e9 é""#,
    r#""\uD83D\uDE00""#,
    r#""a\u00e9b\uD83D\uDE00c\n""#,
    "[[[[]]]]",
    "-0.0E-0",
];

fn main() {
    harness::run(&PARSERS, &SAMPLES);
}

thread_local! {
    static TREE_SITTER: RefCell<tree_sitter::Parser> = RefCell::new({
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_json::LANGUAGE.into()).unwrap();
        parser
    });
}

/// `None` on a syntax error, like the others, rather than a tree with
/// error nodes.
fn tree_sitter_parse(input: &str) -> Option<tree_sitter::Tree> {
    let tree = TREE_SITTER.with_borrow_mut(|parser| parser.parse(input, None))?;
    (!tree.root_node().has_error()).then_some(tree)
}

fn pest_parse(input: &str) -> Option<pest::iterators::Pairs<'_, Rule>> {
    JsonParser::parse(Rule::json, input).ok()
}

/// An atom's text, decoded by serde_json so the check is independent.
fn atom(text: &str) -> JsonValue {
    assert_eq!(text.trim(), text, "an atom's range takes in whitespace");
    serde_json::from_str::<serde_json::Value>(text)
        .unwrap()
        .into()
}

fn key(text: &str) -> String {
    match atom(text) {
        JsonValue::Str(key) => key,
        other => panic!("key {other:?}"),
    }
}

/// `text` spans a whole object or array.
fn bracketed(text: &str, open: char, close: char) {
    assert!(
        text.starts_with(open) && text.ends_with(close),
        "{text:.40?}"
    );
}

fn tree_sitter_value(node: tree_sitter::Node, input: &str) -> JsonValue {
    let text = &input[node.byte_range()];
    let mut cursor = node.walk();
    match node.kind() {
        "document" => tree_sitter_value(node.named_child(0).unwrap(), input),
        "object" => {
            bracketed(text, '{', '}');
            let pairs = node.named_children(&mut cursor).map(|pair| {
                let field = |name| pair.child_by_field_name(name).unwrap();
                let key = key(&input[field("key").byte_range()]);
                (key, tree_sitter_value(field("value"), input))
            });
            JsonValue::Object(pairs.collect())
        }
        "array" => {
            bracketed(text, '[', ']');
            let items = node.named_children(&mut cursor);
            JsonValue::Array(items.map(|item| tree_sitter_value(item, input)).collect())
        }
        _ => atom(text),
    }
}

fn tygr_value(node: &tygr_tree::Node, input: &str) -> JsonValue {
    use tygr_tree::Node;
    match node {
        Node::Object(object) => {
            let pairs = tygr_items(object, input, "{", "}").into_iter().map(|pair| {
                assert_eq!(&input[pair.colon.1.range()], ":");
                let key = key(tygr_string(&pair.key, input));
                (key, tygr_value(&pair.value, input))
            });
            JsonValue::Object(pairs.collect())
        }
        Node::Array(array) => {
            let items = tygr_items(array, input, "[", "]").into_iter();
            JsonValue::Array(items.map(|item| tygr_value(item, input)).collect())
        }
        Node::String(string) => atom(tygr_string(string, input)),
        Node::Number(number) => atom(&input[number.range()]),
        Node::True(token) => atom(&input[token.range()]),
        Node::False(token) => atom(&input[token.range()]),
        Node::Null(token) => atom(&input[token.range()]),
    }
}

/// A string's text, once its parts' ranges are checked to tile it.
fn tygr_string<'a>(string: &tygr_tree::JsonString, input: &'a str) -> &'a str {
    use tygr_tree::StringPart;
    assert_eq!(&input[string.open.range()], "\"");
    assert_eq!(&input[string.close.range()], "\"");
    let mut end = string.open.end;
    for part in &string.parts {
        let range = match part {
            StringPart::Content(content) => content.range(),
            StringPart::Escape(escape) => escape.range(),
        };
        assert_eq!(range.start, end);
        let text = &input[range.clone()];
        match part {
            StringPart::Content(_) => assert!(!text.contains(['"', '\\'])),
            StringPart::Escape(_) => assert!(text.starts_with('\\') && text.len() > 1),
        }
        end = range.end;
    }
    assert_eq!(string.close.start, end);
    &input[string.open.start..string.close.end]
}

/// An object's or array's items, once its delimiters' ranges are checked.
fn tygr_items<'a, Open, T, Close>(
    node: &'a Wrap<(Leaf<Open>, Ws), Option<(T, Vec<(Comma, T)>)>, (Ws, Leaf<Close>)>,
    input: &str,
    open: &str,
    close: &str,
) -> Vec<&'a T> {
    assert_eq!(&input[node.before.0.range()], open);
    assert_eq!(&input[node.after.1.range()], close);
    let Some((first, rest)) = &node.wrapped else {
        return Vec::new();
    };
    let mut items = vec![first];
    for ((_, comma, _), item) in rest {
        assert_eq!(&input[comma.range()], ",");
        items.push(item);
    }
    items
}

fn pest_value(pair: Pair<'_, Rule>) -> JsonValue {
    let text = pair.as_str();
    match pair.as_rule() {
        Rule::json | Rule::value => pest_value(pair.into_inner().next().unwrap()),
        Rule::object => {
            bracketed(text, '{', '}');
            let pairs = pair.into_inner().map(|member| {
                let mut parts = member.into_inner();
                let key = key(parts.next().unwrap().as_str());
                (key, pest_value(parts.next().unwrap()))
            });
            JsonValue::Object(pairs.collect())
        }
        Rule::array => {
            bracketed(text, '[', ']');
            JsonValue::Array(pair.into_inner().map(pest_value).collect())
        }
        _ => atom(text),
    }
}
