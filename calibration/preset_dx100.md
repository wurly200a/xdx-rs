# preset_dx100 録音・比較ワークフロー

`calibration/preset_dx100_1` 〜 `preset_dx100_8` に格納された DX100 実機バンクを対象に、
SysEx 転送・実機録音・ソフトシンセ生成・波形比較を行う手順。

---

## ディレクトリ構成

```
calibration/
  preset_dx100_1/
    dx100_1.syx       ← 実機バンク（32 voice）
    record.json       ← 録音パラメータ設定
    dx100/            ← 実機録音 WAV（record-preset-dir が生成）
    synth/            ← ソフトシンセ WAV（record-preset-dir が生成）
  preset_dx100_2/
    ...（同構成）
  ...
  preset_dx100_8/
    ...（同構成）
```

---

## record.json フィールド

各ディレクトリの `record.json` でプリセットごとの録音パラメータを設定できます。
すべてのフィールドは省略可能（省略時はデフォルト値が使われます）。

| フィールド | デフォルト | 説明 |
|-----------|-----------|------|
| `syx`     | ディレクトリ内の最初の `*.syx` | SysEx ファイル名 |
| `note`    | `69`（A4） | 録音に使う MIDI ノート番号 |
| `hold`    | `3.0` | ノートを押す時間（秒）|
| `release` | `3.0` | ノートオフ後のリリース録音時間（秒）|
| `count`   | 全 32 voice | 録音する voice 数（先頭 N 個） |

パッド系のプリセットで hold を長くしたい場合の例:

```json
{
  "note": 69,
  "hold": 6.0,
  "release": 4.0
}
```

---

## ステップ 1: デバイス名の確認

```bash
cargo run -p xdx-compare --bin record-preset-dir -- --list
```

出力例:

```
=== MIDI OUT ports ===
  0: YAMAHA DX100

=== Audio INPUT devices ===
  0: マイク配列 (Realtek High Definition Audio)  [44100Hz 2ch]
  1: ライン入力 (BEHRINGER USB WDM AUDIO)        [44100Hz 2ch]
```

以降のコマンドでは `--midi-out` と `--audio-in` にこの名前を指定します。

---

## ステップ 2: SysEx 転送のみ（録音なし）

`--audio-in` を省略すると SysEx 転送とソフトシンセ生成のみ実行されます。
DX100 に最新バンクを書き込みたいだけの場合や、ソフトシンセとの比較を先に確認したい場合に使います。

```bash
cargo run -p xdx-compare --bin record-preset-dir -- \
  calibration/preset_dx100_1 \
  --midi-out "YAMAHA DX100"
```

---

## ステップ 3: SysEx 転送 + 実機録音 + ソフトシンセ生成（フル）

```bash
cargo run -p xdx-compare --bin record-preset-dir -- \
  calibration/preset_dx100_1 \
  --midi-out "YAMAHA DX100" \
  --audio-in "ライン入力 (BEHRINGER USB WDM AUDIO)"
```

実行後の出力:

```
=== record-preset-dir ===
Dir:      calibration/preset_dx100_1
SysEx:    calibration/preset_dx100_1/dx100_1.syx
Note:     69  channel: 1
Timing:   3.0s hold + 3.0s release  (32 voices, ~211s total)

Sent bank SysEx (4104 bytes) — waiting 600ms for DX100 to load…

[ 1/32]  IvoryEbony   (est. 211s remaining)
         synth → calibration/preset_dx100_1/synth/01_IvoryEbony.wav
         dx100 → calibration/preset_dx100_1/dx100/01_IvoryEbony.wav
...
Done. 32 voices processed.
```

---

## ステップ 4: 全ディレクトリ一括録音（bash）

```bash
MIDI_OUT="YAMAHA DX100"
AUDIO_IN="ライン入力 (BEHRINGER USB WDM AUDIO)"

for i in 1 2 3 4 5 6 7 8; do
  cargo run -p xdx-compare --bin record-preset-dir -- \
    "calibration/preset_dx100_${i}" \
    --midi-out "${MIDI_OUT}" \
    --audio-in "${AUDIO_IN}"
done
```

32 voice × 8 ディレクトリ = 256 録音。デフォルト設定で約 28 分かかります。

---

## ステップ 5: 波形比較

録音完了後、`xdx-eg-viewer` で波形を目視比較できます。

```bash
cargo run -p xdx-eg-viewer -- calibration/preset_dx100_1
```

定量メトリクスの差分テーブルを出力する場合:

```bash
cargo run -p xdx-compare -- \
  calibration/preset_dx100_1/dx100 \
  calibration/preset_dx100_1/synth
```
