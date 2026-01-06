use rustf::prelude::*;
use serde_json::json;

#[test]
fn test_simple_or() {
    let template = r#"@{M.a || "default"}"#;
    let model = json!({});
    let result = VIEW::render_string(template, model, None);
    eprintln!("Result: {:?}", result);
    if let Ok(html) = result {
        eprintln!("HTML: '{}'", html);
        assert!(html.contains("default"), "Should contain default");
    }
}


