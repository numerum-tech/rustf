//! The environment comes from `RUSTF_ENV` or `[app] environment`, nothing
//! else, and an unknown name stops loading instead of running as development.
//!
//! These tests set `RUSTF_ENV`, which is process-wide: they run one at a time
//! under `ENV_LOCK`, and no other test in this binary reads it.

use rustf::config::{AppConfig, Environment};
use std::path::PathBuf;
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn project(name: &str, config: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("environment_{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("public")).unwrap();
    std::fs::write(dir.join("config.toml"), config).unwrap();
    dir
}

/// Run `f` with `RUSTF_ENV` set to `value` (or unset), restoring nothing:
/// every test sets the state it needs.
fn with_rustf_env<T>(value: Option<&str>, f: impl FnOnce() -> T) -> T {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    match value {
        Some(v) => std::env::set_var("RUSTF_ENV", v),
        None => std::env::remove_var("RUSTF_ENV"),
    }
    let result = f();
    std::env::remove_var("RUSTF_ENV");
    result
}

#[test]
fn an_unknown_rustf_env_stops_loading() {
    let dir = project("unknown", "[server]\nport = 8000\n");
    let error = with_rustf_env(Some("staging"), || {
        AppConfig::load_with_base_dir(&dir).unwrap_err().to_string()
    });
    assert!(error.contains("RUSTF_ENV"), "{error}");
    assert!(error.contains("staging"), "{error}");
}

#[test]
fn an_unknown_app_environment_stops_loading() {
    let dir = project("unknown_app", "[app]\nenvironment = \"staging\"\n");
    let error = with_rustf_env(None, || {
        AppConfig::from_file(dir.join("config.toml"))
            .unwrap_err()
            .to_string()
    });
    assert!(error.contains("[app] environment"), "{error}");
}

#[test]
fn rustf_env_selects_production_and_its_overlay() {
    let dir = project("prod", "[server]\nport = 8000\n");
    std::fs::write(dir.join("config.prod.toml"), "[server]\nport = 9000\n").unwrap();

    let config = with_rustf_env(Some("prod"), || {
        AppConfig::load_with_base_dir(&dir).unwrap()
    });

    assert_eq!(config.environment, Environment::Production);
    assert_eq!(config.server.port, 9000);
}

#[test]
fn other_conventions_are_ignored() {
    // NODE_ENV and friends belong to other tools in the same container.
    let dir = project("ignored", "[server]\nport = 8000\n");
    let config = with_rustf_env(None, || {
        for name in [
            "NODE_ENV",
            "RAILS_ENV",
            "RUST_ENV",
            "APP_ENV",
            "RUSTF_ENVIRONMENT",
        ] {
            std::env::set_var(name, "production");
        }
        let config = AppConfig::load_with_base_dir(&dir).unwrap();
        for name in [
            "NODE_ENV",
            "RAILS_ENV",
            "RUST_ENV",
            "APP_ENV",
            "RUSTF_ENVIRONMENT",
        ] {
            std::env::remove_var(name);
        }
        config
    });
    assert_eq!(config.environment, Environment::Development);
}

#[test]
fn app_environment_declares_production_without_variables() {
    let dir = project("declared", "[app]\nenvironment = \"production\"\n");
    let config = with_rustf_env(None, || {
        AppConfig::from_file(dir.join("config.toml")).unwrap()
    });
    assert!(config.environment.is_production());
}

#[test]
fn code_without_a_config_uses_rustf_env_and_fails_safe() {
    let (unset, prod, invalid) = (
        with_rustf_env(None, Environment::from_process_env_or_production_on_error),
        with_rustf_env(
            Some("prod"),
            Environment::from_process_env_or_production_on_error,
        ),
        with_rustf_env(
            Some("staging"),
            Environment::from_process_env_or_production_on_error,
        ),
    );
    assert_eq!(unset, Environment::Development);
    assert_eq!(prod, Environment::Production);
    // An invalid name cannot be returned as an error there: production is
    // the safe side (secure cookies, no detailed error pages).
    assert_eq!(invalid, Environment::Production);
}

#[cfg(unix)]
#[test]
fn a_non_utf8_rustf_env_is_an_error_not_absent() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    std::env::set_var("RUSTF_ENV", OsStr::from_bytes(b"prod\xff"));
    let result = Environment::from_process_env();
    std::env::remove_var("RUSTF_ENV");

    let error = result.unwrap_err().to_string();
    assert!(error.contains("not valid UTF-8"), "{error}");
}

/// The only test in this binary that initialises the process-wide `CONF`.
#[test]
fn debug_follows_the_configuration_loaded_after_it_was_first_read() {
    use rustf::configuration::{CONF, DEBUG};

    let dir = project("debug", "[app]\nenvironment = \"production\"\n");
    let config = with_rustf_env(None, || {
        // Read before the configuration exists: development by default.
        assert!(*DEBUG);
        AppConfig::from_file(dir.join("config.toml")).unwrap()
    });

    CONF::init(config).unwrap();
    assert!(!*DEBUG, "DEBUG must follow the production configuration");
    assert!(!DEBUG.get());
}
