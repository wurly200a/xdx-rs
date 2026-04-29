# キャリブレーション・ワークフロー

> このドキュメントは「コードを読んでも分からない手順・判断基準」を記録するためのものです。

---

## 基本方針

xdx-synth の音声合成パラメータ（EG 時定数・スケーリング係数など）は、
**実機 DX100 の録音データを根拠として導出**しています。

「理論値を実装して後で調整する」のではなく、
「実機の振る舞いを測定し、その測定値から係数を逆算する」アプローチを取っています。
係数は理論的にきれいな値ではなく実測値であるため、
変更する場合は必ずこのドキュメントに記載のワークフローを再実施してください。

---

## ツール一覧

| ツール | 役割 | 入力 | 出力 |
|--------|------|------|------|
| `xdx-e2e` (examples) | テスト用バンク SysEx 生成 | なし | `.syx` バンクファイル |
| `xdx-compare record-eg-bank` | HW 録音 + ソフトシンセ一括レンダリング | `bank.syx`、実機接続 | `dx100/*.wav`, `synth/*.wav` |
| `xdx-compare compare_eg` | EG メトリクス差分テーブル出力 | `dx100/*.wav`, `synth/*.wav` ペア | 標準出力テーブル / ASCII 波形 |
| `xdx-eg-viewer` | EG 波形ビジュアル比較（GUI） | 同上ペア | GUI 表示 |
| `xdx-compare analyze-kbs-calib` | kbd スケーリング定量分析 | 複数ノートの WAV | 標準出力テーブル |
| `xdx-e2e gen_detune_calib` | デチューンキャリブレーション用 SysEx 生成 | なし | `testdata/syx/detune_calib.syx` |
| `xdx-e2e analyze_detune_calib` | ビート周波数からデチューン量を算出 | `<dir>/dx100/*.wav` | 標準出力テーブル |

---

## ワークフロー A — EG パラメータ（AR/D1R/D2R/RR/D1L）

### A-1. テストバンク生成

```bash
cargo run -p xdx-e2e --example gen_eg_test_bank
# → testdata/syx/eg_test_bank.syx（24 voice）
```

バンクの構成:
- **A: AR スイープ** (6 voice) — AR=5/10/15/20/25/31、他パラメータ固定
- **B: D1R スイープ** — D1L を中間値固定で D1R を変化
- **C: D1L スイープ** — D1R 固定で D1L の各段階
- **D: D2R スイープ** — D1L=0 にして D2R が支配的な条件
- **E: RR スイープ** — リリース速度の各段階
- **F: テンプレート** — Piano/Organ/Pluck（総合確認用）

### A-2. 実機に送信

xdx-gui の 32 VOICES パネルから `bank.syx` を開き、SysEx → **Send** で DX100 に転送。

### A-3. ハードウェア録音 + ソフトシンセレンダリング

```bash
cargo run -p xdx-compare --bin record-eg-bank -- \
  --bank testdata/syx/eg_test_bank.syx \
  --midi-out "（MIDI デバイス名）" \
  --audio-in "（オーディオデバイス名）" \
  --out testdata/wav/eg_bank
# → testdata/wav/eg_bank/dx100/01_NAME.wav ... 24 本
# → testdata/wav/eg_bank/synth/01_NAME.wav ... 24 本（ソフトシンセ同条件レンダリング）
```

**録音条件の固定事項（再現性のため）:**
- MIDI ノート: デフォルト 69 (A4)、変える場合は両者で同一
- velocity: 100 固定
- DX100 本体のマスターボリューム・EQ は変えない
- DAW や OS のエフェクトをバイパス

### A-4. 差分確認

```bash
# バッチ: 全ボイスのメトリクステーブル
cargo run -p xdx-compare --example compare_eg -- --dir testdata/wav/eg_bank

# 単体: ASCII 波形で詳細確認
cargo run -p xdx-compare --example compare_eg -- \
  testdata/wav/eg_bank/dx100/01_AR05.wav testdata/wav/eg_bank/synth/01_AR05.wav
```

出力される主なメトリクス:
- `atk90`: アタックが 90% に達するまでの時間 (ms)
- `d1l`: Decay-1 が落ち着いたサステインレベル（正規化）
- `rls50` / `rls90`: ノートオフから 50%/90% 減衰するまでの時間 (ms)

### A-5. GUI で波形を目視確認

```bash
cargo run -p xdx-eg-viewer -- --dir testdata/wav/eg_bank
```

HW（青）vs Synth（橙）を重ねて表示。
マッチング度は緑（良好）/黄（許容）/赤（乖離）で色分けされる。
数値が合っていても波形の形が違う場合はここで発見できる。

### A-6. 係数の調整

`xdx-synth/src/lib.rs` の以下の関数を修正:

| パラメータ | 関数 | 現在の係数 | 根拠 |
|-----------|------|-----------|------|
| AR | `rate_inc_t` | `0.000085`、指数 `0.55` | AR=20 → atk90 ≈ 5ms を実測から逆算 |
| D1R/D2R | `rate_mul` | coeff `0.000092`、指数 `0.55` | 複数レートでの半減期実測 |
| RR | `rate_mul` | coeff `0.0014`、指数 `0.55` | RR スイープ実測 |
| D1L | EG `init()` | `2^((d1l-15)*0.5)` (3dB/step) | ハードウェアマニュアル + 実測確認 |
| detune | `Note::init()` | `k=0.00469`、指数 `0.58` | ワークフロー E 参照 |

> **注意**: 係数を変更したら必ずバンク全体（24 voice）で再テストすること。
> 1 ボイスで合わせると他がずれる場合がある。

---

## ワークフロー B — キーボードスケーリング（kbd_lev_scl / kbd_rate_scl）

### B-1. テストバンク生成

```bash
cargo run -p xdx-e2e --example gen_kbs_calib
# → testdata/syx/kbs_calib.syx（8 voice）
```

バンクの構成:
- **Group A**: kbd_lev_scl スイープ（レベルスケーリングの強度変化）
- **Group B**: kbd_rate_scl スイープ（レートスケーリングの強度変化）

### B-2. 複数ノートで録音

```bash
cargo run -p xdx-compare --bin analyze-kbs-calib -- \
  --bank testdata/syx/kbs_calib.syx \
  --notes 48,60,72,84 \
  --midi-out "..." --audio-in "..." \
  --out testdata/wav/kbs_calib
```

C3/C4/C5/C6（48/60/72/84）の 4 ノートで各ボイスを録音。
ノート番号に対してレベル・レートがどう変化するかを測定する。

### B-3. 分析

分析結果から近似式を導出する（現在の実装式）:

```
kbd_lev_scl:
  kls_reduction = floor(kls * 2^(note/12) / 400)
  → out_level から kls_reduction を減算

kbd_rate_scl:
  effective_krs = krs * (krs+1) / 2   // 0,1,3,6
  rate_boost = round(effective_krs * note / 72)
  → AR/D1R/D2R/RR に rate_boost を加算（最大値にクランプ）
```

---

## ワークフロー C — 単一ボイスのクイック確認（xdx-compare）

個別のボイスを素早く確認したい場合（バンク全体を使わない）:

```bash
cargo run -p xdx-compare -- \
  path/to/voice.syx \
  --midi-out "..." --audio-in "..." \
  --note 60 --duration 2.0 --release 1.0 \
  --out testdata/wav/quick
# → testdata/wav/quick/dx100.wav, testdata/wav/quick/synth.wav
# → RMS・ピーク・レベルマッチゲイン(dB) を標準出力に表示
```

---

## 判断基準（どこまで合えば OK か）

厳密な基準は未定義ですが、現状の目安:

| メトリクス | 目標 | 現状 |
|-----------|------|------|
| `atk90` | ±20% 以内 | 主要レートで達成済み |
| `rls50` / `rls90` | ±20% 以内 | 達成済み |
| `d1l` | ±10% 以内 | 達成済み |
| 波形の形状（目視） | 大きく形が違わないこと | 要継続確認 |

---

## ワークフロー D — LFO パラメータ

### D-1. テストバンク生成

```bash
cargo run -p xdx-e2e --example gen_lfo_calib
# → testdata/syx/lfo_calib.syx（17 voice + padding 32 voices）
```

バンクの構成:
- **Group A**: lfo_speed スイープ (speed=0/16/33/50/66/83/99)、波形=TRI、PMD=99、PMS=7
- **Group B**: pitch-mod depth 測定 (PMD=50/99 × PMS=3/7)、speed=5
- **Group C**: amp-mod depth 測定 (AMD=99、AMS=1/2/3)、speed=5
- **Group D**: lfo_delay 測定 (delay=25/50/75)、speed=33、PMD=99、PMS=7

### D-2. 実機送信・録音

```bash
# 全グループまとめて録音（hold=8s）
cargo run -p xdx-compare --bin record-eg-bank --release -- \
  testdata/syx/lfo_calib.syx \
  --midi-out "UM-ONE" --audio-in "<device>" \
  --note 60 --hold 8.0 --release 1.0 --out testdata/wav/lfo_calib/grp_a
```

### D-3. 分析

```bash
cargo run -p xdx-compare --bin analyze-lfo-calib --release -- \
  --dir testdata/wav/lfo_calib/grp_a
```

### D-4. 2024年実測値（DX100 ハードウェア、note=60）

**Group A: lfo_speed → Hz**

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

**Group B: pitch-mod depth（speed=5、note=60）**

| PMD | PMS | 上昇(cents) | 下降(cents) | 備考 |
|----:|----:|------------:|------------:|------|
|  50 |   3 |        +7.6 |       -13.2 | |
|  99 |   3 |       +19.0 |       -22.5 | |
|  50 |   7 |      +262.9 |      -448.4 | |
|  99 |   7 |      +467.3 |      -959.3 | |

> 上下の非対称は TRI 波形 LFO の 8-bit 符号付き範囲 [-128, +127] の非対称に由来する可能性がある。
> 測定誤差が ±20-30% 程度含まれる。

**Group C: amp-mod depth（AMD=99、speed=5）**

| AMS | peak-trough dB | 備考 |
|----:|---------------:|------|
|   1 |          48 dB | AMD=99 ではいずれも飽和 |
|   2 |          48 dB | ノイズフロア限界 |
|   3 |          49 dB | |

> AMD=99 で AMS=1〜3 全て ≈ 48 dB（飽和）。AMS 間の差を測定するには AMD=30〜50 での再録音が必要。

**Group D: lfo_delay onset（speed=33、PMD=99、PMS=7）**

| delay | onset(ms) | delay エンコード値 | 備考 |
|------:|----------:|------------------:|------|
|    25 |       302 |              100  | |
|    50 |      1311 |              288  | |
|    75 |      3731 |              864  | delay=75 は 8s 録音でギリギリ検出 |

> delay エンコード: `a = (16 + (delay & 15)) << (1 + (delay >> 4))`
> onset は delay 終了後のランプアップ開始時点を閾値 10 cents で検出。

### D-5. LFO 実装時の係数

（実装後に埋める）

---

## ワークフロー E — デチューン係数

DX100 のデチューンは「何 Hz/step か」を直接録音から測定する。
デチューンされた 2 つのキャリアが同時に鳴るとビート（うなり）が生じ、
その周波数 = ステップ数 × デチューン量 で測定できる。

### E-1. テストバンク生成

```bash
cargo run -p xdx-e2e --example gen_detune_calib
# → testdata/syx/detune_calib.syx（9 voice）
#   Voice 1–6: ±1/±2/±3 step at A4 (440 Hz)
#   Voice 7–9: +1/+2/+3 step at A3 (220 Hz)  ← 周波数依存性の確認用
```

各ボイスは 2 キャリア構成（FM なし・LFO なし・フィードバックなし）:
- OP1: detune = 3（センター、リファレンス）
- OP2: detune = 3 ± step

### E-2. 実機録音

```bash
cargo run -p xdx-compare --bin record-eg-bank --release -- \
  testdata/syx/detune_calib.syx \
  --midi-out "UM-ONE" --audio-in "<device>" \
  --note 69 --hold 30.0 --release 0.5 \
  --out testdata/wav/detune_calib
```

**hold=30.0s が必要**。ビート周期は最大 10 秒程度になるため、
短い録音ではオートコリレーションが破綻する。

### E-3. 解析

```bash
cargo run -p xdx-e2e --example analyze_detune_calib -- testdata/wav/detune_calib
```

出力例:

```
Voice                Steps  Base Hz   Beat Hz    Hz/step  Cents/step
--------------------------------------------------------------------
01_+1_440Hz              1      440     0.1609     0.1609      0.6340
02_+2_440Hz              2      440     0.3211     0.1606      0.6327
03_+3_440Hz              3      440     0.4799     0.1600      0.6301
07_+1_220Hz              1      220     0.1074     0.1074      0.8446
08_+2_220Hz              2      220     0.2137     0.1069      0.8400
09_+3_220Hz              3      220     0.3181     0.1060      0.8331
```

### E-4. べき乗モデルの導出

Hz/step は周波数に対してべき乗で増加する（セント比例でも Hz 固定でもない）:

```
detune_hz = k × f^α × steps

α = log2(Hz440 / Hz220) / log2(440 / 220)
  = log2(0.160 / 0.107) / 1
  ≈ 0.58

k = Hz440 / 440^0.58
  = 0.160 / 34.1
  ≈ 0.00469
```

| キャリア Hz | 実測 Hz/step | モデル Hz/step |
|-----------:|-------------:|---------------:|
|     440 Hz |        0.160 |          0.160 |
|     220 Hz |        0.107 |          0.107 |

### E-5. 係数の更新

`xdx-synth/src/lib.rs` の `Note::init()` 内:

```rust
let op_hz = base_hz * ratio;
let detune_hz = (p.detune as f32 - 3.0) * 0.00469 * op_hz.powf(0.58);
op.freq = op_hz + detune_hz;
```

- `k`（`0.00469`）: E-4 の算式で再計算
- 指数（`0.58`）: 測定点が 2 点以上あれば回帰で再計算

> **注意**: `±1 step` のボイスはビート周期が長く (> 9s) 30 秒録音でも
> 検出精度が低い。係数の根拠には `±2` / `±3` step の値を優先すること。

---

## 未キャリブレーションのパラメータ

以下は現時点で「理論値 or 近似式」のみで実機測定未実施:

- `key_vel_sens`: ベロシティ感度（実装済み、精度未検証）
- `pitch_eg_rate/level`: ピッチ EG（未実装）
- LFO amp-mod (AMD < 99 での AMS スケーリング)

---

## 作業ディレクトリ構成（参考）

```
testdata/wav/
  eg_bank/
    dx100/   01_AR05.wav ... 24本
    synth/   01_AR05.wav ... 24本
  kbs_calib/
    dx100/   C3_01_KLS00.wav ...
    synth/   C3_01_KLS00.wav ...
  detune_calib/
    dx100/   01_+1_440Hz.wav ... 9本（analyze_detune_calib が読む）
  quick/
    dx100.wav
    synth.wav
```
