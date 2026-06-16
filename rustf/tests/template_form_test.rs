//! Tests for Total.js-style form-input helpers: `@{text(...)}` / `@{textarea(...)}`.
//!
//! They auto-bind their value from the model field named by the first argument
//! and render the element with the attributes given in the object literal
//! (`{ key: 'val', flag: true }`).

use rustf::prelude::*;
use serde_json::json;

#[test]
fn text_binds_model_value() {
    let html =
        VIEW::render_string("@{text('Email')}", json!({ "Email": "a@b.com" }), None).unwrap();
    assert_eq!(html, r#"<input type="text" name="Email" value="a@b.com" />"#);
}

#[test]
fn text_with_attributes() {
    let html = VIEW::render_string(
        "@{text('Email', { class: 'x', readonly: true })}",
        json!({ "Email": "a@b.com" }),
        None,
    )
    .unwrap();
    assert_eq!(
        html,
        r#"<input type="text" name="Email" value="a@b.com" class="x" readonly />"#
    );
}

#[test]
fn textarea_full_example() {
    let html = VIEW::render_string(
        "@{textarea('nick', { class: 'form', maxlength: 30, placeholder: 'Your name', required: true })}",
        json!({ "nick": "Bob" }),
        None,
    )
    .unwrap();
    assert_eq!(
        html,
        r#"<textarea name="nick" class="form" maxlength="30" placeholder="Your name" required>Bob</textarea>"#
    );
}

#[test]
fn value_is_html_escaped() {
    let html = VIEW::render_string("@{text('x')}", json!({ "x": "a\"<b>" }), None).unwrap();
    assert!(html.contains(r#"value="a&quot;&lt;b&gt;""#), "got: {html}");
}

#[test]
fn missing_field_renders_empty_value() {
    let html = VIEW::render_string("@{text('nope')}", json!({}), None).unwrap();
    assert_eq!(html, r#"<input type="text" name="nope" value="" />"#);
}

#[test]
fn false_boolean_attr_is_omitted() {
    let html = VIEW::render_string(
        "@{text('x', { required: false, disabled: true })}",
        json!({ "x": "v" }),
        None,
    )
    .unwrap();
    assert_eq!(html, r#"<input type="text" name="x" value="v" disabled />"#);
}
