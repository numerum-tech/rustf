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
    assert_eq!(
        html,
        r#"<input type="text" name="Email" value="a@b.com" />"#
    );
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

#[test]
fn password_and_hidden() {
    let m = json!({ "pass": "secret", "tok": "abc" });
    assert_eq!(
        VIEW::render_string("@{password('pass', { required: true })}", m.clone(), None).unwrap(),
        r#"<input type="password" name="pass" value="secret" required />"#
    );
    assert_eq!(
        VIEW::render_string("@{hidden('tok')}", m, None).unwrap(),
        r#"<input type="hidden" name="tok" value="abc" />"#
    );
}

#[test]
fn checkbox_checked_with_label() {
    let html = VIEW::render_string(
        "@{checkbox('agree', 'I agree')}",
        json!({ "agree": true }),
        None,
    )
    .unwrap();
    assert_eq!(
        html,
        r#"<label><input type="checkbox" name="agree" checked /> I agree</label>"#
    );
}

#[test]
fn checkbox_unchecked_no_label() {
    let html = VIEW::render_string("@{checkbox('news')}", json!({}), None).unwrap();
    assert_eq!(html, r#"<input type="checkbox" name="news" />"#);
}

#[test]
fn radio_checked_when_value_matches() {
    let html = VIEW::render_string(
        "@{radio('gender', 'male', 'Male')}",
        json!({ "gender": "male" }),
        None,
    )
    .unwrap();
    assert_eq!(
        html,
        r#"<label><input type="radio" name="gender" value="male" checked /> Male</label>"#
    );
}

#[test]
fn radio_with_object_label_and_attrs() {
    let html = VIEW::render_string(
        "@{radio('gender', 'female', { label: 'Female', class: 'f' })}",
        json!({ "gender": "male" }),
        None,
    )
    .unwrap();
    assert_eq!(
        html,
        r#"<label><input type="radio" name="gender" value="female" class="f" /> Female</label>"#
    );
}

#[test]
fn description_stored_and_read() {
    let html = VIEW::render_string(
        "@{description('My page')}<meta name=\"description\" content=\"@{description}\">",
        json!({}),
        None,
    )
    .unwrap();
    assert_eq!(html, r#"<meta name="description" content="My page">"#);
}
