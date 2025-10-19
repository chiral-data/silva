#!/bin/sh
# Silva TUI Installation Script
# Usage: curl -fsSL https://raw.githubusercontent.com/YOUR_USERNAME/research-silva/main/install.sh | sh

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Repository information
REPO="YOUR_USERNAME/research-silva"
BINARY_NAME="research-silva"

# Detect OS
detect_os() {
    OS=$(uname -s)
    case "$OS" in
        Linux*)     OS_TYPE="linux";;
        Darwin*)    OS_TYPE="macos";;
        MINGW*|MSYS*|CYGWIN*) OS_TYPE="windows";;
        *)
            echo "${RED}Error: Unsupported operating system: $OS${NC}"
            exit 1
            ;;
    esac
}

# Detect architecture
detect_arch() {
    ARCH=$(uname -m)
    case "$ARCH" in
        x86_64|amd64)   ARCH_TYPE="x86_64";;
        aarch64|arm64)  ARCH_TYPE="aarch64";;
        *)
            echo "${RED}Error: Unsupported architecture: $ARCH${NC}"
            exit 1
            ;;
    esac
}

# Get latest release version
get_latest_version() {
    echo "${YELLOW}Fetching latest release...${NC}"

    # Try to get the latest release from GitHub API
    LATEST_RELEASE=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')

    if [ -z "$LATEST_RELEASE" ]; then
        echo "${RED}Error: Could not fetch latest release${NC}"
        exit 1
    fi

    echo "${GREEN}Latest version: $LATEST_RELEASE${NC}"
}

# Construct download URL
construct_download_url() {
    if [ "$OS_TYPE" = "windows" ]; then
        ASSET_NAME="${BINARY_NAME}-${OS_TYPE}-${ARCH_TYPE}.exe.zip"
    else
        ASSET_NAME="${BINARY_NAME}-${OS_TYPE}-${ARCH_TYPE}.tar.gz"
    fi

    DOWNLOAD_URL="https://github.com/$REPO/releases/download/$LATEST_RELEASE/$ASSET_NAME"
    echo "${YELLOW}Download URL: $DOWNLOAD_URL${NC}"
}

# Download and extract
download_and_extract() {
    TMP_DIR=$(mktemp -d)
    echo "${YELLOW}Downloading to $TMP_DIR...${NC}"

    cd "$TMP_DIR"

    if ! curl -fsSL -o "$ASSET_NAME" "$DOWNLOAD_URL"; then
        echo "${RED}Error: Failed to download $DOWNLOAD_URL${NC}"
        rm -rf "$TMP_DIR"
        exit 1
    fi

    echo "${GREEN}Download completed${NC}"

    # Extract
    if [ "$OS_TYPE" = "windows" ]; then
        unzip -q "$ASSET_NAME"
        BINARY_PATH="${BINARY_NAME}.exe"
    else
        tar -xzf "$ASSET_NAME"
        BINARY_PATH="$BINARY_NAME"
    fi

    if [ ! -f "$BINARY_PATH" ]; then
        echo "${RED}Error: Binary not found after extraction${NC}"
        rm -rf "$TMP_DIR"
        exit 1
    fi

    chmod +x "$BINARY_PATH"
}

# Install binary
install_binary() {
    # Determine installation directory
    if [ -w "/usr/local/bin" ]; then
        INSTALL_DIR="/usr/local/bin"
    elif [ -d "$HOME/.local/bin" ]; then
        INSTALL_DIR="$HOME/.local/bin"
    else
        mkdir -p "$HOME/.local/bin"
        INSTALL_DIR="$HOME/.local/bin"
    fi

    echo "${YELLOW}Installing to $INSTALL_DIR...${NC}"

    # Copy binary
    if [ "$OS_TYPE" = "windows" ]; then
        cp "$BINARY_PATH" "$INSTALL_DIR/${BINARY_NAME}.exe"
        INSTALLED_PATH="$INSTALL_DIR/${BINARY_NAME}.exe"
    else
        # Use sudo if needed for /usr/local/bin
        if [ "$INSTALL_DIR" = "/usr/local/bin" ] && [ ! -w "/usr/local/bin" ]; then
            sudo cp "$BINARY_PATH" "$INSTALL_DIR/$BINARY_NAME"
        else
            cp "$BINARY_PATH" "$INSTALL_DIR/$BINARY_NAME"
        fi
        INSTALLED_PATH="$INSTALL_DIR/$BINARY_NAME"
    fi

    echo "${GREEN}Installation completed: $INSTALLED_PATH${NC}"

    # Cleanup
    rm -rf "$TMP_DIR"
}

# Check if binary is in PATH
check_path() {
    if command -v "$BINARY_NAME" >/dev/null 2>&1; then
        echo "${GREEN}✓ $BINARY_NAME is available in your PATH${NC}"
    else
        echo "${YELLOW}⚠ $BINARY_NAME is not in your PATH${NC}"

        if [ "$INSTALL_DIR" = "$HOME/.local/bin" ]; then
            echo ""
            echo "Add the following line to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
            echo ""
            echo "    export PATH=\"\$HOME/.local/bin:\$PATH\""
            echo ""
            echo "Then run: source ~/.bashrc (or restart your shell)"
        fi
    fi
}

# Verify installation
verify_installation() {
    echo ""
    echo "${YELLOW}Verifying installation...${NC}"

    if [ -f "$INSTALLED_PATH" ]; then
        echo "${GREEN}✓ Binary installed successfully${NC}"

        # Try to run version check
        if command -v "$BINARY_NAME" >/dev/null 2>&1; then
            echo ""
            echo "Run the application with:"
            echo "    $BINARY_NAME"
        fi
    else
        echo "${RED}✗ Installation verification failed${NC}"
        exit 1
    fi
}

# Main installation flow
main() {
    echo "${GREEN}========================================${NC}"
    echo "${GREEN}  Silva TUI Installation${NC}"
    echo "${GREEN}========================================${NC}"
    echo ""

    detect_os
    detect_arch

    echo "Detected system: ${GREEN}$OS_TYPE $ARCH_TYPE${NC}"
    echo ""

    get_latest_version
    construct_download_url
    download_and_extract
    install_binary
    verify_installation
    check_path

    echo ""
    echo "${GREEN}========================================${NC}"
    echo "${GREEN}  Installation Complete!${NC}"
    echo "${GREEN}========================================${NC}"
}

main
