//! HTML minifier that collapses unnecessary whitespace from rendered templates.
//!
//! Protected blocks (`<pre>`, `<script>`, `<style>`) are preserved verbatim.
//! The minifier uses compiled static regexes so there is no per-call overhead.

use regex::Regex;
use std::sync::OnceLock;

/// Matches `<pre>`, `<script>`, or `<style>` blocks (with their contents) case-insensitively.
/// The `(?s)` flag makes `.` match newlines so multiline blocks are captured correctly.
static PROTECTED_RE: OnceLock<Regex> = OnceLock::new();

/// Matches one or more whitespace characters between a `>` and a `<`.
static BETWEEN_TAGS_RE: OnceLock<Regex> = OnceLock::new();

/// Matches two or more horizontal whitespace characters (spaces / tabs).
static MULTI_SPACE_RE: OnceLock<Regex> = OnceLock::new();

fn protected_re() -> &'static Regex {
    PROTECTED_RE.get_or_init(|| {
        Regex::new(r"(?si)(<(?:pre|script|style)[^>]*>.*?</(?:pre|script|style)\s*>)")
            .expect("Invalid protected block regex")
    })
}

fn between_tags_re() -> &'static Regex {
    BETWEEN_TAGS_RE.get_or_init(|| {
        Regex::new(r">\s+<").expect("Invalid between-tags regex")
    })
}

fn multi_space_re() -> &'static Regex {
    MULTI_SPACE_RE.get_or_init(|| {
        Regex::new(r"[ \t]{2,}").expect("Invalid multi-space regex")
    })
}

/// Minify an HTML string by collapsing unnecessary whitespace.
///
/// - Whitespace between tags (`>   <`) is removed entirely.
/// - Consecutive spaces/tabs in text content are collapsed to a single space.
/// - Content inside `<pre>`, `<script>`, and `<style>` blocks is left unchanged.
pub fn minify_html(html: &str) -> String {
    let protected = protected_re();

    // Step 1: stash protected blocks and replace them with self-closing tag placeholders.
    //         The placeholder must start with `<` and end with `>` so that the
    //         `>\s+<` regex can correctly collapse whitespace at block boundaries.
    let mut stash: Vec<String> = Vec::new();
    let mut working = String::with_capacity(html.len());
    let mut last_end = 0;

    for m in protected.find_iter(html) {
        working.push_str(&html[last_end..m.start()]);
        working.push_str(&format!("<rustf-pb-{}/>", stash.len()));
        stash.push(m.as_str().to_string());
        last_end = m.end();
    }
    working.push_str(&html[last_end..]);

    // Step 2: remove whitespace between tags (works across placeholders too)
    let working = between_tags_re().replace_all(&working, "><");

    // Step 3: collapse runs of spaces/tabs within text content
    let working = multi_space_re().replace_all(working.as_ref(), " ");

    // Step 4: trim leading/trailing whitespace from the whole document
    let mut result = working.trim().to_string();

    // Step 5: restore protected blocks
    for (idx, block) in stash.iter().enumerate() {
        result = result.replace(&format!("<rustf-pb-{}/>", idx), block);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapses_whitespace_between_tags() {
        let input = "<div>  \n  <p>Hello</p>  \n  </div>";
        let output = minify_html(input);
        assert_eq!(output, "<div><p>Hello</p></div>");
    }

    #[test]
    fn collapses_multiple_spaces_in_text() {
        let input = "<p>Hello   World</p>";
        let output = minify_html(input);
        assert_eq!(output, "<p>Hello World</p>");
    }

    #[test]
    fn preserves_pre_block() {
        let input = "<div>\n  <pre>  spaced\n  content  </pre>\n</div>";
        let output = minify_html(input);
        assert_eq!(output, "<div><pre>  spaced\n  content  </pre></div>");
    }

    #[test]
    fn preserves_script_block() {
        let input = "<head>\n  <script>\n    var x = 1;\n  </script>\n</head>";
        let output = minify_html(input);
        assert_eq!(output, "<head><script>\n    var x = 1;\n  </script></head>");
    }

    #[test]
    fn preserves_style_block() {
        let input = "<head>\n  <style>\n    body { margin: 0; }\n  </style>\n</head>";
        let output = minify_html(input);
        assert_eq!(output, "<head><style>\n    body { margin: 0; }\n  </style></head>");
    }

    #[test]
    fn handles_empty_input() {
        assert_eq!(minify_html(""), "");
    }

    #[test]
    fn handles_plain_text() {
        assert_eq!(minify_html("Hello"), "Hello");
    }

    #[test]
    fn real_world_layout() {
        let input = r#"<!DOCTYPE html>
<html>
  <head>
    <title>Test</title>
  </head>
  <body>
    <div class="container">
      <h1>Hello</h1>
      <p>World</p>
    </div>
  </body>
</html>"#;
        let output = minify_html(input);
        assert_eq!(
            output,
            r#"<!DOCTYPE html><html><head><title>Test</title></head><body><div class="container"><h1>Hello</h1><p>World</p></div></body></html>"#
        );
    }
}
