# DX100 SysEx プロトコル仕様

## 共通フォーマット

すべての DX100 SysEx は YAMAHA 形式に従う。

```
F0 43 0n <format> <byte_count_hi> <byte_count_lo> <payload...> <checksum> F7
```

- `n`: MIDI チャンネル (0-15)
- `byte_count`: 7-bit エンコード（`(hi << 7) | lo`）
- `checksum`: `((!sum_of_payload) + 1) & 0x7F`

## リクエスト（シンセへのダンプ要求）

```
F0 43 20 03 F7   ← 1-voice ダンプ要求
F0 43 20 04 F7   ← 32-voice ダンプ要求
```

---

## 1-voice SysEx (VCED)

### ヘッダー

```
F0 43 0n 03 00 5D <93 bytes payload> <checksum> F7
```

- 総バイト数: 101
- フォーマットバイト: `0x03`
- バイトカウント: `0x00 5D` = 93

### ペイロードレイアウト（93 バイト）

**OP ブロック（13 バイト × 4）— SysEx 上の並び順**

| オフセット | OP |
|------------|-----|
| 0-12  | OP4 |
| 13-25 | OP2 |
| 26-38 | OP3 |
| 39-51 | OP1 |

各 OP ブロック内のバイト配置:

| +0 | +1 | +2 | +3 | +4 | +5 | +6 | +7 | +8 | +9 | +10 | +11 | +12 |
|----|----|----|----|----|----|----|----|----|-----|-----|-----|-----|
| ar | d1r| d2r| rr | d1l|kbd_lev_scl|kbd_rate_scl|eg_bias_sens|amp_mod_en|key_vel_sens|out_level|freq_ratio|detune|

**グローバルパラメータ（オフセット 52-92）**

| オフセット | パラメータ |
|------------|-----------|
| 52 | algorithm (0-7) |
| 53 | feedback (0-7) |
| 54 | lfo_speed (0-99) |
| 55 | lfo_delay (0-99) |
| 56 | lfo_pmd (0-99) |
| 57 | lfo_amd (0-99) |
| 58 | lfo_sync (0-1) |
| 59 | lfo_wave (0-3: SAW/SQU/TRI/S&H) |
| 60 | pitch_mod_sens (0-7) |
| 61 | amp_mod_sens (0-3) |
| 62 | transpose (0-48, center=24) |
| 63 | poly_mono (0=POLY, 1=MONO) |
| 64 | pb_range (0-12) |
| 65 | porta_mode (0=Full, 1=Fing) |
| 66 | porta_time (0-99) |
| 67 | fc_volume (0-99) |
| 68 | sustain (0-1) |
| 69 | portamento foot sw (0-1) |
| 70 | chorus (0-1) |
| 71 | mw_pitch (0-99) |
| 72 | mw_amplitude (0-99) |
| 73 | bc_pitch (0-99) |
| 74 | bc_amplitude (0-99) |
| 75 | bc_pitch_bias (0-99) |
| 76 | bc_eg_bias (0-99) |
| 77-86 | name (10 ASCII bytes) |
| 87-89 | pitch_eg_rate[0..2] (0-99) |
| 90-92 | pitch_eg_level[0..2] (0-99) |

---

## 32-voice SysEx (VMEM)

### ヘッダー

```
F0 43 0n 04 20 00 <4096 bytes payload> <checksum> F7
```

- 総バイト数: 4104
- フォーマットバイト: `0x04`
- バイトカウント: `0x20 00` = 4096（7-bit エンコード: `(0x20 << 7) | 0x00`）
- ペイロード: 32 voice × 128 バイト（VMEM ブロック）

### VMEM ブロックレイアウト（128 バイト/voice）

有効データは先頭 73 バイト（[73-127] はダミー/予約ゼロ）。

**OP ブロック（10 バイト × 4）— VMEM 上の並び順**

| オフセット | OP |
|------------|-----|
| 0-9   | OP4 |
| 10-19 | OP2 |
| 20-29 | OP3 |
| 30-39 | OP1 |

各 OP ブロック内のビットパッキング:

| VMEM オフセット | 内容 |
|----------------|------|
| base+0 | ar |
| base+1 | d1r |
| base+2 | d2r |
| base+3 | rr |
| base+4 | d1l |
| base+5 | kbd_lev_scl |
| base+6 | `(amp_mod_en & 0x1) << 6` \| `(eg_bias_sens & 0x7) << 3` \| `(key_vel_sens & 0x7)` |
| base+7 | out_level |
| base+8 | freq_ratio |
| base+9 | `(kbd_rate_scl & 0x1F) << 3` \| `(detune & 0x7)` |

**グローバルパラメータ（オフセット 40-72）**

| オフセット | 内容 |
|------------|------|
| 40 | `(lfo_sync & 0x3) << 6` \| `(feedback & 0x7) << 3` \| `(algorithm & 0x7)` |
| 41 | lfo_speed |
| 42 | lfo_delay |
| 43 | lfo_pmd |
| 44 | lfo_amd |
| 45 | `(pitch_mod_sens & 0x7) << 4` \| `(amp_mod_sens & 0x3) << 2` \| `(lfo_wave & 0x3)` |
| 46 | transpose |
| 47 | pb_range |
| 48 | `(chorus & 0x1) << 4` \| `(poly_mono & 0x1) << 3` \| `(sustain & 0x1) << 2` \| `(portamento & 0x1) << 1` \| `(porta_mode & 0x1)` |
| 49 | porta_time |
| 50 | fc_volume |
| 51 | mw_pitch |
| 52 | mw_amplitude |
| 53 | bc_pitch |
| 54 | bc_amplitude |
| 55 | bc_pitch_bias |
| 56 | bc_eg_bias |
| 57-66 | name (10 ASCII bytes) |
| 67-69 | pitch_eg_rate[0..2] |
| 70-72 | pitch_eg_level[0..2] |
| 73-127 | ダミー（ゼロ） |

### DX100 のスロット数

VMEM フォーマットは 32 voice 分の領域を持つが、**DX100 の内部メモリは 24 voice**（I-01〜I-24）。
スロット 25-32（インデックス 24-31）は DX100 では使用されない。
→ GUI では `DX100_BANK_VOICES = 24` として 24 エントリのみ表示する。

### VCED vs VMEM の OP 並び順まとめ

| 格納形式 | SysEx 上の順序 | Rust 構造体 `ops[]` との対応 |
|---------|--------------|--------------------------|
| VCED（1-voice）| OP4, OP2, OP3, OP1 | `[3]=OP4, [1]=OP2, [2]=OP3, [0]=OP1` |
| VMEM（32-voice）| OP4(0), OP2(10), OP3(20), OP1(30) | 同上 |
| Rust 構造体 | — | `ops[0]=OP1, ops[1]=OP2, ops[2]=OP3, ops[3]=OP4` |

---

## パラメータ値の範囲一覧

| パラメータ | 範囲 | 備考 |
|-----------|------|------|
| ar, d1r, d2r | 0-31 | |
| rr | 0-15 | |
| d1l | 0-15 | |
| kbd_lev_scl | 0-99 | |
| kbd_rate_scl | 0-3 | VCED では 0-3、VMEM では 5bit (0x1F) でマスクするが実質 0-3 |
| eg_bias_sens | 0-7 | |
| amp_mod_en | 0-1 | |
| key_vel_sens | 0-7 | |
| out_level | 0-99 | |
| freq_ratio | 0-63 | FREQ_TBL で表示変換 |
| detune | 0-6 | 中央値=3（表示: -3〜+3） |
| algorithm | 0-7 | 表示: 1-8 |
| feedback | 0-7 | |
| lfo_speed, lfo_delay, lfo_pmd, lfo_amd | 0-99 | |
| lfo_sync | 0-1 | |
| lfo_wave | 0-3 | SAW/SQU/TRI/S&H |
| pitch_mod_sens | 0-7 | |
| amp_mod_sens | 0-3 | |
| transpose | 0-48 | 中央値=24（C3） |
| poly_mono | 0-1 | 0=POLY, 1=MONO |
| pb_range | 0-12 | |
| porta_mode | 0-1 | 0=Full, 1=Fing |
| porta_time | 0-99 | |
| fc_volume | 0-99 | |
| sustain, portamento, chorus | 0-1 | |
| mw_pitch, mw_amplitude | 0-99 | |
| bc_pitch, bc_amplitude, bc_pitch_bias, bc_eg_bias | 0-99 | |
| pitch_eg_rate[0-2] | 0-99 | |
| pitch_eg_level[0-2] | 0-99 | |
