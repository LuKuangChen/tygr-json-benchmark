//! JSON parse throughput of serde_json, tygr, winnow and nom; see the README.

mod nom_json;
mod tygr_json;
mod value;
mod winnow_json;

use std::hint::black_box;
use std::time::Instant;

use value::JsonValue;

const FILES: [(&str, &str); 3] = [
    ("canada", include_str!("../data/canada.json")),
    ("citm_catalog", include_str!("../data/citm_catalog.json")),
    ("twitter", include_str!("../data/twitter.json")),
];

/// Name, a timed parse (building and dropping the parser's own value), and
/// the same parse as a `JsonValue` for checking. Only serde_json's `Value`
/// needs converting, outside the timing.
type Parser = (
    &'static str,
    fn(&str) -> bool,
    fn(&str) -> Option<JsonValue>,
);

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
const SAMPLES: [&str; 8] = [
    "0",
    " [ ] ",
    r#"{"a": [1, -2.5e+3, true, false, null], "b": {}}"#,
    r#""\"\\\/\b\f\n\r\t\u00e9 é""#,
    r#""\uD83D\uDE00""#,
    r#""a\u00e9b\uD83D\uDE00c\n""#,
    "[[[[]]]]",
    "-0.0E-0",
];

/// Timed runs per parser and file. Of 100 sorted runs, the 40th and 61st
/// bound a 95% confidence interval for the median, assuming no distribution.
const RUNS: usize = 100;

fn main() {
    for input in SAMPLES.into_iter().chain(FILES.map(|(_, input)| input)) {
        let expected = serde_value(input).map(JsonValue::from).unwrap();
        for (name, _, check) in PARSERS {
            assert!(
                check(input).as_ref() == Some(&expected),
                "{name} misparses {input:.40?}"
            );
        }
    }

    // MB/s by parser, file and run. Each round runs every parser, so drift
    // affects them all alike.
    let mut speeds = vec![vec![Vec::with_capacity(RUNS); FILES.len()]; PARSERS.len()];
    for _ in 0..RUNS {
        for (f, (_, input)) in FILES.iter().enumerate() {
            for (p, (_, parse, _)) in PARSERS.iter().enumerate() {
                let start = Instant::now();
                let ok = parse(black_box(input));
                speeds[p][f].push(input.len() as f64 / 1e6 / start.elapsed().as_secs_f64());
                assert!(ok);
            }
        }
    }
    speeds
        .iter_mut()
        .flatten()
        .for_each(|runs| runs.sort_by(f64::total_cmp));
    let median = |p: usize, f: usize| (speeds[p][f][49] + speeds[p][f][50]) / 2.0;

    table("MB/s, median (95% CI)", |p, f| {
        let runs = &speeds[p][f];
        format!("`{:.0} ({:.0}–{:.0})`", median(p, f), runs[39], runs[60])
    });
    table("vs serde_json, medians", |p, f| {
        format!("`{:.2}×`", median(p, f) / median(0, f))
    });
}

/// A Markdown table: a row per parser, a column per file.
fn table(title: &str, cell: impl Fn(usize, usize) -> String) {
    println!(
        "\n| {title} | {} |",
        FILES.map(|(file, _)| file).join(" | ")
    );
    println!("|---|{}", "---:|".repeat(FILES.len()));
    for (p, (name, _, _)) in PARSERS.iter().enumerate() {
        let cells = (0..FILES.len()).map(|f| cell(p, f)).collect::<Vec<_>>();
        println!("| {name} | {} |", cells.join(" | "));
    }
}
