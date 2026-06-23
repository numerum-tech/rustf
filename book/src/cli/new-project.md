# Create a Project

The most common RustF entry point is:

```bash
rustf-cli new project my-app
cd my-app
cargo run
```

Your server will start on `http://127.0.0.1:8000` unless overridden by config.

## What Gets Generated

The scaffold includes the standard RustF layout:

- `src/controllers/`
- `src/models/`
- `src/modules/`
- `src/middleware/`
- `views/`
- `public/`
- `private/uploads/`
- `schemas/`
- `.claude/skills/rustf/SKILL.md`

That gives you a runnable project, a sample controller and view, default
configuration, and AI guidance files aligned with RustF conventions.

## Command

```bash
rustf-cli new project <PROJECT_NAME> [--path <DIR>] [--force]
```

## Notes

- `--force` overwrites an existing target after creating a backup under `.rustf/backups/project/`
- Generated config includes proxy-related server settings and current framework defaults
- The project README points to the RustF book, not legacy static docs

## Next Steps

After the initial scaffold:

1. Read [Project Structure](../getting-started/project-structure.md)
2. Add routes in `src/controllers/`
3. Add views in `views/`
4. If you use a database, continue with [Database & Schemas](database-schemas.md)
