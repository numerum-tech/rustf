use super::ast::{
    AttrValue, BinaryOperator, Expression, FormFieldKind, Node, Template, UnaryOperator,
};
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
                // Check if the variable name contains any expression operators
                let expression = if self.contains_expression_operators(name) {
                    // Parse as expression
                    match self.parse_expression(name) {
                        Ok(expr) => Some(expr),
                        Err(_) => None, // Fall back to treating as variable name
                    }
                } else {
                    None
                };

                let node = Node::Variable {
                    name: name.clone(),
                    raw: false,
                    expression,
                };
                self.advance();
                Ok(node)
            }

            TokenKind::RawVariable(name) => {
                // Check if the variable name contains any expression operators
                // (binary, ternary, etc.) so raw output supports the same
                // expressions as escaped output — e.g. `@{!M.url || '/img.jpg'}`.
                let expression = if self.contains_expression_operators(name) {
                    // Parse as expression
                    match self.parse_expression(name) {
                        Ok(expr) => Some(expr),
                        Err(_) => None, // Fall back to treating as variable name
                    }
                } else {
                    None
                };

                let node = Node::Variable {
                    name: name.clone(),
                    raw: true,
                    expression,
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

            TokenKind::Title(arg) => {
                let expr = self.parse_expression(arg)?;
                self.advance();
                Ok(Node::Title(expr))
            }

            TokenKind::Description(arg) => {
                let expr = self.parse_expression(arg)?;
                self.advance();
                Ok(Node::Description(expr))
            }

            TokenKind::FormField(kind, args) => {
                let node = Self::parse_form_field(kind.clone(), args)?;
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
                self.current_token.kind, self.current_token.line
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

        // Check for ternary operator (lowest precedence, checked first)
        // Format: condition ? then_expr : else_expr
        if let Some(ternary_pos) = self.find_ternary_operator(trimmed) {
            let condition_str = trimmed[..ternary_pos].trim();
            let rest = &trimmed[ternary_pos + 1..];

            // Find the colon that separates then and else
            if let Some(colon_pos) = self.find_ternary_colon(rest) {
                let then_str = rest[..colon_pos].trim();
                let else_str = rest[colon_pos + 1..].trim();

                let condition = Box::new(self.parse_expression(condition_str)?);
                let then_expr = Box::new(self.parse_expression(then_str)?);
                let else_expr = Box::new(self.parse_expression(else_str)?);

                return Ok(Expression::Ternary {
                    condition,
                    then_expr,
                    else_expr,
                });
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

    /// Parse a form-helper argument list into a `Node::FormField`, dispatching
    /// on the helper kind (different helpers take different argument shapes).
    fn parse_form_field(kind: String, args: &str) -> Result<Node> {
        let parts = Self::split_top_level(args, ',');
        let name = parts
            .first()
            .map(|s| Self::unquote(s.trim()))
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::template(format!("{} helper requires a field name", kind)))?;

        // Find the trailing attribute object `{ ... }`, if any.
        let attr_object = |parts: &[String]| -> Vec<(String, AttrValue)> {
            parts
                .iter()
                .map(|s| s.trim())
                .find(|s| s.starts_with('{') && s.ends_with('}'))
                .map(|obj| Self::parse_attr_object(&obj[1..obj.len() - 1]))
                .unwrap_or_default()
        };

        match kind.as_str() {
            "text" | "password" | "hidden" | "textarea" => {
                let field_kind = match kind.as_str() {
                    "text" => FormFieldKind::Text,
                    "password" => FormFieldKind::Password,
                    "hidden" => FormFieldKind::Hidden,
                    _ => FormFieldKind::Textarea,
                };
                Ok(Node::FormField {
                    kind: field_kind,
                    name,
                    value: None,
                    label: None,
                    attrs: attr_object(&parts[1..]),
                })
            }
            "checkbox" => {
                // @{checkbox('field', [label])} — optional string label.
                let label = parts
                    .get(1)
                    .map(|s| Self::unquote(s.trim()))
                    .filter(|s| !s.is_empty());
                Ok(Node::FormField {
                    kind: FormFieldKind::Checkbox,
                    name,
                    value: None,
                    label,
                    attrs: Vec::new(),
                })
            }
            "radio" => {
                // @{radio('field', 'value', [label | { label, ...attrs }])}
                let value = parts.get(1).map(|s| Self::unquote(s.trim()));
                let mut label = None;
                let mut attrs = Vec::new();
                if let Some(third) = parts.get(2).map(|s| s.trim()) {
                    if third.starts_with('{') && third.ends_with('}') {
                        for (k, v) in Self::parse_attr_object(&third[1..third.len() - 1]) {
                            if k == "label" {
                                if let AttrValue::Str(s) = v {
                                    label = Some(s);
                                }
                            } else {
                                attrs.push((k, v));
                            }
                        }
                    } else {
                        label = Some(Self::unquote(third));
                    }
                }
                Ok(Node::FormField {
                    kind: FormFieldKind::Radio,
                    name,
                    value,
                    label,
                    attrs,
                })
            }
            other => Err(Error::template(format!("Unknown form helper: {}", other))),
        }
    }

    /// Parse an HTML-attribute object body like
    /// `class: 'form', maxlength: 30, required: true` into ordered pairs.
    fn parse_attr_object(body: &str) -> Vec<(String, AttrValue)> {
        let mut attrs = Vec::new();
        for entry in Self::split_top_level(body, ',') {
            let entry = entry.trim();
            if entry.is_empty() {
                continue;
            }
            // Split on the first top-level ':'
            let colon = entry.find(':');
            let Some(colon) = colon else { continue };
            let key = Self::unquote(entry[..colon].trim());
            let raw = entry[colon + 1..].trim();
            let value = if raw == "true" {
                AttrValue::Bool(true)
            } else if raw == "false" {
                AttrValue::Bool(false)
            } else if (raw.starts_with('\'') && raw.ends_with('\''))
                || (raw.starts_with('"') && raw.ends_with('"'))
            {
                AttrValue::Str(Self::unquote(raw))
            } else if let Ok(n) = raw.parse::<f64>() {
                AttrValue::Number(n)
            } else {
                AttrValue::Str(raw.to_string())
            };
            if !key.is_empty() {
                attrs.push((key, value));
            }
        }
        attrs
    }

    /// Split a string on `sep` at the top level, ignoring separators inside
    /// quotes, parentheses, braces, or brackets.
    fn split_top_level(s: &str, sep: char) -> Vec<String> {
        let mut parts = Vec::new();
        let mut current = String::new();
        let mut depth: i32 = 0;
        let mut in_single = false;
        let mut in_double = false;
        let mut prev_backslash = false;
        for ch in s.chars() {
            if prev_backslash {
                current.push(ch);
                prev_backslash = false;
                continue;
            }
            if ch == '\\' && (in_single || in_double) {
                current.push(ch);
                prev_backslash = true;
                continue;
            }
            if ch == '\'' && !in_double {
                in_single = !in_single;
            } else if ch == '"' && !in_single {
                in_double = !in_double;
            } else if !in_single && !in_double {
                match ch {
                    '(' | '{' | '[' => depth += 1,
                    ')' | '}' | ']' => depth -= 1,
                    _ => {}
                }
            }
            if ch == sep && depth == 0 && !in_single && !in_double {
                parts.push(current.clone());
                current.clear();
            } else {
                current.push(ch);
            }
        }
        if !current.trim().is_empty() {
            parts.push(current);
        }
        parts
    }

    /// Strip a single layer of surrounding single or double quotes.
    fn unquote(s: &str) -> String {
        let s = s.trim();
        if s.len() >= 2
            && ((s.starts_with('\'') && s.ends_with('\''))
                || (s.starts_with('"') && s.ends_with('"')))
        {
            s[1..s.len() - 1].to_string()
        } else {
            s.to_string()
        }
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
        // Keep chars and their byte offsets in parallel so we can return a byte
        // index (required by split_at_operator) even when the expression
        // contains multi-byte UTF-8 characters (e.g. 'ç').
        let chars: Vec<char> = expr.chars().collect();
        let byte_offsets: Vec<usize> = expr.char_indices().map(|(b, _)| b).collect();

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
                // Check if any operator matches at this position. Operators are
                // all ASCII, so a byte-length op spans the same number of chars.
                for op in operators {
                    let op_len = op.len();
                    if i + op_len <= chars.len()
                        && op.bytes().zip(&chars[i..i + op_len]).all(|(b, &c)| b as char == c)
                    {
                        // Make sure this isn't part of a longer operator
                        // (e.g., don't match "=" in "===")
                        let before_ok =
                            i == 0 || !matches!(chars[i - 1], '=' | '!' | '<' | '>');
                        let after_ok = i + op_len >= chars.len()
                            || !matches!(chars[i + op_len], '=' | '&' | '|');

                        if before_ok && after_ok {
                            // Return a byte offset so split_at_operator slices correctly.
                            return Some(byte_offsets[i]);
                        }
                    }
                }
            }

            i += 1;
        }

        None
    }

    /// Check if a string contains a ternary operator (not inside quotes)
    fn contains_ternary_operator(&self, expr: &str) -> bool {
        self.find_ternary_operator(expr).is_some()
    }

    /// Check if expression contains any operators (ternary, binary, etc.)
    fn contains_expression_operators(&self, expr: &str) -> bool {
        // Check for ternary operator first
        if self.contains_ternary_operator(expr) {
            return true;
        }

        // Check for binary operators: +, -, *, /, %, ==, !=, <, >, <=, >=, &&, ||
        // Need to respect quotes and parentheses
        let mut paren_depth = 0;
        let mut in_quote = false;
        let mut quote_char = ' ';
        let chars: Vec<char> = expr.chars().collect();

        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];

            // Track quotes
            if (c == '"' || c == '\'') && (i == 0 || chars[i - 1] != '\\') {
                if !in_quote {
                    in_quote = true;
                    quote_char = c;
                } else if c == quote_char {
                    in_quote = false;
                }
                i += 1;
                continue;
            }

            if in_quote {
                i += 1;
                continue;
            }

            // Track parentheses
            if c == '(' {
                paren_depth += 1;
                i += 1;
                continue;
            }
            if c == ')' {
                paren_depth -= 1;
                i += 1;
                continue;
            }

            // Check for operators at paren depth 0
            if paren_depth == 0 {
                // Single char operators
                if c == '+' || c == '-' || c == '*' || c == '/' || c == '%' || c == '<' || c == '>'
                {
                    return true;
                }

                // Two char operators
                if i < chars.len() - 1 {
                    let next = chars[i + 1];
                    if (c == '=' && next == '=')
                        || (c == '!' && next == '=')
                        || (c == '<' && next == '=')
                        || (c == '>' && next == '=')
                        || (c == '&' && next == '&')
                        || (c == '|' && next == '|')
                    {
                        return true;
                    }
                }
            }

            i += 1;
        }

        false
    }

    /// Find the ternary operator (?) in an expression, respecting quotes and parentheses
    fn find_ternary_operator(&self, expr: &str) -> Option<usize> {
        let mut paren_depth = 0;
        let mut in_single_quote = false;
        let mut in_double_quote = false;
        let mut last_was_backslash = false;
        let chars: Vec<char> = expr.chars().collect();
        let byte_offsets: Vec<usize> = expr.char_indices().map(|(b, _)| b).collect();

        // Look for '?' from left to right (ternary is right-associative)
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

            // Only look for '?' at depth 0
            if paren_depth == 0 && ch == '?' {
                return Some(byte_offsets[i]);
            }

            i += 1;
        }

        None
    }

    /// Find the colon (:) that separates then and else in a ternary expression
    /// This should be the first colon after the '?' that's at the same nesting level
    fn find_ternary_colon(&self, expr: &str) -> Option<usize> {
        let mut paren_depth = 0;
        let mut in_single_quote = false;
        let mut in_double_quote = false;
        let mut last_was_backslash = false;
        let chars: Vec<char> = expr.chars().collect();
        let byte_offsets: Vec<usize> = expr.char_indices().map(|(b, _)| b).collect();

        // Look for ':' from left to right
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

            // Only look for ':' at depth 0
            if paren_depth == 0 && ch == ':' {
                return Some(byte_offsets[i]);
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
            Node::Variable {
                name,
                raw,
                expression,
            } => {
                assert_eq!(name, "name");
                assert!(!raw);
                assert!(expression.is_none());
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

    #[test]
    fn test_ternary_expression_parsing() {
        let parser = Parser::new("").unwrap();

        // Test simple ternary
        let expr = parser.parse_expression("true ? 'yes' : 'no'").unwrap();
        match expr {
            Expression::Ternary {
                condition,
                then_expr,
                else_expr,
            } => {
                match *condition {
                    Expression::Boolean(true) => {}
                    _ => panic!("Expected boolean true condition"),
                }
                match *then_expr {
                    Expression::String(ref s) if s == "yes" => {}
                    _ => panic!("Expected 'yes' as then expression"),
                }
                match *else_expr {
                    Expression::String(ref s) if s == "no" => {}
                    _ => panic!("Expected 'no' as else expression"),
                }
            }
            _ => panic!("Expected ternary expression"),
        }
    }

    #[test]
    fn test_ternary_in_variable_interpolation() {
        // Test that @{ condition ? 'ok_value' : 'not_ok_value' } is parsed correctly
        let input = "@{ true ? 'ok_value' : 'not_ok_value' }";
        let mut parser = Parser::new(input).unwrap();
        let template = parser.parse().unwrap();

        assert_eq!(template.nodes.len(), 1);
        match &template.nodes[0] {
            Node::Variable {
                name, expression, ..
            } => {
                assert!(name.contains('?'));
                assert!(expression.is_some());
                match expression.as_ref().unwrap() {
                    Expression::Ternary { .. } => {}
                    _ => panic!("Expected ternary expression in variable"),
                }
            }
            _ => panic!("Expected variable node"),
        }
    }

    #[test]
    fn test_operator_with_multibyte_chars() {
        // Regression: char index was used as a byte index when slicing, panicking
        // on multi-byte UTF-8 chars like 'ç' (e.g. "end byte index ... not a char boundary").
        let parser = Parser::new("").unwrap();

        // Multi-byte literal before a binary operator.
        let expr = parser.parse_expression("'garçon' == name").unwrap();
        match expr {
            Expression::BinaryOp { op, .. } => assert_eq!(op, BinaryOperator::Equal),
            _ => panic!("Expected binary operation"),
        }

        // Multi-byte identifier/literal around a ternary.
        let expr = parser
            .parse_expression("actif ? 'oui garçon' : 'non élève'")
            .unwrap();
        match expr {
            Expression::Ternary { .. } => {}
            _ => panic!("Expected ternary expression"),
        }
    }

    #[test]
    fn test_conditional_with_multibyte_string_literal() {
        // Full template parse path with a multi-byte string literal in the condition.
        let input = "@{if role == 'gérant'}\nOK\n@{fi}";
        let mut parser = Parser::new(input).unwrap();
        let template = parser.parse().unwrap();
        assert!(!template.nodes.is_empty());
    }
}
