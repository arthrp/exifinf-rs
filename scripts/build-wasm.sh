#!/usr/bin/env bash
# Build exifinf-wasm packages (web, bundler) and tarball them under dist/.
set -euo pipefail

WASM_PACK_VERSION="v0.13.1"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

BIN_DIR="$ROOT/.bin"
export PATH="$BIN_DIR:$PATH"

version() {
  if [[ -n "${GITHUB_REF_NAME:-}" ]]; then
    echo "${GITHUB_REF_NAME}"
    return
  fi
  grep -E '^version[[:space:]]*=' exifinf-wasm/Cargo.toml | head -1 | sed -E 's/.*"([^"]+)".*/\1/'
}

host_triple() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"
  case "$os" in
    Linux)
      case "$arch" in
        aarch64 | arm64) echo "aarch64-unknown-linux-musl" ;;
        *) echo "x86_64-unknown-linux-musl" ;;
      esac
      ;;
    Darwin)
      case "$arch" in
        arm64 | aarch64)
          # v0.13.1 has no aarch64-apple-darwin asset; x86_64 binary runs under Rosetta.
          echo "x86_64-apple-darwin"
          ;;
        *) echo "x86_64-apple-darwin" ;;
      esac
      ;;
    *)
      echo "unsupported OS: $os" >&2
      exit 1
      ;;
  esac
}

install_wasm_pack() {
  local triple want_version current
  triple="$(host_triple)"
  want_version="${WASM_PACK_VERSION#v}"

  if command -v wasm-pack >/dev/null 2>&1; then
    current="$(wasm-pack --version 2>/dev/null | awk '{print $2}' || true)"
    if [[ "$current" == "$want_version" ]]; then
      return 0
    fi
  fi

  mkdir -p "$BIN_DIR"
  local url tmpdir archive asset
  asset="wasm-pack-${WASM_PACK_VERSION}-${triple}.tar.gz"
  url="https://github.com/wasm-bindgen/wasm-pack/releases/download/${WASM_PACK_VERSION}/${asset}"
  tmpdir="$(mktemp -d)"
  archive="$tmpdir/${asset}"

  echo "Installing wasm-pack ${WASM_PACK_VERSION} (${triple})..."
  curl -fsSL "$url" -o "$archive"
  tar -xzf "$archive" -C "$tmpdir"
  if [[ -f "$tmpdir/wasm-pack" ]]; then
    install -m 755 "$tmpdir/wasm-pack" "$BIN_DIR/wasm-pack"
  else
    install -m 755 "$(find "$tmpdir" -name wasm-pack -type f | head -1)" "$BIN_DIR/wasm-pack"
  fi
  rm -rf "$tmpdir"

  wasm-pack --version
}

rustup target add wasm32-unknown-unknown

install_wasm_pack

VERSION="$(version)"
echo "Building exifinf-wasm version ${VERSION}"

rm -rf pkg dist
mkdir -p dist

for target in web bundler; do
  echo "==> wasm-pack target: ${target}"
  (
    cd exifinf-wasm
    wasm-pack build --release --target "$target" --out-dir "../pkg/${target}" --out-name exifinf
  )
  tar -C pkg -czf "dist/exifinf-wasm-${VERSION}-${target}.tar.gz" "$target"
done

echo "Artifacts:"
ls -la dist/
