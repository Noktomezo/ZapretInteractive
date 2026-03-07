set windows-shell := ["powershell", "-NoLogo", "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command"]

# List all available recipes
_default:
  @just --list

# Generate icons from SVG
gen-icons:
  bun tauri icon "assets\\app-logo.svg"

# Run in dev mode with hot reload
dev:
	bun run tauri dev

# Final release build
build: gen-icons
  bun run tauri build
  upx --best --lzma "src-tauri\\target\\release\\Zapret Interactive.exe"

# Lint only backend
lint-back:
  cargo check --manifest-path "src-tauri\\Cargo.toml"
  cargo clippy --manifest-path "src-tauri\\Cargo.toml"

# Lint only frontend
lint-front:
  bun run typecheck
  bun run lint

# Lint both backend and frontend
lint: lint-back lint-front
  opengrep scan

# Format only backend
format-back:
  cargo fmt --manifest-path "src-tauri\\Cargo.toml"
  cargo clippy --manifest-path "src-tauri\\Cargo.toml" --fix --allow-dirty

# Format only frontend
format-front:
  bun run format

# Format both backend and frontend
format: format-back format-front
  opengrep scan
