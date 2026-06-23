//! HTML minifier that collapses unnecessary whitespace from rendered templates.
//!
//! - `<pre>` blocks are preserved verbatim (whitespace is significant).
//! - `<style>` blocks have their inner CSS minified (block comments removed,
//!   whitespace collapsed).
//! - `<script>` blocks have their inner JS minified (block comments removed,
//!   blank lines / indentation collapsed). Line comments (`//`) are left
//!   untouched to avoid accidentally breaking URLs or regex literals inside
//!   string values.
//! - The minifier uses compiled static regexes so there is no per-call overhead.

use regex::Regex;
use std::sync::OnceLock;

/// Matches `<pre>`, `<script>`, or `<style>` blocks, capturing:
///   1 = full opening tag (e.g. `<script type="module">`)
///   2 = tag name (`pre` | `script` | `style`)
///   3 = inner content
///   4 = closing tag (e.g. `</script>`)
static PROTECTED_RE: OnceLock<Regex> = OnceLock::new();

/// Matches `/* ... */` block comments (including multiline).
static BLOCK_COMMENT_RE: OnceLock<Regex> = OnceLock::new();

/// Matches one or more whitespace characters between a `>` and a `<`.
static BETWEEN_TAGS_RE: OnceLock<Regex> = OnceLock::new();

/// Matches HTML comments `<!-- ... -->` (including multiline).
static HTML_COMMENT_RE: OnceLock<Regex> = OnceLock::new();

/// Matches a newline with optional surrounding horizontal whitespace.
/// Used to collapse newlines inside tag attributes and text nodes.
static NEWLINE_RE: OnceLock<Regex> = OnceLock::new();

/// Matches two or more horizontal whitespace characters (spaces / tabs).
static MULTI_SPACE_RE: OnceLock<Regex> = OnceLock::new();

fn protected_re() -> &'static Regex {
    PROTECTED_RE.get_or_init(|| {
        Regex::new(r"(?si)(<(pre|script|style)[^>]*>)(.*?)(</(?:pre|script|style)\s*>)")
            .expect("Invalid protected block regex")
    })
}

fn block_comment_re() -> &'static Regex {
    BLOCK_COMMENT_RE
        .get_or_init(|| Regex::new(r"(?s)/\*.*?\*/").expect("Invalid block comment regex"))
}

fn html_comment_re() -> &'static Regex {
    HTML_COMMENT_RE
        .get_or_init(|| Regex::new(r"(?s)<!--.*?-->").expect("Invalid HTML comment regex"))
}

fn between_tags_re() -> &'static Regex {
    BETWEEN_TAGS_RE.get_or_init(|| Regex::new(r">\s+<").expect("Invalid between-tags regex"))
}

fn newline_re() -> &'static Regex {
    NEWLINE_RE.get_or_init(|| Regex::new(r"[ \t]*\n[ \t]*").expect("Invalid newline regex"))
}

fn multi_space_re() -> &'static Regex {
    MULTI_SPACE_RE.get_or_init(|| Regex::new(r"[ \t]{2,}").expect("Invalid multi-space regex"))
}

/// Minify CSS content (inner text of a `<style>` block).
///
/// - Strips `/* */` block comments.
/// - Trims each line and drops blank lines.
/// - Joins surviving lines with a single space.
fn minify_css(css: &str) -> String {
    let no_comments = block_comment_re().replace_all(css, "");
    no_comments
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Minify JS content (inner text of a `<script>` block).
///
/// - Strips `/* */` block comments.
/// - Trims each line and drops blank lines.
/// - Joins surviving lines with `\n` (safer than space for JS — avoids
///   accidental removal of ASI-sensitive line boundaries).
///
/// Line comments (`//`) are intentionally left untouched to avoid
/// breaking URLs (`"https://…"`) or regex literals (`/foo\/bar/`).
fn minify_js(js: &str) -> String {
    let no_block_comments = block_comment_re().replace_all(js, "");
    no_block_comments
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Minify an HTML string.
///
/// - Whitespace between tags (`>   <`) is removed entirely.
/// - Consecutive spaces/tabs in text content are collapsed to a single space.
/// - `<pre>` block content is left unchanged.
/// - `<style>` block content is CSS-minified.
/// - `<script>` block content is JS-minified.
pub fn minify_html(html: &str) -> String {
    let protected = protected_re();

    // Step 1: process protected blocks and replace them with self-closing
    //         tag placeholders. The placeholder must start with `<` and end
    //         with `>` so that the `>\s+<` regex can collapse whitespace at
    //         block boundaries.
    let mut stash: Vec<String> = Vec::new();
    let mut working = String::with_capacity(html.len());
    let mut last_end = 0;

    for cap in protected.captures_iter(html) {
        let m = cap.get(0).unwrap();
        working.push_str(&html[last_end..m.start()]);

        let open_tag = cap.get(1).unwrap().as_str();
        let tag_name = cap.get(2).unwrap().as_str().to_ascii_lowercase();
        let inner = cap.get(3).unwrap().as_str();
        let close_tag = cap.get(4).unwrap().as_str();

        let processed = match tag_name.as_str() {
            "script" => format!("{}{}{}", open_tag, minify_js(inner), close_tag),
            "style" => format!("{}{}{}", open_tag, minify_css(inner), close_tag),
            _ => m.as_str().to_string(), // <pre>: preserve verbatim
        };

        working.push_str(&format!("<rustf-pb-{}/>", stash.len()));
        stash.push(processed);
        last_end = m.end();
    }
    working.push_str(&html[last_end..]);

    // Step 2: strip HTML comments (after stashing so comments inside
    //         <script>/<style>/<pre> blocks are not touched)
    let working = html_comment_re().replace_all(&working, "");

    // Step 3: collapse newlines (with surrounding horizontal whitespace) to a
    //         single space — removes line breaks inside attribute values and
    //         between inline elements in text nodes
    let working = newline_re().replace_all(working.as_ref(), " ");

    // Step 4: remove whitespace between tags (works across placeholders too)
    let working = between_tags_re().replace_all(working.as_ref(), "><");

    // Step 5: collapse runs of spaces/tabs within text content
    let working = multi_space_re().replace_all(working.as_ref(), " ");

    // Step 6: trim leading/trailing whitespace from the whole document
    let mut result = working.trim().to_string();

    // Step 7: restore processed blocks
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
    fn minifies_script_block() {
        let input = "<head>\n  <script>\n    var x = 1;\n    /* unused */\n    var y = 2;\n  </script>\n</head>";
        let output = minify_html(input);
        assert_eq!(
            output,
            "<head><script>var x = 1;\nvar y = 2;</script></head>"
        );
    }

    #[test]
    fn minifies_style_block() {
        let input = "<head>\n  <style>\n    /* reset */\n    body { margin: 0; }\n    p { color: red; }\n  </style>\n</head>";
        let output = minify_html(input);
        assert_eq!(
            output,
            "<head><style>body { margin: 0; } p { color: red; }</style></head>"
        );
    }

    #[test]
    fn script_preserves_line_comments() {
        // // comments must NOT be removed (URLs, regex literals)
        let input = "<script>\n  // init\n  var url = 'https://example.com';\n</script>";
        let output = minify_html(input);
        assert_eq!(
            output,
            "<script>// init\nvar url = 'https://example.com';</script>"
        );
    }

    #[test]
    fn strips_html_comments() {
        let input = "<div><!-- a comment --><p>Hello</p></div>";
        assert_eq!(minify_html(input), "<div><p>Hello</p></div>");
    }

    #[test]
    fn strips_multiline_html_comment() {
        let input = "<div>\n  <!-- \n    multi\n    line\n  -->\n  <p>Hi</p>\n</div>";
        assert_eq!(minify_html(input), "<div><p>Hi</p></div>");
    }

    #[test]
    fn preserves_script_html_comment_wrapper() {
        // Old HTML4 trick: <!-- wraps script body to hide from non-JS browsers.
        // Since the whole <script> block is stashed before comment stripping,
        // this wrapper must survive.
        let input = "<script><!--\nvar x = 1;\n//--></script>";
        assert_eq!(
            minify_html(input),
            "<script><!--\nvar x = 1;\n//--></script>"
        );
    }

    #[test]
    fn collapses_newlines_in_attribute() {
        // Multi-line attribute value (e.g. content="...\n...")
        let input = "<meta name=\"viewport\"\n  content=\"width=device-width\">";
        assert_eq!(
            minify_html(input),
            "<meta name=\"viewport\" content=\"width=device-width\">"
        );
    }

    #[test]
    fn collapses_newlines_in_text_node() {
        // Text node that spans lines should collapse to a single space
        let input = "<span>Hello\n  World</span>";
        assert_eq!(minify_html(input), "<span>Hello World</span>");
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
