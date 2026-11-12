//! `tygr_json`'s sibling: the same JSON syntax, kept as a syntax tree, as
//! tree-sitter and pest keep it. Every node keeps its byte range, and an
//! atom keeps nothing else: it's checked but not decoded.

use std::marker::PhantomData;

use tygr::*;

use crate::tygr_json::{Comma, JsonNumber, JsonString, Ws};

/// An atom's range; its text stays in the input.
pub(crate) type Atom<G> = Ranged<PhantomData<G>>;

#[derive(Parse)]
pub(crate) struct Document(Ws, Node, Ws);

pub(crate) fn parse(input: &str) -> Option<Node> {
    Document::parse(input).ok().map(|document| document.1)
}

#[derive(Parse)]
pub(crate) enum Node {
    Object(Ranged<Wrap<(StringEq!("{"), Ws), Option<VecSep<Pair, Comma>>, (Ws, StringEq!("}"))>>),
    Array(Ranged<Wrap<(StringEq!("["), Ws), Option<VecSep<Node, Comma>>, (Ws, StringEq!("]"))>>),
    String(Atom<JsonString>),
    Number(Atom<JsonNumber>),
    True(Atom<StringEq!("true")>),
    False(Atom<StringEq!("false")>),
    Null(Atom<StringEq!("null")>),
}

/// Its range runs from its key's start to its value's end.
#[derive(Parse)]
pub(crate) struct Pair {
    pub(crate) key: Atom<JsonString>,
    colon: (Ws, StringEq!(":"), Ws),
    pub(crate) value: Node,
}
