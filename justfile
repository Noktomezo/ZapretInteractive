default: check

# Run with auto-reload on source and localization changes
dev:
    CARGO_TARGET_DIR=target/dev CARGO_INCREMENTAL=0 cargo run --package xtask -- patch-gpui
    CARGO_TARGET_DIR=target/dev CARGO_INCREMENTAL=0 watchexec -r -e rs,hlsl,yml,yaml -- cargo run

# Apply the pinned GPUI D3D11 source patch idempotently
patch-gpui:
    cargo run --package xtask -- patch-gpui

# Build optimized release binary and compress with UPX via xtask
build:
    cargo run --package xtask --release -- build

# Run cargo check across all targets
check: patch-gpui
    cargo check --all-targets

# Run unit and integration tests
test: patch-gpui
    cargo test --all-targets

# Run strict Clippy checks
clippy: patch-gpui
    cargo clippy --all-targets -- -D warnings

# Check formatting
fmt:
    cargo fmt --all --check

# Full strict verification
strict: check test clippy fmt

# Refresh managed thirdparty files and their hashes from upstream
update-thirdparty:
    uv run scripts/update-thirdparty.py

# Verify that thirdparty/hashes.json matches the managed files
verify-thirdparty:
    uv run scripts/verify-thirdparty-hashes.py

# Clean build artifacts
clean:
    cargo clean
