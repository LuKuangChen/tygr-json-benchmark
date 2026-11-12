//! What both benchmarks share: the files, the check and the timing.

use std::time::Instant;

use crate::value::JsonValue;

const FILES: [(&str, &str); 3] = [
    ("canada", include_str!("../data/canada.json")),
    ("citm_catalog", include_str!("../data/citm_catalog.json")),
    ("twitter", include_str!("../data/twitter.json")),
];

/// Name, a timed parse (building and dropping the parser's own output), and
/// the same parse as a `JsonValue` for checking, converted outside the timing.
pub(crate) type Parser = (
    &'static str,
    fn(&str) -> bool,
    fn(&str) -> Option<JsonValue>,
);

/// Timed runs per parser and file. Of 100 sorted runs, the 40th and 61st
/// bound a 95% confidence interval for the median, assuming no distribution.
const RUNS: usize = 100;

/// Check every parser against serde_json on `samples` and the files, then
/// time them and print the tables, relative to the first parser.
pub(crate) fn run(parsers: &[Parser], samples: &[&str]) {
    for &input in samples.iter().chain(&FILES.map(|(_, input)| input)) {
        let expected = serde_json::from_str::<serde_json::Value>(input)
            .map(JsonValue::from)
            .unwrap();
        for (name, _, check) in parsers {
            assert!(
                check(input).as_ref() == Some(&expected),
                "{name} misparses {input:.40?}"
            );
        }
    }

    // MB/s by parser, file and run. Each round runs every parser, so drift
    // affects them all alike.
    let mut speeds = vec![vec![Vec::with_capacity(RUNS); FILES.len()]; parsers.len()];
    for _ in 0..RUNS {
        for (f, (_, input)) in FILES.iter().enumerate() {
            for (p, (_, parse, _)) in parsers.iter().enumerate() {
                let start = Instant::now();
                let ok = parse(std::hint::black_box(input));
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

    table(parsers, "MB/s, median (95% CI)", |p, f| {
        let runs = &speeds[p][f];
        format!("`{:.0} ({:.0}–{:.0})`", median(p, f), runs[39], runs[60])
    });
    table(parsers, &format!("vs {}, medians", parsers[0].0), |p, f| {
        format!("`{:.2}×`", median(p, f) / median(0, f))
    });
}

/// A Markdown table: a row per parser, a column per file.
fn table(parsers: &[Parser], title: &str, cell: impl Fn(usize, usize) -> String) {
    println!(
        "\n| {title} | {} |",
        FILES.map(|(file, _)| file).join(" | ")
    );
    println!("|---|{}", "---:|".repeat(FILES.len()));
    for (p, (name, _, _)) in parsers.iter().enumerate() {
        let cells = (0..FILES.len()).map(|f| cell(p, f)).collect::<Vec<_>>();
        println!("| {name} | {} |", cells.join(" | "));
    }
}
