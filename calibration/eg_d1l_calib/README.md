## 録音

```bash
cargo run -p xdx-compare --bin record-preset-dir -- calibration/eg_d1l_calib --midi-out "UM-ONE" --audio-in "Neva Uno 1&2 (ESI Audio Device (WDM) - Neva Uno)"
```

## ソフトシンセによる波形生成

```bash
cargo run -p xdx-compare --bin record-preset-dir -- calibration/eg_d1l_calib
```

## 比較

```bash
cargo run -p xdx-eg-viewer -- --dir calibration/eg_d1l_calib
```

全体サマリー（全15音色 HW vs SY 比較）：

```bash
cargo run -p xdx-e2e --example compare_eg -- --dir calibration/eg_d1l_calib --hold-ms 4000
```

D1L05（voice 5）の詳細エンベロープ表示：

```bash
cargo run -p xdx-e2e --example compare_eg -- --dir calibration/eg_d1l_calib --hold-ms 4000 --detail 5
```

## 音色レイアウト

固定パラメータ: AR=31, D1R=10, D2R=0, RR=10

| # | Name  | D1L | 期待値（SW）   | dB     |
|---|-------|-----|---------------|--------|
| 1 | D1L01 |   1 | 0.0078        | -42 dB |
| 2 | D1L02 |   2 | 0.0110        | -39 dB |
| 3 | D1L03 |   3 | 0.0156        | -36 dB |
| 4 | D1L04 |   4 | 0.0221        | -33 dB |
| 5 | D1L05 |   5 | 0.0313        | -30 dB |
| 6 | D1L06 |   6 | 0.0442        | -27 dB |
| 7 | D1L07 |   7 | 0.0625        | -24 dB |
| 8 | D1L08 |   8 | 0.0884        | -21 dB |
| 9 | D1L09 |   9 | 0.1250        | -18 dB |
|10 | D1L10 |  10 | 0.1768        | -15 dB |
|11 | D1L11 |  11 | 0.2500        | -12 dB |
|12 | D1L12 |  12 | 0.3536        |  -9 dB |
|13 | D1L13 |  13 | 0.5000        |  -6 dB |
|14 | D1L14 |  14 | 0.7071        |  -3 dB |
|15 | D1L15 |  15 | 1.0000        |   0 dB |

> 現在の SW 実装: `2^((D1L-15)/2)` — 3 dB/ステップ
>
> D1R=10 の半減期 ≈0.276 s。D1L=1 でも約 1.9 s で収束。
> hold=4.0 s の末尾 10%（3.6〜4.0 s）がプラトー計測ウィンドウ。

## SYX 生成

```bash
cargo run -p xdx-e2e --example gen_d1l_calib
```
