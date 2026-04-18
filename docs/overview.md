# xdx-rs — プロジェクト概要

## 概要

xdx-rs は、C 言語で書かれた DX100/DX7 パラメータエディタ **xdx** を Rust に移植したものです。
egui を使ったクロスプラットフォーム GUI アプリケーションとして再実装されています。

- オリジナル: https://github.com/wurly200a/xdx (C 実装)
- Rust 移植: https://github.com/wurly200a/xdx-rs

## 対象ハードウェア

| シンセ | 状態 | 備考 |
|--------|------|------|
| YAMAHA DX100 | 実装済み（基本機能） | 1-voice / 32-voice SysEx 対応 |
| YAMAHA DX7   | スタブのみ | 将来対応予定 |

## 開発環境

- 開発: WSL2 (Linux) — `cargo check-stub` エイリアスで virtual-midi feature を使い MIDI なしでビルド確認
- 実行: Windows 11 — WinMM 経由の USB-MIDI デバイス使用
- MIDI インターフェース: USB-MIDI (WinMM バックエンド、midir クレート)

## 実装済み機能

### DX100 1-voice (VCED)
- SysEx デコード / エンコード (`dx100_decode_1voice`, `dx100_encode_1voice`)
- GUI エディタ（全パラメータ編集可能）
- ファイル Open / Save / Save As (.syx)
- SysEx Fetch（シンセからの受信）
- SysEx Send（シンセへの送信）

### DX100 32-voice (VMEM)
- SysEx デコード / エンコード (`dx100_decode_32voice`, `dx100_encode_32voice`)
- 左ペインに 24 voice リスト表示（DX100 内部スロット数）
- ファイル Open / Save / Save As (.syx)
- SysEx Fetch（シンセからの受信）
- SysEx Send（シンセへの送信）※不具合あり → `status.md` 参照

### 1-voice ↔ 32-voice 連携
- `->` ボタン: バンクの選択 voice をエディタにロード
- `<-` ボタン: エディタの voice をバンクの選択スロットに書き込む

## スコープ外（現時点）

- DX7 対応（データ構造のみ定義済み）
- パラメータ変更のリアルタイム MIDI 送信
- MIDI チャンネル選択 UI（現状はチャンネル 0 固定）
- voice 名編集以外のバンク編集機能

## テストデータ

```
testdata/syx/
├── IvoryEbony.syx    # DX100 1-voice サンプル（起動時のデフォルト）
└── all_voices.syx    # DX100 32-voice サンプル（バンクのデフォルト）
```
