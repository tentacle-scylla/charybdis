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

# Rust target triples for cross-compilation
TARGET_LINUX_AMD64 := x86_64-unknown-linux-gnu
TARGET_LINUX_ARM64 := aarch64-unknown-linux-gnu
TARGET_DARWIN_AMD64 := x86_64-apple-darwin
TARGET_DARWIN_ARM64 := aarch64-apple-darwin
TARGET_WINDOWS_AMD64 := x86_64-pc-windows-msvc

.PHONY: all submodule setup build clean test help build-all fmt lint release
.PHONY: build-linux-amd64 build-linux-arm64 build-darwin-amd64 build-darwin-arm64

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

build-linux-amd64: setup ## Build for Linux AMD64
	@echo "[charybdis] building for linux_amd64..."
	@mkdir -p $(LIB_DIR)/linux_amd64
	cargo build --release --target $(TARGET_LINUX_AMD64) -p charybdis-ffi
	@cp target/$(TARGET_LINUX_AMD64)/release/libcharybdis_ffi.a $(LIB_DIR)/linux_amd64/libcharybdis.a
	@echo "[charybdis] built: $(LIB_DIR)/linux_amd64/libcharybdis.a"

build-linux-arm64: setup ## Build for Linux ARM64
	@echo "[charybdis] building for linux_arm64..."
	@mkdir -p $(LIB_DIR)/linux_arm64
	cargo build --release --target $(TARGET_LINUX_ARM64) -p charybdis-ffi
	@cp target/$(TARGET_LINUX_ARM64)/release/libcharybdis_ffi.a $(LIB_DIR)/linux_arm64/libcharybdis.a
	@echo "[charybdis] built: $(LIB_DIR)/linux_arm64/libcharybdis.a"

build-darwin-amd64: setup ## Build for macOS AMD64 (Intel)
	@echo "[charybdis] building for darwin_amd64..."
	@mkdir -p $(LIB_DIR)/darwin_amd64
	cargo build --release --target $(TARGET_DARWIN_AMD64) -p charybdis-ffi
	@cp target/$(TARGET_DARWIN_AMD64)/release/libcharybdis_ffi.a $(LIB_DIR)/darwin_amd64/libcharybdis.a
	@echo "[charybdis] built: $(LIB_DIR)/darwin_amd64/libcharybdis.a"

build-darwin-arm64: setup ## Build for macOS ARM64 (Apple Silicon)
	@echo "[charybdis] building for darwin_arm64..."
	@mkdir -p $(LIB_DIR)/darwin_arm64
	cargo build --release --target $(TARGET_DARWIN_ARM64) -p charybdis-ffi
	@cp target/$(TARGET_DARWIN_ARM64)/release/libcharybdis_ffi.a $(LIB_DIR)/darwin_arm64/libcharybdis.a
	@echo "[charybdis] built: $(LIB_DIR)/darwin_arm64/libcharybdis.a"

# Windows builds disabled - not currently supported
# build-windows-amd64: setup ## Build for Windows AMD64
# 	@echo "[charybdis] building for windows_amd64..."
# 	@mkdir -p $(LIB_DIR)/windows_amd64
# 	cargo build --release --target $(TARGET_WINDOWS_AMD64) -p charybdis-ffi
# 	@cp target/$(TARGET_WINDOWS_AMD64)/release/libcharybdis_ffi.a $(LIB_DIR)/windows_amd64/libcharybdis.a
# 	@echo "[charybdis] built: $(LIB_DIR)/windows_amd64/libcharybdis.a"

build-all: build-linux-amd64 build-darwin-amd64 build-darwin-arm64 ## Build for all common platforms

clean: ## Clean all build artifacts
	rm -rf build/latte-core
	rm -rf target
	rm -f $(LIB_DIR)/*/libcharybdis.a
	rm -f $(INCLUDE_DIR)/charybdis.h

test: ## Run tests
	cargo test --workspace

fmt: ## Format Rust code
	cargo fmt --all

lint: ## Run Clippy linter
	cargo clippy --workspace --all-targets -- -D warnings

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'

# Release target - runs all validation before creating a release
# Usage: make release VERSION=v0.1.0
release: fmt lint test
	@echo "All checks passed!"
	@if [ -z "$(VERSION)" ]; then \
		echo "Error: VERSION not specified. Usage: make release VERSION=v0.1.0"; \
		exit 1; \
	fi
	@echo "Creating release $(VERSION)..."
	@if git diff --quiet; then \
		echo "Working directory is clean"; \
	else \
		echo "Error: Working directory has uncommitted changes."; \
		exit 1; \
	fi
	git push origin main
	gh release create $(VERSION) --generate-notes --latest
	@echo "Release $(VERSION) created and published!"
