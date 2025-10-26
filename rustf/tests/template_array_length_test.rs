use rustf::views::totaljs::parser::Parser;
use rustf::views::totaljs::renderer::{RenderContext, Renderer};
use serde_json::json;

#[test]
fn test_array_length_in_condition() {
    // Template with array length check
    let template = "@{if M.banks && M.banks.length > 0}OK@{else}NOK@{fi}";

    // Parse the template
    let mut parser = Parser::new(template).expect("Failed to create parser");
    let ast = parser.parse().expect("Failed to parse template");

    // Test with array that has items
    let data_with_banks = json!({
        "banks": ["Bank1", "Bank2", "Bank3"]
    });

    let context = RenderContext::new(data_with_banks);
    let mut renderer = Renderer::new(context);
    let result = renderer.render(&ast).expect("Failed to render with banks");
    assert_eq!(
        result.trim(),
        "OK",
        "Should show OK when banks array has items"
    );

    // Test with empty array
    let data_empty_banks = json!({
        "banks": []
    });

    let context = RenderContext::new(data_empty_banks);
    let mut renderer = Renderer::new(context);
    let result = renderer
        .render(&ast)
        .expect("Failed to render with empty banks");
    assert_eq!(
        result.trim(),
        "NOK",
        "Should show NOK when banks array is empty"
    );

    // Test with no banks property
    let data_no_banks = json!({});

    let context = RenderContext::new(data_no_banks);
    let mut renderer = Renderer::new(context);
    let result = renderer
        .render(&ast)
        .expect("Failed to render without banks");
    assert_eq!(
        result.trim(),
        "NOK",
        "Should show NOK when banks property doesn't exist"
    );
}

#[test]
fn test_string_length_property() {
    let template = "@{if name.length > 5}Long name@{else}Short name@{fi}";

    let mut parser = Parser::new(template).expect("Failed to create parser");
    let ast = parser.parse().expect("Failed to parse template");

    // Test with long string
    let data_long = json!({
        "name": "Alexander"
    });

    let context = RenderContext::new(data_long);
    let mut renderer = Renderer::new(context);
    let result = renderer.render(&ast).expect("Failed to render");
    assert_eq!(result.trim(), "Long name");

    // Test with short string
    let data_short = json!({
        "name": "Bob"
    });

    let context = RenderContext::new(data_short);
    let mut renderer = Renderer::new(context);
    let result = renderer.render(&ast).expect("Failed to render");
    assert_eq!(result.trim(), "Short name");
}

#[test]
fn test_array_index_access() {
    let template = "@{items.0} and @{items.1}";

    let mut parser = Parser::new(template).expect("Failed to create parser");
    let ast = parser.parse().expect("Failed to parse template");

    let data = json!({
        "items": ["First", "Second", "Third"]
    });

    let context = RenderContext::new(data);
    let mut renderer = Renderer::new(context);
    let result = renderer.render(&ast).expect("Failed to render");
    assert_eq!(result.trim(), "First and Second");
}
