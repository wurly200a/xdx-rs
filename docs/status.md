# 実装状況・既知の不具合

## 実装済み機能

| 機能 | 状態 | 備考 |
|------|------|------|
| DX100 1-voice VCED デコード | ✅ | `dx100_decode_1voice` |
| DX100 1-voice VCED エンコード | ✅ | `dx100_encode_1voice` |
| DX100 32-voice VMEM デコード | ✅ | `dx100_decode_32voice` |
| DX100 32-voice VMEM エンコード | ✅ | `dx100_encode_32voice` |
| SysEx コーデック テスト | ✅ | 14 tests / `xdx-core/tests/` |
| 1-voice GUI エディタ | ✅ | 全パラメータ編集可能 |
| 32-voice バンクリスト表示 | ✅ | 24 voice まで表示 |
| 1-voice ↔ 32-voice 転送 | ✅ | `->` / `<-` ボタン |
| 1-voice ファイル Open/Save | ✅ | .syx 形式 |
| 32-voice ファイル Open/Save | ✅ | .syx 形式 |
| SysEx Fetch 1（シンセから受信）| ✅ | タイムアウト・Cancel 対応 |
| SysEx Send 1（シンセへ送信） | ✅ | |
| SysEx Fetch 32（シンセから受信）| ✅ | SysEx チャンク累積対応済み |
| SysEx Send 32（シンセへ送信） | ✅ | UM-ONE 経由で動作確認済み |
| MIDI ポートスキャン | ✅ | バックグラウンド・タイムアウト・再試行対応 |
| MIDI 接続インジケーター | ✅ | IN/OUT ドット、フラッシュ表示 |
| MIDI チャンネル選択 UI | ✅ | Settings メニュー CH 1-16 |
| アプリ終了時の MIDI クリーンアップ | ✅ | `on_exit()` |

> ソフトシンセのパラメータ実装状況は [`docs/synth_impl.md`](synth_impl.md) を参照。

---

## 既知の不具合・未解決課題

### 🔴 高優先度

#### [BUG-02] MIDI デバイスロックによる PC 再起動が必要になるケース

- **症状**: 特定の操作後、MIDI が完全に死んで再起動しないと復帰しない
- **原因**: WinMM のデッドロック（詳細は `midi-design.md` 参照）
- **現状の対策**: `on_exit()` で正常終了時はクリーンアップ
- **残課題**: Ctrl+C / クラッシュ時の対応なし

### 🟡 中優先度

#### [TODO-02] DX7 対応

- `Dx7Voice` 構造体のみ定義済み、コーデック未実装
- `dx7_decode_1voice` / `dx100_to_dx7` は `todo!()` マクロ

### 🟢 低優先度

#### [TODO-03] ファイル Open 時の 1-voice / 32-voice 自動判別

- 現状、左ペインの Open は 32-voice 専用、右ペインは 1-voice 専用
- ファイルサイズ（101 vs 4104 バイト）でフォーマット判定できる

---

## 今後の作業ロードマップ

1. **BUG-02 緩和** — クラッシュ時の MIDI ハンドル解放（可能な範囲で）
2. **TODO-02 DX7 対応** — コーデック実装
