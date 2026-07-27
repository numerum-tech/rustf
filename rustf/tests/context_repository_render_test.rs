//! Covers the `Context::view` -> repository path.
//!
//! `Context` holds its repository as a `Value::Object` and hands the engine
//! `&self.repository` directly (no intermediate clone). These tests pin the
//! observable behaviour of that path: repository keys must reach the template
//! as `@{R.key}` while the model reaches it as `@{M.field}`, and the two must
//! stay in their own namespaces.

use rustf::prelude::*;
use serde_json::json;
use std::fs;
use std::sync::Arc;
use tempfile::TempDir;

fn ctx_with_template(body: &str) -> (Context, TempDir) {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("page.html"), body).unwrap();

    let views = Arc::new(rustf::views::ViewEngine::from_directory(
        dir.path().to_str().unwrap(),
    ));
    let request = rustf::http::Request::new("GET", "/test", "HTTP/1.1");

    let mut ctx = Context::new(request, views);
    ctx.layout(""); // no layout file in the temp dir
    (ctx, dir)
}

fn rendered(ctx: &Context) -> String {
    String::from_utf8(ctx.get_response().unwrap().body.clone()).unwrap()
}

#[test]
fn repository_and_model_reach_the_template_in_separate_namespaces() {
    let (mut ctx, _dir) = ctx_with_template("T=@{R.title}|N=@{M.name}|ID=@{R.item.id}");

    ctx.repository_set("title", "Hello");
    ctx.repository_set("item", json!({ "id": 42 }));

    ctx.view("page", json!({ "name": "World" })).unwrap();

    let out = rendered(&ctx);
    assert!(out.contains("T=Hello"), "repository scalar missing: {out}");
    assert!(out.contains("N=World"), "model field missing: {out}");
    assert!(out.contains("ID=42"), "nested repository value missing: {out}");
}

#[test]
fn repository_survives_multiple_renders_of_the_same_context() {
    // The engine used to receive a freshly built clone each render; it now gets
    // a borrow of the live map. Rendering twice must not consume or mutate it.
    let (mut ctx, _dir) = ctx_with_template("T=@{R.title}");
    ctx.repository_set("title", "Persisted");

    ctx.view("page", json!({})).unwrap();
    assert!(rendered(&ctx).contains("T=Persisted"));

    ctx.view("page", json!({})).unwrap();
    assert!(
        rendered(&ctx).contains("T=Persisted"),
        "repository was consumed by the first render"
    );
}

#[test]
fn repository_set_overwrites_and_get_reads_back() {
    let (mut ctx, _dir) = ctx_with_template("T=@{R.title}");

    ctx.repository_set("title", "first");
    ctx.repository_set("title", "second");

    assert_eq!(
        ctx.repository_get("title").and_then(|v| v.as_str()),
        Some("second")
    );
    assert!(ctx.repository_get("absent").is_none());

    ctx.view("page", json!({})).unwrap();
    assert!(rendered(&ctx).contains("T=second"));
}

#[test]
fn repository_clear_preserves_the_object_invariant() {
    // `repository` must remain a Value::Object after clearing, otherwise
    // subsequent `repository_set` calls would silently no-op.
    let (mut ctx, _dir) = ctx_with_template("T=@{R.title}");

    ctx.repository_set("title", "before");
    ctx.repository_clear();
    assert!(ctx.repository_get("title").is_none());

    ctx.repository_set("title", "after");
    assert_eq!(
        ctx.repository_get("title").and_then(|v| v.as_str()),
        Some("after"),
        "repository_set no-opped after clear — object invariant broken"
    );

    ctx.view("page", json!({})).unwrap();
    assert!(rendered(&ctx).contains("T=after"));
}
