# プリセット音色 比較

## 実機録音

```bash
cargo run -p xdx-compare --bin record-eg-bank --release -- calibration/preset_bank/all_voices.syx --midi-out "UM-ONE" --audio-in "Neva Uno 1&2 (ESI Audio Device (WDM) - Neva Uno)" --out calibration/preset_bank
```

## ソフトシンセによる波形生成

```bash
cargo run -p xdx-compare --bin record-eg-bank --release -- calibration/preset_bank/all_voices.syx --out calibration/preset_bank
```

## 比較

```bash
cargo run -p xdx-eg-viewer --release -- --dir calibration/preset_bank
```
