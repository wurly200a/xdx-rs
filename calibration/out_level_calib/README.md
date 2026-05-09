## 目的

OUT LEVEL (0.75 dB/step) の振幅スケーリング実装を HW 実機と比較して検証する。

EG をサステイン状態（AR=31, D1R=0, D1L=15, D2R=0, RR=15）に固定し、  
サステイン中の RMS レベルが OUT LEVEL に対して 0.75 dB/step の比率になっているかを確認する。  
DX100 マニュアルの警告に従い OL=91–99 は除外（歪み発生）、OL=0 は無音のため除外。

## SYX 生成

```bash
cargo run -p xdx-e2e --example gen_out_level_calib
```

## 録音

```bash
cargo run -p xdx-compare --bin record-preset-dir -- calibration/out_level_calib --midi-out "UM-ONE" --audio-in "Neva Uno 1&2 (ESI Audio Device (WDM) - Neva Uno)"
```

## ソフトシンセによる波形生成

```bash
cargo run -p xdx-compare --bin record-preset-dir -- calibration/out_level_calib
```

## 比較（GUI）

```bash
cargo run -p xdx-eg-viewer -- --dir calibration/out_level_calib
```

## 比較（定量、0.75 dB/step 確認）

```bash
cargo run -p xdx-e2e --example compare_out_level -- --dir calibration/out_level_calib --hold-ms 3000
```

出力カラムの見方：

| カラム | 意味 | 理想値 |
|--------|------|--------|
| theory | (OL − 90) × 0.75 dB（理論値）| — |
| HW(dB) | HW 録音のサステイン RMS（絶対値）| — |
| SY(dB) | SY レンダリングのサステイン RMS（絶対値）| — |
| HW-thy | HW の OL=90 基準相対レベル − theory | 0.00 dB |
| SY-thy | SY の OL=90 基準相対レベル − theory | 0.00 dB |
| HW-SY  | HW 相対レベル − SY 相対レベル | 0.00 dB |

## 音色レイアウト

固定パラメータ: AR=31, D1R=0, D1L=15, D2R=0, RR=15, algo=0, feedback=0, LFO=off

| # | Name  | OL | dB(rel)  | amp(rel) |
|---|-------|----|----------|----------|
|  1 | OL90 | 90 |    0.00  |  1.00000 |
|  2 | OL86 | 86 |   -3.00  |  0.70795 |
|  3 | OL82 | 82 |   -6.00  |  0.50119 |
|  4 | OL78 | 78 |   -9.00  |  0.35481 |
|  5 | OL74 | 74 |  -12.00  |  0.25119 |
|  6 | OL70 | 70 |  -15.00  |  0.17783 |
|  7 | OL66 | 66 |  -18.00  |  0.12589 |
|  8 | OL62 | 62 |  -21.00  |  0.08913 |
|  9 | OL58 | 58 |  -24.00  |  0.06310 |
| 10 | OL54 | 54 |  -27.00  |  0.04467 |
| 11 | OL50 | 50 |  -30.00  |  0.03162 |
| 12 | OL46 | 46 |  -33.00  |  0.02239 |
| 13 | OL42 | 42 |  -36.00  |  0.01585 |
| 14 | OL38 | 38 |  -39.00  |  0.01122 |
| 15 | OL34 | 34 |  -42.00  |  0.00794 |
| 16 | OL30 | 30 |  -45.00  |  0.00562 |
| 17 | OL26 | 26 |  -48.00  |  0.00398 |
| 18 | OL22 | 22 |  -51.00  |  0.00282 |
| 19 | OL18 | 18 |  -54.00  |  0.00200 |
| 20 | OL14 | 14 |  -57.00  |  0.00141 |
| 21 | OL10 | 10 |  -60.00  |  0.00100 |
| 22 | OL06 |  6 |  -63.00  |  0.00071 |
| 23 | OL03 |  3 |  -65.25  |  0.00055 |
| 24 | OL01 |  1 |  -66.75  |  0.00046 |

> OL=6 以下は実機録音の SNR が低く、HW 側の誤差が大きくなる可能性がある。  
> SY-thy（ソフトシンセの理論誤差）が全 voice で ≤ 0.1 dB であれば実装は妥当。
