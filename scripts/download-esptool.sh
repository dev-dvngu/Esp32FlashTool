#!/usr/bin/env bash
set -euo pipefail

ESPTOOL_VERSION="${ESPTOOL_VERSION:-v5.1.0}"
OUT_DIR="src-tauri/binaries"

mkdir -p "$OUT_DIR"

platform="$(uname -s | tr '[:upper:]' '[:lower:]')"
arch="$(uname -m)"

case "${platform}-${arch}" in
  linux-x86_64)
    asset="esptool-${ESPTOOL_VERSION}-linux-amd64.tar.gz"
    out_name="esptool-x86_64-unknown-linux-gnu"
    inner_path="esptool-linux-amd64/esptool"
    format="tgz"
    ;;
  linux-aarch64|linux-arm64)
    asset="esptool-${ESPTOOL_VERSION}-linux-aarch64.tar.gz"
    out_name="esptool-aarch64-unknown-linux-gnu"
    inner_path="esptool-linux-aarch64/esptool"
    format="tgz"
    ;;
  darwin-x86_64)
    asset="esptool-${ESPTOOL_VERSION}-macos-amd64.tar.gz"
    out_name="esptool-x86_64-apple-darwin"
    inner_path="esptool-macos-amd64/esptool"
    format="tgz"
    ;;
  darwin-arm64)
    asset="esptool-${ESPTOOL_VERSION}-macos-arm64.tar.gz"
    out_name="esptool-aarch64-apple-darwin"
    inner_path="esptool-macos-arm64/esptool"
    format="tgz"
    ;;
  msys*-x86_64|mingw*-x86_64|cygwin*-x86_64)
    asset="esptool-${ESPTOOL_VERSION}-windows-amd64.zip"
    out_name="esptool-x86_64-pc-windows-msvc.exe"
    inner_path="esptool-windows-amd64/esptool.exe"
    format="zip"
    ;;
  *)
    echo "Unsupported platform: ${platform}-${arch}"
    exit 1
    ;;
esac

base_url="https://github.com/espressif/esptool/releases/download/${ESPTOOL_VERSION}"
tmp_file="$(mktemp -t esptool.XXXXXX)"

echo "Downloading ${asset}..."
curl -L "${base_url}/${asset}" -o "${tmp_file}"

echo "Extracting to ${OUT_DIR}/${out_name}..."
if [ "$format" == "tgz" ]; then
  tar -O -xf "${tmp_file}" "${inner_path}" > "${OUT_DIR}/${out_name}"
else
  unzip -p "${tmp_file}" "${inner_path}" > "${OUT_DIR}/${out_name}"
fi

chmod +x "${OUT_DIR}/${out_name}"
rm -f "${tmp_file}"

echo "Done: ${OUT_DIR}/${out_name}"
