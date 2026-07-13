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

1. `VISIOFLOW_MODELS_DIR` environment variable (directory must exist if set)
2. `models/` under the current working directory
3. `models/` beside `visioflow.exe` (walks executable ancestors)
