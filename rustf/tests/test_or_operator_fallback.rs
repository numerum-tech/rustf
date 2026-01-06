use rustf::prelude::*;
use serde_json::json;

#[test]
fn test_or_operator_with_empty_string() {
    let template = r#"
        <p>@{M.bio || "No bio provided"}</p>
    "#;

    let model = json!({
        "bio": ""
    });

    let result = VIEW::render_string(template, model, None);
    assert!(result.is_ok(), "Template should render successfully");

    let html = result.unwrap();
    eprintln!("Generated HTML: {}", html);
    assert!(html.contains("No bio provided"), "Should use fallback for empty string");
    assert!(!html.contains("<p></p>"), "Should not render empty string");
}

#[test]
fn test_or_operator_with_null() {
    let template = r#"
        <img src="@{M.photo_url || '/default.jpg'}" alt="Avatar">
    "#;

    let model = json!({
        "photo_url": null
    });

    let result = VIEW::render_string(template, model, None);
    assert!(result.is_ok(), "Template should render successfully");

    let html = result.unwrap();
    assert!(html.contains("src=\"/default.jpg\""), "Should use fallback for null");
}

#[test]
fn test_or_operator_with_missing_property() {
    let template = r#"
        <img src="@{M.photo_url || '/assets/img/avatar/1.jpg'}" alt="Avatar">
    "#;

    let model = json!({
        "name": "John"
    });

    let result = VIEW::render_string(template, model, None);
    assert!(result.is_ok(), "Template should render successfully");

    let html = result.unwrap();
    assert!(html.contains("src=\"/assets/img/avatar/1.jpg\""), "Should use fallback for missing property");
}

#[test]
fn test_or_operator_with_truthy_value() {
    let template = r#"
        <img src="@{M.photo_url || '/default.jpg'}" alt="Avatar">
    "#;

    let model = json!({
        "photo_url": "/photos/user123.jpg"
    });

    let result = VIEW::render_string(template, model, None);
    assert!(result.is_ok(), "Template should render successfully");

    let html = result.unwrap();
    assert!(html.contains("src=\"/photos/user123.jpg\""), "Should use actual value when truthy");
    assert!(!html.contains("/default.jpg"), "Should not use fallback when value exists");
}

#[test]
fn test_or_operator_chaining() {
    let template = r#"
        <h1>@{M.custom_title || M.default_title || "Untitled"}</h1>
    "#;

    // Test 1: All values missing
    let model1 = json!({});
    let result1 = VIEW::render_string(template, model1.clone(), None).unwrap();
    assert!(result1.contains("Untitled"), "Should use final fallback");

    // Test 2: Only default_title exists
    let model2 = json!({
        "default_title": "Default Title"
    });
    let result2 = VIEW::render_string(template, model2.clone(), None).unwrap();
    assert!(result2.contains("Default Title"), "Should use default_title");

    // Test 3: custom_title exists
    let model3 = json!({
        "custom_title": "Custom Title",
        "default_title": "Default Title"
    });
    let result3 = VIEW::render_string(template, model3.clone(), None).unwrap();
    assert!(result3.contains("Custom Title"), "Should use custom_title");
    assert!(!result3.contains("Default Title"), "Should not use default_title when custom exists");
}

#[test]
fn test_or_operator_with_session_data() {
    let template = r#"
        <img src="@{session.user.photo_url || '/assets/img/avatar/1.jpg'}" alt="Avatar">
    "#;

    // Test with missing photo_url
    let session = json!({
        "user": {
            "name": "John"
        }
    });

    let result = VIEW::render_string(template, json!({}), Some(session));
    assert!(result.is_ok(), "Template should render successfully");

    let html = result.unwrap();
    assert!(html.contains("src=\"/assets/img/avatar/1.jpg\""), "Should use fallback when session.user.photo_url is missing");

    // Test with existing photo_url
    let session2 = json!({
        "user": {
            "name": "John",
            "photo_url": "/photos/john.jpg"
        }
    });

    let result2 = VIEW::render_string(template, json!({}), Some(session2));
    assert!(result2.is_ok(), "Template should render successfully");

    let html2 = result2.unwrap();
    assert!(html2.contains("src=\"/photos/john.jpg\""), "Should use actual photo_url when present");
    assert!(!html2.contains("/assets/img/avatar/1.jpg"), "Should not use fallback when value exists");
}

#[test]
fn test_or_operator_with_numeric_defaults() {
    let template = r#"
        <p>Items per page: @{M.page_size || 20}</p>
    "#;

    // Test with missing value
    let model1 = json!({});
    let result1 = VIEW::render_string(template, model1, None).unwrap();
    assert!(result1.contains("20"), "Should use numeric fallback");

    // Test with zero (falsy)
    let model2 = json!({
        "page_size": 0
    });
    let result2 = VIEW::render_string(template, model2, None).unwrap();
    assert!(result2.contains("20"), "Should use fallback for zero");

    // Test with truthy value
    let model3 = json!({
        "page_size": 50
    });
    let result3 = VIEW::render_string(template, model3, None).unwrap();
    assert!(result3.contains("50"), "Should use actual value");
    assert!(!result3.contains("20"), "Should not use fallback when value exists");
}

#[test]
fn test_or_operator_in_conditionals() {
    let template = r#"
        @{if M.is_admin || M.is_moderator}
            <div class="staff">Staff Area</div>
        @{fi}
    "#;

    // Test with both false
    let model1 = json!({
        "is_admin": false,
        "is_moderator": false
    });
    let result1 = VIEW::render_string(template, model1, None).unwrap();
    assert!(!result1.contains("Staff Area"), "Should not show staff area when both are false");

    // Test with is_admin true
    let model2 = json!({
        "is_admin": true,
        "is_moderator": false
    });
    let result2 = VIEW::render_string(template, model2, None).unwrap();
    assert!(result2.contains("Staff Area"), "Should show staff area when is_admin is true");

    // Test with is_moderator true
    let model3 = json!({
        "is_admin": false,
        "is_moderator": true
    });
    let result3 = VIEW::render_string(template, model3, None).unwrap();
    assert!(result3.contains("Staff Area"), "Should show staff area when is_moderator is true");
}

