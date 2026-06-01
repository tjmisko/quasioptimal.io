//! Markdown rendering.
//!
//! This is called ONLY on `public-*` frontmatter values — never on a note's body. It uses the
//! same CommonMark engine (`pulldown-cmark`) that Zola uses internally, so a blank line becomes a
//! paragraph and `*emphasis*` / `**strong**` / `[links](…)` render consistently with posts.

use pulldown_cmark::{html, Options, Parser};

/// Render a `public-*` field's Markdown to an HTML fragment. Raw inline HTML in the source passes
/// through (so a hand-written `<em>x</em>` works too). Pure CommonMark, no extensions, to match
/// Zola's default post rendering.
pub fn render_markdown(input: &str) -> String {
    let parser = Parser::new_ext(input, Options::empty());
    let mut out = String::new();
    html::push_html(&mut out, parser);
    out.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_a_single_paragraph() {
        assert_eq!(render_markdown("Just one line."), "<p>Just one line.</p>");
    }

    #[test]
    fn blank_line_starts_a_new_paragraph() {
        let html = render_markdown("First para.\n\nSecond para.");
        assert_eq!(html, "<p>First para.</p>\n<p>Second para.</p>");
    }

    #[test]
    fn renders_inline_emphasis_and_links() {
        let html = render_markdown("**bold** and *italic* and [link](https://example.com).");
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<em>italic</em>"));
        assert!(html.contains("<a href=\"https://example.com\">link</a>"));
    }

    #[test]
    fn passes_through_inline_html() {
        let html = render_markdown("plain <em>raw</em> html");
        assert!(html.contains("<em>raw</em>"));
    }
}
