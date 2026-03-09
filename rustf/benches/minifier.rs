use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rustf::views::minifier::minify_html;

// Tiny page — no inline CSS/JS
static SMALL_HTML: &str = r#"<!DOCTYPE html>
<html>
  <head>
    <title>Hello</title>
  </head>
  <body>
    <div class="container">
      <h1>Hello World</h1>
      <p>Some   text   here.</p>
    </div>
  </body>
</html>"#;

// Realistic page — inline style + script block
static MEDIUM_HTML: &str = r#"<!DOCTYPE html>
<html>
  <head>
    <meta charset="UTF-8">
    <meta name="viewport"
          content="width=device-width, initial-scale=1.0">
    <title>Dashboard</title>
    <style>
      /* reset */
      * { box-sizing: border-box; margin: 0; padding: 0; }

      body {
        font-family: sans-serif;
        background: #f5f5f5;
      }

      .container {
        max-width: 1200px;
        margin: 0 auto;
        padding: 2rem;
      }

      .card {
        background: white;
        border-radius: 8px;
        padding: 1.5rem;
        box-shadow: 0 2px 4px rgba(0,0,0,.1);
      }

      table { width: 100%; border-collapse: collapse; }
      th, td { padding: .75rem; text-align: left; border-bottom: 1px solid #ddd; }
      th { background: #f9fafb; font-weight: 600; }
    </style>
  </head>
  <body>
    <!-- page header -->
    <header class="container">
      <h1>Dashboard</h1>
    </header>

    <main class="container">
      <div class="card">
        <h2>Recent Orders</h2>
        <table>
          <thead>
            <tr>
              <th>ID</th>
              <th>Customer</th>
              <th>Amount</th>
              <th>Status</th>
            </tr>
          </thead>
          <tbody>
            <tr><td>1001</td><td>Alice</td><td>$120.00</td><td>Shipped</td></tr>
            <tr><td>1002</td><td>Bob</td><td>$85.50</td><td>Pending</td></tr>
            <tr><td>1003</td><td>Carol</td><td>$240.00</td><td>Delivered</td></tr>
          </tbody>
        </table>
      </div>
    </main>

    <script>
      /* Dashboard initialisation */
      (function () {
        var rows = document.querySelectorAll('tbody tr');
        var total = 0;

        rows.forEach(function (row) {
          var cell = row.cells[2].textContent.replace('$', '');
          total += parseFloat(cell);
        });

        console.log('Total:', total.toFixed(2));
      }());
    </script>
  </body>
</html>"#;

// Heavy page — large inline style, repeated content, HTML comments
fn make_large_html() -> String {
    let mut s = String::from(
        r#"<!DOCTYPE html>
<html>
  <head>
    <title>Large Page</title>
    <!-- generated styles -->
    <style>
      /* base reset */
      * { box-sizing: border-box; margin: 0; padding: 0; }
      body { font-family: sans-serif; }
"#,
    );
    // 50 CSS rules
    for i in 0..50 {
        s.push_str(&format!(
            "      .cls-{i} {{ color: #{i:06x}; padding: {i}px; margin: {i}px; }}\n"
        ));
    }
    s.push_str("    </style>\n  </head>\n  <body>\n");
    // 100 rows of content
    for i in 0..100 {
        s.push_str(&format!(
            r#"    <!-- row {i} -->
    <div class="cls-{mod50}">
      <h2>Section   {i}</h2>
      <p>This   is   paragraph   {i}   with   extra   spaces.</p>
    </div>
"#,
            i = i,
            mod50 = i % 50
        ));
    }
    s.push_str("  </body>\n</html>");
    s
}

fn bench_minifier(c: &mut Criterion) {
    let large = make_large_html();

    let inputs: &[(&str, &str)] = &[
        ("small (~400B)", SMALL_HTML),
        ("medium (~2KB)", MEDIUM_HTML),
    ];

    let mut group = c.benchmark_group("minify_html");

    for (label, html) in inputs {
        group.throughput(Throughput::Bytes(html.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(label), html, |b, input| {
            b.iter(|| minify_html(black_box(input)));
        });
    }

    group.throughput(Throughput::Bytes(large.len() as u64));
    group.bench_with_input(
        BenchmarkId::from_parameter(format!("large (~{}KB)", large.len() / 1024)),
        &large,
        |b, input| {
            b.iter(|| minify_html(black_box(input)));
        },
    );

    group.finish();
}

criterion_group!(benches, bench_minifier);
criterion_main!(benches);
