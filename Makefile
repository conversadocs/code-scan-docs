# CSD Docker Makefile

# Configuration
IMAGE_NAME ?= conversadocs/csd
TAG ?= latest
REGISTRY ?= docker.io
FULL_IMAGE = $(REGISTRY)/$(IMAGE_NAME):$(TAG)

# Get version from Cargo.toml
VERSION := $(shell grep '^version = ' Cargo.toml | sed 's/version = "\(.*\)"/\1/')

# Directories
CODE_DIR ?= $(CURDIR)
OUTPUT_DIR ?= $(CURDIR)/csd-output
CACHE_DIR ?= $(CURDIR)/.csd_cache

.PHONY: help build push pull run clean test lint

# Default target
help: ## Show this help message
	@echo "CSD Docker Makefile"
	@echo "==================="
	@echo ""
	@echo "Available targets:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'
	@echo ""
	@echo "Configuration:"
	@echo "  IMAGE_NAME = $(IMAGE_NAME)"
	@echo "  TAG        = $(TAG)"
	@echo "  REGISTRY   = $(REGISTRY)"
	@echo "  VERSION    = $(VERSION)"

build: ## Build the Docker image
	@echo "🔨 Building CSD Docker image..."
	docker build \
		--tag $(FULL_IMAGE) \
		--tag $(REGISTRY)/$(IMAGE_NAME):$(VERSION) \
		--tag $(REGISTRY)/$(IMAGE_NAME):latest \
		--label "org.opencontainers.image.created=$(shell date -u +'%Y-%m-%dT%H:%M:%SZ')" \
		--label "org.opencontainers.image.version=$(VERSION)" \
		--label "org.opencontainers.image.title=Code-Scan-Docs" \
		--label "org.opencontainers.image.description=A comprehensive code analysis and documentation tool" \
		--label "org.opencontainers.image.source=https://github.com/conversadocs/csd" \
		--label "org.opencontainers.image.licenses=MIT" \
		.
	@echo "✅ Build completed: $(FULL_IMAGE)"

build-multiarch: ## Build multi-architecture image
	@echo "🔨 Building multi-architecture CSD image..."
	docker buildx build \
		--platform linux/amd64,linux/arm64 \
		--tag $(FULL_IMAGE) \
		--tag $(REGISTRY)/$(IMAGE_NAME):$(VERSION) \
		--label "org.opencontainers.image.created=$(shell date -u +'%Y-%m-%dT%H:%M:%SZ')" \
		--label "org.opencontainers.image.version=$(VERSION)" \
		--label "org.opencontainers.image.title=Code-Scan-Docs" \
		--label "org.opencontainers.image.description=A comprehensive code analysis and documentation tool" \
		--label "org.opencontainers.image.source=https://github.com/conversadocs/csd" \
		--label "org.opencontainers.image.licenses=MIT" \
		--push \
		.

push: ## Push the image to registry
	@echo "📤 Pushing $(FULL_IMAGE) to registry..."
	docker push $(REGISTRY)/$(IMAGE_NAME):$(VERSION)
	docker push $(REGISTRY)/$(IMAGE_NAME):latest
	@echo "✅ Push completed"

pull: ## Pull the latest image from registry
	@echo "📥 Pulling $(FULL_IMAGE)..."
	docker pull $(FULL_IMAGE)

run: ## Run CSD with current directory (usage: make run ARGS="init")
	@mkdir -p $(OUTPUT_DIR) $(CACHE_DIR)
	@echo "🚀 Running CSD..."
	@echo "📁 Code: $(CODE_DIR)"
	@echo "📤 Output: $(OUTPUT_DIR)"
	@echo "💾 Cache: $(CACHE_DIR)"
	docker run --rm \
		-v $(CODE_DIR):/app/code:ro \
		-v $(OUTPUT_DIR):/app/.csd_output \
		-v $(CACHE_DIR):/app/.csd_cache \
		-e RUST_LOG=info \
		$(FULL_IMAGE) $(ARGS)

run-init: ## Run CSD init command
	$(MAKE) run ARGS="init"

run-docs: ## Run CSD docs command
	$(MAKE) run ARGS="docs --format markdown"

run-plugins: ## List available plugins
	$(MAKE) run ARGS="plugins --detailed"

run-shell: ## Start interactive shell in container
	@mkdir -p $(OUTPUT_DIR) $(CACHE_DIR)
	docker run -it --rm \
		-v $(CODE_DIR):/app/code \
		-v $(OUTPUT_DIR):/app/.csd_output \
		-v $(CACHE_DIR):/app/.csd_cache \
		-e RUST_LOG=debug \
		$(FULL_IMAGE) /bin/bash

test: ## Test the Docker image
	@echo "🧪 Testing CSD Docker image..."
	docker run --rm $(FULL_IMAGE) --version
	docker run --rm $(FULL_IMAGE) --help
	@echo "✅ Basic tests passed"

lint: ## Lint the Dockerfile
	@echo "🔍 Linting Dockerfile..."
	@if command -v hadolint >/dev/null 2>&1; then \
		hadolint Dockerfile; \
	else \
		echo "⚠️  hadolint not installed, skipping lint"; \
		echo "   Install with: brew install hadolint"; \
	fi

clean: ## Clean up Docker artifacts
	@echo "🧹 Cleaning up Docker artifacts..."
	docker system prune -f
	docker volume prune -f
	@echo "✅ Cleanup completed"

clean-images: ## Remove CSD Docker images
	@echo "🗑️  Removing CSD Docker images..."
	-docker rmi $(REGISTRY)/$(IMAGE_NAME):latest
	-docker rmi $(REGISTRY)/$(IMAGE_NAME):$(VERSION)
	@echo "✅ Images removed"

inspect: ## Inspect the Docker image
	@echo "🔍 Inspecting $(FULL_IMAGE)..."
	docker inspect $(FULL_IMAGE)

size: ## Show image size information
	@echo "📏 Image size information:"
	docker images $(REGISTRY)/$(IMAGE_NAME) --format "table {{.Repository}}\t{{.Tag}}\t{{.Size}}\t{{.CreatedAt}}"

logs: ## Show logs from last container run
	docker logs $(shell docker ps -lq)

# Composition targets
compose-up: ## Start services with docker-compose
	docker-compose up --build

compose-down: ## Stop services with docker-compose
	docker-compose down

compose-dev: ## Start development shell with docker-compose
	docker-compose run --rm csd-dev

# Release targets
release-check: ## Check if ready for release
	@echo "🔍 Checking release readiness..."
	@git status --porcelain | wc -l | grep -q '^0$$' || (echo "❌ Working directory not clean" && exit 1)
	@git tag -l | grep -q "^v$(VERSION)$$" && (echo "❌ Tag v$(VERSION) already exists" && exit 1) || true
	@echo "✅ Ready for release $(VERSION)"

release: release-check build ## Build and tag a release
	@echo "🚀 Creating release $(VERSION)..."
	git tag -a "v$(VERSION)" -m "Release version $(VERSION)"
	$(MAKE) push
	@echo "✅ Release $(VERSION) completed"
	@echo "🏷️  Tagged: v$(VERSION)"
	@echo "📤 Pushed: $(REGISTRY)/$(IMAGE_NAME):$(VERSION)"
