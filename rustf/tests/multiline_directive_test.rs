use rustf::views::totaljs::parser::Parser;
use rustf::views::totaljs::renderer::{RenderContext, Renderer};
use serde_json::json;

#[test]
fn test_multiline_if_directive() {
    // Test case with multi-line if directive - exactly as user reported
    let template = r#"<input type="checkbox" @{if 
    M.is_active}checked@{fi}>"#;

    // Parse the template
    let mut parser = Parser::new(template).expect("Failed to create parser");
    let ast = parser.parse().expect("Failed to parse template");

    // Test with is_active = true
    let data_true = json!({
        "is_active": true
    });

    let context = RenderContext::new(data_true);
    let mut renderer = Renderer::new(context);
    let result = renderer
        .render(&ast)
        .expect("Failed to render with is_active=true");

    assert_eq!(
        result.trim(),
        r#"<input type="checkbox" checked>"#,
        "Should render 'checked' when is_active is true"
    );

    // Test with is_active = false
    let data_false = json!({
        "is_active": false
    });

    let context = RenderContext::new(data_false);
    let mut renderer = Renderer::new(context);
    let result = renderer
        .render(&ast)
        .expect("Failed to render with is_active=false");

    assert_eq!(
        result.trim(),
        r#"<input type="checkbox" >"#,
        "Should not render 'checked' when is_active is false"
    );
}

#[test]
fn test_multiline_foreach_directive() {
    // Test multi-line foreach directive
    let template = r#"@{foreach 
    item 
    in 
    M.items}@{item} @{end}"#;

    let mut parser = Parser::new(template).expect("Failed to create parser");
    let ast = parser.parse().expect("Failed to parse template");

    let data = json!({
        "items": ["A", "B", "C"]
    });

    let context = RenderContext::new(data);
    let mut renderer = Renderer::new(context);
    let result = renderer.render(&ast).expect("Failed to render foreach");

    assert_eq!(
        result.trim(),
        "A B C",
        "Should handle multi-line foreach directive"
    );
}
