//! zk2data — project the public frontmatter of a private Obsidian vault into the two JSON files
//! Zola renders: `data/commonplace.json` and `data/bibliography.json`.
//!
//! The Markdown body of a note is NEVER read into any published value (see `frontmatter.rs`).
//! A note publishes a Commonplace entry iff it has a `public-quote` field, and a Sources entry
//! iff it has a `public-note` field — presence is necessary and sufficient.

mod frontmatter;
mod model;
mod render;

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::Parser as ClapParser;
use walkdir::{DirEntry, WalkDir};

use frontmatter::extract_frontmatter;
use model::{citation_type, resolve_review, BibEntry, NoteFrontmatter, Quote};
use render::render_markdown;

#[derive(ClapParser)]
#[command(
    name = "zk2data",
    about = "Generate commonplace + bibliography data from zettelkasten frontmatter",
    long_about = "Reads only the YAML frontmatter of an Obsidian vault's notes and writes \
                  data/commonplace.json and data/bibliography.json. The Markdown body is never \
                  published. A note appears in the commonplace iff it has a `public-quote` field, \
                  and in the bibliography iff it has a `public-note` field."
)]
struct Cli {
    /// Path to the Obsidian vault. Falls back to the QUASIOPTIMAL_VAULT environment variable.
    #[arg(long)]
    vault: Option<PathBuf>,

    /// Directory to write the generated JSON files into.
    #[arg(long, default_value = "data")]
    out: PathBuf,
}

fn main() {
    if let Err(e) = run(Cli::parse()) {
        eprintln!("zk2data: error: {e:#}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    let vault = resolve_vault(cli.vault.as_deref())?;
    if !vault.is_dir() {
        bail!("vault path is not a directory: {}", vault.display());
    }

    let (quotes, entries, stats) = generate(&vault)?;

    std::fs::create_dir_all(&cli.out)
        .with_context(|| format!("creating output dir {}", cli.out.display()))?;
    write_json(&cli.out.join("bibliography.json"), &entries)?;
    write_json(&cli.out.join("commonplace.json"), &quotes)?;

    eprintln!(
        "zk2data: scanned {} file(s) → {} quote(s), {} source(s) \
         (skipped {} without frontmatter, {} without a public-* field)",
        stats.scanned,
        quotes.len(),
        entries.len(),
        stats.skipped_no_frontmatter,
        stats.skipped_no_public,
    );
    for w in &stats.warnings {
        eprintln!("  warning: {w}");
    }
    Ok(())
}

/// Resolve the vault path: `--vault` wins, then `QUASIOPTIMAL_VAULT`, else an error. No private
/// path is ever baked into the repo.
fn resolve_vault(arg: Option<&Path>) -> Result<PathBuf> {
    if let Some(p) = arg {
        return Ok(p.to_path_buf());
    }
    match std::env::var("QUASIOPTIMAL_VAULT") {
        Ok(v) if !v.is_empty() => Ok(PathBuf::from(v)),
        _ => bail!("no vault: pass --vault <path> or set QUASIOPTIMAL_VAULT"),
    }
}

#[derive(Default)]
struct Stats {
    scanned: usize,
    skipped_no_frontmatter: usize,
    skipped_no_public: usize,
    warnings: Vec<String>,
}

/// Walk the vault and build the two output collections from frontmatter only. Exposed (crate-
/// public) so it could be unit-tested, but the leak-guard test drives the compiled binary so it
/// exercises the real output path end to end.
fn generate(vault: &Path) -> Result<(Vec<Quote>, Vec<BibEntry>, Stats)> {
    let mut quotes = Vec::new();
    let mut entries = Vec::new();
    let mut stats = Stats::default();

    for md in markdown_files(vault) {
        stats.scanned += 1;
        let content =
            std::fs::read_to_string(&md).with_context(|| format!("reading {}", md.display()))?;

        // ONLY the frontmatter slice is ever obtained; the body stays in `content` and is dropped.
        let Some(fm_slice) = extract_frontmatter(&content) else {
            stats.skipped_no_frontmatter += 1;
            continue;
        };
        if fm_slice.trim().is_empty() {
            stats.skipped_no_frontmatter += 1;
            continue;
        }

        let fm: NoteFrontmatter = match serde_norway::from_str(fm_slice) {
            Ok(fm) => fm,
            Err(e) => {
                stats
                    .warnings
                    .push(format!("{}: frontmatter parse error: {e}", md.display()));
                continue;
            }
        };

        let mut published = false;

        if let Some(quote_md) = &fm.public_quote {
            quotes.push(Quote {
                quote: render_markdown(quote_md),
                author: fm.author.clone(),
                source: fm.source.clone(),
                url: fm.public_link.clone(),
            });
            published = true;
        }

        if let Some(note_md) = &fm.public_note {
            match &fm.title {
                Some(title) => {
                    let tags = fm.tags.clone().map(|t| t.into_vec()).unwrap_or_default();
                    entries.push(BibEntry {
                        title: title.clone(),
                        author: fm.author.clone(),
                        year: fm.year.clone(),
                        kind: citation_type(&tags),
                        topic: fm
                            .topic
                            .clone()
                            .unwrap_or_else(|| "Uncategorized".to_string()),
                        url: fm.public_link.clone(),
                        wikipedia: fm
                            .public_wikipedia
                            .as_deref()
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                            .map(str::to_string),
                        review: fm.public_review.as_deref().and_then(resolve_review),
                        note: Some(render_markdown(note_md)),
                    });
                    published = true;
                }
                None => stats.warnings.push(format!(
                    "{}: has public-note but no title; skipping bibliography entry",
                    md.display()
                )),
            }
        }

        if !published {
            stats.skipped_no_public += 1;
        }
    }

    // Deterministic ordering keeps the committed JSON diffs clean. (The bibliography template
    // re-sorts by topic/author at render time and commonplace order is randomized client-side, so
    // this ordering matters only for the diff, not the page.)
    quotes.sort_by(|a, b| {
        a.author
            .cmp(&b.author)
            .then_with(|| a.source.cmp(&b.source))
            .then_with(|| a.quote.cmp(&b.quote))
    });
    entries.sort_by(|a, b| {
        a.topic
            .cmp(&b.topic)
            .then_with(|| a.author.cmp(&b.author))
            .then_with(|| a.title.cmp(&b.title))
    });

    Ok((quotes, entries, stats))
}

/// Collect `*.md` files under the vault, skipping hidden directories (`.obsidian`, `.trash`,
/// `.git`, …). Sorted for deterministic processing order.
fn markdown_files(vault: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = WalkDir::new(vault)
        .into_iter()
        .filter_entry(|e| e.depth() == 0 || !is_hidden(e))
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(DirEntry::into_path)
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("md")))
        .collect();
    files.sort();
    files
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .is_some_and(|s| s.starts_with('.'))
}

/// Write a value as pretty JSON (2-space indent, matching the existing data files) plus a
/// trailing newline.
fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
    let mut json = serde_json::to_string_pretty(value).context("serializing JSON")?;
    json.push('\n');
    std::fs::write(path, json).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}
