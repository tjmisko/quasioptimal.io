# zk2data

Generates the site's `data/commonplace.json` and `data/bibliography.json` from the **YAML
frontmatter** of a private Obsidian vault. It is a standalone Rust tool, **not** part of the
Zola build — run it on the machine that holds the vault, commit the resulting JSON, and deploy
as usual (the server only ever sees the committed JSON, never the vault).

## The one rule

**A note's Markdown body can never reach the published output.** Only an explicit allowlist of
frontmatter fields is published. This is enforced structurally, in five independent layers:

1. **The parser returns only the frontmatter slice** (`src/frontmatter.rs`). The body substring
   is never sliced into a return value — it cannot flow anywhere. A file without an opening
   `---` fence yields `None` and is skipped. Only the first top-of-file fence pair counts, so a
   `---` rule inside the body is never mistaken for a delimiter.
2. **The frontmatter is deserialized into a fixed struct** (`NoteFrontmatter` in `src/model.rs`)
   whose fields *are* the allowlist. Serde drops every unknown key; the body is not a key, so it
   is unrepresentable.
3. **Opt-in is required.** A note publishes a commonplace entry iff it has `public-quote`, and a
   bibliography entry iff it has `public-note`. Presence is necessary and sufficient; a note
   with neither is silently skipped.
4. **Markdown rendering touches only `public-*` values** (`src/render.rs`).
5. **A sentinel test enforces it** (`tests/leak_guard.rs`). Every note in the dummy vault
   (`mocks/vault/`) embeds `BODY_SENTINEL_DO_NOT_PUBLISH` in its body; the test runs the tool
   and asserts that string never appears in the generated JSON. `make verify-no-leak` also greps
   `data/` and the built `public/`.

If you extend the tool, **never** add a field that captures the body and never `flatten` a
free-form map into `NoteFrontmatter`.

## Usage

```bash
# From the repo root (recommended — writes data/ in place):
make data VAULT=/path/to/vault        # or: export QUASIOPTIMAL_VAULT=/path/to/vault

# Or directly:
cargo run --release -- --vault /path/to/vault --out ../../data
```

The tool reports counts to stderr: files scanned, quotes and sources published, and how many
notes were skipped (no frontmatter / no `public-*` field).

## Frontmatter schema

Author long-form fields as a YAML **literal block scalar** (`|`): it's verbatim (no quoting
pitfalls with `:`/`"`/`#`) and a blank line becomes a paragraph. Markdown is rendered with the
same CommonMark engine Zola uses, so `*emphasis*`, `**strong**` and `[links](…)` work; raw
inline HTML passes through.

**Commonplace note** (`public-quote` ⇒ published):

| field          | required | notes                                         |
|----------------|----------|-----------------------------------------------|
| `public-quote` | yes      | the quote; Markdown, may be multi-paragraph    |
| `author`       | no       | attribution                                    |
| `source`       | no       | the cited work (rendered in `<cite>`)          |
| `public-link`  | no       | URI that links the source                      |

**Source note** (`public-note` ⇒ published):

| field         | required | notes                                                        |
|---------------|----------|--------------------------------------------------------------|
| `public-note` | yes      | the blurb; Markdown, may be multi-paragraph                  |
| `title`       | yes      | without it the entry is skipped (with a warning)             |
| `author`      | no       | store as `Last, First` — surname shown, sorts by surname     |
| `year`        | no       | number or string                                             |
| `tags`        | no       | first recognised citation type (`book`, `article`, …) wins; `book` italicises the title |
| `topic`       | no       | groups entries on the page; defaults to `Uncategorized`      |
| `public-link` | no       | URI that links the title                                     |
| `public-review` | no     | link to a longer review (book-and-pencil icon). A bare slug → `/reviews/<slug>/`; a full `http(s)` URL is used verbatim |

A note may carry both `public-quote` and `public-note`; it then appears in both sections.

### Reviews

`public-review` links a source to a longer, blog-style review. Those reviews are **public Zola
pages** authored directly in `content/reviews/` (the note body still never publishes — only the
link does). Drop `content/reviews/<slug>.md` with `template = "review.html"`, set the note's
`public-review: <slug>`, and a book-and-pencil icon on that bibliography entry links to it. The
section also lists at `/reviews/`.

## Tests

```bash
cargo test        # unit tests + the end-to-end leak guard
```
