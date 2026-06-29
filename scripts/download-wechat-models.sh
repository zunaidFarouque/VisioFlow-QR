#!/usr/bin/env bash
set -euo pipefail

BASE="https://raw.githubusercontent.com/WeChatCV/opencv_3rdparty/a8b69ccc738421293254aec5ddb38bd523503252"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODELS_DIR="${ROOT}/models"
mkdir -p "${MODELS_DIR}"

for file in detect.prototxt detect.caffemodel sr.prototxt sr.caffemodel; do
  echo "Downloading ${file}..."
  curl -fsSL "${BASE}/${file}" -o "${MODELS_DIR}/${file}"
done

echo "WeChat models downloaded to ${MODELS_DIR}"
