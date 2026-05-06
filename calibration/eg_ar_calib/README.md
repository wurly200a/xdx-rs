## 録音

```bash
cargo run -p xdx-compare --bin record-preset-dir -- calibration/eg_ar_calib --midi-out "UM-ONE" --audio-in "Neva Uno 1&2 (ESI Audio Device (WDM) - Neva Uno)"
```

## ソフトシンセによる波形生成

```bash
cargo run -p xdx-compare --bin record-preset-dir -- calibration/eg_ar_calib
```

## 比較

```bash

cargo run -p xdx-eg-viewer -- --dir calibration/eg_ar_calib
```

全体サマリー（atk90 の HW vs SY 比較）：

```bash
cargo run -p xdx-e2e --example compare_eg -- --dir calibration/eg_ar_calib --hold-ms 10000
```

次に AR05（voice 5）の詳細エンベロープ表示：

```bash
cargo run -p xdx-e2e --example compare_eg -- --dir calibration/eg_ar_calib --hold-ms 10000 --detail 5
```

AR10（voice 10）も同様に：

```bash
cargo run -p xdx-e2e --example compare_eg -- --dir calibration/eg_ar_calib --hold-ms 10000 --detail 10
```
