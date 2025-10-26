#[cfg(test)]
mod tests {
    use crate::views::totaljs::parser::Parser;
    use crate::views::totaljs::lexer::Lexer;
    use crate::views::totaljs::renderer::RenderContext;
    use crate::views::totaljs::ast::{Node, Expression};
    use serde_json::json;

    #[test]
    fn test_elif_parsing() {
        let input = r#"
@{if score > 90}
Excellent
@{elif score > 80}
Good
@{elif score > 70}
Fair
@{else}
Poor
@{fi}
"#;
        
        let lexer = Lexer::new(input);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let result = parser.parse();
        
        assert!(result.is_ok());
        let template = result.unwrap();
        
        // Should have one conditional node after trimming text
        let conditionals: Vec<_> = template.nodes.iter().filter_map(|node| {
            match node {
                Node::Conditional { .. } => Some(node),
                _ => None
            }
        }).collect();
        
        assert_eq!(conditionals.len(), 1);
        
        // Verify elif branches exist
        if let Node::Conditional { else_if_branches, else_branch, .. } = conditionals[0] {
            assert_eq!(else_if_branches.len(), 2); // Two elif branches
            assert!(else_branch.is_some()); // Has else branch
        } else {
            panic!("Expected conditional node");
        }
    }

    #[tokio::test]
    async fn test_elif_rendering() {
        let template_str = r#"@{if score > 90}A@{elif score > 80}B@{elif score > 70}C@{else}F@{fi}"#;
        
        let lexer = Lexer::new(template_str);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let template = parser.parse().expect("Failed to parse");
        
        // Test score = 95 (should render "A")
        let mut context = RenderContext::new();
        context.set("score", json!(95));
        let renderer = crate::views::totaljs::renderer::Renderer::new();
        let output = renderer.render_nodes(&template.nodes, &mut context).await.expect("Failed to render");
        assert_eq!(output.trim(), "A");
        
        // Test score = 85 (should render "B")
        let mut context = RenderContext::new();
        context.set("score", json!(85));
        let output = renderer.render_nodes(&template.nodes, &mut context).await.expect("Failed to render");
        assert_eq!(output.trim(), "B");
        
        // Test score = 75 (should render "C")
        let mut context = RenderContext::new();
        context.set("score", json!(75));
        let output = renderer.render_nodes(&template.nodes, &mut context).await.expect("Failed to render");
        assert_eq!(output.trim(), "C");
        
        // Test score = 50 (should render "F")
        let mut context = RenderContext::new();
        context.set("score", json!(50));
        let output = renderer.render_nodes(&template.nodes, &mut context).await.expect("Failed to render");
        assert_eq!(output.trim(), "F");
    }

    #[tokio::test]
    async fn test_range_with_elif() {
        let template_str = r#"@{foreach n in range(3)}@{if n == 0}first@{elif n == 2}last@{else}middle@{fi} @{end}"#;
        
        let lexer = Lexer::new(template_str);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let template = parser.parse().expect("Failed to parse");
        
        let mut context = RenderContext::new();
        let renderer = crate::views::totaljs::renderer::Renderer::new();
        let output = renderer.render_nodes(&template.nodes, &mut context).await.expect("Failed to render");
        
        // Should output "first middle last "
        assert_eq!(output, "first middle last ");
    }
}