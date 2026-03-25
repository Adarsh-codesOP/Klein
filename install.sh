#!/bin/bash
set -e

# App Directory on Windows if running in WSL/Git Bash
APP_DIR="$LOCALAPPDATA/Klein"
if [ -z "$LOCALAPPDATA" ]; then
    # Fallback if running on purely Linux or older bash
    APP_DIR="$HOME/.config/klein"
fi
CONFIG_PATH="$APP_DIR/config.toml"

# Colors
CYAN='\033[0;36m'
WHITE='\033[1;37m'
DARKGRAY='\033[1;30m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

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

# show header
print_banner

echo -e "${YELLOW}Starting installation...${NC}"

if [ ! -d "$APP_DIR" ]; then
    mkdir -p "$APP_DIR"
    echo "Created application directory at $APP_DIR"
fi

prompt_configuration() {
    echo -e "\n${CYAN}╭────────────┤ Configuration ├────────────╮${NC}"
    
    # Check for Git Bash installation path in default Windows locations
    if [ ! -d "/c/Program Files/Git" ] && [ ! -d "/c/Users/$USER/AppData/Local/Programs/Git" ] && ! command -v bash &> /dev/null; then
        echo "WARNING: Git Bash was not found in standard locations."
        echo "We highly recommend installing Git Bash for the best terminal experience in Klein."
        echo "You can download it from: https://gitforwindows.org/"
        read -p "Would you like to install Git Bash later? (y/n) " install_git
        if [ "$install_git" = "n" ]; then
            echo "You can continue, but terminal features might be limited."
        fi
    fi

    read -p "Enter your default workspace/projects path [Default: $HOME]: " workspace
    workspace=${workspace:-$HOME}

    if [ ! -d "$workspace" ]; then
        read -p "Path '$workspace' does not exist. Create it? (y/N) " create_ws
        if [[ "$create_ws" =~ ^[Yy]$ ]]; then
            mkdir -p "$workspace"
        else
            echo "Warning: Workspace path may be invalid."
        fi
    fi

    if [ -d "/c/Program Files/Git" ] || [ -d "/c/Users/$USER/AppData/Local/Programs/Git" ] || command -v bash &> /dev/null; then
        shell="bash"
    else
        shell="auto"
    fi

    cat > "$CONFIG_PATH" <<EOF
# Klein TIDE Configuration
default_workspace = "$workspace"
shell = "$shell"
EOF

    echo -e "Configuration saved to $CONFIG_PATH"
}

if [[ "$1" == "--reconfigure" || "$1" == "-Reconfigure" ]]; then
    prompt_configuration
    echo -e "\n${GREEN}✔ Reconfiguration complete!${NC}"
    exit 0
fi

echo -e "\n${CYAN}╭────────────┤ Installation ├────────────╮${NC}"

# Detect OS and architecture
OS=$(uname -s)
ARCH=$(uname -m)
BIN_NAME="klein"
DOWNLOAD_URL=""

# Set download URL based on OS and architecture
if [[ "$OS" == "Linux" ]]; then
    if [[ "$ARCH" == "x86_64" ]]; then
        DOWNLOAD_URL="https://github.com/Adarsh-codesOP/Klein/releases/download/v0.2.5/klein-linux-x86_64"
    elif [[ "$ARCH" == "aarch64" ]]; then
        DOWNLOAD_URL="https://github.com/Adarsh-codesOP/Klein/releases/download/v0.2.5/klein-linux-aarch64"
    fi
elif [[ "$OS" == "Darwin" ]]; then
    echo -e "${YELLOW}Warning: macOS support is not yet available.${NC}"
    echo "Please build from source: cargo install --path ."
    exit 1
fi

if [ -z "$DOWNLOAD_URL" ]; then
    echo -e "${YELLOW}Unsupported OS/Architecture: $OS $ARCH${NC}"
    echo "Please build from source: cargo install --path ."
    exit 1
fi

BIN_PATH="$APP_DIR/$BIN_NAME"

echo -e "${YELLOW}Downloading Klein binary for $OS $ARCH...${NC}"
echo -e "URL: $DOWNLOAD_URL"

if curl -fsSL "$DOWNLOAD_URL" -o "$BIN_PATH"; then
    chmod +x "$BIN_PATH"
    echo -e "${GREEN}Successfully downloaded to $BIN_PATH${NC}"
    
    # Add to PATH
    BASHRC="$HOME/.bashrc"
    ZSHRC="$HOME/.zshrc"
    
    for RC_FILE in "$BASHRC" "$ZSHRC"; do
        if [ -f "$RC_FILE" ] && ! grep -q "$APP_DIR" "$RC_FILE"; then
            echo -e "\nexport PATH=\"\$PATH:$APP_DIR\"" >> "$RC_FILE"
            echo -e "${GREEN}Added $APP_DIR to PATH in $RC_FILE${NC}"
        elif [ ! -f "$RC_FILE" ] && [ "$RC_FILE" = "$BASHRC" ]; then
            echo -e "export PATH=\"\$PATH:$APP_DIR\"" > "$RC_FILE"
            echo -e "${GREEN}Created $RC_FILE and added $APP_DIR to PATH${NC}"
        fi
    done
    
    echo -e "${YELLOW}Please run 'source ~/.bashrc' or restart your terminal to use the 'klein' command globally.${NC}"
else
    echo -e "${YELLOW}Failed to download the binary. Attempting to build from source...${NC}"
    
    if ! command -v cargo &> /dev/null; then
        echo -e "${RED}Rust is not installed. Please install Rust from https://rustup.rs/${NC}"
        exit 1
    fi
    
    echo "Building Klein from source..."
    if cargo install --path .; then
        echo -e "${GREEN}Successfully built and installed Klein from source!${NC}"
    else
        echo -e "${RED}Failed to build Klein. Please check the error messages above.${NC}"
        exit 1
    fi
fi

prompt_configuration

echo -e "\n${GREEN}╔════════════════════════════════════════╗${NC}"
echo -e "${GREEN}✔ Installation & Configuration Complete!${NC}"
echo -e "${GREEN}You can run this script later with '--reconfigure' to update your settings.${NC}"
echo -e "${GREEN}╚════════════════════════════════════════╝${NC}"
