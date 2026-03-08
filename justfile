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

# Final release build with UPX compression.
# Use `just build-uncompressed` if you need an artifact without UPX packing.
build: gen-icons
  bun run tauri build
  upx --best --lzma "src-tauri/target/release/Zapret Interactive.exe"

# Final release build without UPX compression
build-uncompressed: gen-icons
  bun run tauri build

# Lint only backend
lint-back:
  cargo clippy --manifest-path "src-tauri/Cargo.toml"

# Lint only frontend
lint-front:
  bun run typecheck
  bun run lint

# Lint both backend and frontend
lint: lint-back lint-front
  opengrep scan

# Run opengrep standalone
scan:
  opengrep scan

# Format only backend
format-back:
  cargo fmt --manifest-path "src-tauri/Cargo.toml"

# Format only frontend
format-front:
  bun run format

# Format both backend and frontend
format: format-back format-front
