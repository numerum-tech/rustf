use super::ast::{BinaryOperator, Expression, Node, Template, UnaryOperator};
use super::lexer::{Lexer, Token, TokenKind};
use crate::error::{Error, Result};

/// Parser for Total.js templates
pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
    current_token: Token,
}

impl Parser {
    /// Create a new parser from input string
    pub fn new(input: &str) -> Result<Self> {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        if tokens.is_empty() {
            return Err(Error::template("Empty template".to_string()));
        }

        let current_token = tokens[0].clone();

        Ok(Self {
            tokens,
            position: 0,
            current_token,
        })
    }

    /// Advance to the next token
    fn advance(&mut self) {
        self.position += 1;
        if self.position < self.tokens.len() {
            self.current_token = self.tokens[self.position].clone();
        }
    }

    /// Peek at the next token without advancing
    #[allow(dead_code)]
    fn peek(&self) -> Option<&Token> {
        if self.position + 1 < self.tokens.len() {
            Some(&self.tokens[self.position + 1])
        } else {
            None
        }
    }

    /// Check if we're at the end of tokens
    fn is_at_end(&self) -> bool {
        matches!(self.current_token.kind, TokenKind::Eof)
    }

    /// Parse the entire template
    pub fn parse(&mut self) -> Result<Template> {
        let mut template = Template::new();

        while !self.is_at_end() {
            // Skip newlines at the top level
            if matches!(self.current_token.kind, TokenKind::Newline) {
                self.advance();
                continue;
            }

            let node = self.parse_node()?;
            template.nodes.push(node);
        }

        // Extract sections and helpers from the main node list
        template.extract_sections();
        template.extract_helpers();

        Ok(template)
    }

    /// Parse a single node
    fn parse_node(&mut self) -> Result<Node> {
        match &self.current_token.kind {
            TokenKind::Text(text) => {
                let node = Node::Text(text.clone());
                self.advance();
                Ok(node)
            }

            TokenKind::Variable(name) => {
                let node = Node::Variable {
                    name: name.clone(),
                    raw: false,
                };
                self.advance();
                Ok(node)
            }

            TokenKind::RawVariable(name) => {
                let node = Node::Variable {
                    name: name.clone(),
                    raw: true,
                };
                self.advance();
                Ok(node)
            }

            TokenKind::If(condition) => self.parse_conditional(condition.clone()),

            TokenKind::Foreach(item, collection) => {
                self.parse_loop(item.clone(), collection.clone())
            }

            TokenKind::SectionDef(name) => self.parse_section(name.clone()),

            TokenKind::SectionCall(name) => {
                let node = Node::SectionCall(name.clone());
                self.advance();
                Ok(node)
            }

            TokenKind::HelperDef(name, params) => {
                self.parse_helper_def(name.clone(), params.clone())
            }

            TokenKind::HelperCall(name, args) => {
                let node = Node::HelperCall {
                    name: name.clone(),
                    args: args.iter().map(|a| Expression::parse_value(a)).collect(),
                };
                self.advance();
                Ok(node)
            }

            TokenKind::View(name, model) => {
                let node = Node::View {
                    name: name.clone(),
                    model: model.as_ref().map(|m| Expression::parse_value(m)),
                };
                self.advance();
                Ok(node)
            }

            TokenKind::Import(files) => {
                let node = Node::Import(files.clone());
                self.advance();
                Ok(node)
            }

            TokenKind::Meta(title, desc, keywords) => {
                let node = Node::Meta {
                    title: title.clone(),
                    description: desc.clone(),
                    keywords: keywords.clone(),
                };
                self.advance();
                Ok(node)
            }

            TokenKind::Body => {
                self.advance();
                Ok(Node::Body)
            }

            TokenKind::Head => {
                self.advance();
                Ok(Node::Head)
            }

            TokenKind::Content => {
                self.advance();
                Ok(Node::Content)
            }

            TokenKind::Csrf => {
                self.advance();
                Ok(Node::Csrf)
            }

            TokenKind::Translate(text) => {
                let node = Node::Translate {
                    text: text.clone(),
                    is_key: false,
                };
                self.advance();
                Ok(node)
            }

            TokenKind::TranslateKey(key) => {
                let node = Node::Translate {
                    text: key.clone(),
                    is_key: true,
                };
                self.advance();
                Ok(node)
            }

            TokenKind::Config(key) => {
                let node = Node::Config(key.clone());
                self.advance();
                Ok(node)
            }

            TokenKind::Repository(key) => {
                let node = Node::Repository(key.clone());
                self.advance();
                Ok(node)
            }

            TokenKind::Session(key) => {
                let node = Node::Session(key.clone());
                self.advance();
                Ok(node)
            }

            TokenKind::Query(key) => {
                let node = Node::Query(key.clone());
                self.advance();
                Ok(node)
            }

            TokenKind::User(prop) => {
                let node = Node::User(prop.clone());
                self.advance();
                Ok(node)
            }

            TokenKind::App(key) => {
                let node = Node::App(key.clone());
                self.advance();
                Ok(node)
            }

            TokenKind::Main(key) => {
                let node = Node::Main(key.clone());
                self.advance();
                Ok(node)
            }

            TokenKind::R(key) => {
                let node = Node::R(key.clone());
                self.advance();
                Ok(node)
            }

            TokenKind::Model(key) => {
                let node = Node::Model(key.clone());
                self.advance();
                Ok(node)
            }

            TokenKind::M(key) => {
                let node = Node::M(key.clone());
                self.advance();
                Ok(node)
            }

            TokenKind::Index => {
                self.advance();
                Ok(Node::Index)
            }

            TokenKind::Break => {
                self.advance();
                Ok(Node::Break)
            }

            TokenKind::Continue => {
                self.advance();
                Ok(Node::Continue)
            }

            TokenKind::Newline => {
                // Include newlines in the AST for proper formatting
                let node = Node::Text("\n".to_string());
                self.advance();
                Ok(node)
            }

            _ => Err(Error::template(format!(
                "Unexpected token: {} at line {}",
                self.current_token.kind,
                self.current_token.line
            ))),
        }
    }

    /// Parse a conditional block
    fn parse_conditional(&mut self, condition: String) -> Result<Node> {
        self.advance(); // Skip @{if ...}

        let mut then_branch = Vec::new();
        let mut else_if_branches = Vec::new();
        let mut else_branch = None;

        // Parse the then branch
        while !self.is_at_end() {
            match &self.current_token.kind {
                TokenKind::Else => {
                    self.advance();
                    break;
                }
                TokenKind::ElseIf(cond) => {
                    let else_if_condition = self.parse_expression(cond)?;
                    self.advance();
                    let mut else_if_body = Vec::new();

                    while !self.is_at_end() {
                        match &self.current_token.kind {
                            TokenKind::Else | TokenKind::ElseIf(_) | TokenKind::Fi => break,
                            _ => {
                                let node = self.parse_node()?;
                                else_if_body.push(node);
                            }
                        }
                    }

                    else_if_branches.push((else_if_condition, else_if_body));
                }
                TokenKind::Fi => {
                    self.advance();
                    break;
                }
                _ => {
                    let node = self.parse_node()?;
                    then_branch.push(node);
                }
            }
        }

        // Parse else branch if present
        if matches!(self.tokens[self.position - 1].kind, TokenKind::Else) {
            let mut else_nodes = Vec::new();

            while !self.is_at_end() {
                if matches!(self.current_token.kind, TokenKind::Fi) {
                    self.advance();
                    break;
                }

                let node = self.parse_node()?;
                else_nodes.push(node);
            }

            else_branch = Some(else_nodes);
        }

        Ok(Node::Conditional {
            condition: self.parse_expression(&condition)?,
            then_branch,
            else_if_branches,
            else_branch,
        })
    }

    /// Parse a loop block
    fn parse_loop(&mut self, item_name: String, collection: String) -> Result<Node> {
        self.advance(); // Skip @{foreach ...}

        let mut body = Vec::new();

        while !self.is_at_end() {
            if matches!(self.current_token.kind, TokenKind::End) {
                self.advance();
                break;
            }

            let node = self.parse_node()?;
            body.push(node);
        }

        Ok(Node::Loop {
            item_name,
            collection: self.parse_expression(&collection)?,
            body,
        })
    }

    /// Parse a section definition
    fn parse_section(&mut self, name: String) -> Result<Node> {
        self.advance(); // Skip @{section ...}

        let mut content = Vec::new();

        while !self.is_at_end() {
            if matches!(self.current_token.kind, TokenKind::End) {
                self.advance();
                break;
            }

            let node = self.parse_node()?;
            content.push(node);
        }

        Ok(Node::SectionDef { name, content })
    }

    /// Parse a helper definition
    fn parse_helper_def(&mut self, name: String, params: Vec<String>) -> Result<Node> {
        self.advance(); // Skip @{helper ...}

        let mut body = Vec::new();

        while !self.is_at_end() {
            if matches!(self.current_token.kind, TokenKind::End) {
                self.advance();
                break;
            }

            let node = self.parse_node()?;
            body.push(node);
        }

        Ok(Node::HelperDef { name, params, body })
    }

    /// Parse an expression from a string
    fn parse_expression(&self, expr_str: &str) -> Result<Expression> {
        // This is a simplified expression parser
        // In a full implementation, this would handle complex expressions
        // with operators, function calls, etc.

        let trimmed = expr_str.trim();

        // Handle parentheses for grouping
        if trimmed.starts_with('(') && trimmed.ends_with(')') {
            // Check if the parentheses are balanced and this is a grouped expression
            let inner = &trimmed[1..trimmed.len() - 1];
            if self.is_balanced_parentheses(inner) {
                return self.parse_expression(inner);
            }
        }

        // Check for function calls (must come before operator parsing)
        // Function call pattern: name(args) where name is alphanumeric/underscore
        if let Some(paren_pos) = trimmed.find('(') {
            let potential_name = &trimmed[..paren_pos];
            // Check if this looks like a function name (alphanumeric + underscore only)
            if potential_name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_')
                && trimmed.ends_with(')')
            {
                // This looks like a function call
                let args_str = &trimmed[paren_pos + 1..trimmed.len() - 1];
                let args = self.parse_function_args(args_str)?;
                return Ok(Expression::FunctionCall {
                    name: potential_name.to_string(),
                    args,
                });
            }
        }

        // Check for binary operators
        if let Some(pos) = self.find_operator(trimmed) {
            let (left_str, op, right_str) = self.split_at_operator(trimmed, pos)?;

            let left = Box::new(self.parse_expression(left_str)?);
            let right = Box::new(self.parse_expression(right_str)?);

            return Ok(Expression::BinaryOp { left, op, right });
        }

        // Check for unary operators
        if trimmed.starts_with('!') {
            let operand = Box::new(self.parse_expression(&trimmed[1..])?);
            return Ok(Expression::UnaryOp {
                op: UnaryOperator::Not,
                operand,
            });
        }

        if trimmed.starts_with('-') && !trimmed[1..].starts_with(|c: char| c.is_ascii_digit()) {
            let operand = Box::new(self.parse_expression(&trimmed[1..])?);
            return Ok(Expression::UnaryOp {
                op: UnaryOperator::Minus,
                operand,
            });
        }

        // Parse as a simple value
        Ok(Expression::parse_value(trimmed))
    }

    /// Parse function arguments
    fn parse_function_args(&self, args_str: &str) -> Result<Vec<Expression>> {
        if args_str.trim().is_empty() {
            return Ok(Vec::new());
        }

        let mut args = Vec::new();
        let mut current_arg = String::new();
        let mut paren_depth = 0;
        let mut in_single_quote = false;
        let mut in_double_quote = false;

        for ch in args_str.chars() {
            if ch == '(' && !in_single_quote && !in_double_quote {
                paren_depth += 1;
            } else if ch == ')' && !in_single_quote && !in_double_quote {
                paren_depth -= 1;
            } else if ch == '\'' && !in_double_quote {
                in_single_quote = !in_single_quote;
            } else if ch == '"' && !in_single_quote {
                in_double_quote = !in_double_quote;
            }

            if ch == ',' && paren_depth == 0 && !in_single_quote && !in_double_quote {
                // End of argument
                args.push(self.parse_expression(current_arg.trim())?);
                current_arg.clear();
            } else {
                current_arg.push(ch);
            }
        }

        // Don't forget the last argument
        if !current_arg.trim().is_empty() {
            args.push(self.parse_expression(current_arg.trim())?);
        }

        Ok(args)
    }

    /// Check if parentheses are balanced in a string
    fn is_balanced_parentheses(&self, s: &str) -> bool {
        let mut depth = 0;
        let mut in_single_quote = false;
        let mut in_double_quote = false;
        let mut last_was_backslash = false;

        for ch in s.chars() {
            if last_was_backslash {
                last_was_backslash = false;
                continue;
            }

            if ch == '\\' && (in_single_quote || in_double_quote) {
                last_was_backslash = true;
                continue;
            }

            if ch == '\'' && !in_double_quote {
                in_single_quote = !in_single_quote;
            } else if ch == '"' && !in_single_quote {
                in_double_quote = !in_double_quote;
            }

            if !in_single_quote && !in_double_quote {
                if ch == '(' {
                    depth += 1;
                } else if ch == ')' {
                    depth -= 1;
                    if depth < 0 {
                        return false;
                    }
                }
            }
        }

        depth == 0
    }

    /// Find the position of a binary operator in an expression
    fn find_operator(&self, expr: &str) -> Option<usize> {
        // Operator precedence levels (lower number = lower precedence, evaluated last)
        let precedence_levels = vec![
            vec!["||"],                     // Level 1: Logical OR
            vec!["&&"],                     // Level 2: Logical AND
            vec!["==", "!=", "===", "!=="], // Level 3: Equality
            vec!["<", ">", "<=", ">="],     // Level 4: Comparison
            vec!["+", "-"],                 // Level 5: Addition/Subtraction
            vec!["*", "/", "%"],            // Level 6: Multiplication/Division
        ];

        // Find the lowest precedence operator at depth 0 (not in parentheses)
        for operators in &precedence_levels {
            if let Some(pos) = self.find_operator_at_level(expr, operators) {
                return Some(pos);
            }
        }

        None
    }

    /// Find an operator at a specific precedence level, respecting parentheses and quotes
    fn find_operator_at_level(&self, expr: &str, operators: &[&str]) -> Option<usize> {
        let mut paren_depth = 0;
        let mut in_single_quote = false;
        let mut in_double_quote = false;
        let mut last_was_backslash = false;
        let chars: Vec<char> = expr.chars().collect();

        // Look for operators from right to left (for left associativity)
        let mut i = 0;
        while i < chars.len() {
            let ch = chars[i];

            // Handle escape sequences
            if last_was_backslash {
                last_was_backslash = false;
                i += 1;
                continue;
            }

            if ch == '\\' && (in_single_quote || in_double_quote) {
                last_was_backslash = true;
                i += 1;
                continue;
            }

            // Track quote state
            if ch == '\'' && !in_double_quote {
                in_single_quote = !in_single_quote;
            } else if ch == '"' && !in_single_quote {
                in_double_quote = !in_double_quote;
            }

            // Skip if we're inside quotes
            if in_single_quote || in_double_quote {
                i += 1;
                continue;
            }

            // Track parentheses depth
            if ch == '(' {
                paren_depth += 1;
            } else if ch == ')' {
                paren_depth -= 1;
            }

            // Only look for operators at depth 0
            if paren_depth == 0 {
                // Check if any operator matches at this position
                for op in operators {
                    if i + op.len() <= expr.len() {
                        let slice = &expr[i..i + op.len()];
                        if slice == *op {
                            // Make sure this isn't part of a longer operator
                            // (e.g., don't match "=" in "===")
                            let before_ok =
                                i == 0 || !matches!(chars[i - 1], '=' | '!' | '<' | '>');
                            let after_ok = i + op.len() >= chars.len()
                                || !matches!(chars[i + op.len()], '=' | '&' | '|');

                            if before_ok && after_ok {
                                return Some(i);
                            }
                        }
                    }
                }
            }

            i += 1;
        }

        None
    }

    /// Split an expression at an operator position
    fn split_at_operator<'a>(
        &self,
        expr: &'a str,
        pos: usize,
    ) -> Result<(&'a str, BinaryOperator, &'a str)> {
        let left = &expr[..pos].trim();

        // Determine operator type and length
        let (op, op_len) = if expr[pos..].starts_with("||") {
            (BinaryOperator::Or, 2)
        } else if expr[pos..].starts_with("&&") {
            (BinaryOperator::And, 2)
        } else if expr[pos..].starts_with("===") || expr[pos..].starts_with("==") {
            (
                BinaryOperator::Equal,
                if expr[pos..].starts_with("===") { 3 } else { 2 },
            )
        } else if expr[pos..].starts_with("!==") || expr[pos..].starts_with("!=") {
            (
                BinaryOperator::NotEqual,
                if expr[pos..].starts_with("!==") { 3 } else { 2 },
            )
        } else if expr[pos..].starts_with("<=") {
            (BinaryOperator::LessThanOrEqual, 2)
        } else if expr[pos..].starts_with(">=") {
            (BinaryOperator::GreaterThanOrEqual, 2)
        } else if expr[pos..].starts_with('<') {
            (BinaryOperator::LessThan, 1)
        } else if expr[pos..].starts_with('>') {
            (BinaryOperator::GreaterThan, 1)
        } else if expr[pos..].starts_with('+') {
            (BinaryOperator::Add, 1)
        } else if expr[pos..].starts_with('-') {
            (BinaryOperator::Subtract, 1)
        } else if expr[pos..].starts_with('*') {
            (BinaryOperator::Multiply, 1)
        } else if expr[pos..].starts_with('/') {
            (BinaryOperator::Divide, 1)
        } else if expr[pos..].starts_with('%') {
            (BinaryOperator::Modulo, 1)
        } else {
            return Err(Error::template(format!(
                "Unknown operator at position {}",
                pos
            )));
        };

        let right = expr[pos + op_len..].trim();

        Ok((left, op, right))
    }
}

// Helper for Display implementation
impl std::fmt::Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenKind::Text(s) => write!(f, "Text({})", s),
            TokenKind::Variable(v) => write!(f, "Variable({})", v),
            TokenKind::RawVariable(v) => write!(f, "RawVariable({})", v),
            TokenKind::If(c) => write!(f, "If({})", c),
            TokenKind::Else => write!(f, "Else"),
            TokenKind::ElseIf(c) => write!(f, "ElseIf({})", c),
            TokenKind::Fi => write!(f, "Fi"),
            TokenKind::Foreach(i, c) => write!(f, "Foreach({} in {})", i, c),
            TokenKind::End => write!(f, "End"),
            TokenKind::Break => write!(f, "Break"),
            TokenKind::Continue => write!(f, "Continue"),
            TokenKind::Index => write!(f, "Index"),
            _ => write!(f, "{:?}", self),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_text() {
        let mut parser = Parser::new("Hello World").unwrap();
        let template = parser.parse().unwrap();

        assert_eq!(template.nodes.len(), 1);
        match &template.nodes[0] {
            Node::Text(t) => assert_eq!(t, "Hello World"),
            _ => panic!("Expected text node"),
        }
    }

    #[test]
    fn test_parse_variable() {
        let mut parser = Parser::new("Hello @{name}!").unwrap();
        let template = parser.parse().unwrap();

        assert_eq!(template.nodes.len(), 3);
        match &template.nodes[1] {
            Node::Variable { name, raw } => {
                assert_eq!(name, "name");
                assert!(!raw);
            }
            _ => panic!("Expected variable node"),
        }
    }

    #[test]
    fn test_parse_conditional() {
        let input = "@{if user.active}\nActive\n@{else}\nInactive\n@{fi}";
        let mut parser = Parser::new(input).unwrap();
        let template = parser.parse().unwrap();

        assert_eq!(template.nodes.len(), 1);
        match &template.nodes[0] {
            Node::Conditional {
                then_branch,
                else_branch,
                ..
            } => {
                assert!(!then_branch.is_empty());
                assert!(else_branch.is_some());
            }
            _ => panic!("Expected conditional node"),
        }
    }

    #[test]
    fn test_parse_loop() {
        let input = "@{foreach item in items}\n@{item}\n@{end}";
        let mut parser = Parser::new(input).unwrap();
        let template = parser.parse().unwrap();

        assert_eq!(template.nodes.len(), 1);
        match &template.nodes[0] {
            Node::Loop {
                item_name, body, ..
            } => {
                assert_eq!(item_name, "item");
                assert!(!body.is_empty());
            }
            _ => panic!("Expected loop node"),
        }
    }

    #[test]
    fn test_parse_section() {
        let input = "@{section header}\nHeader Content\n@{end}\nMain content";
        let mut parser = Parser::new(input).unwrap();
        let template = parser.parse().unwrap();

        // Section should be extracted
        assert!(template.sections.contains_key("header"));
        // Main content should remain
        assert!(!template.nodes.is_empty());
    }

    #[test]
    fn test_expression_parsing() {
        let parser = Parser::new("").unwrap();

        // Test simple comparison
        let expr = parser.parse_expression("a == b").unwrap();
        match expr {
            Expression::BinaryOp { op, .. } => {
                assert_eq!(op, BinaryOperator::Equal);
            }
            _ => panic!("Expected binary operation"),
        }

        // Test logical AND
        let expr = parser.parse_expression("x && y").unwrap();
        match expr {
            Expression::BinaryOp { op, .. } => {
                assert_eq!(op, BinaryOperator::And);
            }
            _ => panic!("Expected binary operation"),
        }

        // Test NOT operator
        let expr = parser.parse_expression("!active").unwrap();
        match expr {
            Expression::UnaryOp { op, .. } => {
                assert_eq!(op, UnaryOperator::Not);
            }
            _ => panic!("Expected unary operation"),
        }
    }
}
