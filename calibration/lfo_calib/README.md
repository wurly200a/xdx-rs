# LFO キャリブレーション

`lfo_speed` の Hz マッピング、ピッチ変調深度（PMD/PMS）、振幅変調深度（AMD/AMS）、
`lfo_delay` の onset 時間を実機録音から測定する。

---

## ディレクトリ構成

```
calibration/lfo_calib/
  lfo_calib.syx      ← キャリブレーション用バンク（17 voice + padding）
  grp_a/dx100/       ← DX100 ハードウェア録音（全グループまとめて録音）
```

---

## ステップ 1: テストバンク生成

```bash
cargo run -p xdx-e2e --example gen_lfo_calib
# testdata/syx/lfo_calib.syx に出力されるので calibration/lfo_calib/ に移動
```

バンクの構成:

| グループ | Voice | 内容 |
|---------|-------|------|
| A | 1–7 | `lfo_speed` スイープ (0/16/33/50/66/83/99)、波形=TRI、PMD=99、PMS=7 |
| B | 8–11 | pitch-mod depth 測定 (PMD=50/99 × PMS=3/7)、speed=5 |
| C | 12–14 | amp-mod depth 測定 (AMD=99、AMS=1/2/3)、speed=5 |
| D | 15–17 | `lfo_delay` 測定 (delay=25/50/75)、speed=33、PMD=99、PMS=7 |

---

## ステップ 2: 実機録音

```bash
cargo run -p xdx-compare --bin record-eg-bank --release -- \
  calibration/lfo_calib/lfo_calib.syx \
  --midi-out "<MIDI デバイス名>" \
  --audio-in "<オーディオデバイス名>" \
  --note 60 --hold 8.0 --release 1.0 \
  --out calibration/lfo_calib/grp_a
# → calibration/lfo_calib/grp_a/dx100/*.wav
```

---

## ステップ 3: 分析

```bash
cargo run -p xdx-compare --bin analyze-lfo-calib --release -- \
  --dir calibration/lfo_calib/grp_a
```

---

## 実測値（DX100 ハードウェア、note=60）

### Group A: lfo_speed → Hz

| speed | 実測 Hz | DX7 参照 Hz | 備考 |
|------:|--------:|------------:|------|
|     0 |       — |       0.063 | 8s では検出不可（period ~16s） |
|    16 |    1.51 |        2.56 | |
|    33 |    6.18 |        5.32 | |
|    50 |   13.21 |        7.99 | |
|    66 |   24.85 |       12.56 | |
|    83 |   39.25 |       29.05 | |
|    99 |   52.95 |       49.26 | |

> DX100 の LFO speed テーブルは DX7 (DEXED) とは大きく異なる。DX7 参照値は実装には使用しない。

### Group B: pitch-mod depth（speed=5、note=60）

| PMD | PMS | 上昇(cents) | 下降(cents) | 備考 |
|----:|----:|------------:|------------:|------|
|  50 |   3 |        +7.6 |       -13.2 | |
|  99 |   3 |       +19.0 |       -22.5 | |
|  50 |   7 |      +262.9 |      -448.4 | |
|  99 |   7 |      +467.3 |      -959.3 | |

> 上下の非対称は TRI 波形 LFO の 8-bit 符号付き範囲 [-128, +127] の非対称に由来する可能性がある。
> 測定誤差が ±20–30% 程度含まれる。

### Group C: amp-mod depth（AMD=99、speed=5）

| AMS | peak-trough dB | 備考 |
|----:|---------------:|------|
|   1 |          48 dB | AMD=99 ではいずれも飽和 |
|   2 |          48 dB | ノイズフロア限界 |
|   3 |          49 dB | |

> AMD=99 で AMS=1〜3 全て ≈ 48 dB（飽和）。
> AMS 間の差を測定するには AMD=30〜50 での再録音が必要。

### Group D: lfo_delay onset（speed=33、PMD=99、PMS=7）

| delay | onset(ms) | delay エンコード値 | 備考 |
|------:|----------:|------------------:|------|
|    25 |       302 |               100 | |
|    50 |      1311 |               288 | |
|    75 |      3731 |               864 | delay=75 は 8s 録音でギリギリ検出 |

> delay エンコード: `a = (16 + (delay & 15)) << (1 + (delay >> 4))`
> onset は delay 終了後のランプアップ開始時点を閾値 10 cents で検出。

---

## 係数の更新箇所

LFO 実装完了後に `xdx-synth/src/lib.rs` の LFO 演算部分を更新する。
（実装後に埋める）
