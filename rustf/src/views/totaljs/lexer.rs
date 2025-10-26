use std::fmt;

/// Token types for Total.js template syntax
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    Text(String),

    // Variable interpolation
    Variable(String),    // @{variable}
    RawVariable(String), // @{!variable}

    // Control flow
    If(String),     // @{if condition}
    Else,           // @{else}
    ElseIf(String), // @{elif condition}
    Fi,             // @{fi}

    // Loops
    Foreach(String, String), // @{foreach item in collection}
    End,                     // @{end}
    Break,                   // @{break}
    Continue,                // @{continue}

    // Sections
    SectionDef(String),  // @{section name}
    SectionCall(String), // @{section('name')}

    // Helpers
    HelperDef(String, Vec<String>),  // @{helper name(args)}
    HelperCall(String, Vec<String>), // @{name(args)}

    // Views and imports
    View(String, Option<String>), // @{view('name', model)}
    Import(Vec<String>),          // @{import('file1', 'file2')}

    // Special directives
    Meta(Option<String>, Option<String>, Option<String>), // @{meta(title, desc, keywords)}
    Body,    // @{body} - Standard Total.js layout content placeholder
    Head,    // @{head} - Additional head section content
    Content, // @{content} - Our extension, same as @{body} for backward compatibility
    Csrf,    // @{csrf}

    // Localization
    Translate(String),    // @(text)
    TranslateKey(String), // @(#key)

    // Configuration
    Config(String), // @{'%config-key'}

    // Special variables
    Repository(String),   // @{repository.key} or @{R.key}
    App(String),          // @{APP.key} - Global repository
    Main(String),         // @{MAIN.key} - Global repository (alias)
    R(String),            // @{R.key} - Context repository (alias)
    Model(String),        // @{model.key} - Model data
    M(String),            // @{M.key} - Model data (alias)
    Session(String),      // @{session.key}
    Query(String),        // @{query.key}
    User(Option<String>), // @{user} or @{user.property}

    // Utility
    Index, // @{index} in loops
    Newline,
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub column: usize,
}

impl Token {
    pub fn new(kind: TokenKind, line: usize, column: usize) -> Self {
        Self { kind, line, column }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} at {}:{}", self.kind, self.line, self.column)
    }
}

/// Lexer for Total.js templates
pub struct Lexer {
    input: Vec<char>,
    position: usize,
    current_char: Option<char>,
    line: usize,
    column: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        let chars: Vec<char> = input.chars().collect();
        let current_char = if chars.is_empty() {
            None
        } else {
            Some(chars[0])
        };

        Self {
            input: chars,
            position: 0,
            current_char,
            line: 1,
            column: 1,
        }
    }

    /// Advance to the next character
    fn advance(&mut self) {
        if self.current_char == Some('\n') {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }

        self.position += 1;
        self.current_char = if self.position < self.input.len() {
            Some(self.input[self.position])
        } else {
            None
        };
    }

    /// Peek at the next character without advancing
    fn peek(&self) -> Option<char> {
        if self.position + 1 < self.input.len() {
            Some(self.input[self.position + 1])
        } else {
            None
        }
    }

    /// Peek ahead n characters
    #[allow(dead_code)]
    fn peek_n(&self, n: usize) -> Option<char> {
        if self.position + n < self.input.len() {
            Some(self.input[self.position + n])
        } else {
            None
        }
    }

    /// Check if we're at a Total.js directive start
    fn is_directive_start(&self) -> bool {
        self.current_char == Some('@') && (self.peek() == Some('{') || self.peek() == Some('('))
    }

    /// Skip whitespace
    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.current_char {
            if ch.is_whitespace() && ch != '\n' {
                self.advance();
            } else {
                break;
            }
        }
    }

    /// Read until a specific character or pattern
    fn read_until(&mut self, delimiter: char) -> String {
        let mut result = String::new();
        let mut brace_depth = 0;

        while let Some(ch) = self.current_char {
            if ch == '{' {
                brace_depth += 1;
            } else if ch == '}' {
                if brace_depth == 0 && delimiter == '}' {
                    break;
                }
                brace_depth -= 1;
            } else if ch == delimiter && brace_depth == 0 {
                break;
            }

            result.push(ch);
            self.advance();
        }

        result
    }

    /// Read plain text until we hit a directive
    fn read_text(&mut self) -> String {
        let mut result = String::new();

        while let Some(ch) = self.current_char {
            if self.is_directive_start() {
                break;
            }

            result.push(ch);
            self.advance();
        }

        result
    }

    /// Parse a Total.js directive @{...}
    fn parse_directive(&mut self) -> Token {
        let line = self.line;
        let column = self.column;

        // Skip @{
        self.advance(); // @
        self.advance(); // {

        self.skip_whitespace();

        // Check for raw variable @{!...}
        if self.current_char == Some('!') {
            self.advance();
            let var_name = self.read_until('}').trim().to_string();
            self.advance(); // Skip }
            return Token::new(TokenKind::RawVariable(var_name), line, column);
        }

        // Read the directive content
        let content = self.read_until('}').trim().to_string();
        self.advance(); // Skip }

        // Parse the directive content
        let token_kind = self.parse_directive_content(&content);
        Token::new(token_kind, line, column)
    }

    /// Parse the content of a directive to determine its type
    fn parse_directive_content(&self, content: &str) -> TokenKind {
        let trimmed = content.trim();

        // Control flow
        if trimmed.starts_with("if ") {
            let condition = trimmed[3..].trim().to_string();
            return TokenKind::If(condition);
        }

        if trimmed == "else" {
            return TokenKind::Else;
        }

        if trimmed.starts_with("elif ") {
            let condition = trimmed[5..].trim().to_string();
            return TokenKind::ElseIf(condition);
        }

        if trimmed == "fi" {
            return TokenKind::Fi;
        }

        // Loops
        if trimmed.starts_with("foreach ") {
            let parts = trimmed[8..].trim();
            if let Some(in_pos) = parts.find(" in ") {
                let item = parts[..in_pos].trim().to_string();
                let collection = parts[in_pos + 4..].trim().to_string();
                return TokenKind::Foreach(item, collection);
            }
        }

        if trimmed == "end" {
            return TokenKind::End;
        }

        if trimmed == "break" {
            return TokenKind::Break;
        }

        if trimmed == "continue" {
            return TokenKind::Continue;
        }

        if trimmed == "index" {
            return TokenKind::Index;
        }

        // Sections
        if trimmed.starts_with("section ") {
            let name = trimmed[8..].trim().to_string();
            return TokenKind::SectionDef(name);
        }

        if trimmed.starts_with("section(") && trimmed.ends_with(')') {
            let name = trimmed[8..trimmed.len() - 1]
                .trim_matches('\'')
                .trim_matches('"')
                .to_string();
            return TokenKind::SectionCall(name);
        }

        // Helpers
        if trimmed.starts_with("helper ") {
            if let Some(paren_pos) = trimmed.find('(') {
                let name = trimmed[7..paren_pos].trim().to_string();
                let args_str = &trimmed[paren_pos + 1..trimmed.len() - 1];
                let args = self.parse_args(args_str);
                return TokenKind::HelperDef(name, args);
            }
        }

        // Views
        if trimmed.starts_with("view(") && trimmed.ends_with(')') {
            let args_str = &trimmed[5..trimmed.len() - 1];
            let parts: Vec<&str> = args_str.split(',').collect();
            let view_name = parts[0].trim_matches('\'').trim_matches('"').to_string();
            let model = if parts.len() > 1 {
                Some(parts[1].trim().to_string())
            } else {
                None
            };
            return TokenKind::View(view_name, model);
        }

        // Import
        if trimmed.starts_with("import(") && trimmed.ends_with(')') {
            let args_str = &trimmed[7..trimmed.len() - 1];
            let files = self.parse_string_args(args_str);
            return TokenKind::Import(files);
        }

        // Meta
        if trimmed.starts_with("meta") {
            if trimmed == "meta" {
                return TokenKind::Meta(None, None, None);
            }
            if trimmed.starts_with("meta(") && trimmed.ends_with(')') {
                let args_str = &trimmed[5..trimmed.len() - 1];
                let parts = self.parse_string_args(args_str);
                let title = parts.first().cloned();
                let desc = parts.get(1).cloned();
                let keywords = parts.get(2).cloned();
                return TokenKind::Meta(title, desc, keywords);
            }
        }

        // Special directives
        if trimmed == "body" {
            return TokenKind::Body;
        }

        if trimmed == "head" {
            return TokenKind::Head;
        }

        if trimmed == "content" {
            return TokenKind::Content;
        }

        if trimmed == "csrf" {
            return TokenKind::Csrf;
        }

        // Special variables with dot notation
        // Global repository (APP or MAIN)
        if trimmed.starts_with("APP.") {
            let key = trimmed[4..].to_string();
            return TokenKind::App(key);
        }

        if trimmed.starts_with("MAIN.") {
            let key = trimmed[5..].to_string();
            return TokenKind::Main(key);
        }

        // Context repository (repository or R)
        if trimmed.starts_with("repository.") {
            let key = trimmed[11..].to_string();
            return TokenKind::Repository(key);
        }

        if trimmed.starts_with("R.") {
            let key = trimmed[2..].to_string();
            return TokenKind::R(key);
        }

        // Model data (model)
        if trimmed.starts_with("model.") {
            let key = trimmed[6..].to_string();
            return TokenKind::Model(key);
        }

        // Model data (M - alias)
        if trimmed.starts_with("M.") {
            let key = trimmed[2..].to_string();
            return TokenKind::M(key);
        }

        if trimmed.starts_with("session.") {
            let key = trimmed[8..].to_string();
            return TokenKind::Session(key);
        }

        if trimmed.starts_with("query.") {
            let key = trimmed[6..].to_string();
            return TokenKind::Query(key);
        }

        if trimmed == "user" {
            return TokenKind::User(None);
        }

        if trimmed.starts_with("user.") {
            let property = trimmed[5..].to_string();
            return TokenKind::User(Some(property));
        }

        // Configuration variables
        if trimmed.starts_with('\'') && trimmed.ends_with('\'') {
            let inner = &trimmed[1..trimmed.len() - 1];
            if inner.starts_with('%') {
                let config_key = inner[1..].to_string();
                return TokenKind::Config(config_key);
            }
        }

        // Helper function calls
        if let Some(paren_pos) = trimmed.find('(') {
            if trimmed.ends_with(')') {
                let name = trimmed[..paren_pos].trim().to_string();
                let args_str = &trimmed[paren_pos + 1..trimmed.len() - 1];
                let args = self.parse_args(args_str);
                return TokenKind::HelperCall(name, args);
            }
        }

        // Default: treat as variable
        TokenKind::Variable(trimmed.to_string())
    }

    /// Parse a localization directive @(...)
    fn parse_localization(&mut self) -> Token {
        let line = self.line;
        let column = self.column;

        // Skip @(
        self.advance(); // @
        self.advance(); // (

        self.skip_whitespace();

        // Check for key reference @(#key)
        if self.current_char == Some('#') {
            self.advance();
            let key = self.read_until(')').trim().to_string();
            self.advance(); // Skip )
            return Token::new(TokenKind::TranslateKey(key), line, column);
        }

        // Regular translation
        let text = self.read_until(')').trim().to_string();
        self.advance(); // Skip )

        Token::new(TokenKind::Translate(text), line, column)
    }

    /// Parse comma-separated arguments
    fn parse_args(&self, args_str: &str) -> Vec<String> {
        args_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// Parse comma-separated string arguments (removes quotes)
    fn parse_string_args(&self, args_str: &str) -> Vec<String> {
        args_str
            .split(',')
            .map(|s| s.trim().trim_matches('\'').trim_matches('"').to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// Get the next token
    pub fn next_token(&mut self) -> Token {
        // Check for EOF
        if self.current_char.is_none() {
            return Token::new(TokenKind::Eof, self.line, self.column);
        }

        // Check for newline
        if self.current_char == Some('\n') {
            let token = Token::new(TokenKind::Newline, self.line, self.column);
            self.advance();
            return token;
        }

        // Check for Total.js directives
        if self.current_char == Some('@') {
            if self.peek() == Some('{') {
                return self.parse_directive();
            } else if self.peek() == Some('(') {
                return self.parse_localization();
            }
        }

        // Otherwise, read as plain text
        let text = self.read_text();
        if !text.is_empty() {
            Token::new(TokenKind::Text(text), self.line, self.column)
        } else {
            // This shouldn't happen, but handle gracefully
            self.advance();
            self.next_token()
        }
    }

    /// Tokenize the entire input
    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();

        loop {
            let token = self.next_token();
            if token.kind == TokenKind::Eof {
                tokens.push(token);
                break;
            }
            tokens.push(token);
        }

        tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_text() {
        let mut lexer = Lexer::new("Hello World");
        let tokens = lexer.tokenize();

        assert_eq!(tokens.len(), 2);
        match &tokens[0].kind {
            TokenKind::Text(t) => assert_eq!(t, "Hello World"),
            _ => panic!("Expected text token"),
        }
    }

    #[test]
    fn test_variable() {
        let mut lexer = Lexer::new("Hello @{name}!");
        let tokens = lexer.tokenize();

        assert_eq!(tokens.len(), 4);
        match &tokens[1].kind {
            TokenKind::Variable(v) => assert_eq!(v, "name"),
            _ => panic!("Expected variable token"),
        }
    }

    #[test]
    fn test_raw_variable() {
        let mut lexer = Lexer::new("@{!htmlContent}");
        let tokens = lexer.tokenize();

        match &tokens[0].kind {
            TokenKind::RawVariable(v) => assert_eq!(v, "htmlContent"),
            _ => panic!("Expected raw variable token"),
        }
    }

    #[test]
    fn test_if_else() {
        let mut lexer = Lexer::new("@{if user.isActive}\nActive\n@{else}\nInactive\n@{fi}");
        let tokens = lexer.tokenize();

        // Verify the if token
        match &tokens[0].kind {
            TokenKind::If(cond) => assert_eq!(cond, "user.isActive"),
            _ => panic!("Expected if token"),
        }

        // Find and verify the Else token
        let else_pos = tokens
            .iter()
            .position(|t| matches!(t.kind, TokenKind::Else))
            .expect("Should have an Else token");
        assert!(matches!(tokens[else_pos].kind, TokenKind::Else));

        // Find and verify the Fi token
        let fi_pos = tokens
            .iter()
            .position(|t| matches!(t.kind, TokenKind::Fi))
            .expect("Should have a Fi token");
        assert!(matches!(tokens[fi_pos].kind, TokenKind::Fi));
    }

    #[test]
    fn test_foreach() {
        let mut lexer = Lexer::new("@{foreach item in items}\n@{item.name}\n@{end}");
        let tokens = lexer.tokenize();

        match &tokens[0].kind {
            TokenKind::Foreach(item, collection) => {
                assert_eq!(item, "item");
                assert_eq!(collection, "items");
            }
            _ => panic!("Expected foreach token"),
        }
    }

    #[test]
    fn test_section() {
        let mut lexer = Lexer::new("@{section header}\nHeader content\n@{end}");
        let tokens = lexer.tokenize();

        match &tokens[0].kind {
            TokenKind::SectionDef(name) => assert_eq!(name, "header"),
            _ => panic!("Expected section definition token"),
        }
    }

    #[test]
    fn test_view() {
        let mut lexer = Lexer::new("@{view('partial', model)}");
        let tokens = lexer.tokenize();

        match &tokens[0].kind {
            TokenKind::View(name, model) => {
                assert_eq!(name, "partial");
                assert_eq!(model.as_ref().unwrap(), "model");
            }
            _ => panic!("Expected view token"),
        }
    }

    #[test]
    fn test_import() {
        let mut lexer = Lexer::new("@{import('style.css', 'script.js')}");
        let tokens = lexer.tokenize();

        match &tokens[0].kind {
            TokenKind::Import(files) => {
                assert_eq!(files.len(), 2);
                assert_eq!(files[0], "style.css");
                assert_eq!(files[1], "script.js");
            }
            _ => panic!("Expected import token"),
        }
    }

    #[test]
    fn test_localization() {
        let mut lexer = Lexer::new("@(Hello World)");
        let tokens = lexer.tokenize();

        match &tokens[0].kind {
            TokenKind::Translate(text) => assert_eq!(text, "Hello World"),
            _ => panic!("Expected translate token"),
        }
    }

    #[test]
    fn test_localization_key() {
        let mut lexer = Lexer::new("@(#welcome.message)");
        let tokens = lexer.tokenize();

        match &tokens[0].kind {
            TokenKind::TranslateKey(key) => assert_eq!(key, "welcome.message"),
            _ => panic!("Expected translate key token"),
        }
    }

    #[test]
    fn test_special_variables() {
        let mut lexer = Lexer::new("@{repository.data} @{session.user} @{query.page}");
        let tokens = lexer.tokenize();

        // Filter out text/whitespace tokens to find the special tokens
        let special_tokens: Vec<_> = tokens
            .iter()
            .filter(|t| !matches!(t.kind, TokenKind::Text(_) | TokenKind::Eof))
            .collect();

        match &special_tokens[0].kind {
            TokenKind::Repository(key) => assert_eq!(key, "data"),
            _ => panic!(
                "Expected repository token, got {:?}",
                special_tokens[0].kind
            ),
        }

        match &special_tokens[1].kind {
            TokenKind::Session(key) => assert_eq!(key, "user"),
            _ => panic!("Expected session token, got {:?}", special_tokens[1].kind),
        }

        match &special_tokens[2].kind {
            TokenKind::Query(key) => assert_eq!(key, "page"),
            _ => panic!("Expected query token, got {:?}", special_tokens[2].kind),
        }
    }
}
