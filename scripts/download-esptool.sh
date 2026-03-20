#!/usr/bin/env bash
set -euo pipefail

ESPTOOL_VERSION="${ESPTOOL_VERSION:-v4.8.1}"
OUT_DIR="src-tauri/binaries"

mkdir -p "$OUT_DIR"

platform="$(uname -s | tr '[:upper:]' '[:lower:]')"
arch="$(uname -m)"

case "${platform}-${arch}" in
  linux-x86_64)
    asset="esptool-${ESPTOOL_VERSION}-linux-amd64.zip"
    out_name="esptool-x86_64-unknown-linux-gnu"
    zip_inner_path="esptool-linux-amd64/esptool"
    ;;
  linux-aarch64|linux-arm64)
    asset="esptool-${ESPTOOL_VERSION}-linux-arm64.zip"
    out_name="esptool-aarch64-unknown-linux-gnu"
    zip_inner_path="esptool-linux-arm64/esptool"
    ;;
  darwin-x86_64)
    asset="esptool-${ESPTOOL_VERSION}-macos.zip"
    out_name="esptool-x86_64-apple-darwin"
    zip_inner_path="esptool-macos/esptool"
    ;;
  darwin-arm64)
    asset="esptool-${ESPTOOL_VERSION}-macos.zip"
    out_name="esptool-aarch64-apple-darwin"
    zip_inner_path="esptool-macos/esptool"
    ;;
  msys*-x86_64|mingw*-x86_64|cygwin*-x86_64)
    asset="esptool-${ESPTOOL_VERSION}-win64.zip"
    out_name="esptool-x86_64-pc-windows-msvc.exe"
    zip_inner_path="esptool-win64/esptool.exe"
    ;;
  *)
    echo "Unsupported platform: ${platform}-${arch}"
    exit 1
    ;;
esac

# Allow overriding target triple for CI cross-platform builds
if [ -n "${TARGET_TRIPLE:-}" ]; then
  case "$TARGET_TRIPLE" in
    x86_64-unknown-linux-gnu)
      asset="esptool-${ESPTOOL_VERSION}-linux-amd64.zip"
      out_name="esptool-x86_64-unknown-linux-gnu"
      zip_inner_path="esptool-linux-amd64/esptool"
      ;;
    aarch64-unknown-linux-gnu)
      asset="esptool-${ESPTOOL_VERSION}-linux-arm64.zip"
      out_name="esptool-aarch64-unknown-linux-gnu"
      zip_inner_path="esptool-linux-arm64/esptool"
      ;;
    x86_64-apple-darwin)
      asset="esptool-${ESPTOOL_VERSION}-macos.zip"
      out_name="esptool-x86_64-apple-darwin"
      zip_inner_path="esptool-macos/esptool"
      ;;
    aarch64-apple-darwin)
      asset="esptool-${ESPTOOL_VERSION}-macos.zip"
      out_name="esptool-aarch64-apple-darwin"
      zip_inner_path="esptool-macos/esptool"
      ;;
    x86_64-pc-windows-msvc)
      asset="esptool-${ESPTOOL_VERSION}-win64.zip"
      out_name="esptool-x86_64-pc-windows-msvc.exe"
      zip_inner_path="esptool-win64/esptool.exe"
      ;;
    *)
      echo "Unsupported TARGET_TRIPLE: $TARGET_TRIPLE"
      exit 1
      ;;
  esac
fi

base_url="https://github.com/espressif/esptool/releases/download/${ESPTOOL_VERSION}"
tmp_zip="$(mktemp -t esptool.XXXXXX.zip)"

echo "Downloading ${asset}"
curl -L "${base_url}/${asset}" -o "${tmp_zip}"

echo "Extracting to ${OUT_DIR}/${out_name}"
unzip -p "${tmp_zip}" "${zip_inner_path}" > "${OUT_DIR}/${out_name}"

chmod +x "${OUT_DIR}/${out_name}"
rm -f "${tmp_zip}"

echo "Done: ${OUT_DIR}/${out_name}"
