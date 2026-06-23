//! Measure the per-render cost of building the repository & session Value objects
//! in `Context::view()` before the renderer is called.
//!
//! The goal is to surface the cost of:
//!   serde_json::to_value(&self.repository)
//!   session.to_value() + field insertion
//!
//! We don't go through the full template render here — the render itself dominates
//! for non-trivial templates and would mask the preparatory clone cost. Instead we
//! isolate the two clone-heavy operations.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use serde_json::{json, Map, Value};
use std::collections::HashMap;

fn make_repository(keys: usize) -> HashMap<String, Value> {
    let mut repo: HashMap<String, Value> = HashMap::with_capacity(keys);
    for i in 0..keys {
        // Mix simple and nested values — representative of a real controller
        // stashing title, nav, user info, and a small data collection.
        let v = if i % 4 == 0 {
            json!(format!("string value {}", i))
        } else if i % 4 == 1 {
            json!(i as i64 * 17)
        } else if i % 4 == 2 {
            json!({
                "id": i,
                "name": format!("item-{}", i),
                "active": i % 2 == 0,
                "tags": ["alpha", "beta", "gamma"],
            })
        } else {
            json!([
                {"k": "a", "v": i},
                {"k": "b", "v": i + 1},
                {"k": "c", "v": i + 2},
            ])
        };
        repo.insert(format!("key_{}", i), v);
    }
    repo
}

fn make_session_value() -> Value {
    // Typical session payload after middleware populates it: user_id, flash map,
    // authenticated flag, and a couple of app-set keys.
    json!({
        "id": "sess_abcd1234efgh5678",
        "authenticated": true,
        "user_id": 42,
        "flash": {
            "success": "Operation succeeded",
            "info": "Small info message"
        },
        "pref_theme": "light",
        "pref_lang": "en",
        "last_path": "/dashboard",
    })
}

/// Current approach: serialize the HashMap into a Value on every call.
fn build_repo_value_current(repo: &HashMap<String, Value>) -> Value {
    serde_json::to_value(repo).unwrap_or_else(|_| Value::Object(Map::new()))
}

/// Hypothetical: construct Value::Object directly from the HashMap pairs.
/// Same asymptotic cost, but avoids serde's type-dispatch layer.
fn build_repo_value_direct(repo: &HashMap<String, Value>) -> Value {
    let mut map = Map::with_capacity(repo.len());
    for (k, v) in repo {
        map.insert(k.clone(), v.clone());
    }
    Value::Object(map)
}

/// Session preparation currently done inside Context::view().
fn prepare_session_value(session_value: &Value) -> Value {
    // Mimics the field-stamping logic in context.rs:276-300 but without a real Session —
    // we start from the pre-baked value and clone+stamp as view() does.
    let mut v = session_value.clone();
    if let Value::Object(ref mut map) = v {
        map.insert("id".to_string(), Value::String("sess_x".to_string()));
        map.insert("authenticated".to_string(), Value::Bool(true));
        map.insert("user_id".to_string(), Value::Number(42.into()));
    }
    v
}

fn bench_view_render_prep(c: &mut Criterion) {
    let sizes = [0usize, 10, 50, 100];
    let session = make_session_value();

    let mut group = c.benchmark_group("view_render_prep_repository");
    for &n in &sizes {
        let repo = make_repository(n);
        group.bench_with_input(BenchmarkId::new("to_value_current", n), &repo, |b, r| {
            b.iter(|| build_repo_value_current(black_box(r)))
        });
        group.bench_with_input(BenchmarkId::new("direct_construct", n), &repo, |b, r| {
            b.iter(|| build_repo_value_direct(black_box(r)))
        });
    }
    group.finish();

    let mut group = c.benchmark_group("view_render_prep_session");
    group.bench_function("session_clone_and_stamp", |b| {
        b.iter(|| prepare_session_value(black_box(&session)))
    });
    group.finish();
}

criterion_group!(benches, bench_view_render_prep);
criterion_main!(benches);
