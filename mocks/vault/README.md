This is a **dummy** Obsidian vault used as test input and a living example for `zk2data`.
All content here is fabricated and safe to commit.

Every note body below the frontmatter contains the sentinel string
`BODY_SENTINEL_DO_NOT_PUBLISH`. The leak-guard test asserts that this string never appears in
the generated JSON — proving that no note body can reach the published data.

BODY_SENTINEL_DO_NOT_PUBLISH (this README has no frontmatter, so the tool skips it entirely.)
