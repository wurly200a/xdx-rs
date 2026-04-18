# xdx-rs — アーキテクチャ

## クレート構成と依存関係

```
xdx-gui   ──depends──►  xdx-core
    │      ──depends──►  xdx-midi
    │
    └─ egui / eframe / rfd（外部クレート）

xdx-core  ──（依存なし）
xdx-midi  ──（外部クレートのみ: midir）
```

各クレートは単方向の依存のみ。循環依存なし。

---

## xdx-core

### 役割

- DX100/DX7 のボイスパラメータをメモリ上で表現するデータ構造の定義
- SysEx バイト列とデータ構造の相互変換（エンコード / デコード）
- MIDI や GUI に依存しない純粋なコアロジック

### モジュール構成

| モジュール | 内容 |
|-----------|------|
| `dx100.rs` | `Dx100Operator`、`Dx100Voice` 構造体、`Default` 実装、`name_str()` |
| `dx7.rs`   | `Dx7Voice` 構造体（スタブ、コーデック未実装） |
| `sysex.rs` | SysEx エンコード / デコード関数、`SysExError` 列挙型 |

### 公開インターフェース

**データ構造**

- `Dx100Operator` — オペレータ単位のパラメータ（AR, D1R, D2R, RR, D1L, 周波数比, デチューン 等）
- `Dx100Voice` — ボイス全体のパラメータ（4 オペレータ + グローバル + LFO + パフォーマンス）

**コーデック関数**

| 関数 | 入力 | 出力 |
|------|------|------|
| `dx100_decode_1voice(&[u8])` | 101 バイト VCED SysEx | `Result<Dx100Voice, SysExError>` |
| `dx100_encode_1voice(&Dx100Voice, channel)` | ボイス + MIDI チャンネル | `Vec<u8>`（101 バイト） |
| `dx100_decode_32voice(&[u8])` | 4104 バイト VMEM SysEx | `Result<Vec<Dx100Voice>, SysExError>` |
| `dx100_encode_32voice(&[Dx100Voice], channel)` | ボイス配列 + MIDI チャンネル | `Vec<u8>`（4104 バイト） |

**エラー型**

`SysExError` は以下のバリアントを持つ:
- `TooShort` — データ長不足
- `InvalidHeader` — フォーマットバイト不一致
- `InvalidFooter` — F7 終端なし
- `InvalidByteCount` — バイトカウントフィールド不一致
- `ChecksumMismatch { expected, actual }` — チェックサム不一致

---

## xdx-midi

### 役割

- MIDI IN/OUT ポートの列挙・開閉・送受信の抽象化
- Windows（WinMM / midir）と WSL2（スタブ）のバックエンド切り替え
- 大きな SysEx の非同期送信（GUI スレッドのブロック防止）
- SysEx チャンク分割への対応（WinMM が大きな SysEx を複数コールバックで届ける問題）

### バックエンド切り替え

Feature flag `virtual-midi` により実装を切り替える。

| Feature | 使用場面 | 実装 |
|---------|---------|------|
| なし（デフォルト） | Windows 実行 | midir (WinMM) |
| `virtual-midi` | WSL2 ビルド確認 | スタブ（全操作をノーオペレーションとして処理） |

WSL2 では `cargo check-stub`（エイリアス）で `virtual-midi` 機能付きビルド確認を行う。

### 公開インターフェース（MidiManager）

**ポート管理**

| メソッド | 説明 |
|---------|------|
| `MidiManager::new()` | インスタンス生成 |
| `list_in_ports() -> Vec<String>` | 利用可能な MIDI IN ポート名一覧（static） |
| `list_out_ports() -> Vec<String>` | 利用可能な MIDI OUT ポート名一覧（static） |
| `open_in(port_name)` | MIDI IN を開き受信を開始 |
| `open_out(port_name)` | MIDI OUT を開く（内部でワーカースレッドを起動） |
| `close_in()` | MIDI IN を閉じる |
| `close_out()` | MIDI OUT を閉じる（ワーカースレッドを終了） |
| `in_connected() -> bool` | MIDI IN 接続中か |
| `out_connected() -> bool` | MIDI OUT 接続中か |

**データ送受信**

| メソッド | 説明 |
|---------|------|
| `send(&[u8])` | MIDI OUT にデータ送信（ノンブロッキング、ワーカーに委譲） |
| `try_recv() -> Option<MidiEvent>` | 受信済みイベントを1件取り出す（ノンブロッキング） |

**公開フィールド**

- `in_port_name: Option<String>` — 現在接続中の IN ポート名
- `out_port_name: Option<String>` — 現在接続中の OUT ポート名

**MidiEvent 列挙型**

- `SysEx(Vec<u8>)` — F0 で始まり F7 で終わる完全な SysEx メッセージ
- `Other(Vec<u8>)` — 通常 MIDI メッセージ

### 内部設計のポイント

**MIDI OUT ワーカースレッド**

`open_out()` 時に `MidiOutputConnection` をワーカースレッドに move し、
GUI スレッドはチャンネル経由でバイト列を渡すだけ。
`close_out()` はチャンネルの Sender を drop することでワーカーを自然終了させる。

**SysEx 累積バッファ**

MIDI IN コールバック内でバイト列を蓄積し、F7 が届いた時点で
完全な SysEx として `MidiEvent::SysEx` を発行する。
WinMM によるチャンク分割を透過的に吸収する。

---

## xdx-gui

### 役割

- egui を使ったメインウィンドウと全 UI 要素の管理
- ユーザー操作に応じた xdx-core / xdx-midi の呼び出し
- アプリケーション状態の保持と更新ループ（`update()` 関数）

### 主要な状態

**SysExState（Fetch 状態機械）**

```
Idle
 ├─[Fetch 1]──→ Fetch1Pending { sent_at: f64 }
 │               ├─[受信 0x03]──→ Idle（voice 更新）
 │               ├─[タイムアウト]──→ Idle
 │               └─[Cancel]────→ Idle
 └─[Fetch 32]─→ Fetch32Pending { sent_at: f64 }
                 ├─[受信 0x04]──→ Idle（bank 更新）
                 ├─[タイムアウト]──→ Idle
                 └─[Cancel]────→ Idle
```

- Fetch1Pending / Fetch32Pending は排他（どちらか一方のみ）
- Pending 中は Send 系ボタンも無効化

**バックグラウンドポートスキャン**

`list_in/out_ports()` が WinMM の問題でブロックする可能性があるため、
スキャンはバックグラウンドスレッドで実行し結果をキャッシュする。
スキャン開始から 5 秒でタイムアウトし再試行可能とする。

### UI パネル構成

```
┌─ menubar ────────────────────────────────────────────────────────────────┐
│ Settings > MIDI IN / MIDI OUT / Scan Ports / MIDI Device Test            │
├─ toolbar ────────────────────────────────────────────────────────────────┤
│ SYNTH: [DX100|DX7]   ● IN:(port)   ● OUT:(port)                          │
├─ statusbar (bottom) ─────────────────────────────────────────────────────┤
│ ステータスメッセージ                                                        │
├─ bank_panel (左) ─────────┬─ transfer ─┬─ CentralPanel (右) ────────────┤
│ 32 VOICES                  │    [->]    │ 1 VOICE                        │
│ FILE: Open / Save / SaveAs │    [<-]    │ FILE: Open / Save / SaveAs     │
│ (filename)                 │            │ (filename)                     │
│ SysEx: [Fetch] [Send]      │            │ SysEx: [Fetch] [Send]          │
│ ─────────────────────      │            │ ─────────────────────────────  │
│ 01  VOICE-NAME             │            │ 1-voice パラメータエディタ       │
│ 02  VOICE-NAME             │            │  PATCHNAME / OP グリッド        │
│  :                         │            │  EG / KEY SCALING / PEG        │
│ 24  VOICE-NAME             │            │  パフォーマンスコントロール       │
└────────────────────────────┴────────────┴────────────────────────────────┘
```

egui パネル追加順序: menubar → toolbar → statusbar → bank_panel → transfer_panel → CentralPanel
（左パネルは追加順に外側から内側へ配置される）

### xdx-core / xdx-midi との連携フロー

**Fetch 1（シンセ → エディタ）**

```
[Fetch ボタン] → ensure_out() → ensure_in()
  → midi_manager.send([F0 43 20 03 F7])
  → sysex_state = Fetch1Pending
  ↓ update() ループ
  → try_recv() → SysEx(bytes) [bytes[3]==0x03]
  → dx100_decode_1voice(bytes) → voice 更新
  → close_in(), close_out(), sysex_state = Idle
```

**Send 1（エディタ → シンセ）**

```
[Send ボタン] → dx100_encode_1voice(&voice, 0)
  → ensure_out() → midi_manager.send(bytes)
  ※ close_out() は呼ばない（ワーカー送信中の競合防止）
```

**32-voice Fetch / Send も同様のパターン（フォーマットバイト 0x04 を使用）**

**バンク ↔ エディタ転送**

```
[->] → voice = bank[bank_sel].clone()  （バンク選択スロット → エディタ）
[<-] → bank[bank_sel] = voice.clone()  （エディタ → バンク選択スロット）
```
