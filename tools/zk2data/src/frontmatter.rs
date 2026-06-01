//! Frontmatter extraction.
//!
//! SAFETY-CRITICAL: this module's entire job is to hand back *only* the YAML frontmatter and to
//! leave the Markdown body unreachable. `extract_frontmatter` returns a slice borrowing the
//! frontmatter region of the input and nothing else — the body region is never sliced into a
//! return value, so no caller can accidentally publish it. If the file does not open with a
//! `---` fence there is no frontmatter and we return `None` (the note is skipped entirely).
//! Only the FIRST top-of-file fence pair is honored, so a `---` horizontal rule inside the body
//! can never be mistaken for a frontmatter delimiter.

/// Return the YAML frontmatter slice — the text between the opening `---` line and the next line
/// that is exactly `---` — or `None` when the content does not begin with a frontmatter fence or
/// that fence is never closed. The Markdown body (everything after the closing fence) is
/// deliberately never returned.
pub fn extract_frontmatter(content: &str) -> Option<&str> {
    // The file must open with a fence line. Obsidian writes `---` as the very first bytes.
    let rest = content
        .strip_prefix("---\n")
        .or_else(|| content.strip_prefix("---\r\n"))?;

    // Find the closing fence: a line whose content is exactly `---`. We scan line by line so a
    // `---` embedded mid-line (e.g. inside a quoted value) cannot match, and so an indented
    // `---` inside a block scalar (always indented past column 0) cannot match either.
    let mut offset = 0usize;
    for line in rest.split_inclusive('\n') {
        let trimmed = line.strip_suffix('\n').unwrap_or(line);
        let trimmed = trimmed.strip_suffix('\r').unwrap_or(trimmed);
        if trimmed == "---" {
            // Frontmatter is everything from the start of `rest` up to (not including) this
            // closing fence line. The body — everything after — is intentionally never sliced.
            return Some(&rest[..offset]);
        }
        offset += line.len();
    }

    // Opening fence but no closing fence: malformed; treat as no frontmatter.
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_only_the_top_fenced_block() {
        let doc = "---\ntitle: X\n---\n\nbody text\n";
        assert_eq!(extract_frontmatter(doc), Some("title: X\n"));
    }

    #[test]
    fn ignores_a_horizontal_rule_in_the_body() {
        // The body's own `---` separator must NOT be treated as the closing fence.
        let doc = "---\ntitle: X\n---\n\nfirst para\n\n---\n\nsecond para\n";
        let fm = extract_frontmatter(doc).unwrap();
        assert_eq!(fm, "title: X\n");
        assert!(
            !fm.contains("para"),
            "body must never appear in the frontmatter slice"
        );
    }

    #[test]
    fn returns_none_without_an_opening_fence() {
        assert_eq!(extract_frontmatter("# Just a heading\n\nbody\n"), None);
        assert_eq!(extract_frontmatter("not frontmatter\n---\nx\n"), None);
    }

    #[test]
    fn returns_none_for_an_unclosed_fence() {
        assert_eq!(
            extract_frontmatter("---\ntitle: X\nbody with no close\n"),
            None
        );
    }

    #[test]
    fn handles_crlf_line_endings() {
        let doc = "---\r\ntitle: X\r\n---\r\n\r\nbody\r\n";
        assert_eq!(extract_frontmatter(doc), Some("title: X\r\n"));
    }

    #[test]
    fn handles_a_closing_fence_with_no_trailing_newline() {
        let doc = "---\ntitle: X\n---";
        assert_eq!(extract_frontmatter(doc), Some("title: X\n"));
    }

    #[test]
    fn an_indented_dash_line_in_a_block_scalar_is_not_a_fence() {
        // Block-scalar content is indented past column 0, so a literal `---` line inside a value
        // does not match the exact `---` fence test.
        let doc = "---\npublic-quote: |\n  before\n  ---\n  after\n---\nbody\n";
        let fm = extract_frontmatter(doc).unwrap();
        assert!(fm.contains("before"));
        assert!(fm.contains("after"));
        assert!(!fm.contains("body"));
    }
}
