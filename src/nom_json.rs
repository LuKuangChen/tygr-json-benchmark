//! nom's own JSON parser, from its `benchmarks/benches/json.rs` at
//! `51c3c4e` (nom 8, MIT; see `NOTICE`), minus the bench harness, and
//! building the shared `JsonValue` (`src/value.rs`, the same shape) in place
//! of its own copy. Kept in nom's formatting.

#![cfg_attr(rustfmt, rustfmt::skip)]

use nom::{
  branch::alt,
  bytes::{tag, take},
  character::{anychar, char, multispace0, none_of},
  combinator::{map, map_opt, map_res, value, verify},
  error::{Error, FromExternalError, ParseError},
  multi::{fold, separated_list0},
  number::double,
  sequence::{delimited, preceded, separated_pair},
  Check, Complete, Emit, Mode, OutputM, Parser,
};

use std::{collections::HashMap, marker::PhantomData, num::ParseIntError};

use crate::value::JsonValue;

fn boolean<'a, E: ParseError<&'a str>>() -> impl Parser<&'a str, Output = bool, Error = E> {
  alt((value(false, tag("false")), value(true, tag("true"))))
}

fn u16_hex<'a, E: ParseError<&'a str> + FromExternalError<&'a str, ParseIntError>>(
) -> impl Parser<&'a str, Output = u16, Error = E> {
  map_res(take(4usize), |s| u16::from_str_radix(s, 16))
}

fn unicode_escape<'a, E: ParseError<&'a str> + FromExternalError<&'a str, ParseIntError>>(
) -> impl Parser<&'a str, Output = char, Error = E> {
  map_opt(
    alt((
      // Not a surrogate
      map(
        verify(u16_hex(), |cp| !(0xD800..0xE000).contains(cp)),
        |cp| cp as u32,
      ),
      // See https://en.wikipedia.org/wiki/UTF-16#Code_points_from_U+010000_to_U+10FFFF for details
      map(
        verify(
          separated_pair(u16_hex(), tag("\\u"), u16_hex()),
          |(high, low)| (0xD800..0xDC00).contains(high) && (0xDC00..0xE000).contains(low),
        ),
        |(high, low)| {
          let high_ten = (high as u32) - 0xD800;
          let low_ten = (low as u32) - 0xDC00;
          (high_ten << 10) + low_ten + 0x10000
        },
      ),
    )),
    // Could probably be replaced with .unwrap() or _unchecked due to the verify checks
    std::char::from_u32,
  )
}

fn character<
  'a,
  E: ParseError<&'a str> + FromExternalError<&'a str, ParseIntError> + FromExternalError<&'a str, ()>,
>() -> impl Parser<&'a str, Output = char, Error = E> {
  Character { e: PhantomData }
  /*let (input, c) = none_of("\"")(input)?;
  if c == '\\' {
    alt((
      map_res(anychar, |c| {
        Ok(match c {
          '"' | '\\' | '/' => c,
          'b' => '\x08',
          'f' => '\x0C',
          'n' => '\n',
          'r' => '\r',
          't' => '\t',
          _ => return Err(()),
        })
      }),
      preceded(char('u'), unicode_escape()),
    ))
    .parse(input)
  } else {
    Ok((input, c))
  }*/
}

struct Character<E> {
  e: PhantomData<E>,
}

impl<'a, E> Parser<&'a str> for Character<E>
where
  E: ParseError<&'a str>
    + FromExternalError<&'a str, ParseIntError>
    + FromExternalError<&'a str, ()>,
{
  type Output = char;

  type Error = E;

  fn process<OM: nom::OutputMode>(
    &mut self,
    input: &'a str,
  ) -> nom::PResult<OM, &'a str, Self::Output, Self::Error> {
    let (input, c): (&str, char) =
      none_of("\"").process::<OutputM<Emit, OM::Error, OM::Incomplete>>(input)?;
    if c == '\\' {
      alt((
        map_res(anychar, |c| {
          Ok(match c {
            '"' | '\\' | '/' => c,
            'b' => '\x08',
            'f' => '\x0C',
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            _ => return Err(()),
          })
        }),
        preceded(char('u'), unicode_escape()),
      ))
      .process::<OM>(input)
    } else {
      Ok((input, OM::Output::bind(|| c)))
    }
  }
}

fn string<
  'a,
  E: ParseError<&'a str> + FromExternalError<&'a str, ParseIntError> + FromExternalError<&'a str, ()>,
>() -> impl Parser<&'a str, Output = String, Error = E> {
  delimited(
    char('"'),
    fold(0.., character(), String::new, |mut string, c| {
      string.push(c);
      string
    }),
    char('"'),
  )
}

fn ws<
  'a,
  O,
  E: ParseError<&'a str> + FromExternalError<&'a str, ParseIntError> + FromExternalError<&'a str, ()>,
  F: Parser<&'a str, Output = O, Error = E>,
>(
  f: F,
) -> impl Parser<&'a str, Output = O, Error = E> {
  delimited(multispace0(), f, multispace0())
}

fn array<
  'a,
  E: ParseError<&'a str> + FromExternalError<&'a str, ParseIntError> + FromExternalError<&'a str, ()>,
>() -> impl Parser<&'a str, Output = Vec<JsonValue>, Error = E> {
  delimited(
    char('['),
    ws(separated_list0(ws(char(',')), json_value())),
    char(']'),
  )
}

fn object<
  'a,
  E: ParseError<&'a str> + FromExternalError<&'a str, ParseIntError> + FromExternalError<&'a str, ()>,
>() -> impl Parser<&'a str, Output = HashMap<String, JsonValue>, Error = E> {
  map(
    delimited(
      char('{'),
      ws(separated_list0(
        ws(char(',')),
        separated_pair(string(), ws(char(':')), json_value()),
      )),
      char('}'),
    ),
    |key_values| key_values.into_iter().collect(),
  )
}

/*
fn json_value<'a, E: ParseError<&'a str> + FromExternalError<&'a str, ParseIntError> + FromExternalError<&'a str, ()>,
>() -> Box<dyn Parser<&'a str, Output = JsonValue, Error = E>> {
  use JsonValue::*;

  Box::new(alt((
    value(Null, tag("null")),
    map(boolean(), Bool),
    map(string(), Str),
    map(double, Num),
    map(array(), Array),
    map(object(), Object),
  )))
}
*/

fn json_value<
  'a,
  E: ParseError<&'a str> + FromExternalError<&'a str, ParseIntError> + FromExternalError<&'a str, ()>,
>() -> JsonParser<E> {
  JsonParser { e: PhantomData }
}

struct JsonParser<E> {
  e: PhantomData<E>,
}

// the main Parser implementation is done explicitely on a real type,
// because haaving json_value return `impl Parser` would result in
// "recursive opaque type" errors
impl<'a, E> Parser<&'a str> for JsonParser<E>
where
  E: ParseError<&'a str>
    + FromExternalError<&'a str, ParseIntError>
    + FromExternalError<&'a str, ()>,
{
  type Output = JsonValue;
  type Error = E;

  fn process<OM: nom::OutputMode>(
    &mut self,
    input: &'a str,
  ) -> nom::PResult<OM, &'a str, Self::Output, Self::Error> {
    use JsonValue::*;

    alt((
      value(Null, tag("null")),
      map(boolean(), Bool),
      map(string(), Str),
      map(double(), Num),
      map(array(), Array),
      map(object(), Object),
    ))
    .process::<OM>(input)
  }
}

fn json<
  'a,
  E: ParseError<&'a str> + FromExternalError<&'a str, ParseIntError> + FromExternalError<&'a str, ()>,
>() -> impl Parser<&'a str, Output = JsonValue, Error = E> {
  ws(json_value())
}

/// Builds the value; errors carry nothing (`Check`).
pub(crate) fn parse(input: &str) -> Option<JsonValue> {
  match json::<Error<&str>>().process::<OutputM<Emit, Check, Complete>>(input) {
    Ok(("", value)) => Some(value),
    _ => None,
  }
}
