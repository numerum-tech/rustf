use rustf::views::totaljs::parser::Parser;

#[test]
fn test_simple_property_access() {
    let template = "@{if M.banks}content@{fi}";
    let mut parser = Parser::new(template).expect("Failed to create parser");
    let ast = parser
        .parse()
        .expect("Failed to parse simple property access");
    assert!(!ast.nodes.is_empty());
}

#[test]
fn test_complex_expression_with_length() {
    // This is the problematic expression that was failing
    let template = "@{if M.banks && M.banks.length > 0}content@{fi}";
    let mut parser = Parser::new(template).expect("Failed to create parser");
    let ast = parser
        .parse()
        .expect("Failed to parse complex expression with length");
    assert!(!ast.nodes.is_empty());
}

#[test]
fn test_expression_with_parentheses() {
    let template = "@{if (M.banks && M.banks.length) > 0}content@{fi}";
    let mut parser = Parser::new(template).expect("Failed to create parser");
    let ast = parser
        .parse()
        .expect("Failed to parse expression with parentheses");
    assert!(!ast.nodes.is_empty());
}

#[test]
fn test_multiple_operators_precedence() {
    let template = "@{if a > 0 && b < 10 || c == 5}content@{fi}";
    let mut parser = Parser::new(template).expect("Failed to create parser");
    let ast = parser.parse().expect("Failed to parse multiple operators");
    assert!(!ast.nodes.is_empty());
}

#[test]
fn test_nested_property_chains() {
    let template = "@{if user.profile.settings.notifications.email}content@{fi}";
    let mut parser = Parser::new(template).expect("Failed to create parser");
    let ast = parser
        .parse()
        .expect("Failed to parse nested property chains");
    assert!(!ast.nodes.is_empty());
}

#[test]
fn test_comparison_with_property_access() {
    let template = "@{if items.length >= 10}content@{fi}";
    let mut parser = Parser::new(template).expect("Failed to create parser");
    let ast = parser
        .parse()
        .expect("Failed to parse comparison with property access");
    assert!(!ast.nodes.is_empty());
}
