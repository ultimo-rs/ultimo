.PHONY: help test coverage clean build

help: ## Show this help message
	@echo "Ultimo Framework - Development Commands"
	@echo "========================================"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

build: ## Build the project
	cargo build --release

test: ## Run all tests
	cargo test --lib --no-fail-fast

test-watch: ## Run tests in watch mode
	cargo watch -x 'test --lib'

test-summary: ## Show test statistics
	@./scripts/test-summary.sh

coverage: ## Generate coverage report
	@cargo run --release -p ultimo-coverage

coverage-open: coverage ## Generate and open coverage report
	@open target/coverage/html/index.html || xdg-open target/coverage/html/index.html

check: ## Run clippy and fmt checks
	cargo clippy --all-targets --all-features
	cargo fmt --all -- --check

fmt: ## Format code
	cargo fmt --all

clean: ## Clean build artifacts
	cargo clean
	rm -rf target/coverage
	rm -f coverage.json

doc: ## Generate and open documentation
	cargo doc --no-deps --open

install-tools: ## Install development tools
	cargo install cargo-watch
	rustup component add llvm-tools-preview

ci: check test coverage ## Run CI checks locally

.DEFAULT_GOAL := help
