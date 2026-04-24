//! Measure the HTTP-date formatter used on every static-file response.
//!
//! Compares:
//!   - `format_http_date_chrono`: old code path — chrono's strftime via
//!     `dt.format("...").to_string()`
//!   - `format_http_date_hand`:   the new shared hand-rolled formatter
//!     in `utils::http_date`

use chrono::{TimeZone, Utc};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rustf::utils::http_date::format_http_date;

fn format_http_date_chrono(timestamp: u64) -> String {
    let dt = Utc
        .timestamp_opt(timestamp as i64, 0)
        .single()
        .unwrap_or_else(|| Utc.timestamp_opt(0, 0).unwrap());
    dt.format("%a, %d %b %Y %H:%M:%S GMT").to_string()
}

fn bench_http_date(c: &mut Criterion) {
    // Representative timestamps: Unix epoch, a common mtime (~2023), a future one.
    let timestamps: Vec<u64> = vec![0, 1_700_000_000, 2_000_000_000];

    let mut group = c.benchmark_group("http_date_format");
    group.bench_function("chrono_strftime", |b| {
        b.iter(|| {
            for ts in &timestamps {
                black_box(format_http_date_chrono(black_box(*ts)));
            }
        });
    });
    group.bench_function("hand_rolled", |b| {
        b.iter(|| {
            for ts in &timestamps {
                black_box(format_http_date(black_box(*ts)));
            }
        });
    });
    group.finish();
}

criterion_group!(benches, bench_http_date);
criterion_main!(benches);
