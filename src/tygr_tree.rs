//! `tygr_json`'s sibling: the same JSON syntax, kept as a syntax tree, as
//! tree-sitter and pest keep it. Every atom and delimiter keeps its byte
//! range and nothing else: it's checked but not decoded. An object's or
//! array's range runs from its opening to its closing bracket.

use std::marker::PhantomData;

use tygr::*;

use crate::tygr_json::{Escape, JsonNumber, Unescaped, Ws};

/// An atom's range; its text stays in the input.
pub(crate) type Atom<G> = Ranged<PhantomData<G>>;

pub(crate) type Comma = (Ws, Atom<StringEq!(",")>, Ws);

#[derive(Parse)]
pub(crate) struct Document(Ws, Node, Ws);

pub(crate) fn parse(input: &str) -> Option<Node> {
    Document::parse(input).ok().map(|document| document.1)
}

#[derive(Parse)]
pub(crate) enum Node {
    Object(Wrap<(Atom<StringEq!("{")>, Ws), Option<VecSep<Pair, Comma>>, (Ws, Atom<StringEq!("}")>)>),
    Array(Wrap<(Atom<StringEq!("[")>, Ws), Option<VecSep<Node, Comma>>, (Ws, Atom<StringEq!("]")>)>),
    String(JsonString),
    Number(Atom<JsonNumber>),
    True(Atom<StringEq!("true")>),
    False(Atom<StringEq!("false")>),
    Null(Atom<StringEq!("null")>),
}

/// Its range runs from its key's start to its value's end.
#[derive(Parse)]
pub(crate) struct Pair {
    pub(crate) key: JsonString,
    pub(crate) colon: (Ws, Atom<StringEq!(":")>, Ws),
    pub(crate) value: Node,
}

/// As tree-sitter splits it: the quotes, then runs of content and escapes.
/// Unlike tree-sitter, `\uXXXX` is one escape.
#[derive(Parse)]
pub(crate) struct JsonString {
    pub(crate) open: Atom<StringEq!("\"")>,
    pub(crate) parts: Vec<StringPart>,
    pub(crate) close: Atom<StringEq!("\"")>,
}

#[derive(Parse)]
pub(crate) enum StringPart {
    Content(Atom<StringOf1<Unescaped>>),
    Escape(Atom<Prefix<StringEq!("\\"), Escape>>),
}
