use rustf::views::totaljs::parser::Parser;
use rustf::views::totaljs::renderer::{RenderContext, Renderer};
use serde_json::json;

#[test]
fn test_m_variable_resolution() {
    // Test simple M variable access
    let template = "@{M.banks}";
    let mut parser = Parser::new(template).unwrap();
    let ast = parser.parse().unwrap();

    let data = json!({
        "banks": ["Bank1", "Bank2", "Bank3"]
    });

    let context = RenderContext::new(data);
    let mut renderer = Renderer::new(context);
    let result = renderer.render(&ast).unwrap();

    // Arrays should be rendered as JSON
    assert!(result.contains("Bank1"));
    assert!(result.contains("Bank2"));
    assert!(result.contains("Bank3"));
}

#[test]
fn test_m_variable_with_length() {
    let template = "@{if M.banks && M.banks.length > 0}HAS_BANKS@{else}NO_BANKS@{fi}";
    let mut parser = Parser::new(template).unwrap();
    let ast = parser.parse().unwrap();

    // Test with array
    let data_with_banks = json!({
        "banks": ["Bank1", "Bank2", "Bank3"]
    });

    let context = RenderContext::new(data_with_banks);
    let mut renderer = Renderer::new(context);
    let result = renderer.render(&ast).unwrap();
    assert_eq!(
        result.trim(),
        "HAS_BANKS",
        "Should show HAS_BANKS when array has items"
    );

    // Test with empty array
    let data_empty = json!({
        "banks": []
    });

    let context = RenderContext::new(data_empty);
    let mut renderer = Renderer::new(context);
    let result = renderer.render(&ast).unwrap();
    assert_eq!(
        result.trim(),
        "NO_BANKS",
        "Should show NO_BANKS when array is empty"
    );

    // Test without banks property
    let data_no_banks = json!({});

    let context = RenderContext::new(data_no_banks);
    let mut renderer = Renderer::new(context);
    let result = renderer.render(&ast).unwrap();
    assert_eq!(
        result.trim(),
        "NO_BANKS",
        "Should show NO_BANKS when property is missing"
    );
}

#[test]
fn test_parentheses_expression() {
    let template = "@{if (M.banks && M.banks.length > 0)}OK@{else}NOK@{fi}";
    let mut parser = Parser::new(template).unwrap();
    let ast = parser.parse().unwrap();

    let data = json!({
        "banks": ["Bank1", "Bank2"]
    });

    let context = RenderContext::new(data);
    let mut renderer = Renderer::new(context);
    let result = renderer.render(&ast).unwrap();
    assert_eq!(result.trim(), "OK");
}
