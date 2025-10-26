use rustf::views::totaljs::parser::Parser;
use rustf::views::totaljs::renderer::{RenderContext, Renderer};
use serde_json::json;

#[test]
fn test_real_world_multiline_checkbox() {
    // Exact template from user's report with multi-line if directive
    let template = r#"                        <div class="form-check">
                            <input class="form-check-input" type="checkbox" id="is_active" name="is_active" @{if 
                            M.is_active}checked@{fi}>
                            <label class="form-check-label" for="is_active">
                                @(Participant actif)
                            </label>
                        </div>"#;

    // Parse the template
    let mut parser = Parser::new(template).expect("Failed to create parser");
    let ast = parser.parse().expect("Failed to parse multi-line template");

    // Test with is_active = true
    let data_true = json!({
        "is_active": true
    });

    let context = RenderContext::new(data_true);
    let mut renderer = Renderer::new(context);
    let result = renderer
        .render(&ast)
        .expect("Failed to render with is_active=true");

    // Check that 'checked' appears in the output
    assert!(
        result.contains("checked"),
        "Should contain 'checked' when is_active is true. Output: {}",
        result
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

    // Check that 'checked' does NOT appear in the output
    assert!(
        !result.contains("checked"),
        "Should not contain 'checked' when is_active is false. Output: {}",
        result
    );

    // Verify the rest of the structure is preserved
    println!("Result without checked:\n{}", result);
    assert!(result.contains(r#"type="checkbox""#));
    assert!(result.contains(r#"id="is_active""#));
    assert!(result.contains(r#"name="is_active""#));
    // Localization tags are preserved as-is since we don't have a translator
    assert!(result.contains("@(Participant actif)") || result.contains("Participant actif"));
}
