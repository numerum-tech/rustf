# Views Directory

This directory contains Total.js-style HTML templates used by `ctx.view(...)`.

The canonical view guide lives in the RustF book:

- Book: <https://numerum-tech.github.io/rustf/guides/views.html>
- Forms and CSRF: <https://numerum-tech.github.io/rustf/guides/security.html>

## Quick Rules

- RustF uses the Total.js template engine.
- Template names passed to `ctx.view(...)` do **not** include `.html`.
- Layouts live in `views/layouts/`.
- Template data comes from:
  - model: `@{M.key}`
  - repository: `@{R.key}`
  - app globals/helpers: built-in view helpers

## Minimal Example

Controller:

```rust
async fn index(ctx: &mut Context) -> rustf::Result<()> {
    ctx.repository_set("items", json!(["a", "b", "c"]));
    ctx.view("home/index", json!({"title": "Home"}))
}
```

Template `views/home/index.html`:

```html
<section>
    <h1>@{M.title}</h1>

    @{if R.items.length > 0}
    <ul>
        @{foreach item in R.items}
        <li>@{item}</li>
        @{end}
    </ul>
    @{else}
    <p>No items.</p>
    @{fi}
</section>
```

## Forms

```html
<form method="POST" action="/contact">
    <input type="hidden" name="_csrf_token" value="@{csrf_token}">
    <input type="text" name="name">
    <button type="submit">Send</button>
</form>
```

Controller:

```rust
async fn contact_form(ctx: &mut Context) -> rustf::Result<()> {
    ctx.view("contact/index", json!({
        "csrf_token": ctx.generate_csrf(None)?,
    }))
}
```

## Common Syntax

```html
@{title}                  <!-- escaped output -->
@{!html}                  <!-- raw output -->
@{if condition}...@{fi}
@{foreach item in items}...@{end}
@{view('partials/card')}
```

## Notes

- Use the book for the full syntax and helper catalog.
- The older Tera/Jinja examples are obsolete; Total.js is the supported engine.
