//! End-to-end leak guard.
//!
//! These tests run the *compiled binary* against the dummy vault in the repo's `mocks/` directory
//! and inspect the JSON it writes — the same path `make data` uses in production. The central
//! assertion is that the body sentinel embedded in every mock note can never appear in the
//! generated output, which is the enforceable form of "a note's body cannot reach the internet."

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Embedded in the body of every mock note (and in files that should be skipped entirely).
const SENTINEL: &str = "BODY_SENTINEL_DO_NOT_PUBLISH";

/// The dummy vault lives at the repo root (`mocks/vault/`), two levels up from this crate.
fn mock_vault() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mocks/vault")
}

/// Run the tool against the mock vault, writing into `out_dir`, and return the two JSON files'
/// contents as `(commonplace, bibliography)`.
fn generate_into(out_dir: &Path) -> (String, String) {
    let _ = fs::remove_dir_all(out_dir);
    let status = Command::new(env!("CARGO_BIN_EXE_zk2data"))
        .arg("--vault")
        .arg(mock_vault())
        .arg("--out")
        .arg(out_dir)
        .status()
        .expect("failed to run the zk2data binary");
    assert!(status.success(), "zk2data exited with a non-zero status");
    let commonplace = fs::read_to_string(out_dir.join("commonplace.json")).unwrap();
    let bibliography = fs::read_to_string(out_dir.join("bibliography.json")).unwrap();
    (commonplace, bibliography)
}

fn out(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name)
}

#[test]
fn body_sentinel_never_reaches_generated_json() {
    let (commonplace, bibliography) = generate_into(&out("sentinel"));
    assert!(
        !commonplace.contains(SENTINEL),
        "a note body leaked into commonplace.json"
    );
    assert!(
        !bibliography.contains(SENTINEL),
        "a note body leaked into bibliography.json"
    );
}

#[test]
fn only_opted_in_notes_are_published() {
    let (commonplace, bibliography) = generate_into(&out("counts"));
    let quotes: serde_json::Value = serde_json::from_str(&commonplace).unwrap();
    let entries: serde_json::Value = serde_json::from_str(&bibliography).unwrap();

    // Exactly the notes carrying a public-* field, and no others.
    assert_eq!(
        quotes.as_array().unwrap().len(),
        3,
        "expected 3 commonplace entries"
    );
    assert_eq!(
        entries.as_array().unwrap().len(),
        4,
        "expected 4 bibliography entries"
    );

    // A fully-described book note WITHOUT public-note must not appear (opt-in is required)…
    assert!(
        !bibliography.contains("A Private Book"),
        "a note lacking public-note must not be published"
    );
    // …and a note inside a hidden directory must never be scanned at all.
    assert!(
        !commonplace.contains("Should Not Appear"),
        "a note in a hidden directory must not be scanned"
    );
}

#[test]
fn rendered_markdown_and_shape_are_correct() {
    let (commonplace, bibliography) = generate_into(&out("shape"));
    // Multi-paragraph + inline formatting survive the round-trip.
    assert!(commonplace.contains("<strong>not</strong>"));
    assert!(commonplace.contains("<em>take away</em>"));
    assert!(commonplace.contains("<a href=\\\"https://example.com/terre\\\">"));
    // Citation type is derived from tags.
    assert!(bibliography.contains("\"type\": \"book\""));
    assert!(bibliography.contains("\"type\": \"article\""));
    // A note's `public-link` is processed into the `url` the templates link.
    let entries: serde_json::Value = serde_json::from_str(&bibliography).unwrap();
    let seeing = entries
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["title"] == "Seeing Like a State")
        .expect("Seeing Like a State should be published");
    assert_eq!(
        seeing["url"], "https://en.wikipedia.org/wiki/Seeing_Like_a_State",
        "public-link must become the entry's url"
    );
    // A bare `public-review` slug resolves to an on-site /reviews/ path…
    assert_eq!(
        seeing["review"], "/reviews/seeing-like-a-state/",
        "a public-review slug must resolve under /reviews/"
    );
    // …and a full http(s) `public-review` URL is passed through verbatim.
    let kuhn = entries
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["title"] == "The Structure of Scientific Revolutions")
        .expect("Kuhn should be published");
    assert_eq!(kuhn["review"], "https://example.com/reviews/kuhn");
}

#[test]
fn output_is_deterministic() {
    let first = generate_into(&out("determinism_a"));
    let second = generate_into(&out("determinism_b"));
    assert_eq!(
        first, second,
        "two runs over the same vault must produce identical JSON"
    );
}
