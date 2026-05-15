#!/bin/bash
set -euo pipefail

# --- Configuration ---
REPO_OWNER="nathan-mittelette"
REPO_NAME="boldify-mcp"
BINARY_NAME="boldify-mcp"
GITHUB_API_URL="https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases/latest"

# --- Colors ---
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info()    { echo -e "${BLUE}[INFO]${NC} $1" >&2; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1" >&2; }
log_warning() { echo -e "${YELLOW}[WARNING]${NC} $1" >&2; }
log_error()   { echo -e "${RED}[ERROR]${NC} $1" >&2; }

# --- Detect OS and architecture ---
detect_platform() {
    local os arch

    case "$(uname -s)" in
        Darwin) os="macos" ;;
        Linux)  os="linux" ;;
        *)
            log_error "Unsupported OS: $(uname -s)"
            exit 1
            ;;
    esac

    case "$(uname -m)" in
        x86_64|amd64)   arch="amd64" ;;
        arm64|aarch64)  arch="arm64" ;;
        *)
            log_error "Unsupported architecture: $(uname -m)"
            exit 1
            ;;
    esac

    echo "${os}-${arch}"
}

# --- Download helper (curl or wget) ---
download() {
    local url="$1"
    local dest="$2"

    if command -v curl >/dev/null 2>&1; then
        curl -fsSL --retry 3 --retry-delay 2 -o "$dest" "$url"
    elif command -v wget >/dev/null 2>&1; then
        wget -q --tries=3 --waitretry=2 -O "$dest" "$url"
    else
        log_error "Neither curl nor wget is available. Please install one of them."
        exit 1
    fi
}

# --- Fetch latest release download URL from GitHub API ---
get_download_url() {
    local platform="$1"
    local artifact="${BINARY_NAME}-${platform}"
    local api_response temp_file

    temp_file="$(mktemp)"

    log_info "Fetching latest release info from GitHub..."
    if ! download "$GITHUB_API_URL" "$temp_file"; then
        rm -f "$temp_file"
        log_error "Failed to reach GitHub API. Check your internet connection."
        exit 1
    fi

    # Parse browser_download_url for the matching artifact (no jq required)
    local download_url
    download_url=$(grep -o "\"browser_download_url\": *\"[^\"]*${artifact}[^\"]*\"" "$temp_file" \
        | head -1 \
        | grep -o "https://[^\"]*")

    rm -f "$temp_file"

    if [ -z "$download_url" ]; then
        log_error "No binary found for platform: ${platform}"
        log_error "Available artifacts may differ — check: https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/latest"
        exit 1
    fi

    echo "$download_url"
}

# --- Download binary ---
download_binary() {
    local url="$1"
    local temp_file="$2"

    log_info "Downloading from: $url"
    if ! download "$url" "$temp_file"; then
        log_error "Download failed."
        exit 1
    fi
    chmod +x "$temp_file"
    log_success "Download complete."
}

# --- Install binary ---
install_binary() {
    local temp_file="$1"
    local install_dir target_file

    # Try /usr/local/bin first, fall back to ~/.local/bin
    if [ -w "/usr/local/bin" ]; then
        install_dir="/usr/local/bin"
    elif command -v sudo >/dev/null 2>&1; then
        install_dir="/usr/local/bin"
    else
        install_dir="${HOME}/.local/bin"
    fi

    target_file="${install_dir}/${BINARY_NAME}"

    if [ "$install_dir" = "/usr/local/bin" ] && [ ! -w "/usr/local/bin" ]; then
        log_warning "Root permissions required for ${install_dir}, using sudo..."
        sudo mv "$temp_file" "$target_file"
        sudo chmod +x "$target_file"
    else
        mkdir -p "$install_dir"
        mv "$temp_file" "$target_file"
        chmod +x "$target_file"
    fi

    log_success "Installed to: $target_file"
    echo "$install_dir"
}

# --- Verify installation ---
verify_installation() {
    local install_dir="$1"

    # Add install_dir to PATH for the check if needed
    if ! command -v "$BINARY_NAME" >/dev/null 2>&1; then
        export PATH="${install_dir}:${PATH}"
    fi

    if command -v "$BINARY_NAME" >/dev/null 2>&1; then
        log_success "Installation successful!"
        log_info "Run '${BINARY_NAME} --help' to get started."
    else
        log_warning "Binary installed to ${install_dir} but it is not in your PATH."
        log_info "Add this to your shell profile and restart your terminal:"
        echo ""
        echo "  export PATH=\"${install_dir}:\$PATH\""
        echo ""
    fi
}

# --- Cleanup on error ---
cleanup() {
    local exit_code=$?
    if [ -n "${TEMP_FILE:-}" ] && [ -f "${TEMP_FILE}" ]; then
        rm -f "$TEMP_FILE"
    fi
    if [ $exit_code -ne 0 ]; then
        log_error "Installation failed (exit code: $exit_code)."
    fi
}
trap cleanup EXIT

# --- Main ---
main() {
    echo "Installing ${BINARY_NAME}..."
    echo "==============================="

    local platform download_url install_dir
    TEMP_FILE="$(mktemp)"

    platform=$(detect_platform)
    log_info "Platform: $platform"

    download_url=$(get_download_url "$platform")
    download_binary "$download_url" "$TEMP_FILE"

    install_dir=$(install_binary "$TEMP_FILE")
    TEMP_FILE=""  # already moved, prevent cleanup from deleting it

    verify_installation "$install_dir"

    echo ""
    echo "Done! boldify-mcp is ready to use."
}

main "$@"
