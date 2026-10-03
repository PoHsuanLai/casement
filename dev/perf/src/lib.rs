//! The timing report both benches share: percentiles of microsecond samples, printed for a
//! person or as tab-separated rows for `dev/perf/run.sh` (`label p50 p95 p99 max`).

use std::time::Instant;

/// How a bench prints its rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// One readable line per row, with the load average around the run.
    Human,
    /// `label<TAB>p50<TAB>p95<TAB>p99<TAB>max`, microseconds.
    Tsv,
}

/// What the command line gave: how many iterations, and the format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    /// Timed iterations per row.
    pub iterations: usize,
    /// The output format.
    pub format: Format,
}

impl Options {
    /// `[ITERATIONS] [--tsv]` from the process arguments.
    pub fn from_args() -> Options {
        let args: Vec<String> = std::env::args().skip(1).collect();
        Options {
            iterations: args
                .iter()
                .find_map(|arg| arg.parse().ok())
                .unwrap_or(20_000),
            format: if args.iter().any(|arg| arg == "--tsv") {
                Format::Tsv
            } else {
                Format::Human
            },
        }
    }
}

/// The 1, 5 and 15 minute load averages, for the human rows (a loaded machine times badly).
pub fn load() -> String {
    std::fs::read_to_string("/proc/loadavg")
        .map(|text| {
            text.split_whitespace()
                .take(3)
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default()
}

/// The sorted samples' value at fraction `p` of the way up.
fn percentile(sorted: &[f64], p: f64) -> f64 {
    sorted[((sorted.len() as f64 - 1.0) * p).round() as usize]
}

/// Times `f` for `warmup` untimed and `options.iterations` timed calls (`f` gets the call's
/// number) and prints one row.
pub fn time_row(label: &str, options: Options, warmup: u64, mut f: impl FnMut(u64)) {
    let before = load();
    (0..warmup).for_each(&mut f);
    let mut samples: Vec<f64> = (0..options.iterations as u64)
        .map(|k| {
            let start = Instant::now();
            f(k);
            start.elapsed().as_secs_f64() * 1e6
        })
        .collect();
    samples.sort_by(f64::total_cmp);
    let [p50, p95, p99] = [0.5, 0.95, 0.99].map(|p| percentile(&samples, p));
    let max = samples[samples.len() - 1];
    match options.format {
        Format::Tsv => println!("{label}\t{p50:.1}\t{p95:.1}\t{p99:.1}\t{max:.1}"),
        Format::Human => println!(
            "{label:<44} p50 {p50:>6.1} us  p95 {p95:>6.1} us  p99 {p99:>6.1} us  max {max:>7.1} us  load {before} -> {}",
            load()
        ),
    }
}
