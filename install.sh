#!/usr/bin/env bash
set -euo pipefail

REPO="Azmekk/vimg"
INSTALL_DIR="${HOME}/.local/bin"

case "$(uname -s)" in
    Linux*)  OS=linux ;;
    *) echo "Unsupported OS: $(uname -s) (vimg ships for Linux and Windows only)"; exit 1 ;;
esac

PATTERN="vimg-${OS}-x86_64"
URL=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
    | grep "browser_download_url.*${PATTERN}" \
    | head -1 \
    | cut -d '"' -f 4)

[ -z "${URL}" ] && { echo "No release asset for ${OS}"; exit 1; }

mkdir -p "${INSTALL_DIR}"
TMP=$(mktemp -d)
trap 'rm -rf "${TMP}"' EXIT

curl -fsSL "${URL}" -o "${TMP}/vimg.tar.gz"
tar -xzf "${TMP}/vimg.tar.gz" -C "${TMP}"
install -m 755 "${TMP}/vimg" "${INSTALL_DIR}/vimg"

echo "vimg installed to ${INSTALL_DIR}/vimg"
case ":${PATH}:" in
    *":${INSTALL_DIR}:"*) ;;
    *) echo "Note: add ${INSTALL_DIR} to your PATH" ;;
esac
