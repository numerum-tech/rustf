//! Tests for the Total.js-style `@{title('value')}` page-title helper.
//!
//! `@{title(...)}` stores the title in meta data and renders nothing inline;
//! `@{title}` outputs the stored value (used by layouts).

use rustf::prelude::*;
use serde_json::json;

#[test]
fn title_setter_renders_nothing_and_is_readable() {
    let html = VIEW::render_string(
        "@{title('Connexion')}<title>@{title}</title>",
        json!({}),
        None,
    )
    .unwrap();
    assert_eq!(html, "<title>Connexion</title>");
}

#[test]
fn title_accepts_expression_argument() {
    let html = VIEW::render_string(
        "@{title(M.page)}[@{title}]",
        json!({ "page": "Accueil" }),
        None,
    )
    .unwrap();
    assert_eq!(html, "[Accueil]");
}

#[test]
fn title_supports_or_fallback() {
    let html =
        VIEW::render_string("@{title(M.page || 'Default')}[@{title}]", json!({}), None).unwrap();
    assert_eq!(html, "[Default]");
}

#[test]
fn title_unset_is_empty() {
    let html = VIEW::render_string("[@{title}]", json!({}), None).unwrap();
    assert_eq!(html, "[]");
}

#[test]
fn title_is_html_escaped_on_output() {
    let html = VIEW::render_string("@{title('<x>')}[@{title}]", json!({}), None).unwrap();
    // Stored raw, escaped when output via @{title}
    assert!(html.contains("&lt;x&gt;"), "got: {html}");
}
