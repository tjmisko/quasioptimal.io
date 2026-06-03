//! Data model: the frontmatter allowlist and the output shapes.
//!
//! SAFETY-CRITICAL: `NoteFrontmatter`'s fields *are* the allowlist. Serde silently ignores any
//! frontmatter key not named here, and the Markdown body is not a key at all, so nothing outside
//! this struct can ever flow into an output entry. Do not add `#[serde(flatten)]` of a free-form
//! map, and never add a field that captures the body.

use serde::{Deserialize, Serialize};

/// A year may be a number (`1978`) or an occasional string (`"n.d."`); accept both so one odd
/// note can't fail the run, and round-trip it to JSON unchanged.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Year {
    Int(i64),
    Str(String),
}

/// `tags:` in Obsidian frontmatter may be a YAML list or a single bare string; accept either.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Tags {
    One(String),
    Many(Vec<String>),
}

impl Tags {
    pub fn into_vec(self) -> Vec<String> {
        match self {
            Tags::One(s) => vec![s],
            Tags::Many(v) => v,
        }
    }
}

/// The ONLY frontmatter fields this tool reads. Everything else in a note — including the entire
/// Markdown body — is unrepresentable here and therefore unpublishable.
#[derive(Debug, Default, Deserialize)]
pub struct NoteFrontmatter {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub year: Option<Year>,
    #[serde(default)]
    pub tags: Option<Tags>,
    #[serde(default)]
    pub topic: Option<String>,
    /// The cited work for a Commonplace quote (rendered in `<cite>`).
    #[serde(default)]
    pub source: Option<String>,
    /// Public URI for the source/quote. Links the bibliography title and the commonplace
    /// `<cite>` (emitted as the `url` key the templates already use).
    #[serde(default, rename = "public-link")]
    pub public_link: Option<String>,
    /// Link to the source's Wikipedia article. Surfaced as its own icon in the bibliography,
    /// independent of `public-link` (emitted as the `wikipedia` key the templates use).
    #[serde(default, rename = "public-wikipedia")]
    pub public_wikipedia: Option<String>,
    /// Presence of this field is necessary and sufficient to publish a Commonplace entry.
    #[serde(default, rename = "public-quote")]
    pub public_quote: Option<String>,
    /// Presence of this field is necessary and sufficient to publish a Sources entry.
    #[serde(default, rename = "public-note")]
    pub public_note: Option<String>,
    /// Optional link to a longer public review. A bare slug resolves to `/reviews/<slug>/`; a
    /// full http(s) URL is used verbatim. See `resolve_review`.
    #[serde(default, rename = "public-review")]
    pub public_review: Option<String>,
}

/// One Commonplace entry. Field names mirror the existing `data/commonplace.json` shape so
/// `templates/macros.html` needs no structural change.
#[derive(Debug, Serialize)]
pub struct Quote {
    /// Rendered HTML (paragraphs + inline emphasis), produced only from `public-quote`.
    pub quote: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// One Sources/bibliography entry. Field names mirror the existing `data/bibliography.json` shape.
#[derive(Debug, Serialize)]
pub struct BibEntry {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<Year>,
    /// Citation type derived from tags; `"book"` italicises the title, anything else quotes it.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    pub topic: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// URL of the source's Wikipedia article, if the note set `public-wikipedia`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wikipedia: Option<String>,
    /// Resolved URL of a longer review, if the note links one (from `public-review`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review: Option<String>,
    /// Rendered HTML blurb, produced only from `public-note`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Citation types we recognise in `tags:`. The first matching tag (case-insensitive) becomes the
/// entry's `type`. Order is the match priority.
const KNOWN_TYPES: &[&str] = &[
    "book", "article", "paper", "essay", "chapter", "report", "thesis", "talk", "lecture", "video",
    "podcast", "post",
];

/// Resolve a `public-review` value to a URL. A full `http(s)://` link is used verbatim; anything
/// else is treated as a slug under the on-site `/reviews/` section (a leading `reviews/` and any
/// surrounding slashes are tolerated). Returns `None` for an empty value.
pub fn resolve_review(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    if value.starts_with("http://") || value.starts_with("https://") {
        return Some(value.to_string());
    }
    let slug = value.trim_matches('/');
    let slug = slug.strip_prefix("reviews/").unwrap_or(slug);
    Some(format!("/reviews/{slug}/"))
}

/// Derive the citation type from a note's tags, returning the canonical lowercase type string.
pub fn citation_type(tags: &[String]) -> Option<String> {
    for tag in tags {
        let lower = tag.trim().trim_start_matches('#').to_lowercase();
        if let Some(found) = KNOWN_TYPES.iter().find(|t| **t == lower) {
            return Some((*found).to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frontmatter_ignores_unknown_keys() {
        // `secret` and a stray `body`-like key must be dropped: only allowlist fields survive.
        let yaml = "title: X\nsecret: leak-me\nbody: also-leak\npublic-note: hi\n";
        let fm: NoteFrontmatter = serde_norway::from_str(yaml).unwrap();
        assert_eq!(fm.title.as_deref(), Some("X"));
        assert_eq!(fm.public_note.as_deref(), Some("hi"));
        // There is no field that could hold `secret`/`body`, so they cannot be reached.
    }

    #[test]
    fn citation_type_picks_first_known_tag() {
        assert_eq!(
            citation_type(&["math".into(), "book".into()]),
            Some("book".into())
        );
        assert_eq!(citation_type(&["#article".into()]), Some("article".into()));
        assert_eq!(citation_type(&["misc".into()]), None);
    }

    #[test]
    fn tags_accepts_string_or_list() {
        let one: NoteFrontmatter = serde_norway::from_str("tags: book\n").unwrap();
        assert_eq!(one.tags.unwrap().into_vec(), vec!["book".to_string()]);
        let many: NoteFrontmatter = serde_norway::from_str("tags: [book, math]\n").unwrap();
        assert_eq!(
            many.tags.unwrap().into_vec(),
            vec!["book".to_string(), "math".to_string()]
        );
    }

    #[test]
    fn year_accepts_int_or_string() {
        let i: NoteFrontmatter = serde_norway::from_str("year: 1978\n").unwrap();
        assert!(matches!(i.year, Some(Year::Int(1978))));
        let s: NoteFrontmatter = serde_norway::from_str("year: n.d.\n").unwrap();
        assert!(matches!(s.year, Some(Year::Str(_))));
    }

    #[test]
    fn resolve_review_handles_urls_and_slugs() {
        assert_eq!(
            resolve_review("https://example.com/r"),
            Some("https://example.com/r".to_string())
        );
        assert_eq!(
            resolve_review("seeing-like-a-state"),
            Some("/reviews/seeing-like-a-state/".to_string())
        );
        // Surrounding slashes and a leading `reviews/` are tolerated.
        assert_eq!(
            resolve_review("/reviews/seeing-like-a-state/"),
            Some("/reviews/seeing-like-a-state/".to_string())
        );
        assert_eq!(resolve_review("   "), None);
    }

    #[test]
    fn public_link_deserializes_from_the_hyphenated_key() {
        let fm: NoteFrontmatter =
            serde_norway::from_str("public-link: https://example.com/x\n").unwrap();
        assert_eq!(fm.public_link.as_deref(), Some("https://example.com/x"));
        // A bare `url:` is not on the allowlist and is therefore ignored.
        let none: NoteFrontmatter = serde_norway::from_str("url: https://example.com/y\n").unwrap();
        assert_eq!(none.public_link, None);
    }

    #[test]
    fn public_wikipedia_deserializes_from_the_hyphenated_key() {
        let fm: NoteFrontmatter =
            serde_norway::from_str("public-wikipedia: https://en.wikipedia.org/wiki/X\n").unwrap();
        assert_eq!(
            fm.public_wikipedia.as_deref(),
            Some("https://en.wikipedia.org/wiki/X")
        );
        // A bare `wikipedia:` is not on the allowlist and is therefore ignored.
        let none: NoteFrontmatter =
            serde_norway::from_str("wikipedia: https://en.wikipedia.org/wiki/Y\n").unwrap();
        assert_eq!(none.public_wikipedia, None);
    }
}
