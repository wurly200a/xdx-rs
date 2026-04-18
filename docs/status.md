# 実装状況・既知の不具合・テスト方針

## 実装済み機能（動作確認済み）

| 機能 | 状態 | 備考 |
|------|------|------|
| DX100 1-voice VCED デコード | ✅ | `dx100_decode_1voice` |
| DX100 1-voice VCED エンコード | ✅ | `dx100_encode_1voice` |
| DX100 32-voice VMEM デコード | ✅ | `dx100_decode_32voice` |
| DX100 32-voice VMEM エンコード | ✅ | `dx100_encode_32voice` |
| 1-voice GUI エディタ | ✅ | 全パラメータ編集可能 |
| 32-voice バンクリスト表示 | ✅ | 24 voice まで表示 |
| 1-voice ↔ 32-voice 転送 | ✅ | `->` / `<-` ボタン |
| 1-voice ファイル Open/Save | ✅ | .syx 形式 |
| 32-voice ファイル Open/Save | ✅ | .syx 形式 |
| SysEx Fetch 1（シンセから受信）| ✅ | タイムアウト・Cancel 対応 |
| SysEx Send 1（シンセへ送信） | ✅ | |
| SysEx Fetch 32（シンセから受信）| ✅ | SysEx チャンク累積対応済み |
| SysEx Send 32（シンセへ送信） | ⚠️ | 動作するが未検証・不安定の可能性あり |
| MIDI ポートスキャン | ✅ | バックグラウンド・タイムアウト・再試行対応 |
| MIDI 接続インジケーター | ✅ | IN/OUT ドット、フラッシュ表示 |
| アプリ終了時の MIDI クリーンアップ | ✅ | `on_exit()` |

## 既知の不具合・未解決課題

### 🔴 高優先度

#### [BUG-01] Send 32 がシンセに反映されない

- **症状**: Send 32 後、DX100 のボイスが変わらない
- **状況**: 送信自体は行われているが、DX100 が受け入れていない可能性
- **仮説**:
  1. MIDI チャンネルの不一致（現在チャンネル 0 = MIDI ch1 固定、シンセ側の設定に依存）
  2. VMEM エンコードのビットパッキングに誤りがある
  3. 送信タイミングの問題（Send 後すぐ切断すると最後まで届かない可能性）
- **調査方法**: SysExDump ツールで実際の送信バイト列をキャプチャして検証

#### [BUG-02] MIDI デバイスロックによる PC 再起動が必要になるケース

- **症状**: 特定の操作後、MIDI が完全に死んで再起動しないと復帰しない
- **原因**: WinMM のデッドロック（詳細は `midi-design.md` 参照）
- **現状の対策**: `on_exit()` で正常終了時はクリーンアップ
- **残課題**: Ctrl+C / クラッシュ時の対応なし

### 🟡 中優先度

#### [BUG-03] MIDI チャンネルが固定（チャンネル 0）

- SysEx ヘッダーの `0x43 0x00` の `00` 部分が常にチャンネル 0
- シンセ側の受信チャンネルが異なる場合は動作しない
- UI でチャンネル選択できるようにする必要がある

#### [TODO-01] 32-voice Send の動作検証

- 実機での Send 32 → Fetch 32 のラウンドトリップ検証未実施
- SysExDump を使った実際のバイト列比較が必要

#### [TODO-02] DX7 対応

- `Dx7Voice` 構造体のみ定義済み、コーデック未実装
- `dx7_decode_1voice` は `todo!()` マクロ

### 🟢 低優先度

#### [TODO-03] ファイル Open 時の 1-voice / 32-voice 自動判別

- 現状、左ペインの Open は 32-voice 専用、右ペインは 1-voice 専用
- ファイルサイズ（101 vs 4104 バイト）でフォーマットバイト判定できる

---

## Integration Test 方針

### テスト対象と優先度

#### 1. SysEx コーデック（最優先）

`xdx-core/src/sysex.rs` の encode/decode のラウンドトリップ検証。

```rust
// テストケース例
#[test]
fn roundtrip_1voice() {
    let original = include_bytes!("../../testdata/syx/IvoryEbony.syx");
    let voice = dx100_decode_1voice(original).unwrap();
    let re_encoded = dx100_encode_1voice(&voice, 0);
    assert_eq!(&original[..], &re_encoded[..]);
}

#[test]
fn roundtrip_32voice() {
    let original = include_bytes!("../../testdata/syx/all_voices.syx");
    let voices = dx100_decode_32voice(original).unwrap();
    let re_encoded = dx100_encode_32voice(&voices, 0);
    assert_eq!(&original[..], &re_encoded[..]);
}
```

#### 2. VMEM ↔ VCED 変換の一貫性

32-voice SysEx の各 voice を VMEM デコードし、VCED でエンコードして
再度 VMEM デコードしたとき、全パラメータが一致することを検証。

```rust
#[test]
fn vmem_to_vced_params_consistent() {
    let data = include_bytes!("../../testdata/syx/all_voices.syx");
    let voices = dx100_decode_32voice(data).unwrap();
    for (i, voice) in voices.iter().enumerate().take(24) {
        let vced = dx100_encode_1voice(voice, 0);
        let decoded = dx100_decode_1voice(&vced).unwrap();
        assert_eq!(voice, &decoded, "voice {} mismatch", i + 1);
    }
}
```

#### 3. SysEx エラー処理

- `TooShort` — 短すぎるデータ
- `InvalidHeader` — フォーマットバイト不一致
- `InvalidFooter` — F7 なし
- `InvalidByteCount` — バイトカウント不一致
- `ChecksumMismatch` — チェックサム不一致

#### 4. チェックサム計算

```rust
#[test]
fn checksum_correctness() {
    // known-good SysEx からチェックサムを抽出して verify
}
```

### テストファイルの配置

```
xdx-core/src/sysex.rs  内の #[cfg(test)] モジュール
または
xdx-core/tests/integration.rs  （外部 integration test）
```

外部テストファイルは `testdata/syx/` にアクセスするため、
`xdx-core` から見た相対パスの扱いに注意（`include_bytes!` vs `std::fs::read`）。

### テスト実行

```bash
cargo test -p xdx-core
```

WSL2 でも実行可能（MIDI 不要）。

---

## 今後の作業ロードマップ

1. **Integration Test 実装** — コーデックのラウンドトリップ検証
2. **BUG-01 調査・修正** — Send 32 がシンセに反映されない問題
3. **BUG-02 緩和** — クラッシュ時の MIDI ハンドル解放（可能な範囲で）
4. **BUG-03 対応** — MIDI チャンネル選択 UI
5. **DX7 対応** — コーデック実装
