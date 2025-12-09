use rustf::prelude::*;
use serde_json::json;

#[test]
fn test_index_simple() {
    // Test simple: just display index
    let template = r##"
        @{foreach item in M.items}
            Index: @{index}
        @{end}
    "##;

    let model = json!({
        "items": ["A", "B"]
    });

    let result = VIEW::render_string(template, model, None);
    assert!(result.is_ok());
    
    let html = result.unwrap();
    eprintln!("Simple index test HTML:\n{}", html);
    assert!(html.contains("Index: 0"));
    assert!(html.contains("Index: 1"));
}

#[test]
fn test_index_in_binary_expression() {
    // Test: index in binary expression
    let template = r##"
        @{foreach item in M.items}
            @{if index == 0}First@{fi}
        @{end}
    "##;

    let model = json!({
        "items": ["A", "B"]
    });

    let result = VIEW::render_string(template, model, None);
    assert!(result.is_ok());
    
    let html = result.unwrap();
    eprintln!("Binary expression test HTML:\n{}", html);
    assert!(html.contains("First"));
}

