//! Regression tests for `@{break}`/`@{continue}` inside conditionals and the
//! `@{mobile}` view property.

use rustf::prelude::*;
use rustf::views::totaljs::parser::Parser;
use rustf::views::totaljs::renderer::{RenderContext, Renderer};
use serde_json::json;

fn render_string(tpl: &str, model: serde_json::Value) -> String {
    VIEW::render_string(tpl, model, None).unwrap()
}

#[test]
fn break_inside_if_stops_loop() {
    let out = render_string(
        "@{foreach m in M.items}@{m}@{if m == 2}@{break}@{fi}@{end}",
        json!({ "items": [1, 2, 3, 4] }),
    );
    assert_eq!(out, "12");
}

#[test]
fn continue_inside_if_skips_iteration() {
    let out = render_string(
        "@{foreach m in M.items}@{if m == 2}@{continue}@{fi}@{m}@{end}",
        json!({ "items": [1, 2, 3, 4] }),
    );
    assert_eq!(out, "134");
}

#[test]
fn break_directly_in_loop_body_still_works() {
    let out = render_string(
        "@{foreach m in M.items}@{m}@{break}@{end}",
        json!({ "items": [1, 2, 3] }),
    );
    assert_eq!(out, "1");
}

#[test]
fn break_inside_else_branch() {
    let out = render_string(
        "@{foreach m in M.items}@{if m == 0}x@{else}@{break}@{fi}@{end}",
        json!({ "items": [5, 6] }),
    );
    assert_eq!(out, "");
}

fn render_mobile(mobile: bool) -> String {
    let ast = Parser::new("@{if mobile}M@{else}D@{fi}")
        .unwrap()
        .parse()
        .unwrap();
    Renderer::new(RenderContext::new(json!({})).with_mobile(mobile))
        .render(&ast)
        .unwrap()
}

#[test]
fn mobile_property_reflects_flag() {
    assert_eq!(render_mobile(true), "M");
    assert_eq!(render_mobile(false), "D");
}

#[test]
fn is_mobile_detects_user_agents() {
    assert!(rustf::views::is_mobile(
        "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X)"
    ));
    assert!(rustf::views::is_mobile("Mozilla/5.0 (Linux; Android 14)"));
    assert!(!rustf::views::is_mobile(
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15)"
    ));
}
