#!/usr/bin/env bash
set -euo pipefail

REPO="dawnlabsai/lws"
INSTALL_DIR="${LWS_INSTALL_DIR:-$HOME/.lws/bin}"

info()  { printf '\033[1;34m==>\033[0m %s\n' "$*" >&2; }
warn()  { printf '\033[1;33mwarn:\033[0m %s\n' "$*" >&2; }
err()   { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

TMPDIR=""
REPO_DIR=""
cleanup() {
  if [ -n "$TMPDIR" ] && [ -d "$TMPDIR" ]; then
    rm -rf "$TMPDIR"
  fi
}
trap cleanup EXIT

# --- Clone repo into TMPDIR (idempotent) ---
ensure_repo_cloned() {
  if [ -n "$REPO_DIR" ] && [ -d "$REPO_DIR/.git" ]; then
    return
  fi
  if [ -z "$TMPDIR" ] || [ ! -d "$TMPDIR" ]; then
    TMPDIR="$(mktemp -d)"
  fi
  REPO_DIR="$TMPDIR/repo"
  info "Cloning repository..."
  git clone --depth 1 "https://github.com/${REPO}.git" "$REPO_DIR" --quiet
}

# --- Detect platform ---
detect_platform() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"

  case "$os" in
    Linux)   os="linux" ;;
    Darwin)  os="darwin" ;;
    *)       err "unsupported OS: $os" ;;
  esac

  case "$arch" in
    x86_64|amd64)   arch="x86_64" ;;
    aarch64|arm64)   arch="aarch64" ;;
    *)               err "unsupported architecture: $arch" ;;
  esac

  echo "${os}-${arch}"
}

# --- Try to download prebuilt binary ---
try_download() {
  local platform="$1"
  local release_url="https://github.com/${REPO}/releases/latest/download/lws-${platform}"

  info "Downloading prebuilt binary for ${platform}..."
  TMPDIR="$(mktemp -d)"
  local bin_path="${TMPDIR}/lws"

  if curl -fsSL -o "$bin_path" "$release_url" 2>/dev/null; then
    chmod +x "$bin_path"
    # Verify it's actually an executable
    if file "$bin_path" | grep -qi "executable\|mach-o\|elf"; then
      echo "$bin_path"
      return 0
    fi
  fi

  return 1
}

# --- Build from source (fallback) ---
build_from_source() {
  info "No prebuilt binary available — building from source..."

  # Check prerequisites
  command -v git &>/dev/null || err "git is required. Install git first."

  if command -v rustup &>/dev/null; then
    info "Rust already installed ($(rustc --version))"
  elif command -v rustc &>/dev/null; then
    info "rustc found ($(rustc --version))"
  else
    info "Installing Rust via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --quiet
    export PATH="$HOME/.cargo/bin:$PATH"
  fi

  if ! command -v cargo &>/dev/null; then
    if [ -f "$HOME/.cargo/env" ]; then
      . "$HOME/.cargo/env"
    else
      err "cargo not found. Install Rust: https://rustup.rs"
    fi
  fi

  ensure_repo_cloned

  info "Building lws..."
  cd "$REPO_DIR/lws"
  cargo build --workspace --release

  local bin_path="$REPO_DIR/lws/target/release/lws"
  if [ ! -f "$bin_path" ]; then
    err "Build failed — binary not found"
  fi

  echo "$bin_path"
}

# --- Install binary ---
install_bin() {
  local bin_path="$1"

  mkdir -p "$INSTALL_DIR"
  cp "$bin_path" "$INSTALL_DIR/lws"
  chmod +x "$INSTALL_DIR/lws"

  # Set strict permissions on the vault root
  chmod 700 "$(dirname "$INSTALL_DIR")" 2>/dev/null || true

  info "Installed lws to $INSTALL_DIR/lws"
}

# --- Add INSTALL_DIR to PATH ---
setup_path() {
  if echo "$PATH" | tr ':' '\n' | grep -qx "$INSTALL_DIR"; then
    return
  fi

  local line="export PATH=\"$INSTALL_DIR:\$PATH\""
  local shell_name
  shell_name="$(basename "${SHELL:-/bin/bash}")"

  case "$shell_name" in
    zsh)
      local rc="$HOME/.zshrc"
      if [ -f "$rc" ] && grep -qF "$INSTALL_DIR" "$rc"; then return; fi
      echo "$line" >> "$rc"
      info "Added $INSTALL_DIR to PATH in $rc"
      ;;
    bash)
      local rc="$HOME/.bashrc"
      if [ -f "$HOME/.bash_profile" ]; then rc="$HOME/.bash_profile"; fi
      if [ -f "$rc" ] && grep -qF "$INSTALL_DIR" "$rc"; then return; fi
      echo "$line" >> "$rc"
      info "Added $INSTALL_DIR to PATH in $rc"
      ;;
    fish)
      local rc="$HOME/.config/fish/config.fish"
      local fish_line="fish_add_path $INSTALL_DIR"
      if [ -f "$rc" ] && grep -qF "$INSTALL_DIR" "$rc"; then return; fi
      mkdir -p "$(dirname "$rc")"
      echo "$fish_line" >> "$rc"
      info "Added $INSTALL_DIR to PATH in $rc"
      ;;
    *)
      info "Add $INSTALL_DIR to your PATH manually"
      ;;
  esac
}

# --- Install Python bindings ---
install_python_bindings() {
  if ! command -v python3 &>/dev/null; then
    info "python3 not found — skipping Python bindings"
    return
  fi
  if ! command -v pip3 &>/dev/null && ! python3 -m pip --version &>/dev/null 2>&1; then
    info "pip not found — skipping Python bindings"
    return
  fi

  # Prefer pip3 command, fall back to python3 -m pip
  local pip_cmd
  if command -v pip3 &>/dev/null; then
    pip_cmd="pip3"
  else
    pip_cmd="python3 -m pip"
  fi

  info "Installing Python bindings..."

  if ! command -v maturin &>/dev/null; then
    info "Installing maturin..."
    $pip_cmd install maturin
  fi

  cd "$REPO_DIR/bindings/python"
  python3 -m maturin build --release

  local wheel
  wheel="$(find "$REPO_DIR/bindings/python/target/wheels" -name '*.whl' | head -1)"
  if [ -z "$wheel" ]; then
    warn "Python bindings build failed — no wheel produced"
    return
  fi

  $pip_cmd install "$wheel"
  info "Python bindings installed successfully"
}

# --- Install Node bindings ---
install_node_bindings() {
  if ! command -v node &>/dev/null; then
    info "node not found — skipping Node bindings"
    return
  fi
  if ! command -v npm &>/dev/null; then
    info "npm not found — skipping Node bindings"
    return
  fi

  info "Installing Node bindings..."

  cd "$REPO_DIR/bindings/node"
  npm install --ignore-scripts
  npm run build

  npm install -g .
  info "Node bindings installed successfully"
}

# --- Install bindings ---
install_bindings() {
  ensure_repo_cloned
  install_python_bindings
  install_node_bindings
}

# --- Main ---
main() {
  info "LWS installer"
  echo

  local platform bin_path
  platform="$(detect_platform)"

  if bin_path="$(try_download "$platform")"; then
    install_bin "$bin_path"
  else
    warn "Prebuilt binary not found for ${platform}, falling back to source build"
    bin_path="$(build_from_source)"
    install_bin "$bin_path"
  fi

  setup_path

  echo
  info "Installing language bindings..."
  install_bindings

  echo
  info "LWS installed successfully!"
  info "Run 'lws --help' to get started (you may need to restart your shell)"
}

main "$@"
