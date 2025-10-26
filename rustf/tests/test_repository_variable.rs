use rustf::views::totaljs::parser::Parser;
use rustf::views::totaljs::renderer::{RenderContext, Renderer};
use serde_json::json;

#[test]
fn test_r_variable_with_length() {
    let template = "@{if R.banks && R.banks.length > 0}HAS_BANKS@{else}NO_BANKS@{fi}";
    let mut parser = Parser::new(template).unwrap();
    let ast = parser.parse().unwrap();

    // Test with repository containing banks array
    let data = json!({}); // Empty model data
    let repository = json!({
        "banks": ["Bank1", "Bank2", "Bank3"]
    });

    let context = RenderContext::new(data).with_repository(repository);
    let mut renderer = Renderer::new(context);
    let result = renderer.render(&ast).unwrap();
    assert_eq!(
        result.trim(),
        "HAS_BANKS",
        "Should show HAS_BANKS when repository has banks array"
    );

    // Test with empty array in repository
    let repository_empty = json!({
        "banks": []
    });

    let context = RenderContext::new(json!({})).with_repository(repository_empty);
    let mut renderer = Renderer::new(context);
    let result = renderer.render(&ast).unwrap();
    assert_eq!(
        result.trim(),
        "NO_BANKS",
        "Should show NO_BANKS when repository banks array is empty"
    );

    // Test without banks in repository
    let repository_no_banks = json!({});

    let context = RenderContext::new(json!({})).with_repository(repository_no_banks);
    let mut renderer = Renderer::new(context);
    let result = renderer.render(&ast).unwrap();
    assert_eq!(
        result.trim(),
        "NO_BANKS",
        "Should show NO_BANKS when repository has no banks"
    );
}

#[test]
fn test_app_variable_with_length() {
    let template = "@{if APP.items && APP.items.length > 2}MANY_ITEMS@{else}FEW_ITEMS@{fi}";
    let mut parser = Parser::new(template).unwrap();
    let ast = parser.parse().unwrap();

    // Test with global repository containing items
    let data = json!({});
    let global_repository = json!({
        "items": ["Item1", "Item2", "Item3", "Item4"]
    });

    let context = RenderContext::new(data).with_global_repository(global_repository);
    let mut renderer = Renderer::new(context);
    let result = renderer.render(&ast).unwrap();
    assert_eq!(
        result.trim(),
        "MANY_ITEMS",
        "Should show MANY_ITEMS when APP has more than 2 items"
    );

    // Test with fewer items
    let global_repository_few = json!({
        "items": ["Item1"]
    });

    let context = RenderContext::new(json!({})).with_global_repository(global_repository_few);
    let mut renderer = Renderer::new(context);
    let result = renderer.render(&ast).unwrap();
    assert_eq!(
        result.trim(),
        "FEW_ITEMS",
        "Should show FEW_ITEMS when APP has 2 or fewer items"
    );
}

#[test]
fn test_main_alias_for_app() {
    let template = "@{if MAIN.config && MAIN.config.enabled}ENABLED@{else}DISABLED@{fi}";
    let mut parser = Parser::new(template).unwrap();
    let ast = parser.parse().unwrap();

    let global_repository = json!({
        "config": {
            "enabled": true
        }
    });

    let context = RenderContext::new(json!({})).with_global_repository(global_repository);
    let mut renderer = Renderer::new(context);
    let result = renderer.render(&ast).unwrap();
    assert_eq!(
        result.trim(),
        "ENABLED",
        "MAIN should work as alias for APP"
    );
}
