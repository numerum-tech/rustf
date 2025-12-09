use rustf::prelude::*;
use serde_json::json;

#[test]
fn test_index_variable_in_loop() {
    let template = r##"
        <div class="carousel-indicators">
        @{foreach pub in R.latest_publications}
            <button type="button" data-bs-target="#carouselExampleIndicators"
                data-bs-slide-to="@{index}" class="@{index == 1 ? "active" : ""}" aria-current="true"
                aria-label="Slide @{index}"></button>
        @{end}
        </div>
    "##;

    let repository = json!({
        "latest_publications": [
            {"title": "Publication 1"},
            {"title": "Publication 2"},
            {"title": "Publication 3"}
        ]
    });

    let result = VIEW::render_string(template, json!({}), Some(repository));
    assert!(result.is_ok(), "Template should render successfully");

    let html = result.unwrap();
    
    // Debug: print the HTML to see what was generated
    eprintln!("Generated HTML:\n{}", html);
    
    // Check that index values are rendered
    assert!(html.contains(r#"data-bs-slide-to="0""#), "Should contain index 0");
    assert!(html.contains(r#"data-bs-slide-to="1""#), "Should contain index 1");
    assert!(html.contains(r#"data-bs-slide-to="2""#), "Should contain index 2");
    
    // Check that the ternary expression works (index == 1 should be 'active')
    assert!(html.contains(r#"class="active""#), "Index 1 should have 'active' class");
    
    // Check that other indices don't have 'active' class (they should have empty class)
    // Count occurrences of 'active' - should be exactly 1
    let active_count = html.matches(r#"class="active""#).count();
    assert_eq!(active_count, 1, "Only index 1 should have 'active' class");
}

#[test]
fn test_index_in_ternary_expression() {
    let template = r##"
        @{foreach item in M.items}
            <div class="@{index == 0 ? "first" : index == 1 ? "second" : "other"}">
                Item @{index}: @{item}
            </div>
        @{end}
    "##;

    let model = json!({
        "items": ["A", "B", "C"]
    });

    let result = VIEW::render_string(template, model, None);
    assert!(result.is_ok(), "Template should render successfully");

    let html = result.unwrap();
    
    // Check that the ternary expressions work correctly
    assert!(html.contains(r#"class="first""#), "Index 0 should have 'first' class");
    assert!(html.contains(r#"class="second""#), "Index 1 should have 'second' class");
    assert!(html.contains(r#"class="other""#), "Index 2 should have 'other' class");
    
    // Check that index values are displayed
    assert!(html.contains("Item 0:"), "Should display index 0");
    assert!(html.contains("Item 1:"), "Should display index 1");
    assert!(html.contains("Item 2:"), "Should display index 2");
}

#[test]
fn test_index_comparison_operations() {
    let template = r##"
        @{foreach item in M.items}
            @{if index == 0}
                <div class="first">First item</div>
            @{else if index == 1}
                <div class="second">Second item</div>
            @{else if index > 1}
                <div class="other">Other item @{index}</div>
            @{fi}
        @{end}
    "##;

    let model = json!({
        "items": ["A", "B", "C", "D"]
    });

    let result = VIEW::render_string(template, model, None);
    assert!(result.is_ok(), "Template should render successfully");

    let html = result.unwrap();
    
    // Debug: print the HTML to see what was generated
    eprintln!("Comparison operations test HTML:\n{}", html);
    
    // Check that conditional logic works with index
    assert!(html.contains(r#"class="first""#), "Index 0 should match first condition");
    assert!(html.contains(r#"class="second""#), "Index 1 should match second condition");
    assert!(html.contains(r#"class="other""#), "Indices > 1 should match third condition");
    assert!(html.contains("Other item 2"), "Should display index 2");
    assert!(html.contains("Other item 3"), "Should display index 3");
}

