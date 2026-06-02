# Server Setup — nginx + Zola

How to stand up and run `quasioptimal.io` as a Zola-built static site served by nginx,
edited in place over SSH. The model is deliberately simple:

```
edit content/template in vi  →  zola build  →  nginx serves public/  (no restart)
```

Nothing is dynamic at request time. Zola compiles the whole site into `public/`, and nginx
just serves those files. A rebuild is the only "deploy" step.

When the private vault lives on this same server, §8 closes the loop automatically: a timer
rescans the vault's frontmatter into `data/` and rebuilds whenever that data changes, so
`vault edit → data/*.json → zola build → public/` runs hands-off.

- [1. How the project is laid out](#1-how-the-project-is-laid-out)
- [2. Install Zola on the server](#2-install-zola-on-the-server)
- [3. Put the site on the server](#3-put-the-site-on-the-server)
- [4. First build](#4-first-build)
- [5. nginx configuration](#5-nginx-configuration)
- [6. HTTPS with Certbot](#6-https-with-certbot)
- [7. The edit → publish workflow](#7-the-edit--publish-workflow)
- [8. Run the scan + rebuild on the server (the robust loop)](#8-run-the-scan--rebuild-on-the-server-the-robust-loop)
- [9. Optional: git push-to-deploy](#9-optional-git-push-to-deploy)
- [10. Preview before publishing](#10-preview-before-publishing)
- [11. Cutover from the old flat HTML](#11-cutover-from-the-old-flat-html)
- [12. Adding content](#12-adding-content)
- [13. Troubleshooting](#13-troubleshooting)
- [14. Quick reference](#14-quick-reference)

Throughout, replace `quasioptimal.io` with your domain if different. Commands assume a
Debian/Ubuntu host (`apt`, `www-data`); RHEL/Fedora notes are called out where they differ
(`dnf`, nginx runs as the `nginx` user).

---

## 1. How the project is laid out

```
quasioptimal.io/            ← Zola project root (this git repo)
├── config.toml             ← site config; base_url lives here
├── content/                ← pages & posts (Markdown + TOML frontmatter)
│   ├── _index.md           ← home page text (the site-brief blurb)
│   ├── bibliography.md     ← thin page → templates/bibliography.html
│   ├── commonplace.md      ← thin page → templates/commonplace.html
│   └── posts/
│       ├── _index.md       ← the "Writing" section (sorted by date)
│       └── scaling-the-good.md
├── data/                   ← JSON the templates loop over (load_data)
│   ├── bibliography.json
│   └── commonplace.json
├── templates/              ← Tera templates (the shared layout lives here)
│   ├── base.html           ← the shell: <head>, header/nav, content slot
│   ├── index.html, section.html, post.html
│   ├── bibliography.html, commonplace.html, macros.html, 404.html
├── static/                 ← copied verbatim to the site root
│   ├── style.css           ← served at /style.css
│   └── js/commonplace.js
└── public/                 ← BUILD OUTPUT (git-ignored). nginx serves this.
```

Key fact: **nginx's web root is `public/`, not the repo root.** You never point nginx at
the Markdown or templates — only at what `zola build` emits.

---

## 2. Install Zola on the server

Zola is a single static binary, no runtime. Pick one method.

### a) Official binary (x86_64 Linux — most servers)

Check <https://github.com/getzola/zola/releases> for the latest version, then:

```bash
ZOLA_VERSION=0.22.1   # ← bump to the current release; this scaffold was verified on 0.22.1
cd /tmp
curl -fsSL -o zola.tar.gz \
  "https://github.com/getzola/zola/releases/download/v${ZOLA_VERSION}/zola-v${ZOLA_VERSION}-x86_64-unknown-linux-gnu.tar.gz"
tar xf zola.tar.gz
sudo install -m 0755 zola /usr/local/bin/zola
zola --version
```

### b) ARM64 / aarch64 Linux servers

The official release tarballs are **x86_64-only**, so on an ARM server use a package
manager or build it:

```bash
# Debian/Ubuntu (may lag a version or two behind):
sudo apt install zola

# or Snap (tracks latest):
sudo snap install --edge zola

# or build from source (needs the Rust toolchain; ~minutes):
cargo install --locked zola
```

### c) Distro packages (any arch, convenience over freshness)

```bash
sudo apt install zola          # Debian/Ubuntu
sudo dnf install zola          # Fedora
```

Whatever you choose, confirm `zola --version` resolves before continuing.

---

## 3. Put the site on the server

Clone the repo to where it will live and be built. We use `/var/www/quasioptimal.io` as the
project root; `public/` underneath it becomes the web root.

```bash
sudo mkdir -p /var/www
sudo chown "$USER":"$USER" /var/www
git clone <your-repo-url> /var/www/quasioptimal.io
cd /var/www/quasioptimal.io
```

No remote yet? You can also `rsync` the project up, or just `scp` it and `git init` later —
Zola only needs the files, not git. Git just makes "edit → commit → keep history" pleasant.

Make sure `base_url` in `config.toml` matches the real URL (it's already
`https://quasioptimal.io`). It must be correct or internal links and the sitemap break.

---

## 4. First build

```bash
cd /var/www/quasioptimal.io
zola build
```

This writes the complete site to `public/`. Sanity-check it:

```bash
ls public/                       # index.html, bibliography/, commonplace/, posts/, style.css …
zola check                       # validates internal links & structure (optional)
```

nginx's user must be able to read the tree. The default `www-data` (Debian) / `nginx`
(RHEL) can read world-readable files, but it must be able to **traverse** every parent dir:

```bash
chmod o+x /var/www /var/www/quasioptimal.io
# files Zola writes are already world-readable; if in doubt:
chmod -R a+rX /var/www/quasioptimal.io/public
```

---

## 5. nginx configuration

Create `/etc/nginx/sites-available/quasioptimal.io` (Debian layout). On RHEL/Fedora there's
no `sites-available`; drop the `server { … }` block into `/etc/nginx/conf.d/quasioptimal.io.conf`
instead and skip the `ln -s` step.

```nginx
server {
    listen 80;
    listen [::]:80;
    server_name quasioptimal.io www.quasioptimal.io;

    # Web root = Zola's build output.
    root /var/www/quasioptimal.io/public;
    index index.html;

    # Clean URLs: /bibliography/ → /bibliography/index.html. The $uri/ fallback lets
    # /bibliography (no trailing slash) resolve to the directory index too.
    location / {
        try_files $uri $uri/ =404;
    }

    # Zola writes a 404.html at the root (from templates/404.html).
    error_page 404 /404.html;

    # Cache static assets; HTML stays uncached so edits show up immediately.
    location ~* \.(?:css|js|woff2?|ttf|otf|eot|svg|png|jpe?g|gif|ico|webp|avif)$ {
        expires 7d;
        add_header Cache-Control "public";
        access_log off;
    }

    # Compression.
    gzip on;
    gzip_vary on;
    gzip_min_length 256;
    gzip_types text/plain text/css application/javascript application/json
               image/svg+xml application/atom+xml application/xml;

    # Modest security headers. (No Content-Security-Policy here: the post page pulls
    # highlight.js/KaTeX from CDNs and runs small inline init scripts, so a strict CSP
    # would need to allowlist those origins plus 'unsafe-inline'. Add one deliberately
    # if you want it — see Troubleshooting.)
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header Referrer-Policy "strict-origin-when-cross-origin" always;

    # Don't serve dotfiles (e.g. a stray .git if the repo root were ever exposed).
    location ~ /\.(?!well-known) { deny all; }
}
```

Enable and reload:

```bash
sudo ln -s /etc/nginx/sites-available/quasioptimal.io /etc/nginx/sites-enabled/   # Debian only
sudo nginx -t          # syntax check — always do this before reload
sudo systemctl reload nginx
```

Visit `http://quasioptimal.io/`. You should see the site over plain HTTP. TLS is next.

---

## 6. HTTPS with Certbot

Let Certbot obtain a certificate and rewrite the nginx block to add the `443` server and an
HTTP→HTTPS redirect.

```bash
# Debian/Ubuntu
sudo apt install certbot python3-certbot-nginx
# Fedora/RHEL: sudo dnf install certbot python3-certbot-nginx

sudo certbot --nginx -d quasioptimal.io -d www.quasioptimal.io
```

Choose the redirect option when prompted. Certbot installs a renewal timer; verify with:

```bash
sudo certbot renew --dry-run
```

After this, your port-80 block becomes a redirect and a new `listen 443 ssl` block serves
the site. You generally don't hand-edit it again — re-run `certbot` if domains change.

---

## 7. The edit → publish workflow

This is the day-to-day loop, all over SSH:

```bash
cd /var/www/quasioptimal.io
vi content/posts/my-new-post.md     # or any template / data file
zola build                          # regenerate public/
```

That's the whole deploy. nginx serves the new `public/` on the next request — **no reload,
no restart.** If you're tracking with git:

```bash
git add -A && git commit -m "post: my new post"
```

The single discipline to remember: **a change isn't live until `zola build` runs.** Editing
a file alone does nothing, because nginx serves `public/`, not your sources. Sections 8 and 9
remove that footgun if you want.

---

## 8. Run the scan + rebuild on the server (the robust loop)

This is the piece that makes server-side editing hands-off: a timer rescans the vault's
frontmatter into `data/*.json` every few minutes, and the site rebuilds **only when that data
actually changed**. It maps one-to-one onto the workflow — *rescan periodically, rebuild on
change* — and it's pull-based, so it can't miss an edit the way an event watcher can.

```
vault frontmatter ──(timer)──▶ zk2data ──▶ data/*.json ──(if changed)──▶ zola build ──▶ public/
```

This section assumes the vault is present on this server (however you sync it — Syncthing,
`git pull`, rsync, or editing in place). The loop just reads whatever files are on disk.

### 8.1 Build the scanner once

`zk2data` is a small Rust program in `tools/zk2data/`. Build it once on the server — it always
compiles for this box's own architecture, so it's the safe choice on ARM or x86 alike.

```bash
# A current Rust toolchain. Debian's apt `cargo` is often too old for the tool's deps
# (clap 4 / pulldown-cmark 0.13), so rustup is the reliable choice:
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
. "$HOME/.cargo/env"
# (Only if your distro's package is recent — Debian trixie+, Fedora — apt/dnf works too:
#   sudo apt install cargo   #  needs rustc ≥ 1.74)

cd /var/www/quasioptimal.io
cargo build --release --manifest-path tools/zk2data/Cargo.toml
./tools/zk2data/target/release/zk2data --vault "$QUASIOPTIMAL_VAULT" --out data   # smoke test
```

The binary lands at `tools/zk2data/target/release/zk2data`. Rebuild it (same command) only if
you change the tool's source. `target/` is git-ignored, so it never deploys anywhere.

### 8.2 The refresh script

`scripts/refresh` (tracked in the repo) is the whole loop in one idempotent command:

1. rescan the vault into `data/*.json`;
2. **leak-guard** — refuse to go further if a note-body sentinel ever shows up in `data/` or
   `public/` (defence in depth around zk2data's own structural guarantee);
3. run `zola build` **only if** the data changed (it sha-compares `data/*.json` before/after).

Force a refresh by hand any time:

```bash
QUASIOPTIMAL_VAULT=/path/to/vault scripts/refresh
```

It reads three environment variables, all optional except the vault:

| variable             | default                                       | meaning                   |
|----------------------|-----------------------------------------------|---------------------------|
| `QUASIOPTIMAL_VAULT` | — (required)                                  | the vault to scan         |
| `ZOLA`               | `zola` on `PATH`, else `/usr/local/bin/zola`  | the Zola binary           |
| `ZK2DATA`            | `tools/zk2data/target/release/zk2data`        | the scanner built in 8.1  |

Because it rebuilds only on a real change, running it every few minutes is cheap and quiet.

### 8.3 Run it on a timer

Two unit files turn `scripts/refresh` into a periodic job. Replace `deploy` with the user that
owns `/var/www/quasioptimal.io`, and set the vault path.

`/etc/systemd/system/quasioptimal-refresh.service`
```ini
[Unit]
Description=Rescan the zettelkasten and rebuild quasioptimal.io on change

[Service]
Type=oneshot
User=deploy
WorkingDirectory=/var/www/quasioptimal.io
Environment=QUASIOPTIMAL_VAULT=/home/deploy/vault
Environment=ZOLA=/usr/local/bin/zola
ExecStart=/var/www/quasioptimal.io/scripts/refresh
Nice=10
```

`/etc/systemd/system/quasioptimal-refresh.timer`
```ini
[Unit]
Description=Rescan the zettelkasten every few minutes

[Timer]
OnBootSec=2min
OnUnitActiveSec=5min      # 5 minutes after each run finishes; tune to taste
Persistent=true           # catch up a missed run after downtime

[Install]
WantedBy=timers.target
```

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now quasioptimal-refresh.timer
systemctl list-timers quasioptimal-refresh.timer     # confirm it's scheduled
journalctl -u quasioptimal-refresh.service -f         # watch a run live
```

A `oneshot` service can't overlap itself: if a scan is still running when the timer fires
again, systemd just queues the next one. And because this service is the only thing that writes
`data/` and builds, there's no race with the optional watcher below.

### 8.4 Optional: instant rebuilds when you SSH-edit content

The timer covers vault edits. If you also edit posts or templates directly on the server and
want them live immediately (rather than at the next `zola build` you run by hand), add a path
watcher for *those* sources. Note it deliberately does **not** watch `data/`, which §8.3 owns:

`/etc/systemd/system/zola-build.service`
```ini
[Unit]
Description=Rebuild the Zola site

[Service]
Type=oneshot
WorkingDirectory=/var/www/quasioptimal.io
ExecStart=/usr/local/bin/zola build
```

`/etc/systemd/system/zola-build.path`
```ini
[Unit]
Description=Watch site sources and rebuild

[Path]
PathChanged=/var/www/quasioptimal.io/content
PathChanged=/var/www/quasioptimal.io/templates
PathChanged=/var/www/quasioptimal.io/static
PathChanged=/var/www/quasioptimal.io/config.toml
Unit=zola-build.service

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now zola-build.path
```

A systemd `.path` unit fires on changes to the *named directory's* own entries; for changes
deep in subtrees, an `inotifywait` loop or `entr` is more thorough:

```bash
while inotifywait -r -e modify,create,delete,move \
    content templates static config.toml; do
  zola build
done
```

---

## 9. Optional: git push-to-deploy

If you'd rather edit locally and `git push` to publish, use a bare repo with a
`post-receive` hook that checks out and builds.

```bash
# on the server, once:
git init --bare /srv/quasioptimal.io.git
```

`/srv/quasioptimal.io.git/hooks/post-receive` (make it executable, `chmod +x`):
```bash
#!/usr/bin/env bash
set -euo pipefail
WORKTREE=/var/www/quasioptimal.io
git --work-tree="$WORKTREE" --git-dir=/srv/quasioptimal.io.git checkout -f main
cd "$WORKTREE"
/usr/local/bin/zola build
echo "Built and published $(date)"
```

Then locally:
```bash
git remote add live <user>@quasioptimal.io:/srv/quasioptimal.io.git
git push live main      # builds and goes live on the server
```

This and §8 are mutually exclusive in spirit — pick the one matching how you like to work.

---

## 10. Preview before publishing

`zola serve` runs a live-reload dev server (default `127.0.0.1:1111`) that rebuilds on every
keystroke-save — it does **not** touch `public/`, so it's safe to run alongside the live site.

Easiest is an SSH tunnel from your laptop so the preview never faces the internet:

```bash
# on your laptop
ssh -L 1111:127.0.0.1:1111 <user>@quasioptimal.io
# then, in that SSH session, on the server:
cd /var/www/quasioptimal.io && zola serve
# open http://127.0.0.1:1111 in your laptop browser
```

When it looks right, stop `zola serve` (Ctrl-C) and run `zola build` to publish.

`make serve` and `make build` are shortcuts for these (see the `Makefile`).

---

## 11. Cutover from the old flat HTML

The repo still contains the original hand-written pages so you can diff/compare. Zola
**ignores** them (it only reads `config.toml`, `content/`, `templates/`, `static/`, `data/`,
`sass/`, `themes/`), so they don't affect the build. Once you've confirmed the Zola output
looks right, delete the legacy files to avoid confusion:

```bash
cd /var/www/quasioptimal.io
git rm index.html bibliography.html commonplace.html style.css
git rm -r posts/ scaling-the-good/ js/
git commit -m "refactor: remove pre-Zola flat HTML after cutover"
```

Keep `data/` — Zola reads it. The canonical stylesheet is now `static/style.css`; the old
root `style.css` is the one being removed above. (`static/js/commonplace.js` replaces the old
`js/commonplace.js`; the old `js/bibliography.js` is gone entirely because the bibliography is
rendered server-side now.)

Old URLs (`/posts/scaling-the-good.html`, `/scaling-the-good/`) keep working — the post's
frontmatter declares them as `aliases`, so Zola emits redirect stubs to the new
`/posts/scaling-the-good/`.

---

## 12. Adding content

**A new post** — create `content/posts/<slug>.md`:
```markdown
+++
title = "My Title"
date = 2026-06-15
template = "post.html"
+++

Write in Markdown. Raw HTML is allowed (that's how the first post keeps its custom
footnote popovers and KaTeX). For math, the KaTeX delimiters \(inline\) and \[display\]
work because the post template loads KaTeX.
```
It appears on the home page automatically — `index.html` loops the posts section, newest
first. No list to hand-edit.

**Sources and commonplace quotes are GENERATED, not hand-edited.** `data/bibliography.json`
and `data/commonplace.json` are produced by the `zk2data` tool (`tools/zk2data/`) from the
**YAML frontmatter** of a private Obsidian vault. The note **body is never read** — only an
explicit allowlist of frontmatter fields is published, so nothing private can leak. See
`tools/zk2data/README.md` for the full safety model.

A note opts in by carrying a `public-*` field — presence is necessary and sufficient:

- **A source** — any note with a `public-note` field (the blurb explaining why it's listed):
  ```yaml
  ---
  title: "Categories for the Working Mathematician"
  author: "Mac Lane, Saunders"   # "Last, First": surname shown, sorts by surname
  year: 1978
  tags: [book]                   # book → italic title; anything else is quoted
  topic: "Mathematics"           # groups entries; default "Uncategorized"
  public-link: "https://…"       # optional → links the title
  public-review: my-slug         # optional → book-and-pencil icon → /reviews/my-slug/ (or a full URL)
  public-note: |
    Why it's here. **Markdown** and multiple paragraphs are supported.
  ---
  Private reading notes below — never published.
  ```

  Longer reviews are public, blog-style pages you author in `content/reviews/<slug>.md` (with
  `template = "review.html"`); `public-review` just links to one. See `tools/zk2data/README.md`.

- **A commonplace quote** — any note with a `public-quote` field:
  ```yaml
  ---
  public-quote: |
    The quote, which **may** span multiple paragraphs with *inline* emphasis or
    [links](https://…).
  author: "…"          # optional
  source: "…"          # optional → rendered in <cite>
  public-link: "…"     # optional → links the source
  ---
  Private gloss below — never published.
  ```

Use a YAML **literal block scalar** (`|`) for the long-form fields: it's verbatim (no quoting
pitfalls) and a blank line becomes a paragraph. The tool renders Markdown with the same engine
Zola uses.

Then regenerate and rebuild. On a laptop that holds the vault, run this and commit the JSON
(the server then only needs the committed JSON, never the vault):
```bash
make data VAULT=/path/to/vault      # or set QUASIOPTIMAL_VAULT
make verify-no-leak                 # sanity check
zola build                          # or rely on §8/§9
git add -A && git commit -m "data: refresh from notes"
```
If instead the vault lives on the server, you don't do any of this by hand: §8's timer rescans
and rebuilds on change. Adding entries never touches a template. (A new post is still added by
hand as above.)

---

## 13. Troubleshooting

**`zola: command not found`** — `/usr/local/bin` isn't on the (possibly non-login) shell's
PATH. Use the absolute path in hooks/units (`/usr/local/bin/zola`), as the examples do.

**Build error: `data file not found`** — `load_data` paths are relative to the project root.
Run `zola build` from `/var/www/quasioptimal.io`, and confirm `data/*.json` exist and are
valid JSON (`zola build` prints the offending file and a parse position).

**Build error about a missing template / macro** — template names in frontmatter
(`template = "post.html"`) and `{% import "macros.html" %}` are resolved inside `templates/`.
Check spelling and that the file exists.

**Clean URLs 404** — the `try_files $uri $uri/ =404;` line is what resolves `/bibliography/`
to its `index.html`. Confirm it's present and that `root` points at `public/`, not the repo
root.

**Page shows but CSS is missing** — `static/style.css` becomes `/style.css`. Confirm it
built (`ls public/style.css`) and that the nginx `root` is `public/`. Hard-refresh; the asset
`location` sets a 7-day cache.

**KaTeX/highlight.js not rendering** — they load from CDNs in `templates/post.html`. Check
the browser console for blocked requests. If you later add a `Content-Security-Policy`, it
must allow `cdn.jsdelivr.net` and `cdnjs.cloudflare.com` for `script-src`/`style-src`, plus
`'unsafe-inline'` for the inline init script — otherwise math/highlighting silently break.

**Edit didn't show up** — you didn't rebuild, or `zola serve` is showing a different view
than `public/`. Run `zola build`. Remember nginx serves `public/`, full stop.

**Permission denied in nginx error log** — nginx's user can't traverse a parent dir. Ensure
`o+x` on `/var/www` and `/var/www/quasioptimal.io`, and `a+rX` on `public/`.

---

## 14. Quick reference

```bash
# build / preview
zola build                     # regenerate public/  (the "deploy")
zola serve                     # live-reload preview on :1111 (doesn't touch public/)
zola check                     # validate links/structure
make build | make serve        # the same, via Makefile

# nginx
sudo nginx -t                  # validate config
sudo systemctl reload nginx    # apply config changes (NOT needed after zola build)
sudo tail -f /var/log/nginx/error.log

# TLS
sudo certbot --nginx -d quasioptimal.io -d www.quasioptimal.io
sudo certbot renew --dry-run

# day-to-day
cd /var/www/quasioptimal.io && vi content/posts/<slug>.md && zola build
```

Paths used in this guide: project root `/var/www/quasioptimal.io`, web root
`/var/www/quasioptimal.io/public`, Zola binary `/usr/local/bin/zola`.
```
