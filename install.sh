#!/bin/bash
set -euo pipefail

# ─────────────────────────────────────────────────────────────────────────────
# Klein — Cross-platform installer (Linux / macOS / Git Bash on Windows)
#
# Usage:
#   curl -sSL https://raw.githubusercontent.com/<owner>/Klein/main/install.sh | bash
#   ./install.sh           # interactive (prompts for workspace config)
#   ./install.sh --yes     # non-interactive (accept all defaults, useful in CI/Docker)
#   ./install.sh --reconfigure  # re-run the configuration step only
#
# Environment variables (all optional):
#   REPO   GitHub repository in owner/repo format — auto-detected from git remote
#          when run from a clone; defaults to the upstream repo.
# ─────────────────────────────────────────────────────────────────────────────

# ── Repository detection ─────────────────────────────────────────────────────
# Derive repo from the git remote of the directory containing this script so
# forks work without any changes. Falls back to the canonical upstream repo.
_detect_repo() {
    local script_dir
    script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" 2>/dev/null && pwd || echo "")"
    if [[ -n "${script_dir}" ]]; then
        local remote_url
        remote_url=$(git -C "${script_dir}" remote get-url origin 2>/dev/null || true)
        if [[ -n "${remote_url}" ]]; then
            # Handle both HTTPS (https://github.com/owner/repo.git) and
            # SSH (git@github.com:owner/repo.git) URL formats
            echo "${remote_url}" \
                | sed -E 's|.*github\.com[:/]([^/]+/[^/]+?)(\.git)?$|\1|'
            return
        fi
    fi
    echo "Adarsh-codesOP/Klein"
}
REPO="${REPO:-$(_detect_repo)}"
REPO_OWNER="${REPO%%/*}"
REPO_NAME="${REPO##*/}"

# ── Application identity ─────────────────────────────────────────────────────
# Single place to update if the project or binary is ever renamed.
# Can also be overridden via environment variables (e.g. in CI).
APP_NAME="${APP_NAME:-Klein}"       # Directory / brand name (e.g. ~/AppData/Local/Klein)
BINARY_NAME="${BINARY_NAME:-klein}" # Executable base name (no extension)

# ── Installation paths ───────────────────────────────────────────────────────
# Windows (Git Bash / WSL): install into %LOCALAPPDATA%\Klein
# Linux / macOS:            install into ~/.local/bin  (or ~/bin as fallback)
if [[ -n "${LOCALAPPDATA:-}" ]]; then
    APP_DIR="${LOCALAPPDATA}/${APP_NAME}"
    BIN_DIR="${APP_DIR}/bin"   # cargo install --root uses <root>/bin/ — keep consistent
else
    BIN_DIR="${HOME}/.local/bin"
    # Fall back to ~/bin if ~/.local/bin is not standard on the system
    if [[ -d "${HOME}/bin" ]] && ! [[ -d "${HOME}/.local/bin" ]]; then
        BIN_DIR="${HOME}/bin"
    fi
    APP_DIR="${HOME}/.config/${BINARY_NAME}"
fi
CONFIG_PATH="${APP_DIR}/config.toml"
BIN_NAME="${BINARY_NAME}"
BIN_PATH="${BIN_DIR}/${BIN_NAME}"

# ── Flags ────────────────────────────────────────────────────────────────────
NON_INTERACTIVE=false
DO_RECONFIGURE=false
for arg in "$@"; do
    case "${arg}" in
        --yes|-y)               NON_INTERACTIVE=true ;;
        --reconfigure|-Reconfigure) DO_RECONFIGURE=true ;;
    esac
done

# ── Colours ──────────────────────────────────────────────────────────────────
CYAN='\033[0;36m'; WHITE='\033[1;37m'; GREEN='\033[0;32m'
YELLOW='\033[1;33m'; RED='\033[0;31m'; NC='\033[0m'

print_banner() {
    echo -e "${CYAN}"
    echo "oooo   oooo ooooo       ooooooooooo ooooo oooo   oooo "
    echo " 888  o88    888         888    88   888   8888o  88  "
    echo " 888888      888         888ooo8     888   88 888o88  "
    echo " 888  88o    888      o  888    oo   888   88   8888  "
    echo "o888o o888o o888ooooo88 o888ooo8888 o888o o88o    88  "
    echo "                                                      "
    echo -e "${NC}"
}

# ── GitHub API helpers ───────────────────────────────────────────────────────

_http_get() {
    if command -v curl &>/dev/null; then
        curl -fsSL "$1"
    elif command -v wget &>/dev/null; then
        wget -qO- "$1"
    else
        echo -e "${RED}Error: neither curl nor wget is available.${NC}" >&2
        return 1
    fi
}

# Returns the latest release tag for $REPO (e.g. "v0.2.5")
get_latest_version() {
    local api_url="https://api.github.com/repos/${REPO}/releases/latest"
    local tag
    tag=$(_http_get "${api_url}" 2>/dev/null \
        | grep '"tag_name"' \
        | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')
    echo "${tag}"
}

# Maps the current OS/arch to the Rust target triple used in release assets.
# Asset naming convention (from the release workflow):
#   klein-<version>-<rust-triple>.tar.gz   (Linux / macOS)
#   klein-<version>-<rust-triple>.zip      (Windows)
get_target_triple() {
    local os arch
    os=$(uname -s)
    arch=$(uname -m)
    case "${os}" in
        Linux)
            case "${arch}" in
                x86_64)          echo "x86_64-unknown-linux-gnu" ;;
                aarch64|arm64)   echo "aarch64-unknown-linux-gnu" ;;
                *) return 1 ;;
            esac ;;
        Darwin)
            case "${arch}" in
                x86_64)          echo "x86_64-apple-darwin" ;;
                arm64|aarch64)   echo "aarch64-apple-darwin" ;;
                *) return 1 ;;
            esac ;;
        MINGW*|MSYS*|CYGWIN*)
            echo "x86_64-pc-windows-msvc" ;;
        *) return 1 ;;
    esac
}

# ── Installation methods ─────────────────────────────────────────────────────

# Try mise github backend — downloads pre-built binary from GitHub Releases.
install_via_mise() {
    if ! command -v mise &>/dev/null; then
        return 1
    fi
    echo -e "${YELLOW}Trying mise (github backend)…${NC}"
    if mise use -g "github:${REPO}" 2>&1; then
        echo -e "${GREEN}✔ Installed via mise!${NC}"
        return 0
    fi
    return 1
}

# Download pre-built archive from GitHub Releases.
# New naming:  klein-<version>-<triple>.tar.gz   (introduced by our workflow)
# Legacy naming: klein-linux-x86_64  /  klein-linux-aarch64  (pre-workflow)
install_via_github_release() {
    local version
    echo -e "${YELLOW}Checking GitHub Releases for ${REPO}…${NC}"
    version=$(get_latest_version)
    if [[ -z "${version}" ]]; then
        echo -e "${YELLOW}No GitHub release found, skipping.${NC}"
        return 1
    fi

    local triple
    if ! triple=$(get_target_triple); then
        echo -e "${YELLOW}Unsupported platform for pre-built binary.${NC}"
        return 1
    fi

    local os
    os=$(uname -s)
    local archive ext
    case "${os}" in
        MINGW*|MSYS*|CYGWIN*) ext=".zip" ;;
        *)                     ext=".tar.gz" ;;
    esac
    archive="${BIN_NAME}-${version}-${triple}${ext}"
    local download_url="https://github.com/${REPO}/releases/download/${version}/${archive}"

    echo -e "${YELLOW}Downloading ${archive} (${version})…${NC}"
    local tmp_dir
    tmp_dir=$(mktemp -d)
    local ok=0
    if _http_get "${download_url}" > "${tmp_dir}/${archive}" 2>/dev/null; then
        ok=1
    else
        # Fallback: try legacy naming (plain binary, no archive)
        local legacy_arch
        case "${triple}" in
            x86_64-unknown-linux-gnu)   legacy_arch="linux-x86_64" ;;
            aarch64-unknown-linux-gnu)  legacy_arch="linux-aarch64" ;;
            *)  legacy_arch="" ;;
        esac
        if [[ -n "${legacy_arch}" ]]; then
            local legacy_url="https://github.com/${REPO}/releases/download/${version}/${BIN_NAME}-${legacy_arch}"
            echo -e "${YELLOW}Archive not found, trying legacy binary ${BIN_NAME}-${legacy_arch}…${NC}"
            if _http_get "${legacy_url}" > "${tmp_dir}/${BIN_NAME}" 2>/dev/null; then
                mkdir -p "${BIN_DIR}"
                install -m 755 "${tmp_dir}/${BIN_NAME}" "${BIN_PATH}"
                rm -rf "${tmp_dir}"
                echo -e "${GREEN}✔ Installed ${version} (legacy binary) to ${BIN_PATH}${NC}"
                return 0
            fi
        fi
        rm -rf "${tmp_dir}"
        echo -e "${YELLOW}Download failed for ${archive}, will try source build.${NC}"
        return 1
    fi

    mkdir -p "${BIN_DIR}"
    tar xzf "${tmp_dir}/${archive}" -C "${tmp_dir}"
    if [[ -f "${tmp_dir}/${BIN_NAME}" ]]; then
        install -m 755 "${tmp_dir}/${BIN_NAME}" "${BIN_PATH}"
        rm -rf "${tmp_dir}"
        echo -e "${GREEN}✔ Installed ${version} to ${BIN_PATH}${NC}"
        return 0
    fi
    rm -rf "${tmp_dir}"
    echo -e "${YELLOW}Binary not found in archive, will try source build.${NC}"
    return 1
}

# Build and install from source using cargo.
install_from_source() {
    if ! command -v cargo &>/dev/null; then
        echo -e "${RED}Rust/cargo not found. Install from https://rustup.rs/${NC}" >&2
        return 1
    fi
    echo -e "${YELLOW}Building from source (this may take a few minutes)…${NC}"
    mkdir -p "${BIN_DIR}"
    # Install from the GitHub repository, NOT from crates.io.
    # A crate named 'klein' exists on crates.io but is an unrelated project.
    cargo install --git "https://github.com/${REPO}" --root "${HOME}/.local" 2>&1 \
        || cargo install --path . --root "${HOME}/.local" 2>&1
}

# ── Configuration ─────────────────────────────────────────────────────────────

prompt_configuration() {
    mkdir -p "${APP_DIR}"

    if [[ "${NON_INTERACTIVE}" == "true" ]]; then
        local workspace="${HOME}"
        local shell_pref="auto"
        if command -v bash &>/dev/null; then shell_pref="bash"; fi
        cat > "${CONFIG_PATH}" <<EOF
# Klein TIDE Configuration (generated by installer)
default_workspace = "${workspace}"
shell = "${shell_pref}"
EOF
        echo -e "${GREEN}Configuration written to ${CONFIG_PATH} (non-interactive defaults).${NC}"
        return
    fi

    echo -e "\n${CYAN}╭────────────┤ Configuration ├────────────╮${NC}"

    # Git Bash check (Windows only)
    if [[ -n "${LOCALAPPDATA:-}" ]]; then
        if [[ ! -d "/c/Program Files/Git" ]] && ! command -v bash &>/dev/null; then
            echo -e "${YELLOW}WARNING: Git Bash was not found. Install from https://gitforwindows.org/${NC}"
            read -rp "Continue anyway? (y/N) " cont
            [[ "${cont}" =~ ^[Yy]$ ]] || exit 0
        fi
    fi

    read -rp "Default workspace/projects path [${HOME}]: " workspace
    workspace="${workspace:-${HOME}}"

    if [[ ! -d "${workspace}" ]]; then
        read -rp "Path '${workspace}' does not exist. Create it? (y/N) " create_ws
        if [[ "${create_ws}" =~ ^[Yy]$ ]]; then
            mkdir -p "${workspace}"
        else
            echo -e "${YELLOW}Warning: workspace path may be invalid.${NC}"
        fi
    fi

    local shell_pref="auto"
    if command -v bash &>/dev/null; then shell_pref="bash"; fi

    cat > "${CONFIG_PATH}" <<EOF
# Klein TIDE Configuration
default_workspace = "${workspace}"
shell = "${shell_pref}"
EOF
    echo -e "${GREEN}Configuration saved to ${CONFIG_PATH}${NC}"
}

# ── PATH setup ────────────────────────────────────────────────────────────────

setup_path() {
    for rc_file in "${HOME}/.bashrc" "${HOME}/.zshrc"; do
        if [[ -f "${rc_file}" ]] && ! grep -qF "${BIN_DIR}" "${rc_file}" 2>/dev/null; then
            printf '\nexport PATH="%s:$PATH"\n' "${BIN_DIR}" >> "${rc_file}"
            echo -e "${GREEN}Added ${BIN_DIR} to PATH in ${rc_file}${NC}"
        fi
    done
    echo -e "${YELLOW}Restart your terminal or run: export PATH=\"${BIN_DIR}:\$PATH\"${NC}"
}

# ── Main ──────────────────────────────────────────────────────────────────────

print_banner
echo -e "${YELLOW}Starting installation (repo: ${REPO})…${NC}"

mkdir -p "${APP_DIR}" "${BIN_DIR}"

if [[ "${DO_RECONFIGURE}" == "true" ]]; then
    prompt_configuration
    echo -e "\n${GREEN}✔ Reconfiguration complete!${NC}"
    exit 0
fi

echo -e "\n${CYAN}╭────────────┤ Installation ├────────────╮${NC}"

installed=false

# 1. mise github backend (fastest — pre-built binary, no archive needed)
if install_via_mise;            then installed=true; fi
# 2. Direct GitHub Release download
if ! ${installed} && install_via_github_release; then installed=true; fi
# 3. Source build fallback
if ! ${installed} && install_from_source;        then installed=true; fi

if ! ${installed}; then
    echo -e "${RED}All installation methods failed. Please install Rust from https://rustup.rs/ and retry.${NC}" >&2
    exit 1
fi

setup_path
prompt_configuration

echo -e "\n${GREEN}╔════════════════════════════════════════╗${NC}"
echo -e "${GREEN}✔ Installation & Configuration Complete!${NC}"
echo -e "${GREEN}  Run 'klein' to start the editor.${NC}"
echo -e "${GREEN}  Use '--reconfigure' to update your settings.${NC}"
echo -e "${GREEN}╚════════════════════════════════════════╝${NC}"
