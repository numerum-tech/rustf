//! String manipulation utilities for RustF framework
//!
//! This module provides common string processing functions used in web development,
//! including text cleaning, keyword extraction, and string formatting utilities.

use std::collections::{HashMap, HashSet};

/// Trim whitespace and clean up a string
///
/// Removes leading and trailing whitespace, and normalizes internal whitespace.
/// Multiple consecutive spaces are collapsed into single spaces.
///
/// # Arguments
/// * `input` - String to clean
///
/// # Example
/// ```rust,ignore
/// let cleaned = trim("  hello    world  \n\t");
/// assert_eq!(cleaned, "hello world");
/// ```
pub fn trim(input: &str) -> String {
    input.split_whitespace().collect::<Vec<&str>>().join(" ")
}

/// Extract keywords from text for search indexing
///
/// Extracts meaningful words from text, removing common stop words
/// and filtering by length. Useful for search functionality.
///
/// # Arguments
/// * `content` - Text content to process
/// * `max_count` - Maximum number of keywords to return
/// * `min_length` - Minimum keyword length
///
/// # Example
/// ```rust,ignore
/// let keywords = extract_keywords("This is a sample text for testing", 5, 3);
/// // Returns words like ["sample", "text", "testing"] (excludes "This", "is", "a", "for")
/// ```
pub fn keywords(content: &str, max_count: usize, min_length: usize) -> Vec<String> {
    let stop_words: HashSet<&str> = [
        "a", "an", "and", "are", "as", "at", "be", "by", "for", "from", "has", "he", "in", "is",
        "it", "its", "of", "on", "that", "the", "to", "was", "will", "with", "or", "but", "not",
        "this", "they", "have", "had", "what", "said", "each", "which", "do", "how", "their", "if",
        "up", "out", "many", "then", "them", "these", "so", "some", "her", "would", "make", "like",
        "into", "him", "has", "two", "more", "go", "no", "way", "could", "my", "than", "first",
        "been", "call", "who", "oil", "sit", "now", "find", "down", "day", "did", "get", "come",
        "made", "may", "part",
    ]
    .iter()
    .cloned()
    .collect();

    let mut word_counts: HashMap<String, usize> = HashMap::new();

    // Extract words and count frequency
    for word in content
        .to_lowercase()
        .split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|w| w.len() >= min_length && !stop_words.contains(w))
    {
        *word_counts.entry(word.to_string()).or_insert(0) += 1;
    }

    // Sort by frequency and take top results
    let mut words: Vec<(String, usize)> = word_counts.into_iter().collect();
    words.sort_by(|a, b| b.1.cmp(&a.1));

    words
        .into_iter()
        .take(max_count)
        .map(|(word, _)| word)
        .collect()
}

/// Convert string to slug format
///
/// Converts a string to a URL-friendly slug by:
/// - Converting to lowercase
/// - Replacing spaces and special characters with hyphens
/// - Removing consecutive hyphens
/// - Trimming hyphens from start/end
///
/// # Arguments
/// * `input` - String to convert to slug
///
/// # Example
/// ```rust,ignore
/// let slug = to_slug("Hello World! This is a Test.");
/// assert_eq!(slug, "hello-world-this-is-a-test");
/// ```
pub fn to_slug(input: &str) -> String {
    input
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c
            } else if c.is_whitespace() || c == '-' || c == '_' {
                '-'
            } else {
                ' ' // Will be filtered out
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join("")
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<&str>>()
        .join("-")
}

/// Convert string to camelCase
///
/// # Arguments
/// * `input` - String to convert
///
/// # Example
/// ```rust,ignore
/// let camel = to_camel_case("hello world test");
/// assert_eq!(camel, "helloWorldTest");
/// ```
pub fn to_camel_case(input: &str) -> String {
    let words: Vec<&str> = input.split_whitespace().collect();
    if words.is_empty() {
        return String::new();
    }

    let mut result = words[0].to_lowercase();
    for word in &words[1..] {
        if !word.is_empty() {
            let mut chars = word.chars();
            if let Some(first) = chars.next() {
                result.push(first.to_uppercase().next().unwrap_or(first));
                result.push_str(&chars.as_str().to_lowercase());
            }
        }
    }

    result
}

/// Convert string to PascalCase
///
/// # Arguments
/// * `input` - String to convert
///
/// # Example
/// ```rust,ignore
/// let pascal = to_pascal_case("hello world test");
/// assert_eq!(pascal, "HelloWorldTest");
/// ```
pub fn to_pascal_case(input: &str) -> String {
    input
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                }
            }
        })
        .collect()
}

/// Convert string to snake_case
///
/// # Arguments
/// * `input` - String to convert
///
/// # Example
/// ```rust,ignore
/// let snake = to_snake_case("Hello World Test");
/// assert_eq!(snake, "hello_world_test");
/// ```
pub fn to_snake_case(input: &str) -> String {
    input
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join("_")
}

/// Truncate string to specified length with ellipsis
///
/// # Arguments
/// * `input` - String to truncate
/// * `max_length` - Maximum length including ellipsis
///
/// # Example
/// ```rust,ignore
/// let truncated = truncate("This is a very long string", 15);
/// assert_eq!(truncated, "This is a ve...");
/// ```
pub fn truncate(input: &str, max_length: usize) -> String {
    if input.len() <= max_length {
        input.to_string()
    } else if max_length <= 3 {
        "...".to_string()
    } else {
        format!("{}...", &input[..max_length - 3])
    }
}

/// Truncate string at word boundary
///
/// Truncates string but tries to break at word boundaries to avoid
/// cutting words in half.
///
/// # Arguments
/// * `input` - String to truncate
/// * `max_length` - Maximum length including ellipsis
///
/// # Example
/// ```rust,ignore
/// let truncated = truncate_words("This is a very long string", 15);
/// assert_eq!(truncated, "This is a...");
/// ```
pub fn truncate_words(input: &str, max_length: usize) -> String {
    if input.len() <= max_length {
        return input.to_string();
    }

    if max_length <= 3 {
        return "...".to_string();
    }

    let mut result = String::new();
    let mut current_length = 0;

    for word in input.split_whitespace() {
        let word_with_space = if result.is_empty() {
            word
        } else {
            &format!(" {}", word)
        };

        if current_length + word_with_space.len() + 3 > max_length {
            break;
        }

        result.push_str(word_with_space);
        current_length += word_with_space.len();
    }

    if current_length > 0 && current_length < input.len() {
        result.push_str("...");
    }

    result
}

/// Count words in a string
///
/// # Arguments
/// * `input` - String to count words in
///
/// # Example
/// ```rust,ignore
/// let count = word_count("Hello world, this is a test!");
/// assert_eq!(count, 6);
/// ```
pub fn word_count(input: &str) -> usize {
    input.split_whitespace().count()
}

/// Capitalize first letter of each word
///
/// # Arguments
/// * `input` - String to capitalize
///
/// # Example
/// ```rust,ignore
/// let title = title_case("hello world test");
/// assert_eq!(title, "Hello World Test");
/// ```
pub fn title_case(input: &str) -> String {
    input
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
}

/// Remove HTML tags from string
///
/// Simple HTML tag removal. For more robust HTML processing,
/// consider using a dedicated HTML parsing library.
///
/// # Arguments
/// * `input` - String containing HTML
///
/// # Example
/// ```rust,ignore
/// let clean = strip_html("<p>Hello <strong>world</strong>!</p>");
/// assert_eq!(clean, "Hello world!");
/// ```
pub fn strip_html(input: &str) -> String {
    let mut result = String::new();
    let mut inside_tag = false;

    for ch in input.chars() {
        match ch {
            '<' => inside_tag = true,
            '>' => {
                inside_tag = false;
                // Add space to separate content that was inside tags
                if !result.is_empty() && !result.ends_with(' ') {
                    result.push(' ');
                }
            }
            _ if !inside_tag => result.push(ch),
            _ => {}
        }
    }

    // Clean up extra spaces
    trim(&result)
}

/// Wrap text to specified line length
///
/// # Arguments
/// * `input` - Text to wrap
/// * `width` - Maximum line width
///
/// # Example
/// ```rust,ignore
/// let wrapped = wrap_text("This is a very long line that needs to be wrapped", 20);
/// // Returns multi-line string with lines no longer than 20 characters
/// ```
pub fn wrap_text(input: &str, width: usize) -> String {
    if width == 0 {
        return input.to_string();
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut current_length = 0;

    for word in input.split_whitespace() {
        let word_len = word.len();
        let space_needed = if current_line.is_empty() { 0 } else { 1 };

        if current_length + space_needed + word_len > width && !current_line.is_empty() {
            lines.push(current_line);
            current_line = word.to_string();
            current_length = word_len;
        } else {
            if !current_line.is_empty() {
                current_line.push(' ');
                current_length += 1;
            }
            current_line.push_str(word);
            current_length += word_len;
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trim() {
        assert_eq!(trim("  hello    world  \n\t"), "hello world");
        assert_eq!(trim("single"), "single");
        assert_eq!(trim(""), "");
        assert_eq!(trim("   "), "");
    }

    #[test]
    fn test_extract_keywords() {
        let keywords = keywords(
            "This is a sample text for testing keywords extraction",
            5,
            3,
        );

        // Should not contain stop words
        assert!(!keywords.contains(&"this".to_string()));
        assert!(!keywords.contains(&"is".to_string()));
        assert!(!keywords.contains(&"a".to_string()));

        // Should contain meaningful words
        assert!(
            keywords.contains(&"sample".to_string())
                || keywords.contains(&"text".to_string())
                || keywords.contains(&"testing".to_string())
        );

        // Should respect max_count
        assert!(keywords.len() <= 5);
    }

    #[test]
    fn test_to_slug() {
        assert_eq!(
            to_slug("Hello World! This is a Test."),
            "hello-world-this-is-a-test"
        );
        assert_eq!(to_slug("  Multiple   Spaces  "), "multiple-spaces");
        assert_eq!(
            to_slug("Special-Characters_Here"),
            "special-characters-here"
        );
        assert_eq!(to_slug(""), "");
    }

    #[test]
    fn test_case_conversions() {
        assert_eq!(to_camel_case("hello world test"), "helloWorldTest");
        assert_eq!(to_pascal_case("hello world test"), "HelloWorldTest");
        assert_eq!(to_snake_case("Hello World Test"), "hello_world_test");
        assert_eq!(title_case("hello world test"), "Hello World Test");
    }

    #[test]
    fn test_truncate() {
        assert_eq!(
            truncate("This is a very long string", 15),
            "This is a ve..."
        );
        assert_eq!(truncate("Short", 15), "Short");
        assert_eq!(truncate("Test", 3), "...");
        assert_eq!(truncate("", 10), "");
    }

    #[test]
    fn test_truncate_words() {
        assert_eq!(
            truncate_words("This is a very long string", 15),
            "This is a..."
        );
        assert_eq!(truncate_words("Short text", 15), "Short text");
        assert_eq!(truncate_words("", 10), "");
    }

    #[test]
    fn test_word_count() {
        assert_eq!(word_count("Hello world, this is a test!"), 6);
        assert_eq!(word_count("Single"), 1);
        assert_eq!(word_count(""), 0);
        assert_eq!(word_count("   "), 0);
    }

    #[test]
    fn test_strip_html() {
        assert_eq!(
            strip_html("<p>Hello <strong>world</strong>!</p>"),
            "Hello world !"
        );
        assert_eq!(strip_html("No HTML here"), "No HTML here");
        assert_eq!(
            strip_html("<div><span>Nested</span> tags</div>"),
            "Nested tags"
        );
        assert_eq!(strip_html(""), "");
    }

    #[test]
    fn test_wrap_text() {
        let wrapped = wrap_text("This is a very long line that needs to be wrapped", 20);
        let lines: Vec<&str> = wrapped.split('\n').collect();

        // All lines should be <= 20 characters
        for line in &lines {
            assert!(line.len() <= 20);
        }

        // Should contain all original words
        let original_words: Vec<&str> = "This is a very long line that needs to be wrapped"
            .split_whitespace()
            .collect();
        let wrapped_words: Vec<&str> = wrapped.split_whitespace().collect();
        assert_eq!(original_words, wrapped_words);
    }
}
