# EG パラメータ キャリブレーション

AR / D1R / D2R / RR / D1L の時定数・スケーリング係数を実機録音から導出する。

---

## ディレクトリ構成

```
calibration/eg_bank/
  eg_test_bank.syx       ← キャリブレーション用 24-voice バンク
  dx100/                 ← DX100 ハードウェア録音
  synth/                 ← ソフトシンセ同条件レンダリング
```

---

## ステップ 1: テストバンク生成

```bash
cargo run -p xdx-e2e --example gen_eg_test_bank
# testdata/syx/eg_test_bank.syx に出力されるので calibration/eg_bank/ に移動
```

バンクの構成:

| グループ | Voice | 内容 |
|---------|-------|------|
| A | 1–6 | AR スイープ (AR=5/10/15/20/25/31)、他パラメータ固定 |
| B | 7–10 | D1R スイープ — D1L を中間値固定で D1R を変化 |
| C | 11–14 | D1L スイープ — D1R 固定で D1L の各段階 |
| D | 15–18 | D2R スイープ — D1L=0 にして D2R が支配的な条件 |
| E | 19–21 | RR スイープ — リリース速度の各段階 |
| F | 22–24 | テンプレート — Piano/Organ/Pluck（総合確認用） |

---

## ステップ 2: 実機に送信

xdx-gui の 32 VOICES パネルから `calibration/eg_bank/eg_test_bank.syx` を開き、
SysEx → **Send** で DX100 に転送する。

---

## ステップ 3: ハードウェア録音 + ソフトシンセレンダリング

```bash
cargo run -p xdx-compare --bin record-eg-bank --release -- \
  calibration/eg_bank/eg_test_bank.syx \
  --midi-out "<MIDI デバイス名>" \
  --audio-in "<オーディオデバイス名>" \
  --out calibration/eg_bank
# → calibration/eg_bank/dx100/01_AR05.wav ... 24 本
# → calibration/eg_bank/synth/01_AR05.wav ... 24 本
```

**録音条件の固定事項（再現性のため）:**
- MIDI ノート: デフォルト 69 (A4)
- velocity: 100 固定
- DX100 本体のマスターボリューム・EQ は変えない
- DAW や OS のエフェクトをバイパス

---

## ステップ 4: 差分確認

```bash
# バッチ: 全ボイスのメトリクステーブル
cargo run -p xdx-compare --example compare_eg -- --dir calibration/eg_bank

# 単体: ASCII 波形で詳細確認
cargo run -p xdx-compare --example compare_eg -- \
  calibration/eg_bank/dx100/01_AR05.wav \
  calibration/eg_bank/synth/01_AR05.wav
```

出力される主なメトリクス:

- `atk90`: アタックが 90% に達するまでの時間 (ms)
- `d1l`: Decay-1 が落ち着いたサステインレベル（正規化）
- `rls50` / `rls90`: ノートオフから 50%/90% 減衰するまでの時間 (ms)

---

## ステップ 5: GUI で波形を目視確認

```bash
cargo run -p xdx-eg-viewer --release -- --dir calibration/eg_bank
```

HW（青）vs Synth（橙）を重ねて表示。
マッチング度は緑（良好）/黄（許容）/赤（乖離）で色分けされる。
数値が合っていても波形の形が違う場合はここで発見できる。

---

## ステップ 6: 係数の調整

`xdx-synth/src/lib.rs` の以下の関数を修正:

| パラメータ | 関数 | 現在の係数 | 根拠 |
|-----------|------|-----------|------|
| AR | `rate_inc_t` | coeff `0.000085`、指数 `0.55` | AR=20 → atk90 ≈ 5ms を実測から逆算 |
| D1R/D2R | `rate_mul` | coeff `0.000092`、指数 `0.55` | 複数レートでの半減期実測 |
| RR | `rate_mul` | coeff `0.0014`、指数 `0.55` | RR スイープ実測 |
| D1L | EG `init()` | `2^((d1l-15)*0.5)` (3dB/step) | ハードウェアマニュアル + 実測確認 |

係数を変更したら必ずバンク全体（24 voice）で再テストすること。
