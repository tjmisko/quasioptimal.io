# Convenience targets for the Zola site. Run `make help` for a list.

.PHONY: help serve build clean data verify-no-leak

help:
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN{FS=":.*?## "}{printf "  %-14s %s\n", $$1, $$2}'

serve: ## Live preview with auto-reload (http://127.0.0.1:1111)
	zola serve

build: ## Build the static site into public/
	zola build

# Regenerate data/*.json from the private zettelkasten. Pass the vault path as VAULT=... or set
# the QUASIOPTIMAL_VAULT environment variable. Only frontmatter is read; note bodies never leak.
# Run this on the machine that has the vault, then commit the JSON — the server only needs that.
data: ## Regenerate data/*.json from the zettelkasten (VAULT=path or $QUASIOPTIMAL_VAULT)
	cargo run --manifest-path tools/zk2data/Cargo.toml --release -- $(if $(VAULT),--vault $(VAULT)) --out data

verify-no-leak: ## Fail if a note-body sentinel ever appears in data/ or public/
	@if grep -rIl "BODY_SENTINEL_DO_NOT_PUBLISH" data public 2>/dev/null; then \
		echo "LEAK: body sentinel found in the files listed above"; exit 1; \
	else \
		echo "ok: no body sentinel in data/ or public/"; \
	fi

clean: ## Remove the build output
	rm -rf public
