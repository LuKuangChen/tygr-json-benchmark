//! JSON (json.org) in tygr. `JsonValue` (in `value.rs`) is itself the
//! grammar: its variants convert from the syntax here with
//! `#[tygr(try_from_infallible = ...)]` and friends. Character classes
//! pick single chars and `StringOf` matches runs.

use std::collections::HashMap;
use std::marker::PhantomData;

use tygr::*;

use crate::value::JsonValue;

char_class!(pub(crate) Digit, "digit", |c| c.is_ascii_digit());
char_class!(pub(crate) OneNine, "onenine", |c| matches!(c, '1'..='9'));
char_class!(pub(crate) Sign, "sign", |c| matches!(c, '+' | '-'));
char_class!(pub(crate) WsChar, "ws", |c| matches!(c, ' ' | '\t' | '\n' | '\r'));
char_class!(pub(crate) Unescaped, "unescaped", |c| c >= ' ' && c != '"' && c != '\\');
char_class!(pub(crate) SimpleEscape, "simple-escape", |c| {
    matches!(c, '"' | '\\' | '/' | 'b' | 'f' | 'n' | 'r' | 't')
});
char_class!(pub(crate) Hex, "hex", |c| c.is_ascii_hexdigit());

/// Scanned, never stored.
pub(crate) type Ws = PhantomData<StringOf<WsChar>>;

type Comma = (Ws, StringEq!(","), Ws);

#[derive(Parse)]
struct Json(Ws, JsonValue, Ws);

pub(crate) fn parse(input: &str) -> Option<JsonValue> {
    Json::parse(input).ok().map(|json| json.1)
}

/// The same parse tracing nothing: a failure says only that it failed.
pub(crate) fn parse_untraced(input: &str) -> Option<JsonValue> {
    Json::parse_with::<success::Value, level::Min>(input).ok().map(|json| json.1)
}

#[derive(Parse)]
pub(crate) struct JsonObject(
    Wrap<
        (StringEq!("{"), Ws),
        Option<(JsonMember, Vec<(Comma, JsonMember)>)>,
        (Ws, StringEq!("}")),
    >,
);

#[derive(Parse)]
pub(crate) struct JsonMember {
    #[tygr(try_from_infallible = JsonString)]
    key: String,
    colon: (Ws, StringEq!(":"), Ws),
    value: JsonValue,
}

impl From<JsonObject> for HashMap<String, JsonValue> {
    fn from(object: JsonObject) -> Self {
        let members = object.0.into_inner().into_iter().flat_map(|(first, rest)| {
            std::iter::once(first).chain(rest.into_iter().map(|(_, member)| member))
        });
        members.map(|m| (m.key, m.value)).collect()
    }
}

#[derive(Parse)]
pub(crate) struct JsonArray(
    Wrap<(StringEq!("["), Ws), Option<(JsonValue, Vec<(Comma, JsonValue)>)>, (Ws, StringEq!("]"))>,
);

impl From<JsonArray> for Vec<JsonValue> {
    fn from(array: JsonArray) -> Self {
        let Some((first, rest)) = array.0.into_inner() else {
            return Vec::new();
        };
        std::iter::once(first)
            .chain(rest.into_iter().map(|(_, item)| item))
            .collect()
    }
}

/// The leading run, then each escape sequence with the run after it.
/// Without escapes the `Vec` stays empty, so it never allocates.
#[derive(Parse)]
pub(crate) struct JsonString(
    StringEq!("\""),
    StringOf<Unescaped>,
    Vec<(EscapeSequence, StringOf<Unescaped>)>,
    StringEq!("\""),
);

/// Adjacent escapes; a surrogate pair is always within one sequence.
#[derive(Parse)]
pub(crate) struct EscapeSequence(Vec1<Prefix<StringEq!("\\"), Escape>>);

#[derive(Parse)]
pub(crate) enum Escape {
    Simple(SimpleEscape),
    Unicode(StringEq!("u"), Hex, Hex, Hex, Hex),
}

/// One escape is one UTF-16 code unit.
impl From<&Escape> for u16 {
    fn from(escape: &Escape) -> u16 {
        match escape {
            Escape::Simple(c) => match c.0 {
                'b' => '\x08' as u16,
                'f' => '\x0C' as u16,
                'n' => '\n' as u16,
                'r' => '\r' as u16,
                't' => '\t' as u16,
                c => c as u16,
            },
            Escape::Unicode(_, a, b, c, d) => {
                let hex: String = [a, b, c, d].iter().map(|h| h.0).collect();
                u16::from_str_radix(&hex, 16).unwrap()
            }
        }
    }
}

/// A lone surrogate becomes U+FFFD.
impl From<EscapeSequence> for String {
    fn from(sequence: EscapeSequence) -> String {
        let units: Vec<u16> = sequence
            .0
            .iter()
            .map(|escape| u16::from(&**escape))
            .collect();
        String::from_utf16_lossy(&units)
    }
}

impl From<JsonString> for String {
    fn from(JsonString(_, run, sequences, _): JsonString) -> Self {
        let mut out = run.into_inner();
        for (sequence, run) in sequences {
            out.push_str(&String::from(sequence));
            out.push_str(&run);
        }
        out
    }
}

#[derive(Parse)]
pub(crate) enum JsonBool {
    True(StringEq!("true")),
    False(StringEq!("false")),
}

impl From<JsonBool> for bool {
    fn from(syntax: JsonBool) -> bool {
        match syntax {
            JsonBool::True(_) => true,
            JsonBool::False(_) => false,
        }
    }
}

#[derive(Parse)]
pub(crate) enum Integer {
    Zero(StringEq!("0")),
    NonZero((OneNine, StringOf<Digit>)),
}

#[derive(Parse)]
pub(crate) struct Exponent {
    e: StringEqCI!("e"),
    sign: Option<Sign>,
    digits: StringOf1<Digit>,
}

#[derive(Parse)]
pub(crate) struct JsonNumber {
    minus: Option<StringEq!("-")>,
    integer: Integer,
    fraction: Option<Prefix<StringEq!("."), StringOf1<Digit>>>,
    exponent: Option<Exponent>,
}
