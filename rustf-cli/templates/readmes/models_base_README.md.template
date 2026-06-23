# Base Models Directory

Files in this directory are generated from your schema files.

Do not edit them directly.

Canonical guides:

- Schemas: <https://numerum-tech.github.io/rustf/guides/schemas.html>
- Database: <https://numerum-tech.github.io/rustf/guides/database.html>

## Rules

- `src/models/base/*.inc.rs` is generated output.
- Regenerate with `rustf-cli schema generate models`.
- Customize behavior in the wrapper model one level up, not here.

## Flow

```text
schemas/users.yaml
    ↓
rustf-cli schema generate models
    ↓
src/models/base/users.inc.rs   # generated
src/models/users.rs            # wrapper you edit
```

If you need a custom finder, validation helper, or convenience query, add it
to the wrapper model file, not the generated base file.
