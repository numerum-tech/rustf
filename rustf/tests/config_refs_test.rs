//! `"(env) NAME"` / `"(file) PATH"` references, through the real loader.
//!
//! Each test uses its own directory and its own variable names, so they can
//! run in parallel.

use rustf::config::{AppConfig, SessionStorageConfig};
use std::path::PathBuf;

fn project_dir(name: &str, config: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("config_refs_{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // The loader validates that the static files directory exists.
    std::fs::create_dir_all(dir.join("public")).unwrap();
    std::fs::write(dir.join("config.toml"), config).unwrap();
    dir
}

#[test]
fn env_and_file_references_resolve_with_framework_types() {
    std::env::set_var("CFG_REFS_A_PORT", "8102");
    std::env::set_var("CFG_REFS_A_COOKIE", "123456");
    std::env::set_var("CFG_REFS_A_SMTP_PORT", "2525");
    std::env::set_var("CFG_REFS_A_DB_MAX", "7");
    let dir = project_dir(
        "a",
        r#"
[server]
port = "(env) CFG_REFS_A_PORT"

[session]
cookie_name = "(env) CFG_REFS_A_COOKIE"

[database]
url = "(file) secrets/db_url"
max_connections = "(env) CFG_REFS_A_DB_MAX"

[custom.mail]
smtp_port = "(env) CFG_REFS_A_SMTP_PORT"
note = "((env) is literal here"
"#,
    );
    std::fs::create_dir_all(dir.join("secrets")).unwrap();
    std::fs::write(dir.join("secrets/db_url"), "sqlite://refs.db\n").unwrap();

    let config = AppConfig::from_file(dir.join("config.toml")).unwrap();

    assert_eq!(config.server.port, 8102);
    assert_eq!(
        config.session.cookie_name, "123456",
        "a string framework key stays a string even when the value looks numeric"
    );
    assert_eq!(config.database.url.as_deref(), Some("sqlite://refs.db"));
    assert_eq!(config.database.max_connections, Some(7));

    let mail = &config.sections["custom"]["mail"];
    assert_eq!(
        mail["smtp_port"].as_str(),
        Some("2525"),
        "a custom value stays a string"
    );
    assert_eq!(mail["note"].as_str(), Some("(env) is literal here"));
}

#[test]
fn a_missing_variable_stops_loading() {
    std::env::remove_var("CFG_REFS_B_ABSENT");
    let dir = project_dir(
        "b",
        "[session]\ncookie_name = \"(env) CFG_REFS_B_ABSENT\"\n",
    );

    let error = AppConfig::from_file(dir.join("config.toml"))
        .unwrap_err()
        .to_string();

    assert!(error.contains("session.cookie_name"), "{error}");
    assert!(error.contains("CFG_REFS_B_ABSENT"), "{error}");
}

#[test]
fn an_unknown_tag_stops_loading() {
    let dir = project_dir("c", "[session]\ncookie_name = \"(evn) CFG_REFS_C\"\n");

    let error = AppConfig::from_file(dir.join("config.toml"))
        .unwrap_err()
        .to_string();

    assert!(error.contains("unknown reference (evn)"), "{error}");
}

#[test]
fn references_work_in_the_environment_overlay() {
    // The overlay is chosen from these; a developer's or CI's value would
    // select config.prod.toml instead.
    for name in ["RUSTF_ENV", "RAILS_ENV", "NODE_ENV"] {
        std::env::remove_var(name);
    }
    std::env::set_var("CFG_REFS_D_PORT", "9001");
    let dir = project_dir("d", "[server]\nport = 8000\n");
    std::fs::write(
        dir.join("config.dev.toml"),
        "[server]\nport = \"(env) CFG_REFS_D_PORT\"\n",
    )
    .unwrap();

    let config = AppConfig::load_with_base_dir(&dir).unwrap();

    assert_eq!(config.server.port, 9001);
}

#[test]
fn values_without_a_typed_default_stay_strings_and_storage_fields_are_typed() {
    std::env::set_var("CFG_REFS_E_PROXY", "123");
    std::env::set_var("CFG_REFS_E_REDIS_URL", "redis://cache:6379");
    std::env::set_var("CFG_REFS_E_PREFIX", "2024");
    std::env::set_var("CFG_REFS_E_POOL", "16");
    std::env::set_var("CFG_REFS_E_TIMEOUT", "250");
    let dir = project_dir(
        "e",
        r#"
[server]
trusted_proxies = ["(env) CFG_REFS_E_PROXY"]

[database]
url = "(file) db_url"
timeout = "(env) CFG_REFS_E_TIMEOUT"

[session.storage]
type = "redis"
url = "(env) CFG_REFS_E_REDIS_URL"
prefix = "(env) CFG_REFS_E_PREFIX"
pool_size = "(env) CFG_REFS_E_POOL"
connection_timeout = "(env) CFG_REFS_E_TIMEOUT"
command_timeout = "(env) CFG_REFS_E_TIMEOUT"
"#,
    );
    // A numeric-looking database URL (an `Option<String>` with no default).
    std::fs::write(dir.join("db_url"), "123456\n").unwrap();

    let config = AppConfig::from_file(dir.join("config.toml")).unwrap();

    assert_eq!(config.server.trusted_proxies, vec!["123".to_string()]);
    assert_eq!(config.database.url.as_deref(), Some("123456"));
    assert_eq!(config.database.timeout, Some(250));
    match &config.session.storage {
        SessionStorageConfig::Redis {
            url,
            prefix,
            pool_size,
            connection_timeout,
            command_timeout,
        } => {
            assert_eq!(url, "redis://cache:6379");
            assert_eq!(prefix, "2024");
            assert_eq!(*pool_size, 16);
            assert_eq!(*connection_timeout, 250);
            assert_eq!(*command_timeout, 250);
        }
        other => panic!("expected Redis storage, got {other:?}"),
    }
}

#[test]
fn loading_errors_never_quote_a_resolved_value() {
    // 70000 is a valid integer for the resolver but not a u16: the error
    // comes from deserialization, after the value was substituted.
    std::env::set_var("CFG_REFS_F_PORT", "70000");
    let dir = project_dir("f", "[server]\nport = \"(env) CFG_REFS_F_PORT\"\n");

    let error = AppConfig::from_file(dir.join("config.toml"))
        .unwrap_err()
        .to_string();

    assert!(!error.contains("70000"), "{error}");
    assert!(error.contains("***"), "{error}");
}

#[test]
fn a_short_resolved_value_is_masked_where_serde_quotes_it() {
    std::env::set_var("CFG_REFS_I_STORAGE", "bad");
    let dir = project_dir("i", "[views]\nstorage = \"(env) CFG_REFS_I_STORAGE\"\n");

    let error = AppConfig::from_file(dir.join("config.toml"))
        .unwrap_err()
        .to_string();

    assert!(error.contains("`***`"), "{error}");
    assert!(!error.contains("`bad`"), "{error}");
}

#[test]
fn an_overlay_reference_takes_the_type_of_the_base_value() {
    for name in ["RUSTF_ENV", "RAILS_ENV", "NODE_ENV"] {
        std::env::remove_var(name);
    }
    std::env::set_var("CFG_REFS_J_RETRIES", "5");
    let dir = project_dir("j", "[worker]\nretries = 3\nname = \"base\"\n");
    std::fs::write(
        dir.join("config.dev.toml"),
        "[worker]\nretries = \"(env) CFG_REFS_J_RETRIES\"\n",
    )
    .unwrap();

    let config = AppConfig::load_with_base_dir(&dir).unwrap();

    assert_eq!(config.sections["worker"]["retries"].as_integer(), Some(5));
    assert_eq!(config.sections["worker"]["name"].as_str(), Some("base"));
}

#[test]
fn a_normalized_value_is_masked_too() {
    std::env::set_var("CFG_REFS_K_PORT", "070000");
    let dir = project_dir("k", "[server]\nport = \"(env) CFG_REFS_K_PORT\"\n");

    let error = AppConfig::from_file(dir.join("config.toml"))
        .unwrap_err()
        .to_string();

    assert!(!error.contains("70000"), "{error}");
}

#[test]
fn an_overlay_reference_can_override_a_datetime() {
    for name in ["RUSTF_ENV", "RAILS_ENV", "NODE_ENV"] {
        std::env::remove_var(name);
    }
    std::env::set_var("CFG_REFS_L_CUTOFF", "2027-01-01T00:00:00Z");
    let dir = project_dir("l", "[billing]\ncutoff = 2026-10-22T12:00:00Z\n");
    std::fs::write(
        dir.join("config.dev.toml"),
        "[billing]\ncutoff = \"(env) CFG_REFS_L_CUTOFF\"\n",
    )
    .unwrap();

    let config = AppConfig::load_with_base_dir(&dir).unwrap();

    assert_eq!(
        config.sections["billing"]["cutoff"].to_string(),
        "2027-01-01T00:00:00Z"
    );
}

#[test]
fn section_errors_after_loading_mask_resolved_values() {
    // A newline makes serde quote the value escaped (`hunter2\n-token`).
    std::env::set_var("CFG_REFS_M_CREDENTIAL", "hunter2\n-token");
    let dir = project_dir(
        "m",
        "[mail]\ncredential = \"(env) CFG_REFS_M_CREDENTIAL\"\n",
    );
    let config = AppConfig::from_file(dir.join("config.toml")).unwrap();

    #[derive(Debug, serde::Deserialize)]
    struct Mail {
        #[allow(dead_code)]
        credential: u32,
    }
    let error = config.section::<Mail>("mail").unwrap_err().to_string();

    assert!(!error.contains("hunter2"), "{error}");
    assert!(error.contains("***"), "{error}");
}

#[test]
fn check_references_validates_tags_without_resolving() {
    std::env::remove_var("CFG_REFS_G_ONLY_IN_PROD");
    let dir = project_dir("g", "[server]\nport = 8000\n");
    std::fs::write(
        dir.join("config.prod.toml"),
        "[database]\nurl = \"(env) CFG_REFS_G_ONLY_IN_PROD\"\n",
    )
    .unwrap();
    std::fs::write(dir.join("broken.toml"), "[database]\nurl = \"(evn) X\"\n").unwrap();

    AppConfig::check_references(dir.join("config.prod.toml")).unwrap();
    let error = AppConfig::check_references(dir.join("broken.toml"))
        .unwrap_err()
        .to_string();
    assert!(error.contains("database.url"), "{error}");
    assert!(error.contains("unknown reference (evn)"), "{error}");
}

/// The only test in this binary that initialises the process-wide `CONF`.
#[test]
fn conf_reads_custom_string_values_as_the_requested_type() {
    std::env::set_var("CFG_REFS_H_PORT", "2525");
    std::env::set_var("CFG_REFS_H_TLS", "true");
    std::env::set_var("CFG_REFS_H_PIN", "0042");
    let dir = project_dir(
        "h",
        r#"
[mail]
port = "(env) CFG_REFS_H_PORT"
tls = "(env) CFG_REFS_H_TLS"
pin = "(env) CFG_REFS_H_PIN"
"#,
    );
    let config = AppConfig::from_file(dir.join("config.toml")).unwrap();

    #[derive(serde::Deserialize)]
    struct Mail {
        port: u16,
        tls: bool,
        pin: String,
    }
    let mail: Mail = config.section("mail").unwrap();
    assert_eq!(
        (mail.port, mail.tls, mail.pin.as_str()),
        (2525, true, "0042")
    );

    rustf::configuration::CONF::init(config).unwrap();
    use rustf::configuration::CONF;
    assert_eq!(CONF::get::<u16>("mail.port"), Some(2525));
    assert_eq!(CONF::get_int("mail.port"), Some(2525));
    assert_eq!(CONF::get_bool("mail.tls"), Some(true));
    assert_eq!(CONF::get_string("mail.pin").as_deref(), Some("0042"));
    assert_eq!(CONF::get_or::<u32>("mail.port", 1), 2525);
}

/// Functional record update must keep working outside the crate.
#[test]
fn app_config_can_still_be_built_with_struct_update_syntax() {
    let defaults = AppConfig::default();
    let mut server = defaults.server.clone();
    server.port = 9100;
    let config = AppConfig {
        server,
        ..AppConfig::default()
    };
    assert_eq!(config.server.port, 9100);
}
