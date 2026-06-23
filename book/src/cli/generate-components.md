# Generate Components

Use the CLI to create framework-shaped files instead of hand-writing boilerplate.

## Common Generators

### Controller

```bash
rustf-cli new controller -n users
rustf-cli new controller -n "users,posts" --routes
```

Creates controller files with `install()` route registration and RustF handler
stubs.

### Middleware

```bash
rustf-cli new middleware -n auth --auth --priority=-50
rustf-cli new middleware -n request-logger --logging --priority=-100
```

Creates middleware templates that match the current inbound/outbound API.

### Module

```bash
rustf-cli new module -n billing
```

Use modules for business logic shared across controllers.

### Worker

```bash
rustf-cli new worker -n digest-email
```

Creates a background worker stub and points to the worker guide.

### Event

```bash
rustf-cli new event -n user-created
```

Creates an event handler scaffold for RustF lifecycle or custom events.

## CRUD Scaffolding

```bash
rustf-cli new crud -n posts
```

This command assumes the model already exists and builds the HTTP layer around
it.

Precondition:

1. Define `schemas/posts.yaml`
2. Run `rustf-cli schema generate models`
3. Run `rustf-cli new crud -n posts`

The generated CRUD stack follows the RustF layering rule:

- controller -> module -> model
- views under `views/<name>/`
- test stub under `tests/`

## When to Use the CLI

Prefer generation when:

- you are creating a new app area
- you want files that already match framework conventions
- you want current API-compatible scaffolds instead of older examples

Hand-write only when you already have a clear custom structure.
