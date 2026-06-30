# Modules Directory

This directory contains business logic and reusable application services.

The canonical module guide lives in the RustF book:

- Book: <https://numerum-tech.github.io/rustf/advanced/modules.html>

## Layering Rule

```text
Base Model -> Model -> Module -> Controller
```

- Controllers handle HTTP only.
- Modules contain business logic.
- Models handle data access.

## Two Module Styles

### 1. Stateless module (default)

Use plain associated functions on a unit struct.

```rust
pub struct UserService;

impl UserService {
    pub async fn find_dashboard_data(user_id: i64) -> rustf::Result<serde_json::Value> {
        Ok(serde_json::json!({ "user_id": user_id }))
    }
}
```

Use directly from controllers:

```rust
let data = crate::modules::user_service::UserService::find_dashboard_data(42).await?;
```

### 2. Shared/stateful module

Use a stateful type only when you need long-lived state such as a mailer,
payment client, cache, or connection wrapper.

Register explicitly:

```rust
MODULE::init()?;
MODULE::register("email-primary", EmailService::new(...))?;
let service = MODULE::get("email-primary")?;
```

## Important Notes

- `auto_modules!()` is for module discovery / IDE support, not automatic
  stateful service registration.
- Do not return `Request` or `Response` from modules.
- Keep framework-specific response work in controllers.
- Put repeated business rules in modules before duplicating controller code.

## Example Split

Controller:

```rust
async fn dashboard(ctx: &mut Context) -> rustf::Result<()> {
    let user_id = ctx.param_int("id")? as i64;
    let data = crate::modules::user_service::UserService::find_dashboard_data(user_id).await?;
    ctx.json(data)
}
```

Module:

```rust
pub struct UserService;

impl UserService {
    pub async fn find_dashboard_data(user_id: i64) -> rustf::Result<serde_json::Value> {
        Ok(serde_json::json!({ "user_id": user_id }))
    }
}
```
