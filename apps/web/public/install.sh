#!/usr/bin/env bash
set -e

# Architekton CLI Installer for Linux and macOS
REPO="IamSamk/Architekton"
INSTALL_DIR="${HOME}/.local/bin"

echo "Installing Architekton CLI..."

# Detect OS and Architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

case "${OS}" in
  Darwin)
    TARGET="aarch64-apple-darwin"
    ;;
  Linux)
    case "${ARCH}" in
      x86_64)
        TARGET="x86_64-unknown-linux-gnu"
        ;;
      *)
        echo "Error: Unsupported Linux architecture ${ARCH}"
        exit 1
        ;;
    esac
    ;;
  *)
    echo "Error: Unsupported operating system ${OS}"
    exit 1
    ;;
esac

# Get latest tag
LATEST_TAG=$(curl -s "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
if [ -z "${LATEST_TAG}" ]; then
  LATEST_TAG="v0.6.0"
fi

URL="https://github.com/IamSamk/Architekton/releases/download/${LATEST_TAG}/architekton-cli-${LATEST_TAG}-${TARGET}.tar.gz"

TMP_DIR=$(mktemp -d)
trap 'rm -rf "${TMP_DIR}"' EXIT

echo "Downloading ${URL}..."
curl -fsSL "${URL}" -o "${TMP_DIR}/archive.tar.gz"

tar -xzf "${TMP_DIR}/archive.tar.gz" -C "${TMP_DIR}"

mkdir -p "${INSTALL_DIR}"
cp "${TMP_DIR}/"architekton-cli-*/architekton "${INSTALL_DIR}/architekton"
chmod +x "${INSTALL_DIR}/architekton"

echo ""
echo "Successfully installed Architekton CLI to ${INSTALL_DIR}/architekton"
echo ""

# Check if INSTALL_DIR is in PATH
case ":${PATH}:" in
  *:"${INSTALL_DIR}":*) ;;
  *)
    echo "Note: Please ensure ${INSTALL_DIR} is in your PATH:"
    echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
    ;;
esac

echo "Run 'architekton --help' to get started."
