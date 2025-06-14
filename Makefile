# API Gateway Service Makefile

SERVICE_NAME := api-gateway
DOCKER_REGISTRY ?= syla
VERSION ?= $(shell git describe --always --dirty 2>/dev/null || echo "dev")
DOCKER_TAG := $(DOCKER_REGISTRY)/$(SERVICE_NAME):$(VERSION)
DOCKER_TAG_LATEST := $(DOCKER_REGISTRY)/$(SERVICE_NAME):latest

.PHONY: help
help:
	@echo "Available targets for $(SERVICE_NAME):"
	@echo "  setup          - Setup proto dependencies"
	@echo "  build          - Build the service"
	@echo "  test           - Run tests"
	@echo "  run            - Run the service locally"
	@echo "  docker-build   - Build Docker image"
	@echo "  docker-push    - Push Docker image"
	@echo "  clean          - Clean build artifacts"
	@echo "  watch          - Run with auto-reload (requires cargo-watch)"

.PHONY: setup
setup:
	@echo "Setting up $(SERVICE_NAME)..."
	@../../../../scripts/setup-service.sh platforms/syla/core/api-gateway

.PHONY: build
build: setup
	@echo "Building $(SERVICE_NAME)..."
	@cargo build --release

.PHONY: test
test: setup
	@echo "Testing $(SERVICE_NAME)..."
	@cargo test

.PHONY: run
run: build
	@echo "Running $(SERVICE_NAME)..."
	@RUST_LOG=debug cargo run --release

.PHONY: docker-build
docker-build: setup
	@echo "Building Docker image $(DOCKER_TAG)..."
	@docker build \
		--build-arg VERSION=$(VERSION) \
		--build-arg SERVICE_NAME=$(SERVICE_NAME) \
		-t $(DOCKER_TAG) \
		-t $(DOCKER_TAG_LATEST) \
		.

.PHONY: docker-push
docker-push:
	@echo "Pushing Docker image $(DOCKER_TAG)..."
	@docker push $(DOCKER_TAG)
	@docker push $(DOCKER_TAG_LATEST)

.PHONY: clean
clean:
	@echo "Cleaning $(SERVICE_NAME)..."
	@cargo clean
	@rm -rf proto-deps
	@rm -f proto/google proto/common

# Development targets
.PHONY: dev
dev: setup
	@echo "Running $(SERVICE_NAME) in development mode..."
	@RUST_LOG=debug cargo run

.PHONY: watch
watch: setup
	@echo "Running $(SERVICE_NAME) with auto-reload..."
	@cargo watch -x 'run'

.PHONY: check
check: setup
	@echo "Checking $(SERVICE_NAME)..."
	@cargo check
	@cargo clippy -- -D warnings
	@cargo fmt -- --check

.PHONY: fmt
fmt:
	@echo "Formatting $(SERVICE_NAME)..."
	@cargo fmt

# Proto-specific targets
.PHONY: proto-clean
proto-clean:
	@echo "Cleaning proto artifacts..."
	@rm -rf target/debug/build/*/out/proto
	@rm -rf target/release/build/*/out/proto

.PHONY: proto-rebuild
proto-rebuild: proto-clean build
	@echo "Proto rebuild complete"