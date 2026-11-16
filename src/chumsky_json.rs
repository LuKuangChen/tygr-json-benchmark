//! chumsky's own JSON example (chumsky 1.0.0-alpha.8, MIT; see `NOTICE`):
//! the parser from `examples/json.rs`, building the shared `JsonValue`
//! (`src/value.rs`, the same shape as its `Json` without `Invalid`), with
//! error recovery and trailing commas removed, strings decoded instead of
//! kept as text, `EmptyErr` as the error type, and `parse` added.

use std::collections::HashMap;

use chumsky::prelude::*;

use crate::value::JsonValue;

type E = extra::Err<EmptyErr>;

/// A run of unescaped chars, or a run of escapes, decoded.
enum Chunk<'a> {
    Text(&'a str),
    Escaped(String),
}

fn parser<'a>() -> impl Parser<'a, &'a str, JsonValue, E> {
    recursive(|value| {
        let digits = text::digits(10).to_slice();
        let frac = just('.').then(digits);
        let exp = just('e')
            .or(just('E'))
            .then(one_of("+-").or_not())
            .then(digits);
        let number = just('-')
            .or_not()
            .then(text::int(10))
            .then(frac.or_not())
            .then(exp.or_not())
            .to_slice()
            .map(|s: &str| s.parse().unwrap());

        // One escape is one UTF-16 code unit; a run of them is decoded
        // together, so a surrogate pair becomes one char.
        let escape = just('\\').ignore_then(choice((
            just('\\').to(b'\\' as u16),
            just('/').to(b'/' as u16),
            just('"').to(b'"' as u16),
            just('b').to(0x08),
            just('f').to(0x0C),
            just('n').to(b'\n' as u16),
            just('r').to(b'\r' as u16),
            just('t').to(b'\t' as u16),
            just('u').ignore_then(
                text::digits(16)
                    .exactly(4)
                    .to_slice()
                    .map(|digits: &str| u16::from_str_radix(digits, 16).unwrap()),
            ),
        )));
        let escapes = escape
            .repeated()
            .at_least(1)
            .collect::<Vec<u16>>()
            .try_map(|units, _| String::from_utf16(&units).map_err(|_| EmptyErr::default()));
        let text = none_of("\\\"").repeated().at_least(1).to_slice();
        let chunk = choice((text.map(Chunk::Text), escapes.map(Chunk::Escaped)));
        let string = empty()
            .map(|()| String::new())
            .foldl(chunk.repeated(), |mut s, chunk| {
                match chunk {
                    Chunk::Text(text) => s.push_str(text),
                    Chunk::Escaped(decoded) => s.push_str(&decoded),
                }
                s
            })
            .delimited_by(just('"'), just('"'));

        let array = value
            .clone()
            .separated_by(just(',').padded())
            .collect::<Vec<JsonValue>>()
            .padded()
            .delimited_by(just('['), just(']'));

        let member = string.clone().then_ignore(just(':').padded()).then(value);
        let object = member
            .separated_by(just(',').padded())
            .collect::<HashMap<String, JsonValue>>()
            .padded()
            .delimited_by(just('{'), just('}'));

        choice((
            just("null").to(JsonValue::Null),
            just("true").to(JsonValue::Bool(true)),
            just("false").to(JsonValue::Bool(false)),
            number.map(JsonValue::Num),
            string.map(JsonValue::Str),
            array.map(JsonValue::Array),
            object.map(JsonValue::Object),
        ))
        .padded()
    })
}

pub(crate) fn parse(input: &str) -> Option<JsonValue> {
    parser().parse(input).into_result().ok()
}
