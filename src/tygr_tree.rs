//! `tygr_json`'s sibling: the same JSON syntax, kept as a syntax tree, as
//! tree-sitter and pest keep it. Every token is a leaf that keeps its byte
//! range and nothing else: it's checked but not decoded. Objects, arrays,
//! pairs and strings keep no range of their own: each runs from its first
//! leaf's start to its last leaf's end.

use std::marker::PhantomData;

use tygr::*;

use crate::tygr_json::{Escape, JsonNumber, Unescaped, Ws};

/// A token's range; its text stays in the input.
pub(crate) type Leaf<G> = Ranged<PhantomData<G>>;

pub(crate) type Comma = (Ws, Leaf<StringEq!(",")>, Ws);

#[derive(Parse)]
struct Document(Ws, Node, Ws);

pub(crate) fn parse(input: &str) -> Option<Node> {
    Document::parse::<Value, Max>(input).ok().map(|document| document.1)
}

/// The same parse tracing nothing.
pub(crate) fn parse_untraced(input: &str) -> Option<Node> {
    Document::parse::<Value, Min>(input).ok().map(|document| document.1)
}

/// Objects and arrays are boxed: their first item isn't behind a `Vec`.
#[derive(Parse)]
pub(crate) enum Node {
    Object(
        Box<
            Wrap<
                (Leaf<StringEq!("{")>, Ws),
                Option<(Pair, Vec<(Comma, Pair)>)>,
                (Ws, Leaf<StringEq!("}")>),
            >,
        >,
    ),
    Array(
        Box<
            Wrap<
                (Leaf<StringEq!("[")>, Ws),
                Option<(Node, Vec<(Comma, Node)>)>,
                (Ws, Leaf<StringEq!("]")>),
            >,
        >,
    ),
    String(JsonString),
    Number(Leaf<JsonNumber>),
    True(Leaf<StringEq!("true")>),
    False(Leaf<StringEq!("false")>),
    Null(Leaf<StringEq!("null")>),
}

#[derive(Parse)]
pub(crate) struct Pair {
    pub(crate) key: JsonString,
    pub(crate) colon: (Ws, Leaf<StringEq!(":")>, Ws),
    pub(crate) value: Node,
}

/// As tree-sitter splits it: the quotes, then runs of content and escapes.
/// Unlike tree-sitter, `\uXXXX` is one escape.
#[derive(Parse)]
pub(crate) struct JsonString {
    pub(crate) open: Leaf<StringEq!("\"")>,
    pub(crate) parts: Vec<StringPart>,
    pub(crate) close: Leaf<StringEq!("\"")>,
}

#[derive(Parse)]
pub(crate) enum StringPart {
    Content(Leaf<StringOf1<Unescaped>>),
    Escape(Leaf<Prefix<StringEq!("\\"), Escape>>),
}
