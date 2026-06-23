# Analysis & Validation

These commands are the fastest way to inspect project health before a commit or
release.

## Project-Level Checks

```bash
rustf-cli analyze discover
rustf-cli analyze project --detailed
rustf-cli validate
```

Use them to catch route conflicts, missing assets, schema drift, middleware
issues, and general project structure problems.

## Common Analysis Commands

### Routes and Controllers

```bash
rustf-cli analyze routes --validate
rustf-cli analyze controllers
```

### Models and Schemas

```bash
rustf-cli analyze models --relationships
rustf-cli schema check-consistency
```

### Middleware and Views

```bash
rustf-cli analyze middleware --conflicts
rustf-cli analyze views --security
```

### Export Results

```bash
rustf-cli export --format json -o analysis.json
```

## Useful Workflow

For a normal development pass:

```bash
rustf-cli analyze discover
rustf-cli validate
rustf-cli analyze routes --validate
rustf-cli analyze project --detailed --format json
```

## Why This Matters

RustF uses conventions heavily. The CLI is the fastest way to verify that the
actual project still matches those conventions after refactors or generated code
changes.
