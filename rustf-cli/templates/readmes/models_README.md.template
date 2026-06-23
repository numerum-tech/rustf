# Models Directory

This directory contains your model wrappers: data-access helpers and
schema-aware methods layered on top of generated base models.

The canonical model/database guides live in the RustF book:

- Database guide: <https://numerum-tech.github.io/rustf/guides/database.html>
- Schemas guide: <https://numerum-tech.github.io/rustf/guides/schemas.html>
- Model API reference: <https://numerum-tech.github.io/rustf/api-reference/models.html>

## Structure

```text
src/models/
├── base/        # generated code, never edit directly
├── users.rs     # wrapper model, edit here
└── posts.rs     # wrapper model, edit here
```

## Rules

- Never edit files under `src/models/base/`.
- Add custom query helpers in wrapper files such as `src/models/users.rs`.
- Keep business logic in modules, not in models.
- Preserve the wrapper `register(...)` function used by model auto-discovery.

## Minimal Wrapper Example

```rust
include!("base/users.inc.rs");

impl Users {
    pub async fn find_by_email(email: &str) -> rustf::Result<Option<Self>> {
        Self::query()?
            .where_eq("email", email)
            .get_first()
            .await
    }
}

pub fn register(registry: &mut rustf::models::ModelRegistry) {
    let _ = registry;
}
```

## Workflow

1. Define or update `schemas/*.yaml`.
2. Run `rustf-cli schema generate models`.
3. Edit the wrapper model only.
4. Put higher-level business rules in `src/modules/`.
