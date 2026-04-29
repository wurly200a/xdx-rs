# デチューン係数 キャリブレーション

DX100 のデチューン 1 ステップあたりの Hz オフセットを実機録音から測定し、
周波数依存モデルの係数を導出する。

デチューンされた 2 つのキャリアが同時に鳴るとビート（うなり）が生じる。
その周波数 = ステップ数 × デチューン量 で測定できる。

---

## ディレクトリ構成

```
calibration/detune_calib/
  detune_calib.syx    ← キャリブレーション用 9-voice バンク
  dx100/              ← DX100 ハードウェア録音（各 30 秒）
```

---

## ステップ 1: テストバンク生成

```bash
cargo run -p xdx-e2e --example gen_detune_calib
# testdata/syx/detune_calib.syx に出力されるので calibration/detune_calib/ に移動
```

バンクの構成（9 voice）:

| Voice | step | carrier Hz | 目的 |
|-------|-----:|-----------:|------|
| 1 | +1 | 440 Hz | |
| 2 | +2 | 440 Hz | 係数導出の主データ |
| 3 | +3 | 440 Hz | |
| 4 | −1 | 440 Hz | |
| 5 | −2 | 440 Hz | 符号確認 |
| 6 | −3 | 440 Hz | |
| 7 | +1 | 220 Hz | |
| 8 | +2 | 220 Hz | 周波数依存性確認 |
| 9 | +3 | 220 Hz | |

各ボイスは 2 キャリア構成（FM なし・LFO なし・フィードバックなし）:
- OP1: `detune = 3`（センター、リファレンス）
- OP2: `detune = 3 ± step`

---

## ステップ 2: 実機録音

```bash
cargo run -p xdx-compare --bin record-eg-bank --release -- \
  calibration/detune_calib/detune_calib.syx \
  --midi-out "<MIDI デバイス名>" \
  --audio-in "<オーディオデバイス名>" \
  --note 69 --hold 30.0 --release 0.5 \
  --out calibration/detune_calib
# → calibration/detune_calib/dx100/01_+1_440Hz.wav ... 9 本
```

**`--hold 30.0` が必須**。±1 step のビート周期は最大 10 秒程度になるため、
短い録音ではオートコリレーションが破綻する。

---

## ステップ 3: 解析

```bash
cargo run -p xdx-e2e --example analyze_detune_calib -- calibration/detune_calib
```

出力例:

```
Voice                Steps  Base Hz   Beat Hz    Hz/step  Cents/step
--------------------------------------------------------------------
01_+1_440Hz              1      440     0.1609     0.1609      0.6340
02_+2_440Hz              2      440     0.3211     0.1606      0.6327
03_+3_440Hz              3      440     0.4799     0.1600      0.6301
04_-1_440Hz             -1      440     0.1601     0.1601      0.6308
05_-2_440Hz             -2      440     0.3198     0.1599      0.6299
06_-3_440Hz             -3      440     0.4789     0.1596      0.6289
07_+1_220Hz              1      220     0.1074     0.1074      0.8446
08_+2_220Hz              2      220     0.2137     0.1069      0.8400
09_+3_220Hz              3      220     0.3181     0.1060      0.8331
```

> `±1 step` はビート周期が長く 30 秒録音でも検出精度が低い。
> 係数導出には `±2` / `±3` step の値を優先すること。

---

## ステップ 4: べき乗モデルの導出

Hz/step は周波数に対してべき乗で増加する（セント比例でも Hz 固定でもない）:

```
detune_hz = k × f^α × steps

α = log2(Hz_440 / Hz_220) / log2(440 / 220)
  = log2(0.160 / 0.107) / 1
  ≈ 0.58

k = Hz_440 / 440^0.58
  = 0.160 / 34.1
  ≈ 0.00469
```

| キャリア Hz | 実測 Hz/step | モデル Hz/step |
|-----------:|-------------:|---------------:|
|     440 Hz |        0.160 |          0.160 |
|     220 Hz |        0.107 |          0.107 |

測定点を増やす（例: 880 Hz での録音を追加）場合は最小二乗回帰で α と k を再計算する。

---

## ステップ 5: 係数の更新

`xdx-synth/src/lib.rs` の `Note::init()` 内:

```rust
let op_hz = base_hz * ratio;
let detune_hz = (p.detune as f32 - 3.0) * 0.00469 * op_hz.powf(0.58);
op.freq = op_hz + detune_hz;
```

- `k`（`0.00469`）: ステップ 4 の算式で再計算
- 指数（`0.58`）: 測定点が 3 点以上あれば回帰で精度向上可能
