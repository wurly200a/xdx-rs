# キャリブレーション・ワークフロー

> このドキュメントは「コードを読んでも分からない手順・判断基準」を記録するためのものです。
> 各キャリブレーションの詳細手順は、それぞれのサブディレクトリの README.md を参照してください。

---

## 基本方針

xdx-synth の音声合成パラメータ（EG 時定数・スケーリング係数など）は、
**実機 DX100 の録音データを根拠として導出**しています。

「理論値を実装して後で調整する」のではなく、
「実機の振る舞いを測定し、その測定値から係数を逆算する」アプローチを取っています。
係数は理論的にきれいな値ではなく実測値であるため、
変更する場合は必ずそれぞれの README.md に記載のワークフローを再実施してください。

---

## ツール一覧

| ツール | 役割 | 入力 | 出力 |
|--------|------|------|------|
| `xdx-e2e gen_eg_test_bank` | EG キャリブレーション用 SysEx 生成 | なし | `eg_test_bank.syx` |
| `xdx-e2e gen_kbs_calib` | KBS キャリブレーション用 SysEx 生成 | なし | `kbs_calib.syx` |
| `xdx-e2e gen_lfo_calib` | LFO キャリブレーション用 SysEx 生成 | なし | `lfo_calib.syx` |
| `xdx-e2e gen_detune_calib` | デチューンキャリブレーション用 SysEx 生成 | なし | `detune_calib.syx` |
| `xdx-compare record-eg-bank` | HW 録音 + ソフトシンセ一括レンダリング | `bank.syx`、実機接続 | `dx100/*.wav`, `synth/*.wav` |
| `xdx-compare compare_eg` | EG メトリクス差分テーブル出力 | `dx100/*.wav`, `synth/*.wav` ペア | 標準出力テーブル / ASCII 波形 |
| `xdx-eg-viewer` | EG 波形ビジュアル比較（GUI） | 同上ペア | GUI 表示 |
| `xdx-compare analyze-kbs-calib` | kbd スケーリング定量分析 | 複数ノートの WAV | 標準出力テーブル |
| `xdx-e2e analyze_detune_calib` | ビート周波数からデチューン量を算出 | `<dir>/dx100/*.wav` | 標準出力テーブル |

> **注意**: `gen_*` コマンドは `testdata/syx/` に出力します。生成後は対応する
> `calibration/<dir>/` にファイルを移動してください。

---

## キャリブレーション一覧

| # | 対象パラメータ | ディレクトリ | 詳細手順 |
|---|---|---|---|
| A | AR / D1R / D2R / RR / D1L | [eg_bank/](eg_bank/) | [eg_bank/README.md](eg_bank/README.md) |
| B | kbd_lev_scl / kbd_rate_scl | [kbs_calib/](kbs_calib/) | [kbs_calib/README.md](kbs_calib/README.md) |
| C | lfo_speed / PMD / AMD / lfo_delay | [lfo_calib/](lfo_calib/) | [lfo_calib/README.md](lfo_calib/README.md) |
| D | detune (k / 指数) | [detune_calib/](detune_calib/) | [detune_calib/README.md](detune_calib/README.md) |

---

## 現在の係数一覧

`xdx-synth/src/lib.rs` で管理している実測係数:

| パラメータ | 現在の係数 | 詳細 |
|-----------|-----------|------|
| AR | coeff `0.000085`、指数 `0.55` | [eg_bank/README.md](eg_bank/README.md) |
| D1R/D2R | coeff `0.000092`、指数 `0.55` | [eg_bank/README.md](eg_bank/README.md) |
| RR | coeff `0.0014`、指数 `0.55` | [eg_bank/README.md](eg_bank/README.md) |
| D1L | `2^((d1l-15)*0.5)` (3dB/step) | ハードウェアマニュアル + 実測確認 |
| detune | `k=0.00469`、指数 `0.58` | [detune_calib/README.md](detune_calib/README.md) |

> **注意**: 係数を変更したら必ずバンク全体（24 voice）で再テストすること。
> 1 ボイスで合わせると他がずれる場合がある。

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

## クイック確認（単一ボイス）

個別のボイスを素早く確認したい場合（バンク全体を使わない）:

```bash
cargo run -p xdx-compare -- \
  path/to/voice.syx \
  --midi-out "..." --audio-in "..." \
  --note 60 --duration 2.0 --release 1.0 \
  --out calibration/quick
# → calibration/quick/dx100.wav, calibration/quick/synth.wav
# → RMS・ピーク・レベルマッチゲイン(dB) を標準出力に表示
```

---

## 未キャリブレーションのパラメータ

以下は現時点で「理論値 or 近似式」のみで実機測定未実施:

- `key_vel_sens`: ベロシティ感度（実装済み、精度未検証）
- `pitch_eg_rate/level`: ピッチ EG（未実装）
- LFO amp-mod (AMD < 99 での AMS スケーリング)
