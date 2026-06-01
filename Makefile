# Convenience targets for the Zola site. Run `make help` for a list.

.PHONY: help serve build clean

help:
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN{FS=":.*?## "}{printf "  %-10s %s\n", $$1, $$2}'

serve: ## Live preview with auto-reload (http://127.0.0.1:1111)
	zola serve

build: ## Build the static site into public/
	zola build

clean: ## Remove the build output
	rm -rf public
