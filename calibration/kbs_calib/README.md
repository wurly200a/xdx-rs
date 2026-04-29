# キーボードスケーリング キャリブレーション

`kbd_lev_scl`（レベルスケーリング）と `kbd_rate_scl`（レートスケーリング）の
スケーリング式を複数ノートの録音から導出する。

---

## ディレクトリ構成

```
calibration/kbs_calib/
  kbs_calib.syx       ← キャリブレーション用バンク
  n48/dx100/          ← MIDI note 48 (C3) での DX100 録音
  n60/dx100/          ← MIDI note 60 (C4) での DX100 録音
  n72/dx100/          ← MIDI note 72 (C5) での DX100 録音
  n84/dx100/          ← MIDI note 84 (C6) での DX100 録音
```

---

## ステップ 1: テストバンク生成

```bash
cargo run -p xdx-e2e --example gen_kbs_calib
# testdata/syx/kbs_calib.syx に出力されるので calibration/kbs_calib/ に移動
```

バンクの構成:

| グループ | 内容 |
|---------|------|
| Group A | `kbd_lev_scl` スイープ（レベルスケーリングの強度変化） |
| Group B | `kbd_rate_scl` スイープ（レートスケーリングの強度変化） |

---

## ステップ 2: 複数ノートで録音

C3/C4/C5/C6（MIDI note 48/60/72/84）の 4 ノートで各ボイスを録音する。

```bash
cargo run -p xdx-compare --bin analyze-kbs-calib --release -- \
  --bank calibration/kbs_calib/kbs_calib.syx \
  --notes 48,60,72,84 \
  --midi-out "<MIDI デバイス名>" \
  --audio-in "<オーディオデバイス名>" \
  --out calibration/kbs_calib
# → calibration/kbs_calib/n48/dx100/*.wav
# → calibration/kbs_calib/n60/dx100/*.wav
# → calibration/kbs_calib/n72/dx100/*.wav
# → calibration/kbs_calib/n84/dx100/*.wav
```

---

## ステップ 3: 分析・係数導出

ノート番号に対してレベル・レートがどう変化するかを測定し、近似式を導出する。

現在の実装式:

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

## 係数の更新箇所

`xdx-synth/src/lib.rs` の `Note::init()` 内のスケーリング計算部分を修正する。
