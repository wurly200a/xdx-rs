# xdx-rs — Claude 向けプロジェクトガイド

## クレートマップ

| クレート | 役割 |
|---------|------|
| `xdx-core` | DX100/DX7 データ構造・SysEx エンコード/デコード |
| `xdx-synth` | FM 音声合成エンジン（FmEngine API） |
| `xdx-midi` | MIDI IN/OUT（WinMM バックエンド） |
| `xdx-gui` | パラメータエディタ GUI（egui/eframe） |
| `xdx-compare` | HW 実機録音 vs ソフトシンセ比較ツール群 |
| `xdx-eg-viewer` | EG 波形ビジュアル比較 GUI |
| `xdx-e2e` | キャリブレーション用テストバンク生成 (examples) |

## 重要ドキュメント

- **[docs/calibration.md](docs/calibration.md)** — EG/KBS パラメータのキャリブレーション手順（必読）
- **[docs/synth_impl.md](docs/synth_impl.md)** — パラメータ実装状況（実装完了後に削除予定）
- **[docs/architecture.md](docs/architecture.md)** — システムアーキテクチャ
- **[docs/dx100-protocol.md](docs/dx100-protocol.md)** — SysEx/MIDI プロトコル仕様

## 開発ルール

- ソースコードコメント、commitログは全て英語で記載すること
- PR のdescrptionは 英語->日本語 の順で記載すること
- PR 前に必ず `cargo fmt --check` を実行してから `gh pr create` すること  
  （fmt NG で CI が落ちることが過去に複数回あった）
- ビルド環境: Windows 11 / WSL2 両対応。MIDI/Audio を使うツールは Windows 実機が必要
- synth パラメータの係数は**実機測定値**。理論的に変えずに [docs/calibration.md](docs/calibration.md) のワークフローで検証すること

## 実行環境メモ

- MIDI: USB-MIDI (WinMM, midir クレート)
- Audio: cpal (WASAPI)
- 開発機: Windows 11 Pro、DX100 実機接続済み
