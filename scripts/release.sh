#!/bin/bash

set -e

TAURI_CONF="src-tauri/tauri.conf.json"
CARGO_TOML="src-tauri/Cargo.toml"
PACKAGE_JSON="package.json"
BRANCH="main"

get_current_version() {
  grep -oP '"version":\s*"\K[^"]+' "$TAURI_CONF"
}

bump_version() {
  local version=$1
  local part=$2

  IFS='.' read -r major minor patch <<<"$version"

  case $part in
  major)
    major=$((major + 1))
    minor=0
    patch=0
    ;;
  minor)
    minor=$((minor + 1))
    patch=0
    ;;
  patch)
    patch=$((patch + 1))
    ;;
  esac

  echo "$major.$minor.$patch"
}

update_files() {
  local new_version=$1

  sed -i "s/\"version\": \"[^\"]*\"/\"version\": \"$new_version\"/" "$TAURI_CONF"
  sed -i "s/^version = \"[^\"]*\"/version = \"$new_version\"/" "$CARGO_TOML"
  sed -i "s/\"version\": \"[^\"]*\"/\"version\": \"$new_version\"/" "$PACKAGE_JSON"
}

current=$(get_current_version)
echo "Current version: $current"
echo ""
echo "Select bump type:"
echo "  1) patch ($current -> $(bump_version $current patch))"
echo "  2) minor ($current -> $(bump_version $current minor))"
echo "  3) major ($current -> $(bump_version $current major))"
echo "  4) custom"
echo ""
read -p "Choice [1-4]: " choice

case $choice in
1) new_version=$(bump_version $current patch) ;;
2) new_version=$(bump_version $current minor) ;;
3) new_version=$(bump_version $current major) ;;
4)
  read -p "Enter new version: " new_version
  if [[ ! $new_version =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "Invalid version format. Use x.y.z"
    exit 1
  fi
  ;;
*)
  echo "Invalid choice"
  exit 1
  ;;
esac

echo ""
echo "Updating to $new_version..."
update_files "$new_version"

echo ""
echo "Committing and tagging..."
git add "$TAURI_CONF" "$CARGO_TOML" "$PACKAGE_JSON"
git commit -m "chore: release v$new_version"
git tag "v$new_version"

echo ""
echo "Pushing to origin..."
git push --set-upstream origin "$BRANCH"
git push origin "v$new_version"

echo ""
echo "Done! Released v$new_version"
