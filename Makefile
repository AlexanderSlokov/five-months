# five-months — shortcuts for the Rust TUI in tui/.
# `make` alone lists the targets.

CARGO    := cargo
MANIFEST := --manifest-path tui/Cargo.toml
BIN      := tui/target/release/five-months

SEED   ?= naught
X      ?= 0
OUT    ?= five-months-$(SEED).png
SVG    ?= five-months-$(SEED).svg
MARKER ?= braille
ARGS   ?=

.DEFAULT_GOAL := help
.PHONY: help run dev birthday build test fmt lint check png svg timing install clean

help: ## List the targets
	@grep -E '^[a-z-]+:.*## ' $(MAKEFILE_LIST) | awk -F ':.*## ' '{printf "  make %-10s %s\n", $$1, $$2}'
	@echo
	@echo "  variables: SEED=$(SEED) X=$(X) MARKER=$(MARKER) OUT=$(OUT) ARGS='$(ARGS)'"

run: ## Run the scroll (release); SEED=, MARKER=, ARGS= to customise
	$(CARGO) run --release $(MANIFEST) -- --seed $(SEED) --x $(X) --marker $(MARKER) $(ARGS)

dev: ## Run a debug build
	$(CARGO) run $(MANIFEST) -- --seed $(SEED) --x $(X) --marker $(MARKER) $(ARGS)

birthday: ## Preview Naught's birthday version
	$(CARGO) run --release $(MANIFEST) -- --seed $(SEED) --birthday $(ARGS)

build: ## Build the release binary
	$(CARGO) build --release $(MANIFEST)

test: ## Run all tests
	$(CARGO) test $(MANIFEST)

fmt: ## Format the code
	$(CARGO) fmt $(MANIFEST)

lint: ## Clippy with warnings as errors
	$(CARGO) clippy $(MANIFEST) --all-targets -- -D warnings

check: ## fmt check + lint + test (run before committing)
	$(CARGO) fmt $(MANIFEST) -- --check
	$(MAKE) --no-print-directory lint test

png: build ## Render one frame to a PNG (SEED=, X=, OUT=, MARKER=)
	$(BIN) --seed $(SEED) --x $(X) --marker $(MARKER) --png $(OUT) $(ARGS)

svg: build ## Export the view as SVG to compare with the web version (SVG=)
	$(BIN) --seed $(SEED) --x $(X) --svg $(SVG) $(ARGS)

timing: ## Measure chunk generation and frame times
	$(CARGO) run --release $(MANIFEST) --example frame_timing

install: ## Install `five-months` into ~/.cargo/bin
	$(CARGO) install --path tui

clean: ## Remove build artifacts
	$(CARGO) clean $(MANIFEST)
