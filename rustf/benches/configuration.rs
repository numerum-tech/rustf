use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rustf::config::AppConfig;
use rustf::CONF;
use std::sync::Arc;

fn setup_config() {
    // Initialize CONF if not already done
    let config = AppConfig::default();
    let _ = CONF::init(config);
}

fn benchmark_conf_integer_access(c: &mut Criterion) {
    setup_config();

    c.bench_function("CONF::get_int", |b| {
        b.iter(|| {
            let value = CONF::get_int(black_box("server.port"));
            black_box(value);
        })
    });
}

fn benchmark_conf_string_access(c: &mut Criterion) {
    setup_config();

    c.bench_function("CONF::get_string", |b| {
        b.iter(|| {
            let value = CONF::get_string(black_box("server.host"));
            black_box(value);
        })
    });
}

fn benchmark_conf_bool_access(c: &mut Criterion) {
    setup_config();

    c.bench_function("CONF::get_bool", |b| {
        b.iter(|| {
            let value = CONF::get_bool(black_box("server.ssl_enabled"));
            black_box(value);
        })
    });
}

fn benchmark_conf_with_default(c: &mut Criterion) {
    setup_config();

    c.bench_function("CONF::get_or", |b| {
        b.iter(|| {
            let value = CONF::get_or(black_box("server.port"), 8000);
            black_box(value);
        })
    });
}

fn benchmark_conf_missing_value(c: &mut Criterion) {
    setup_config();

    c.bench_function("CONF::get_or_missing", |b| {
        b.iter(|| {
            let value = CONF::get_or(black_box("nonexistent.value"), 42);
            black_box(value);
        })
    });
}

fn benchmark_conf_exists_check(c: &mut Criterion) {
    setup_config();

    c.bench_function("CONF::has", |b| {
        b.iter(|| {
            let exists = CONF::has(black_box("server.port"));
            black_box(exists);
        })
    });
}

fn benchmark_conf_deep_path(c: &mut Criterion) {
    setup_config();

    c.bench_function("CONF::get_string_deep", |b| {
        b.iter(|| {
            let value = CONF::get_string(black_box("database.connection.pool.size"));
            black_box(value);
        })
    });
}

fn benchmark_conf_environment_check(c: &mut Criterion) {
    setup_config();

    c.bench_function("CONF::is_production", |b| {
        b.iter(|| {
            let is_prod = CONF::is_production();
            black_box(is_prod);
        })
    });
}

fn benchmark_arc_config(c: &mut Criterion) {
    let config = Arc::new(AppConfig::default());
    let config_clone = config.clone();

    c.bench_function("Arc<Config> field access", |b| {
        b.iter(|| {
            let port = config_clone.server.port;
            black_box(port);
        })
    });
}

fn benchmark_direct_struct_access(c: &mut Criterion) {
    let config = AppConfig::default();

    c.bench_function("Direct struct access", |b| {
        b.iter(|| {
            let port = config.server.port;
            black_box(port);
        })
    });
}

criterion_group!(
    benches,
    benchmark_conf_integer_access,
    benchmark_conf_string_access,
    benchmark_conf_bool_access,
    benchmark_conf_with_default,
    benchmark_conf_missing_value,
    benchmark_conf_exists_check,
    benchmark_conf_deep_path,
    benchmark_conf_environment_check,
    benchmark_arc_config,
    benchmark_direct_struct_access
);
criterion_main!(benches);
