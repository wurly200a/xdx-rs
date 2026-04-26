# xdx-synth パラメータ実装状況

> **注意**: このドキュメントは日本語で記載しています。  
> プロジェクトの原則は英語または英語＋日本語併記ですが、このファイルは  
> 実装完了後に削除予定の作業メモのため日本語のままとしています。

最終更新: 2026-04-26 (ar smoothstep S字カーブ実装)

---

## ほぼ完成 (90〜100%)

### Dx100Operator（オペレーター単位）

| パラメータ | 型 | 実装箇所 | 状態 | 備考 |
|---|---|---|---|---|
| `ar` Attack Rate | 0-31 | `rate_inc_t` → smoothstep S-curve | ✅ | キャリブ済・S字カーブ実装 |
| `d1r` Decay1 Rate | 0-31 | `rate_mul(op.d1r, 31, 0.000092, sr)` | ✅ | キャリブ済 |
| `d2r` Decay2 Rate | 0-31 | `rate_mul(op.d2r, 31, 0.000092, sr)` | ✅ | キャリブ済 |
| `rr` Release Rate | 0-15 | `rate_mul(op.rr, 15, 0.0014, sr)` | ✅ | キャリブ済 |
| `d1l` Decay1 Level | 0-15 | `2^((d1l-15)*0.5)` | ✅ | 3dB/step キャリブ済 |
| `out_level` 出力レベル | 0-99 | `level_to_amp()` 0.75dB/step | ✅ | |
| `freq_ratio` 周波数比 | 0-63 | `FREQ_RATIOS[64]` テーブル参照 | ✅ | |
| `detune` デチューン | 0-6 (中心=3) | `(val - 3) * 3` cents | ✅ | |
| `key_vel_sens` ベロシティ感度 | 0-7 | `vel_factor` 計算 | ⚠️ | 実装済・対ハードウェア精度未検証 |
| `kbd_lev_scl` キーボードレベルスケーリング | 0-99 | `Note::start()` — kls_reduction | ✅ | キャリブ済: `floor(kls * 2^(note/12) / 400)` |
| `kbd_rate_scl` キーボードレートスケーリング | 0-3 | `Envelope::init()` — rate_boost | ✅ | キャリブ済: `round(krs*(krs+1)/2 * note / 72)` |

### Dx100Voice（ボイス全体）

| パラメータ | 型 | 実装箇所 | 状態 | 備考 |
|---|---|---|---|---|
| `algorithm` アルゴリズム | 0-7 | `render_sample()` 8種 | ✅ | |
| `feedback` フィードバック | 0-7 | `feedback_depth()` | ✅ | |
| `transpose` | 0-48 (中心=24) | `midi_to_hz()` | ✅ | |
| `poly_mono` (poly側) | 0=poly | 複数ノート管理 | ⚠️ | mono側未実装 |

---

## 未実装 (0%) — 優先度別

### P1: EG比較精度に直接影響（差の主因候補）

| パラメータ | 型 | 効果 | 優先理由 |
|---|---|---|---|
| ~~`kbd_lev_scl`~~ | ~~0-99~~ | ~~ノート番号でOp出力レベルをスケール~~ | ✅ 実装済（近似式・要キャリブ） |
| ~~`kbd_rate_scl`~~ | ~~0-3~~ | ~~ノート番号でEGレートを加速~~ | ✅ 実装済（近似式・要キャリブ） |
| `pitch_eg_rate[3]` | 0-99 × 3 | ピッチEG（3ステージ）のレート | ピッチが時間変化する音色の必須要件 |
| `pitch_eg_level[3]` | 0-99 × 3 | ピッチEGのレベル | 同上 |

### P2: 聴感上の大きな差（音色の個性）

| パラメータ | 型 | 効果 |
|---|---|---|
| `lfo_speed` | 0-99 | LFO速度 |
| `lfo_delay` | 0-99 | ノートオン後LFO開始までの遅延 |
| `lfo_wave` | 0-3 | LFO波形（三角波/鋸歯波/矩形波/S&H） |
| `lfo_sync` | 0-1 | ノートオン毎にLFO位相リセット |
| `lfo_pmd` + `pitch_mod_sens` | 0-99 / 0-7 | ビブラート（LFO→ピッチ変調） |
| `lfo_amd` + `amp_mod_sens` + `amp_mod_en` (Op) | 0-99 / 0-3 / 0-1 | トレモロ（LFO→振幅変調） |
| `chorus` | 0-1 | DX100内蔵コーラス（BBDアナログ） ※正確なエミュは難度高 |

### P3: 演奏機能（リアルタイム操作）

| パラメータ | 型 | 効果 |
|---|---|---|
| `pb_range` | 0-12 | ピッチベンド幅（MIDI Pitch Bend CCへの応答が必要） |
| `poly_mono` (mono側) | 1=mono | モノフォニック動作・レガート |
| `porta_mode` | 0-1 | ポルタメントモード（フィンガード/フル） |
| `porta_time` | 0-99 | ポルタメント時間 |

### P4: 外部コントローラー依存（録音・比較テストには不要）

| パラメータ | 型 | 効果 |
|---|---|---|
| `mw_pitch` | 0-99 | モジュレーションホイール→ピッチ |
| `mw_amplitude` | 0-99 | モジュレーションホイール→振幅 |
| `bc_pitch` | 0-99 | ブレスコントローラー→ピッチ |
| `bc_amplitude` | 0-99 | ブレスコントローラー→振幅 |
| `bc_pitch_bias` | 0-99 | ブレスコントローラー→ピッチバイアス |
| `bc_eg_bias` | 0-99 | ブレスコントローラー→EGバイアス |
| `eg_bias_sens` (Operator) | 0-7 | BC EGバイアス感度（Op単位） |
| `fc_volume` | 0-99 | フットコントローラー音量 |
| `sustain` | 0-1 | サステインフットスイッチ |
| `portamento` | 0-1 | ポルタメントフットスイッチ |

---

## 実装ロードマップ

```
Step 1  ✅ kbd_lev_scl / kbd_rate_scl  (実装済・近似式。要ハードウェアキャリブ)
Step 2  pitch_eg_rate / pitch_eg_level ← ピッチ変化音色への対応
Step 3  LFOシステム一式              ← まとめて実装（相互依存が多い）
Step 4  chorus                       ← 簡易コーラスから始める
Step 5  pb_range / poly_mono(mono) / portamento
Step 6  P4（外部コントローラー系）    ← 必要性が出たら対応
```
