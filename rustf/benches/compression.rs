use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::Write;

// ~400 B — typical JSON API response
static SMALL_JSON: &str = r#"{"status":"ok","data":{"id":123,"name":"Alice","email":"alice@example.com","role":"admin","created_at":"2026-04-24T12:00:00Z","profile":{"bio":"Lorem ipsum dolor sit amet","country":"TG","city":"Lomé"}},"meta":{"request_id":"abc-123-def-456","duration_ms":12,"cached":false}}"#;

// ~4 KB — realistic rendered HTML page
fn make_medium_html() -> String {
    let mut s = String::from(
        r#"<!DOCTYPE html>
<html>
<head><title>Page</title></head>
<body>
"#,
    );
    for i in 0..80 {
        s.push_str(&format!(
            r#"<div class="row-{}"><h3>Row {}</h3><p>Content for row number {} with some repeated text to compress reasonably.</p></div>
"#,
            i % 10,
            i,
            i
        ));
    }
    s.push_str("</body></html>");
    s
}

// ~64 KB — large data export page
fn make_large_html() -> String {
    let mut s = String::from("<!DOCTYPE html><html><body><table>\n");
    for i in 0..2000 {
        s.push_str(&format!(
            "<tr><td>{}</td><td>user-{}@example.com</td><td>active</td><td>2026-04-{:02}</td></tr>\n",
            i,
            i,
            (i % 28) + 1
        ));
    }
    s.push_str("</table></body></html>");
    s
}

fn gzip(input: &[u8], level: Compression) -> Vec<u8> {
    let mut enc = GzEncoder::new(Vec::new(), level);
    enc.write_all(input).unwrap();
    enc.finish().unwrap()
}

fn bench_compression(c: &mut Criterion) {
    let medium = make_medium_html();
    let large = make_large_html();

    let inputs: Vec<(String, &[u8])> = vec![
        (format!("small-json (~{}B)", SMALL_JSON.len()), SMALL_JSON.as_bytes()),
        (format!("medium-html (~{}KB)", medium.len() / 1024), medium.as_bytes()),
        (format!("large-html (~{}KB)", large.len() / 1024), large.as_bytes()),
    ];

    let mut group = c.benchmark_group("gzip_default");
    for (label, input) in &inputs {
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(label), input, |b, data| {
            b.iter(|| gzip(black_box(data), Compression::default()));
        });
    }
    group.finish();

    let mut group = c.benchmark_group("gzip_fast");
    for (label, input) in &inputs {
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(label), input, |b, data| {
            b.iter(|| gzip(black_box(data), Compression::fast()));
        });
    }
    group.finish();

    // Record compression ratios separately (emitted once, not a bench)
    eprintln!("\n--- gzip compression ratios (default level) ---");
    for (label, input) in &inputs {
        let compressed = gzip(input, Compression::default());
        eprintln!(
            "{:<30}  {:>8} B -> {:>8} B  ({:.1}%)",
            label,
            input.len(),
            compressed.len(),
            100.0 * compressed.len() as f64 / input.len() as f64,
        );
    }
}

criterion_group!(benches, bench_compression);
criterion_main!(benches);
