# WeChat QR Models

Place the following OpenCV WeChat QR model files in this directory:

- `detect.prototxt`
- `detect.caffemodel`
- `sr.prototxt`
- `sr.caffemodel`

You can source these from OpenCV contrib's `wechat_qrcode` model set, or run:

- Windows: `powershell -File scripts/download-wechat-models.ps1`
- Linux/macOS: `bash scripts/download-wechat-models.sh`

Runtime model resolution order:

1. `VISIOFLOW_MODELS_DIR` environment variable
2. A `models` directory found by walking up from the executable path
