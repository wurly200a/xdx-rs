# xdx-rs

Cross-platform Rust port of [xdx](https://github.com/Wurly/xdx) — a YAMAHA DX7/DX100 FM synthesizer parameter editor.

## Crates

| Crate | Description |
|-------|-------------|
| `xdx-core` | Data structures, SysEx codec, DX100→DX7 conversion |
| `xdx-synth` | FM synthesis engine (planned) |
| `xdx-gui` | egui-based panel editor (planned) |
| `xdx-import` | AI image import via Claude Vision (planned) |

## Status

- [ ] xdx-core: DX7/DX100 structs, SysEx decode/encode
- [ ] xdx-core: DX100→DX7 conversion
- [ ] xdx-synth: FM engine
- [ ] xdx-gui: panel editor UI
- [ ] xdx-import: image→parameters via AI

## Test Data

`testdata/` contains SysEx captures from the original Windows application, used as test vectors.
