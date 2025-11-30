use std::collections::HashMap;

/// AST node types for Total.js templates
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    /// Plain text content
    Text(String),

    /// Variable interpolation @{variable} or @{expression}
    Variable {
        name: String,
        raw: bool, // true for @{!variable}
        expression: Option<Expression>, // Some(expr) when name contains an expression like ternary
    },

    /// Conditional block @{if}...@{else}...@{fi}
    Conditional {
        condition: Expression,
        then_branch: Vec<Node>,
        else_if_branches: Vec<(Expression, Vec<Node>)>,
        else_branch: Option<Vec<Node>>,
    },

    /// Loop block @{foreach item in collection}...@{end}
    Loop {
        item_name: String,
        collection: Expression,
        body: Vec<Node>,
    },

    /// Break statement in loops
    Break,

    /// Continue statement in loops
    Continue,

    /// Loop index variable @{index}
    Index,

    /// Section definition @{section name}...@{end}
    SectionDef {
        name: String,
        content: Vec<Node>,
    },

    /// Section reference @{section('name')}
    SectionCall(String),

    /// Helper definition @{helper name(args)}...@{end}
    HelperDef {
        name: String,
        params: Vec<String>,
        body: Vec<Node>,
    },

    /// Helper call @{name(args)}
    HelperCall {
        name: String,
        args: Vec<Expression>,
    },

    /// View inclusion @{view('name', model)}
    View {
        name: String,
        model: Option<Expression>,
    },

    /// Import resources @{import('file1', 'file2')}
    Import(Vec<String>),

    /// Meta tags @{meta(title, description, keywords)}
    Meta {
        title: Option<String>,
        description: Option<String>,
        keywords: Option<String>,
    },

    /// Special placeholders
    Body,
    Head,
    Content,
    Csrf,

    /// Translation @(text) or @(#key)
    Translate {
        text: String,
        is_key: bool,
    },

    /// Configuration value @{'%config-key'}
    Config(String),

    /// Special variables
    Repository(String), // Context repository @{repository.key}
    App(String),   // Global repository @{APP.key}
    Main(String),  // Global repository @{MAIN.key} (alias)
    R(String),     // Context repository @{R.key} (alias)
    Model(String), // Model data @{model.key}
    M(String),     // Model data @{M.key} (alias)
    Session(String),
    Query(String),
    User(Option<String>),
}

/// Expression types for conditions and values
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// String literal
    String(String),

    /// Number literal
    Number(f64),

    /// Boolean literal
    Boolean(bool),

    /// Variable reference
    Variable(String),

    /// Property access (e.g., user.name)
    PropertyAccess {
        object: Box<Expression>,
        property: String,
    },

    /// Array literal
    Array(Vec<Expression>),

    /// Object literal
    Object(HashMap<String, Expression>),

    /// Function call
    FunctionCall { name: String, args: Vec<Expression> },

    /// Binary operation
    BinaryOp {
        left: Box<Expression>,
        op: BinaryOperator,
        right: Box<Expression>,
    },

    /// Unary operation
    UnaryOp {
        op: UnaryOperator,
        operand: Box<Expression>,
    },

    /// Ternary operation (condition ? then_expr : else_expr)
    Ternary {
        condition: Box<Expression>,
        then_expr: Box<Expression>,
        else_expr: Box<Expression>,
    },

    /// Null value
    Null,
}

/// Binary operators
#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOperator {
    // Comparison
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,

    // Logical
    And,
    Or,

    // Arithmetic
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
}

/// Unary operators
#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOperator {
    Not,
    Minus,
}

/// Complete template AST
#[derive(Debug, Clone)]
pub struct Template {
    pub nodes: Vec<Node>,
    pub sections: HashMap<String, Vec<Node>>,
    pub helpers: HashMap<String, Helper>,
}

/// Helper function definition
#[derive(Debug, Clone)]
pub struct Helper {
    pub params: Vec<String>,
    pub body: Vec<Node>,
}

impl Template {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            sections: HashMap::new(),
            helpers: HashMap::new(),
        }
    }

    /// Extract sections from the main node list
    pub fn extract_sections(&mut self) {
        let mut new_nodes = Vec::new();
        let mut i = 0;

        while i < self.nodes.len() {
            match &self.nodes[i] {
                Node::SectionDef { name, content } => {
                    self.sections.insert(name.clone(), content.clone());
                }
                _ => {
                    new_nodes.push(self.nodes[i].clone());
                }
            }
            i += 1;
        }

        self.nodes = new_nodes;
    }

    /// Extract helper definitions from the main node list
    pub fn extract_helpers(&mut self) {
        let mut new_nodes = Vec::new();
        let mut i = 0;

        while i < self.nodes.len() {
            match &self.nodes[i] {
                Node::HelperDef { name, params, body } => {
                    self.helpers.insert(
                        name.clone(),
                        Helper {
                            params: params.clone(),
                            body: body.clone(),
                        },
                    );
                }
                _ => {
                    new_nodes.push(self.nodes[i].clone());
                }
            }
            i += 1;
        }

        self.nodes = new_nodes;
    }
}

impl Default for Template {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper functions for building expressions
impl Expression {
    /// Create a property access expression from a dotted path
    pub fn from_path(path: &str) -> Self {
        let parts: Vec<&str> = path.split('.').collect();
        if parts.is_empty() {
            return Expression::Null;
        }

        let mut expr = Expression::Variable(parts[0].to_string());

        for part in parts.iter().skip(1) {
            expr = Expression::PropertyAccess {
                object: Box::new(expr),
                property: part.to_string(),
            };
        }

        expr
    }

    /// Parse a simple value expression
    pub fn parse_value(value: &str) -> Self {
        let trimmed = value.trim();

        // Check for boolean literals
        if trimmed == "true" {
            return Expression::Boolean(true);
        }
        if trimmed == "false" {
            return Expression::Boolean(false);
        }

        // Check for null
        if trimmed == "null" || trimmed == "NULL" {
            return Expression::Null;
        }

        // Check for numbers
        if let Ok(num) = trimmed.parse::<f64>() {
            return Expression::Number(num);
        }

        // Check for strings (quoted)
        if (trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
        {
            let content = &trimmed[1..trimmed.len() - 1];
            return Expression::String(content.to_string());
        }

        // Check for array literals
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let content = &trimmed[1..trimmed.len() - 1];
            let items: Vec<Expression> = content
                .split(',')
                .map(|s| Expression::parse_value(s.trim()))
                .collect();
            return Expression::Array(items);
        }

        // Otherwise, treat as variable or path
        Expression::from_path(trimmed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expression_from_path() {
        let expr = Expression::from_path("user.profile.name");

        match expr {
            Expression::PropertyAccess { object, property } => {
                assert_eq!(property, "name");
                match *object {
                    Expression::PropertyAccess {
                        object: inner,
                        property: prop,
                    } => {
                        assert_eq!(prop, "profile");
                        match *inner {
                            Expression::Variable(var) => assert_eq!(var, "user"),
                            _ => panic!("Expected variable"),
                        }
                    }
                    _ => panic!("Expected property access"),
                }
            }
            _ => panic!("Expected property access"),
        }
    }

    #[test]
    fn test_parse_value() {
        // Boolean
        assert_eq!(Expression::parse_value("true"), Expression::Boolean(true));
        assert_eq!(Expression::parse_value("false"), Expression::Boolean(false));

        // Number
        assert_eq!(Expression::parse_value("42"), Expression::Number(42.0));
        assert_eq!(Expression::parse_value("3.14"), Expression::Number(3.14));

        // String
        assert_eq!(
            Expression::parse_value("\"hello\""),
            Expression::String("hello".to_string())
        );
        assert_eq!(
            Expression::parse_value("'world'"),
            Expression::String("world".to_string())
        );

        // Null
        assert_eq!(Expression::parse_value("null"), Expression::Null);

        // Variable
        match Expression::parse_value("myVar") {
            Expression::Variable(v) => assert_eq!(v, "myVar"),
            _ => panic!("Expected variable"),
        }
    }

    #[test]
    fn test_template_sections() {
        let mut template = Template::new();

        template.nodes.push(Node::Text("Before".to_string()));
        template.nodes.push(Node::SectionDef {
            name: "header".to_string(),
            content: vec![Node::Text("Header content".to_string())],
        });
        template.nodes.push(Node::Text("After".to_string()));

        template.extract_sections();

        assert_eq!(template.nodes.len(), 2);
        assert!(template.sections.contains_key("header"));

        let header = &template.sections["header"];
        assert_eq!(header.len(), 1);
        match &header[0] {
            Node::Text(t) => assert_eq!(t, "Header content"),
            _ => panic!("Expected text node"),
        }
    }
}
