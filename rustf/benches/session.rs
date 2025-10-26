use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use rustf::session::{Session, SessionStore};
use std::sync::Arc;

fn benchmark_session_operations(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let store = SessionStore::new();

    c.bench_function("session_create", |b| {
        b.iter(|| {
            rt.block_on(async {
                let session = store
                    .get_or_create(black_box("bench_session"))
                    .await
                    .unwrap();
                black_box(session);
            })
        })
    });

    c.bench_function("session_set_get", |b| {
        let session = rt.block_on(async { store.get_or_create("bench_session").await.unwrap() });

        b.iter(|| {
            session.set(black_box("key"), black_box("value")).unwrap();
            let value: Option<String> = session.get(black_box("key"));
            black_box(value);
        })
    });

    c.bench_function("session_remove", |b| {
        let session = rt.block_on(async { store.get_or_create("bench_session").await.unwrap() });

        b.iter(|| {
            session.set("temp_key", "temp_value").unwrap();
            session.remove(black_box("temp_key"));
        })
    });
}

fn benchmark_flash_messages(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let store = SessionStore::new();

    c.bench_function("flash_set_get", |b| {
        let session = rt.block_on(async { store.get_or_create("flash_bench").await.unwrap() });

        b.iter(|| {
            session
                .flash_set(black_box("flash_key"), black_box("flash_value"))
                .unwrap();
            let value: Option<String> = session.flash_get(black_box("flash_key"));
            black_box(value);
        })
    });

    c.bench_function("flash_get_all", |b| {
        let session = rt.block_on(async { store.get_or_create("flash_bench_all").await.unwrap() });

        b.iter(|| {
            // Set multiple flash messages
            for i in 0..10 {
                session
                    .flash_set(&format!("flash_{}", i), &format!("value_{}", i))
                    .unwrap();
            }
            // Get all at once
            let all = session.flash_get_all();
            black_box(all);
        })
    });
}

fn benchmark_concurrent_access(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let store = Arc::new(SessionStore::new());

    let mut group = c.benchmark_group("concurrent_sessions");

    for num_tasks in [10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_tasks),
            num_tasks,
            |b, &num_tasks| {
                b.iter(|| {
                    rt.block_on(async {
                        let mut handles = Vec::new();

                        for i in 0..num_tasks {
                            let store_clone = Arc::clone(&store);
                            let handle = tokio::spawn(async move {
                                let session_id = format!("concurrent_{}", i);
                                let session = store_clone.get_or_create(&session_id).await.unwrap();

                                // Perform some operations
                                session.set("user_id", i.to_string()).unwrap();
                                session.set("timestamp", "2024-01-01").unwrap();
                                let _user: Option<String> = session.get("user_id");

                                session
                            });
                            handles.push(handle);
                        }

                        for handle in handles {
                            let _session = handle.await.unwrap();
                        }
                    })
                })
            },
        );
    }
    group.finish();
}

fn benchmark_session_data_sizes(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let store = SessionStore::new();

    let mut group = c.benchmark_group("session_data_sizes");

    for size in [100, 1000, 10000].iter() {
        let data = "x".repeat(*size);

        group.bench_with_input(BenchmarkId::new("set", size), size, |b, _| {
            let session = rt.block_on(async { store.get_or_create("size_bench").await.unwrap() });

            b.iter(|| {
                session
                    .set(black_box("large_data"), black_box(&data))
                    .unwrap();
            })
        });

        group.bench_with_input(BenchmarkId::new("get", size), size, |b, _| {
            let session = rt.block_on(async {
                let session = store.get_or_create("size_bench_get").await.unwrap();
                session.set("large_data", &data).unwrap();
                session
            });

            b.iter(|| {
                let value: Option<String> = session.get(black_box("large_data"));
                black_box(value);
            })
        });
    }
    group.finish();
}

fn benchmark_session_iteration(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let store = SessionStore::new();

    c.bench_function("session_get_100_keys", |b| {
        let session = rt.block_on(async {
            let session = store.get_or_create("iter_bench").await.unwrap();
            // Add 100 keys
            for i in 0..100 {
                session
                    .set(&format!("key_{}", i), &format!("value_{}", i))
                    .unwrap();
            }
            session
        });

        b.iter(|| {
            let mut count = 0;
            for i in 0..100 {
                let _value: Option<String> = session.get(&format!("key_{}", i));
                count += 1;
            }
            black_box(count);
        })
    });
}

fn benchmark_session_clear(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let store = SessionStore::new();

    c.bench_function("session_clear_100_keys", |b| {
        b.iter(|| {
            rt.block_on(async {
                let session = store.get_or_create("clear_bench").await.unwrap();

                // Add keys
                for i in 0..100 {
                    session
                        .set(&format!("key_{}", i), &format!("value_{}", i))
                        .unwrap();
                }

                // Clear all
                session.clear();

                // Verify empty by checking flash count (no len method)
                let count = session.flash_count();
                black_box(count);
            })
        })
    });
}

fn benchmark_session_json_operations(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let store = SessionStore::new();

    c.bench_function("session_json_set_get", |b| {
        let session = rt.block_on(async { store.get_or_create("json_bench").await.unwrap() });

        b.iter(|| {
            let json_data = serde_json::json!({
                "user": {
                    "id": 123,
                    "name": "Test User",
                    "roles": ["admin", "user"]
                }
            });

            // Use regular set/get with serde_json::Value
            session
                .set(black_box("user_data"), black_box(&json_data))
                .unwrap();
            let value: Option<serde_json::Value> = session.get(black_box("user_data"));
            black_box(value);
        })
    });
}

fn benchmark_session_expiry(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let store = SessionStore::new();

    c.bench_function("session_touch", |b| {
        let session = rt.block_on(async { store.get_or_create("expiry_bench").await.unwrap() });

        b.iter(|| {
            session.touch();
            // No last_accessed() method, so just benchmark touch
            black_box(&session);
        })
    });
}

criterion_group!(
    benches,
    benchmark_session_operations,
    benchmark_flash_messages,
    benchmark_concurrent_access,
    benchmark_session_data_sizes,
    benchmark_session_iteration,
    benchmark_session_clear,
    benchmark_session_json_operations,
    benchmark_session_expiry
);
criterion_main!(benches);
