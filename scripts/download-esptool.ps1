$ErrorActionPreference = "Stop"

# Nâng cấp lên v5.1.0
$ESPTOOL_VERSION = if ($env:ESPTOOL_VERSION) { $env:ESPTOOL_VERSION } else { "v5.1.0" }
$OUT_DIR = "src-tauri/binaries"

New-Item -ItemType Directory -Force -Path $OUT_DIR | Out-Null

$asset = "esptool-${ESPTOOL_VERSION}-windows-amd64.zip"
$out_name = "esptool-x86_64-pc-windows-msvc.exe"
$zip_inner = "esptool-windows-amd64/esptool.exe"

$base_url = "https://github.com/espressif/esptool/releases/download/${ESPTOOL_VERSION}"
$tmp_zip = [System.IO.Path]::GetTempFileName() + ".zip"
$tmp_dir = [System.IO.Path]::GetTempFileName() + "_dir"

Write-Host "Downloading ${asset}..."
Invoke-WebRequest -Uri "${base_url}/${asset}" -OutFile $tmp_zip

Write-Host "Extracting to ${OUT_DIR}/${out_name}..."
Expand-Archive -Path $tmp_zip -DestinationPath $tmp_dir -Force
Copy-Item -Path (Join-Path $tmp_dir $zip_inner) -Destination (Join-Path $OUT_DIR $out_name) -Force

Remove-Item $tmp_zip -Force
Remove-Item $tmp_dir -Recurse -Force

Write-Host "Done: ${OUT_DIR}/${out_name}"
