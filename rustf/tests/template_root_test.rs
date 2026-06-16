//! `@{root}` resolves the configured `views.default_root` with the trailing
//! `/` stripped (Total.js semantics), for apps served under a sub-path.

use rustf::views::totaljs::parser::Parser;
use rustf::views::totaljs::renderer::{RenderContext, Renderer};
use serde_json::json;

fn render_root(default_root: &str) -> String {
    let ast = Parser::new("@{root}/static").unwrap().parse().unwrap();
    let ctx = RenderContext::new(json!({}))
        .with_conf(json!({ "views": { "default_root": default_root } }));
    Renderer::new(ctx).render(&ast).unwrap()
}

#[test]
fn root_strips_trailing_slash() {
    assert_eq!(render_root("/app/"), "/app/static");
}

#[test]
fn root_without_trailing_slash_unchanged() {
    assert_eq!(render_root("/app"), "/app/static");
}

#[test]
fn root_defaults_to_empty() {
    assert_eq!(render_root(""), "/static");
}

#[test]
fn root_reads_nested_views_config() {
    // Regression: the value lives at conf.views.default_root (AppConfig shape),
    // not at the top level — @{root} must look there.
    let ast = Parser::new("[@{root}]").unwrap().parse().unwrap();
    let ctx = RenderContext::new(json!({}))
        .with_conf(json!({ "views": { "default_root": "/sub" }, "server": {} }));
    assert_eq!(Renderer::new(ctx).render(&ast).unwrap(), "[/sub]");
}
