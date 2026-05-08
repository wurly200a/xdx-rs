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

| # | Name | RR | hl (SW)  | rls90 (SW) |
|---|------|----|----------|------------|
| 1 | RR01 |  1 |  291 ms  |   967 ms   |
| 2 | RR02 |  2 |  199 ms  |   661 ms   |
| 3 | RR03 |  3 |  136 ms  |   451 ms   |
| 4 | RR04 |  4 |   93 ms  |   308 ms   |
| 5 | RR05 |  5 |   63 ms  |   211 ms   |
| 6 | RR06 |  6 |   43 ms  |   144 ms   |
| 7 | RR07 |  7 |   30 ms  |    98 ms   |
| 8 | RR08 |  8 |   20 ms  |    67 ms   |
| 9 | RR09 |  9 |   14 ms  |    46 ms   |
|10 | RR10 | 10 |    9 ms  |    31 ms   |
|11 | RR11 | 11 |    6 ms  |    21 ms   |
|12 | RR12 | 12 |    4 ms  |    15 ms   |
|13 | RR13 | 13 |    3 ms  |    10 ms   |
|14 | RR14 | 14 |    2 ms  |     7 ms   |
|15 | RR15 | 15 |  1.4 ms  |     5 ms   |

> SW モデル: half-life = 0.0014 × 2^((15-RR)×0.55) 秒
> rls90 = half-life × log₂(10) ≈ half-life × 3.322
>
> D1R/D2R との違い: max=15（D1R/D2R は 31）、係数=0.0014（D1R/D2R は 0.000092）
>
> 主要メトリクス: `rls90(HW)` vs `rls90(SY)` を比較。
> 全15値が release=2.0 s 以内に完了するため NaN は発生しない。
> RR=14,15 は rls90 < 10 ms（= WINDOW_MS）となり 0 ms と表示される場合がある。

## SYX 生成

```bash
cargo run -p xdx-e2e --example gen_rr_calib
```
