# rustf-schema

[![crates.io](https://img.shields.io/crates/v/rustf-schema.svg)](https://crates.io/crates/rustf-schema)
[![docs.rs](https://img.shields.io/docsrs/rustf-schema)](https://docs.rs/rustf-schema)

YAML-based schema definitions, validation, and code generation for the
[RustF](https://crates.io/crates/rustf) web framework.

Define your data model in YAML and generate type-safe SQLx models from it. Used by
the `rustf-cli` `schema generate` command, and usable standalone.

## What it provides

- **Schema types** — `Schema`, `Table`, `Field`, `FieldType`, relations.
- **Parser** — load a schema directory (`_meta.yaml` + per-table `*.yaml`).
- **Validator** — relationship checks, consistency validation, circular-dependency detection.
- **Codegen** (feature `codegen`) — Handlebars-based generation of SQLx model code.

```rust
use rustf_schema::Schema;

# async fn example() -> Result<(), rustf_schema::SchemaError> {
let schema = Schema::load_from_directory(std::path::Path::new("schemas")).await?;
schema.validate()?;
# Ok(())
# }
```

## Features

| Feature | Default | Enables |
|---------|:-------:|---------|
| `codegen` | ✅ | Handlebars-based SQLx code generation |
| `parser` | — | async directory parser (`tokio`) |
| `chrono` / `rust_decimal` | ✅ | type helpers |

## License

Licensed under either of Apache License 2.0 or MIT license at your option.
