## 録音

```bash
cargo run -p xdx-compare --bin record-preset-dir -- calibration/eg_d1r_calib --midi-out "UM-ONE" --audio-in "Neva Uno 1&2 (ESI Audio Device (WDM) - Neva Uno)"
```

## ソフトシンセによる波形生成

```bash
cargo run -p xdx-compare --bin record-preset-dir -- calibration/eg_d1r_calib
```

## 比較

```bash
cargo run -p xdx-eg-viewer -- --dir calibration/eg_d1r_calib
```

全体サマリー（全24音色 HW vs SY 比較）：

```bash
cargo run -p xdx-e2e --example compare_eg -- --dir calibration/eg_d1r_calib --hold-ms 10000
```

D1R10（voice 10）の詳細エンベロープ表示：

```bash
cargo run -p xdx-e2e --example compare_eg -- --dir calibration/eg_d1r_calib --hold-ms 10000 --detail 10
```

## 音色レイアウト

固定パラメータ: AR=31, D1L=0, D2R=0, RR=15

| # | Name  | D1R | hl (SW)  | dcy90 (SW) | 備考 |
|---|-------|-----|----------|------------|------|
| 1 | D1R01 |   1 |  8527 ms | 28325 ms   | NaN (10s超) |
| 2 | D1R02 |   2 |  5824 ms | 19347 ms   | NaN (10s超) |
| 3 | D1R03 |   3 |  3978 ms | 13214 ms   | NaN (10s超) |
| 4 | D1R04 |   4 |  2717 ms |  9026 ms   | |
| 5 | D1R05 |   5 |  1856 ms |  6165 ms   | |
| 6 | D1R06 |   6 |  1268 ms |  4211 ms   | |
| 7 | D1R07 |   7 |   866 ms |  2876 ms   | |
| 8 | D1R08 |   8 |   591 ms |  1964 ms   | |
| 9 | D1R09 |   9 |   404 ms |  1342 ms   | |
|10 | D1R10 |  10 |   276 ms |   916 ms   | |
|11 | D1R11 |  11 |   188 ms |   626 ms   | |
|12 | D1R12 |  12 |   129 ms |   428 ms   | |
|13 | D1R13 |  13 |    88 ms |   292 ms   | |
|14 | D1R14 |  14 |    60 ms |   199 ms   | |
|15 | D1R15 |  15 |    41 ms |   136 ms   | |
|16 | D1R16 |  16 |    28 ms |    93 ms   | |
|17 | D1R17 |  17 |    19 ms |    64 ms   | |
|18 | D1R18 |  18 |    13 ms |    43 ms   | |
|19 | D1R19 |  19 |     9 ms |    30 ms   | |
|20 | D1R20 |  20 |     6 ms |    20 ms   | |
|21 | D1R22 |  22 |     3 ms |     9 ms   | |
|22 | D1R25 |  25 |   0.9 ms |     3 ms   | |
|23 | D1R28 |  28 |   0.3 ms |     1 ms   | |
|24 | D1R31 |  31 |   0.1 ms |   0.3 ms   | |

> SW モデル: half-life = 0.000092 × 2^((31-D1R)×0.55) 秒
> dcy90 = half-life × log₂(10) ≈ half-life × 3.322
>
> 主要メトリクス: `dcy90(HW)` vs `dcy90(SY)` を比較することで
> `rate_mul` の係数（0.000092）と指数（0.55）の精度を検証できる。
>
> D1R=1〜3 は hold=10s 内に dcy90 未到達（NaN）。
> `d1l` 列（ホールド末尾10%の平均レベル）で緩やかな減衰の様子は確認可能。

## SYX 生成

```bash
cargo run -p xdx-e2e --example gen_d1r_calib
```
