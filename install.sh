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
NC='\033[0m' # No Color

echo -e "\n${CYAN}"
cat << "EOF"
  _  __ _      _         _____  ____  ______ 
 | |/ /| |    (_)       |_   _||  _ \|  ____|
 | ' / | |     _  _ __    | |  | |  \| |__   
 |  <  | |    | || '_ \   | |  | |  |  ___|  
 | . \ | |____| || | | | _| |_ | |_/ / |____ 
 |_|\_\|______|_||_| |_||_____||____/|______|
                                             
EOF
echo -e "${DARKGRAY} ==========================================${NC}"
echo -e "${WHITE}     The aesthetic terminal IDE in Rust    ${NC}"
echo -e "${DARKGRAY} ==========================================${NC}\n"

if [ ! -d "$APP_DIR" ]; then
    mkdir -p "$APP_DIR"
    echo "Created application directory at $APP_DIR"
fi

prompt_configuration() {
    echo -e "\n--- Configuration ---"
    
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
# Klein IDE Configuration
default_workspace = "$workspace"
shell = "$shell"
EOF

    echo -e "Configuration saved to $CONFIG_PATH"
}

if [[ "$1" == "--reconfigure" || "$1" == "-Reconfigure" ]]; then
    prompt_configuration
    echo -e "\nReconfiguration complete!"
    exit 0
fi

echo -e "\n--- Installation ---"
EXE_PATH="$APP_DIR/klein.exe"

echo "Downloading pre-compiled binary from GitHub Releases..."
if curl -fsSL "https://github.com/Adarsh-codesOP/Klein/releases/download/stable/klein.exe" -o "$EXE_PATH"; then
    chmod +x "$EXE_PATH"
    echo -e "Successfully downloaded to $EXE_PATH"
    
    BASHRC="$HOME/.bashrc"
    if [ -f "$BASHRC" ] && ! grep -q "$APP_DIR" "$BASHRC"; then
        echo -e "\nexport PATH=\"\$PATH:$APP_DIR\"" >> "$BASHRC"
        echo "Added $APP_DIR to PATH in $BASHRC."
        echo "Please run 'source ~/.bashrc' or restart your terminal to use the 'klein' command globally."
    elif [ ! -f "$BASHRC" ]; then
        echo -e "export PATH=\"\$PATH:$APP_DIR\"" >> "$BASHRC"
        echo "Created $BASHRC and added $APP_DIR to PATH."
        echo "Please run 'source ~/.bashrc' or restart your terminal to use the 'klein' command globally."
    fi
else
    echo "Failed to download the executable. Please install Rust and run 'cargo install --path .' from the source."
fi

prompt_configuration

echo -e "\nInstallation & Configuration Complete!"
echo "You can run this script later with '--reconfigure' to update your settings."
