# Charybdis - ScyllaDB benchmarking library
# Wraps upstream latte with a clean API

SHELL := /bin/bash

# Paths
LIB_DIR := lib
INCLUDE_DIR := include

# Detect platform
UNAME_S := $(shell uname -s)
UNAME_M := $(shell uname -m)

ifeq ($(UNAME_S),Darwin)
    ifeq ($(UNAME_M),arm64)
        PLATFORM := darwin_arm64
    else
        PLATFORM := darwin_amd64
    endif
else ifeq ($(UNAME_S),Linux)
    PLATFORM := linux_amd64
endif

# Output library path
LIB_OUTPUT := $(LIB_DIR)/$(PLATFORM)/libcharybdis.a

# Rust source files that should trigger a rebuild
RUST_SOURCES := $(shell find adapter ffi -name '*.rs' 2>/dev/null)
CARGO_TOMLS := Cargo.toml adapter/Cargo.toml ffi/Cargo.toml

.PHONY: all submodule setup build clean test help

.DEFAULT_GOAL := help

all: submodule setup $(LIB_OUTPUT) ## Init submodule, setup upstream, and build library

submodule: ## Initialize upstream latte submodule
	@if [ ! -f upstream/Cargo.toml ]; then \
		echo "[charybdis] initializing upstream submodule..."; \
		cd ../.. && git submodule update --init libs/charybdis/upstream; \
	fi

setup: submodule ## Transform upstream latte into latte-core
	@./scripts/setup-upstream.sh

# Real file target - only rebuilds when sources change
$(LIB_OUTPUT): $(RUST_SOURCES) $(CARGO_TOMLS) | setup
	@echo "[charybdis] rebuilding (sources changed)..."
	@mkdir -p $(LIB_DIR)/$(PLATFORM) $(INCLUDE_DIR)
	cargo build --release -p charybdis-ffi
	@cp target/release/libcharybdis_ffi.a $(LIB_OUTPUT)
	@echo "[charybdis] built: $(LIB_OUTPUT)"

# Phony target for backwards compatibility
build: $(LIB_OUTPUT) ## Build charybdis-ffi static library

clean: ## Clean all build artifacts
	rm -rf build/latte-core
	rm -rf target
	rm -f $(LIB_DIR)/*/libcharybdis.a
	rm -f $(INCLUDE_DIR)/charybdis.h

test: ## Run tests
	cargo test --workspace

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'
