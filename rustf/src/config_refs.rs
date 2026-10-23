//! Value references in configuration files.
//!
//! A string value of the form `"(tag) argument"` is replaced, at load time,
//! by a value taken from outside the file:
//!
//! ```toml
//! [database]
//! url = "(env) DATABASE_URL"               # environment variable
//!
//! [custom.mail]
//! password = "(file) /run/secrets/smtp"    # file contents (Docker/K8s secrets)
//!
//! [server]
//! port = "(env) PORT"                      # becomes an integer
//! ```
//!
//! Rules:
//! - A marker is `(` + a lowercase word + `)` + a space, at the start of a
//!   value. Anything else (`"(555) 123-4567"`, `"((foo)|(bar))"`) is a literal.
//! - An unknown tag is an error, so a typo (`"(evn) SECRET"`) cannot slip
//!   through as a literal secret. A missing variable or unreadable file is an
//!   error too: the application refuses to start.
//! - A value that must literally start with a marker doubles the opening
//!   parenthesis: `"((env) text"` loads as `"(env) text"`. Only a doubled
//!   parenthesis followed by a marker is an escape.
//! - `(file)` reads a regular file of at most [`MAX_FILE_BYTES`], relative to
//!   the configuration file's directory; trailing newlines are removed.
//! - Typing: a framework key takes the type it has in `template` (the framework
//!   defaults plus a few type hints, see `AppConfig`); every other value stays
//!   a string. `CONF` and `AppConfig::section` read numbers and booleans out of
//!   such strings when the requested type asks for one.
//!
//! Resolved values are never logged, and `resolve` returns them so that the
//! caller can mask them in later error messages.

use std::path::Path;
use toml::Value;

/// Largest file a `(file)` reference may read.
pub(crate) const MAX_FILE_BYTES: u64 = 1024 * 1024;

/// Shortest resolved value masked where it appears unquoted in an error
/// message. serde quotes the values it reports (`` `bad` ``, `"x"`), and quoted
/// occurrences are masked whatever their length; masking short bare words
/// (`port`) would mangle unrelated text.
const MIN_BARE_MASKED_LEN: usize = 8;

/// A reference found in a configuration string.
#[derive(Debug, PartialEq)]
enum Reference<'a> {
    Env(&'a str),
    File(&'a str),
}

/// What a configuration string turned out to be.
#[derive(Debug, PartialEq)]
enum Parsed<'a> {
    Literal,
    /// `((tag) ...`: the value minus its first character.
    Escaped(&'a str),
    Reference(Reference<'a>),
}

/// How a resolved value is to be typed.
#[derive(Clone, Copy)]
enum Typing<'a> {
    /// The template's value for this key.
    Template(&'a Value),
    /// No template value: kept as text.
    Text,
}

/// Resolve every reference in `config`, in place, and return the resolved
/// values.
///
/// `template` gives framework keys their type. The error names the key and
/// the variable or file, never the value.
pub(crate) fn resolve(
    config: &mut Value,
    template: &Value,
    base_dir: &Path,
) -> Result<Vec<String>, String> {
    let mut resolved = Vec::new();
    if let Value::Table(root) = config {
        for (section, value) in root.iter_mut() {
            let typing = match template.get(section) {
                Some(default) => Typing::Template(default),
                None => Typing::Text,
            };
            resolve_value(value, typing, section, base_dir, &mut resolved)?;
        }
    }
    Ok(resolved)
}

/// Check that every marker in `config` names a known tag and has an argument,
/// without resolving anything. For validating a configuration on a machine
/// that does not have its variables or secret files.
pub(crate) fn check(config: &Value) -> Result<(), String> {
    fn walk(value: &Value, key: &str) -> Result<(), String> {
        match value {
            Value::String(text) => parse(text).map(|_| ()).map_err(|r| format!("{key}: {r}")),
            Value::Table(table) => table
                .iter()
                .try_for_each(|(name, child)| walk(child, &join(key, name))),
            Value::Array(items) => items
                .iter()
                .enumerate()
                .try_for_each(|(index, item)| walk(item, &format!("{key}[{index}]"))),
            _ => Ok(()),
        }
    }
    walk(config, "")
}

/// Replace every resolved value in `message` with `***`.
pub(crate) fn mask(message: &str, secrets: &[String]) -> String {
    let mut sorted: Vec<&String> = secrets.iter().filter(|s| !s.is_empty()).collect();
    // Longest first, so a value containing another is masked whole.
    sorted.sort_by_key(|s| std::cmp::Reverse(s.len()));

    let mut text = message.to_string();
    for secret in sorted {
        // serde reports strings in their `Debug` form (`string "a\nb"`), so a
        // value with quotes, backslashes or control characters appears escaped.
        let escaped = format!("{secret:?}");
        if escaped.len() != secret.len() + 2 {
            text = text.replace(&escaped, "\"***\"");
        }
        for (open, close) in [('`', '`'), ('"', '"'), ('\'', '\'')] {
            text = text.replace(
                &format!("{open}{secret}{close}"),
                &format!("{open}***{close}"),
            );
        }
        if secret.len() >= MIN_BARE_MASKED_LEN {
            text = text.replace(secret.as_str(), "***");
        }
    }
    text
}

fn join(key: &str, name: &str) -> String {
    if key.is_empty() {
        name.to_string()
    } else {
        format!("{key}.{name}")
    }
}

fn resolve_value(
    value: &mut Value,
    typing: Typing,
    key: &str,
    base_dir: &Path,
    resolved: &mut Vec<String>,
) -> Result<(), String> {
    match value {
        Value::String(text) => {
            if let Some(replacement) = resolve_string(text, typing, key, base_dir, resolved)? {
                *value = replacement;
            }
        }
        Value::Table(table) => {
            for (name, child) in table.iter_mut() {
                let child_typing = match typing {
                    Typing::Template(default) => match default.get(name) {
                        Some(child_default) => Typing::Template(child_default),
                        None => Typing::Text,
                    },
                    Typing::Text => Typing::Text,
                };
                resolve_value(child, child_typing, &join(key, name), base_dir, resolved)?;
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter_mut().enumerate() {
                let item_typing = match typing {
                    Typing::Template(Value::Array(defaults)) => match defaults.first() {
                        Some(first) => Typing::Template(first),
                        None => Typing::Text,
                    },
                    _ => Typing::Text,
                };
                let item_key = format!("{key}[{index}]");
                resolve_value(item, item_typing, &item_key, base_dir, resolved)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// The replacement for one string, or `None` when it is a plain literal.
fn resolve_string(
    text: &str,
    typing: Typing,
    key: &str,
    base_dir: &Path,
    resolved: &mut Vec<String>,
) -> Result<Option<Value>, String> {
    let raw = match parse(text).map_err(|reason| format!("{key}: {reason}"))? {
        Parsed::Literal => return Ok(None),
        Parsed::Escaped(rest) => return Ok(Some(Value::String(rest.to_string()))),
        Parsed::Reference(Reference::Env(name)) => std::env::var(name)
            .map_err(|_| format!("{key}: environment variable {name} is not set"))?,
        Parsed::Reference(Reference::File(path)) => read_file(&base_dir.join(path), key)?,
    };

    resolved.push(raw.clone());
    let value = typed(raw, typing, key)?;
    // Errors quote the parsed value (`070000` is reported as `70000`): mask
    // that form too.
    let canonical = match &value {
        Value::Integer(i) => Some(i.to_string()),
        Value::Float(f) => Some(f.to_string()),
        Value::Boolean(b) => Some(b.to_string()),
        Value::Datetime(d) => Some(d.to_string()),
        _ => None,
    };
    resolved.extend(canonical);
    Ok(Some(value))
}

fn read_file(path: &Path, key: &str) -> Result<String, String> {
    use std::io::Read;

    let cannot = |reason: String| format!("{key}: cannot read {}: {}", path.display(), reason);
    let not_regular = || cannot("not a regular file".to_string());
    let too_large = || cannot(format!("larger than {MAX_FILE_BYTES} bytes"));

    // Opening a FIFO blocks, and a device (`/dev/zero`) never ends: check the
    // path before opening it.
    let metadata = std::fs::metadata(path).map_err(|e| cannot(e.to_string()))?;
    if !metadata.is_file() {
        return Err(not_regular());
    }
    if metadata.len() > MAX_FILE_BYTES {
        return Err(too_large());
    }

    // Check the opened file again and bound the read, so a file swapped or
    // grown after the first check is still refused.
    let file = std::fs::File::open(path).map_err(|e| cannot(e.to_string()))?;
    if !file
        .metadata()
        .map_err(|e| cannot(e.to_string()))?
        .is_file()
    {
        return Err(not_regular());
    }
    let mut bytes = Vec::new();
    file.take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| cannot(e.to_string()))?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(too_large());
    }
    let content = String::from_utf8(bytes).map_err(|_| cannot("not valid UTF-8".to_string()))?;
    Ok(content.trim_end_matches(['\n', '\r']).to_string())
}

/// `(tag) argument` at the start of `text`: the tag and the raw argument.
fn marker(text: &str) -> Option<(&str, &str)> {
    let rest = text.strip_prefix('(')?;
    let close = rest.find(')')?;
    let tag = &rest[..close];
    if tag.is_empty() || !tag.bytes().all(|b| b.is_ascii_lowercase()) {
        return None;
    }
    let argument = rest[close + 1..].strip_prefix(' ')?;
    Some((tag, argument))
}

fn parse(text: &str) -> Result<Parsed<'_>, String> {
    if let Some(rest) = text.strip_prefix('(') {
        if rest.starts_with('(') && marker(rest).is_some() {
            return Ok(Parsed::Escaped(rest));
        }
    }

    let Some((tag, argument)) = marker(text) else {
        return Ok(Parsed::Literal);
    };
    let argument = argument.trim();
    if argument.is_empty() {
        return Err(format!("({tag}) needs an argument"));
    }
    match tag {
        "env" => Ok(Parsed::Reference(Reference::Env(argument))),
        "file" => Ok(Parsed::Reference(Reference::File(argument))),
        other => Err(format!(
            "unknown reference ({other}); expected (env) or (file). \
             Write ((... for a literal value starting with a parenthesis"
        )),
    }
}

fn typed(raw: String, typing: Typing, key: &str) -> Result<Value, String> {
    let invalid = |expected: &str| {
        // The value itself is not quoted: it may be a secret.
        format!("{key}: referenced value is not a valid {expected}")
    };

    match typing {
        Typing::Template(Value::Integer(_)) => raw
            .trim()
            .parse::<i64>()
            .map(Value::Integer)
            .map_err(|_| invalid("integer")),
        Typing::Template(Value::Float(_)) => raw
            .trim()
            .parse::<f64>()
            .map(Value::Float)
            .map_err(|_| invalid("number")),
        Typing::Template(Value::Boolean(_)) => match raw.trim().to_ascii_lowercase().as_str() {
            "true" => Ok(Value::Boolean(true)),
            "false" => Ok(Value::Boolean(false)),
            _ => Err(invalid("boolean (true/false)")),
        },
        Typing::Template(Value::Datetime(_)) => raw
            .trim()
            .parse::<toml::value::Datetime>()
            .map(Value::Datetime)
            .map_err(|_| invalid("TOML date or datetime")),
        Typing::Template(_) | Typing::Text => Ok(Value::String(raw)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(source: &str) -> Value {
        toml::from_str(source).unwrap()
    }

    #[test]
    fn parse_recognises_only_the_marker_shape() {
        assert_eq!(parse("plain"), Ok(Parsed::Literal));
        assert_eq!(parse("(555) 123-4567"), Ok(Parsed::Literal));
        assert_eq!(parse("(env)NAME"), Ok(Parsed::Literal));
        assert_eq!(parse("(Env) NAME"), Ok(Parsed::Literal));
        assert_eq!(parse("(env"), Ok(Parsed::Literal));
        assert_eq!(
            parse("(env) NAME"),
            Ok(Parsed::Reference(Reference::Env("NAME")))
        );
        assert_eq!(
            parse("(file)  /run/secrets/x "),
            Ok(Parsed::Reference(Reference::File("/run/secrets/x")))
        );
        assert_eq!(parse("((env) x"), Ok(Parsed::Escaped("(env) x")));
        // A doubled parenthesis that is not followed by a marker is a literal,
        // so existing values such as regexes are left alone.
        assert_eq!(parse("((foo)|(bar))"), Ok(Parsed::Literal));
        assert_eq!(parse("((x"), Ok(Parsed::Literal));
    }

    #[test]
    fn unknown_tags_and_empty_arguments_are_errors() {
        let error = parse("(evn) SECRET").unwrap_err();
        assert!(error.contains("unknown reference (evn)"), "{error}");
        assert!(parse("(env) ").is_err());
    }

    #[test]
    fn framework_keys_take_their_default_type() {
        std::env::set_var("RUSTF_REFS_TEST_PORT", "8102");
        std::env::set_var("RUSTF_REFS_TEST_FLAG", "TRUE");
        std::env::set_var("RUSTF_REFS_TEST_SECRET", "123456");
        let template = table("[server]\nport = 8000\nssl = false\nsecret = \"x\"\n");
        let mut config = table(
            "[server]\nport = \"(env) RUSTF_REFS_TEST_PORT\"\n\
             ssl = \"(env) RUSTF_REFS_TEST_FLAG\"\n\
             secret = \"(env) RUSTF_REFS_TEST_SECRET\"\n",
        );

        resolve(&mut config, &template, Path::new(".")).unwrap();

        assert_eq!(config["server"]["port"], Value::Integer(8102));
        assert_eq!(config["server"]["ssl"], Value::Boolean(true));
        assert_eq!(
            config["server"]["secret"],
            Value::String("123456".into()),
            "a string key stays a string even when the value looks numeric"
        );
    }

    #[test]
    fn keys_without_a_template_value_stay_strings() {
        // `database.url` is an `Option<String>` with no default: a numeric
        // value must not turn into an integer.
        std::env::set_var("RUSTF_REFS_TEST_NUMERIC_URL", "123456");
        std::env::set_var("RUSTF_REFS_TEST_PROXY", "123");
        let template = table("[database]\n[server]\ntrusted_proxies = []\n");
        let mut config = table(
            "[database]\nurl = \"(env) RUSTF_REFS_TEST_NUMERIC_URL\"\n\
             [server]\ntrusted_proxies = [\"(env) RUSTF_REFS_TEST_PROXY\"]\n",
        );

        resolve(&mut config, &template, Path::new(".")).unwrap();

        assert_eq!(config["database"]["url"], Value::String("123456".into()));
        assert_eq!(
            config["server"]["trusted_proxies"],
            Value::Array(vec![Value::String("123".into())])
        );
    }

    #[test]
    fn custom_sections_stay_strings() {
        std::env::set_var("RUSTF_REFS_TEST_PIN", "0042");
        let template = table("[server]\nport = 8000\n");
        let mut config = table("[custom.mail]\npassword = \"(env) RUSTF_REFS_TEST_PIN\"\n");

        resolve(&mut config, &template, Path::new(".")).unwrap();

        assert_eq!(
            config["custom"]["mail"]["password"],
            Value::String("0042".into())
        );
    }

    #[test]
    fn wrong_type_is_an_error_that_does_not_quote_the_value() {
        std::env::set_var("RUSTF_REFS_TEST_BAD_PORT", "s3cr3t");
        let template = table("[server]\nport = 8000\n");
        let mut config = table("[server]\nport = \"(env) RUSTF_REFS_TEST_BAD_PORT\"\n");

        let error = resolve(&mut config, &template, Path::new(".")).unwrap_err();

        assert!(error.contains("server.port"), "{error}");
        assert!(!error.contains("s3cr3t"), "{error}");
    }

    #[test]
    fn missing_variable_names_the_key_and_the_variable() {
        std::env::remove_var("RUSTF_REFS_TEST_ABSENT");
        let template = table("[session]\nsecret = \"x\"\n");
        let mut config = table("[session]\nsecret = \"(env) RUSTF_REFS_TEST_ABSENT\"\n");

        let error = resolve(&mut config, &template, Path::new(".")).unwrap_err();

        assert!(error.contains("session.secret"), "{error}");
        assert!(error.contains("RUSTF_REFS_TEST_ABSENT"), "{error}");
    }

    #[test]
    fn file_references_are_relative_to_the_config_directory() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target/tmp")
            .join(format!("rustf_refs_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("db_url"), "sqlite://app.db\n").unwrap();
        let template = table("[database]\n");
        let mut config = table("[database]\nurl = \"(file) db_url\"\n");

        resolve(&mut config, &template, &dir).unwrap();
        std::fs::remove_dir_all(&dir).ok();

        assert_eq!(
            config["database"]["url"],
            Value::String("sqlite://app.db".into())
        );
    }

    #[test]
    fn references_inside_arrays_and_escapes_resolve() {
        std::env::set_var("RUSTF_REFS_TEST_ORIGIN", "https://example.com");
        let template = table("[cors]\nallowed_origins = [\"*\"]\n");
        let mut config = table(
            "[cors]\nallowed_origins = [\"(env) RUSTF_REFS_TEST_ORIGIN\", \"((env) literal\"]\n",
        );

        resolve(&mut config, &template, Path::new(".")).unwrap();

        assert_eq!(
            config["cors"]["allowed_origins"],
            Value::Array(vec![
                Value::String("https://example.com".into()),
                Value::String("(env) literal".into()),
            ])
        );
    }

    #[test]
    fn resolve_returns_the_values_and_mask_hides_them() {
        std::env::set_var("RUSTF_REFS_TEST_MASKED", "hunter2-token");
        let template = table("[session]\ncookie_name = \"x\"\n");
        let mut config = table("[session]\ncookie_name = \"(env) RUSTF_REFS_TEST_MASKED\"\n");

        let secrets = resolve(&mut config, &template, Path::new(".")).unwrap();

        assert_eq!(secrets, vec!["hunter2-token".to_string()]);
        assert_eq!(
            mask("invalid value: hunter2-token, expected x", &secrets),
            "invalid value: ***, expected x"
        );
        // Quoted, as serde reports values: masked whatever the length.
        let short = vec!["bad".to_string()];
        assert_eq!(
            mask("unknown variant `bad`, expected `filesystem`", &short),
            "unknown variant `***`, expected `filesystem`"
        );
        assert_eq!(
            mask("invalid type: string \"bad\"", &short),
            "invalid type: string \"***\""
        );
        // Escaped the way serde quotes strings.
        let multiline = vec!["hunter2\n-token".to_string()];
        assert_eq!(
            mask(
                "invalid type: string \"hunter2\\n-token\", expected u32",
                &multiline
            ),
            "invalid type: string \"***\", expected u32"
        );
        // Bare and short: an ordinary word, left alone.
        let word = vec!["port".to_string()];
        assert_eq!(
            mask("Server port cannot be 0", &word),
            "Server port cannot be 0"
        );
    }

    #[test]
    fn check_reports_unknown_tags_without_resolving() {
        std::env::remove_var("RUSTF_REFS_TEST_NEVER_SET");
        assert!(check(&table("[a]\nb = \"(env) RUSTF_REFS_TEST_NEVER_SET\"\n")).is_ok());
        let error = check(&table("[a]\nb = [\"(evn) X\"]\n")).unwrap_err();
        assert!(error.starts_with("a.b[0]:"), "{error}");
    }

    #[test]
    fn file_references_must_be_small_regular_files() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target/tmp")
            .join(format!("rustf_refs_limits_{}", std::process::id()));
        std::fs::create_dir_all(dir.join("a_directory")).unwrap();
        let big = vec![b'x'; (MAX_FILE_BYTES + 1) as usize];
        std::fs::write(dir.join("too_big"), big).unwrap();
        let template = table("[custom]\n");

        let mut config = table("[custom]\nv = \"(file) a_directory\"\n");
        let error = resolve(&mut config, &template, &dir).unwrap_err();
        assert!(error.contains("not a regular file"), "{error}");

        let mut config = table("[custom]\nv = \"(file) too_big\"\n");
        let error = resolve(&mut config, &template, &dir).unwrap_err();
        assert!(error.contains("larger than"), "{error}");

        std::fs::remove_dir_all(&dir).ok();
    }
}
