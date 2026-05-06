# プリセット音色 比較

## 実機録音

```bash
cargo run -p xdx-compare --bin record-preset-dir -- calibration/preset_bank --midi-out "UM-ONE" --audio-in "Neva Uno 1&2 (ESI Audio Device (WDM) - Neva Uno)"
```

## ソフトシンセによる波形生成

```bash
cargo run -p xdx-compare --bin record-preset-dir -- calibration/preset_bank
```

## 比較

```bash
cargo run -p xdx-eg-viewer -- --dir calibration/preset_bank
```
