## 録音

```bash
cargo run -p xdx-compare --bin record-preset-dir -- calibration/eg_rr_calib --midi-out "UM-ONE" --audio-in "Neva Uno 1&2 (ESI Audio Device (WDM) - Neva Uno)"
```

## ソフトシンセによる波形生成

```bash
cargo run -p xdx-compare --bin record-preset-dir -- calibration/eg_rr_calib
```

## 比較

```bash
cargo run -p xdx-eg-viewer -- --dir calibration/eg_rr_calib
```

全体サマリー（全15音色 HW vs SY 比較）：

```bash
cargo run -p xdx-e2e --example compare_eg -- --dir calibration/eg_rr_calib --hold-ms 3000
```

RR05（voice 5）の詳細エンベロープ表示：

```bash
cargo run -p xdx-e2e --example compare_eg -- --dir calibration/eg_rr_calib --hold-ms 3000 --detail 5
```

## 音色レイアウト

固定パラメータ: AR=31, D1R=0, D1L=15, D2R=0
（ホールド中は level=1.0 で sustain → note-off から RR が decay 開始）

| # | Name | RR | hl (SW)   | rls90 (SW) |
|---|------|----|-----------|------------|
| 1 | RR01 |  1 | 3132 ms   |  NaN (>2s) |
| 2 | RR02 |  2 | 1631 ms   |  NaN (>2s) |
| 3 | RR03 |  3 |  850 ms   |  NaN (>2s) |
| 4 | RR04 |  4 |  444 ms   |  1474 ms   |
| 5 | RR05 |  5 |  231 ms   |   767 ms   |
| 6 | RR06 |  6 |  120 ms   |   399 ms   |
| 7 | RR07 |  7 |   63 ms   |   209 ms   |
| 8 | RR08 |  8 |   33 ms   |   109 ms   |
| 9 | RR09 |  9 |   17 ms   |    57 ms   |
|10 | RR10 | 10 |  8.9 ms   |    30 ms   |
|11 | RR11 | 11 |  4.6 ms   |    15 ms   |
|12 | RR12 | 12 |  2.4 ms   |     8 ms   |
|13 | RR13 | 13 |  1.3 ms   |     4 ms   |
|14 | RR14 | 14 |  0.7 ms   |     2 ms   |
|15 | RR15 | 15 |  0.3 ms   |     1 ms   |

> SW モデル: half-life = 0.000342 × 2^((15-RR)×0.94) 秒
> rls90 = half-life × log₂(10) ≈ half-life × 3.322
>
> D1R/D2R との違い: max=15（D1R/D2R は 31）、係数=0.000342（D1R/D2R は 0.000092）、
> 指数=0.94（D1R/D2R は 0.55）— DX100 実機測定から導出。
>
> 主要メトリクス: `rls90(HW)` vs `rls90(SY)` を比較。
> RR=1-3 は release=2.0 s 以内に完了しないため rls90 が NaN となる。
> RR=14,15 は rls90 < 10 ms（= WINDOW_MS）となり 0 ms と表示される場合がある。

## SYX 生成

```bash
cargo run -p xdx-e2e --example gen_rr_calib
```
