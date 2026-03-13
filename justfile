set windows-shell := ["powershell", "-NoLogo", "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command"]

# List all available recipes
_default:
  @just --list

# Generate icons from the same source used in CI
gen-icons:
  bun tauri icon assets/app-logo.png

# Run in dev mode with hot reload
dev:
  bun run tauri dev

# Install developer hooks
bootstrap:
  bun install
  cargo check --manifest-path "src-tauri/Cargo.toml"

# Final release build with UPX compression.
build: gen-icons
  bun tauri build
  upx --best --lzma "src-tauri/target/release/Zapret Interactive.exe"

# Local installer build without updater artifacts/latest.json.
build-local: gen-icons
  bun tauri build --no-sign
  upx --best --lzma "src-tauri/target/release/Zapret Interactive.exe"

# Lint only backend
lint-back:
  cargo clippy --manifest-path "src-tauri/Cargo.toml" --all-targets --all-features -- -D warnings

# Lint only frontend
lint-front:
  bun run typecheck
  bun run lint

# Lint both backend and frontend
lint: lint-back lint-front

# Format only backend
format-back:
  cargo clippy --fix --allow-dirty --manifest-path "src-tauri/Cargo.toml" --all-targets --all-features
  cargo fmt --manifest-path "src-tauri/Cargo.toml"

# Format only frontend
format-front:
  bun run format

# Format both backend and frontend
format: format-back format-front

clean:
  bunx poof dist src-tauri/target
