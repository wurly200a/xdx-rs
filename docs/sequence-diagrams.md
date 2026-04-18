# xdx-rs — シーケンス図

## クレート依存関係

```mermaid
graph LR
    GUI["xdx-gui\n(egui App)"]
    Core["xdx-core\n(codec)"]
    Midi["xdx-midi\n(MidiManager)"]
    Midir["vendor/midir"]
    OS["WinMM / CoreMIDI / ALSA"]

    GUI --> Core
    GUI --> Midi
    Midi --> Midir
    Midir --> OS
```

---

## 1-voice: File Open

```mermaid
sequenceDiagram
    actor User
    participant GUI as xdx-gui<br/>(App)
    participant Core as xdx-core<br/>(sysex)
    participant FS as FileSystem

    User->>GUI: click "Open..."
    GUI->>FS: FileDialog::pick_file()
    FS-->>GUI: path (.syx)
    GUI->>FS: fs::read(path)
    FS-->>GUI: Vec[u8]
    GUI->>Core: dx100_decode_1voice(&bytes)
    Core-->>GUI: Dx100Voice
    GUI->>GUI: self.voice = voice<br/>self.name_buf = name<br/>self.file_path = path
```

---

## 1-voice: File Save

```mermaid
sequenceDiagram
    actor User
    participant GUI as xdx-gui<br/>(App)
    participant Core as xdx-core<br/>(sysex)
    participant FS as FileSystem

    User->>GUI: click "Save" / "Save As..."
    alt Save As
        GUI->>FS: FileDialog::save_file()
        FS-->>GUI: path
    end
    GUI->>Core: dx100_encode_1voice(&self.voice, 0)
    Core-->>GUI: Vec[u8] (101 bytes)
    GUI->>FS: fs::write(path, &bytes)
    GUI->>GUI: self.file_path = path
```

---

## 1-voice: SysEx Fetch (synth → PC)

```mermaid
sequenceDiagram
    actor User
    participant GUI as xdx-gui<br/>(App, ~60fps poll)
    participant Midi as xdx-midi<br/>(MidiManager)
    participant DX100 as DX100<br/>(hardware)
    participant Core as xdx-core<br/>(sysex)

    User->>GUI: click "Fetch"
    GUI->>Midi: open_out(port)
    GUI->>Midi: open_in(port)
    GUI->>Midi: send([F0 43 20 03 F7])
    Note over Midi: worker thread sends bytes to MIDI OUT
    Midi->>DX100: F0 43 20 03 F7
    GUI->>GUI: state = Fetch1Pending { sent_at }

    DX100->>Midi: SysEx 101 bytes<br/>(F0 43 04 03 00 5D ... CS F7)
    Note over Midi: IN callback reassembles<br/>multi-packet SysEx → MidiEvent::SysEx

    loop App::update() each frame
        GUI->>Midi: try_recv()
        Midi-->>GUI: MidiEvent::SysEx(bytes)
        GUI->>Core: dx100_decode_1voice(&bytes)
        Core-->>GUI: Dx100Voice
        GUI->>GUI: self.voice = voice<br/>state = Idle
        GUI->>Midi: close_in() / close_out()
    end
```

---

## 1-voice: SysEx Send (PC → synth)

```mermaid
sequenceDiagram
    actor User
    participant GUI as xdx-gui<br/>(App)
    participant Core as xdx-core<br/>(sysex)
    participant Midi as xdx-midi<br/>(MidiManager)
    participant Worker as out worker<br/>thread
    participant DX100 as DX100<br/>(hardware)

    User->>GUI: click "Send"
    GUI->>Midi: open_out(port)
    GUI->>Core: dx100_encode_1voice(&self.voice, 0)
    Core-->>GUI: Vec[u8] (101 bytes)
    GUI->>Midi: send_then_close(&bytes)
    Note over Midi: channel.send(Some(bytes))<br/>channel.send(None)
    Midi->>Worker: Some(bytes) → forward to MIDI OUT
    Worker->>DX100: SysEx 101 bytes
    Worker->>Worker: recv None → conn.close()
    GUI->>GUI: out_tx = None<br/>sysex_out_flash = now
```

---

## 32-voice: File Open

```mermaid
sequenceDiagram
    actor User
    participant GUI as xdx-gui<br/>(App)
    participant Core as xdx-core<br/>(sysex)
    participant FS as FileSystem

    User->>GUI: click "Open Bank..."
    GUI->>FS: FileDialog::pick_file()
    FS-->>GUI: path (.syx)
    GUI->>FS: fs::read(path)
    FS-->>GUI: Vec[u8] (4104 bytes)
    GUI->>Core: dx100_decode_32voice(&bytes)
    Core-->>GUI: Vec[Dx100Voice] (32 voices)
    GUI->>GUI: self.bank = voices<br/>self.bank_sel = 0<br/>self.bank_file_path = path
```

---

## 32-voice: File Save

```mermaid
sequenceDiagram
    actor User
    participant GUI as xdx-gui<br/>(App)
    participant Core as xdx-core<br/>(sysex)
    participant FS as FileSystem

    User->>GUI: click "Save Bank" / "Save Bank As..."
    alt Save As
        GUI->>FS: FileDialog::save_file()
        FS-->>GUI: path
    end
    GUI->>Core: dx100_encode_32voice(&self.bank, 0)
    Core-->>GUI: Vec[u8] (4104 bytes)
    GUI->>FS: fs::write(path, &bytes)
    GUI->>GUI: self.bank_file_path = path
```

---

## 32-voice: SysEx Fetch (synth → PC)

```mermaid
sequenceDiagram
    actor User
    participant GUI as xdx-gui<br/>(App, ~60fps poll)
    participant Midi as xdx-midi<br/>(MidiManager)
    participant DX100 as DX100<br/>(hardware)
    participant Core as xdx-core<br/>(sysex)

    User->>GUI: click "Fetch Bank"
    GUI->>Midi: open_out(port)
    GUI->>Midi: open_in(port)
    GUI->>Midi: send([F0 43 20 04 F7])
    Midi->>DX100: F0 43 20 04 F7
    GUI->>GUI: state = Fetch32Pending { sent_at }

    Note over DX100,Midi: 4104 bytes ÷ 1024 byte/buf = 5 chunks<br/>~1s transmission at 31250 bps
    DX100->>Midi: chunk 1/5 (1024 bytes, F0...)
    DX100->>Midi: chunk 2/5 (1024 bytes)
    DX100->>Midi: chunk 3/5 (1024 bytes)
    DX100->>Midi: chunk 4/5 (1024 bytes)
    DX100->>Midi: chunk 5/5 (8 bytes, ...F7)
    Note over Midi: IN callback accumulates chunks<br/>→ delivers MidiEvent::SysEx when F7 seen

    loop App::update() each frame
        GUI->>Midi: try_recv()
        Midi-->>GUI: MidiEvent::SysEx(bytes) [4104 bytes]
        GUI->>Core: dx100_decode_32voice(&bytes)
        Core-->>GUI: Vec[Dx100Voice] (32 voices)
        GUI->>GUI: self.bank = voices<br/>state = Idle
        GUI->>Midi: close_in() / close_out()
    end
```

---

## 32-voice: SysEx Send (PC → synth)

```mermaid
sequenceDiagram
    actor User
    participant GUI as xdx-gui<br/>(App)
    participant Core as xdx-core<br/>(sysex)
    participant Midi as xdx-midi<br/>(MidiManager)
    participant Worker as out worker<br/>thread
    participant DX100 as DX100<br/>(hardware)

    User->>GUI: click "Send Bank"
    GUI->>Midi: open_out(port)
    GUI->>Core: dx100_encode_32voice(&self.bank, 0)
    Note over Core: device=0: F0 43 00 04 20 00<br/>(DX100 accepts bulk dump on device 0)
    Core-->>GUI: Vec[u8] (4104 bytes)
    GUI->>Midi: send_then_close(&bytes)
    Note over Midi: channel.send(Some(bytes))<br/>channel.send(None)
    Midi->>Worker: Some(bytes)
    Note over Worker,DX100: ~1s transmission at 31250 bps
    Worker->>DX100: SysEx 4104 bytes<br/>(F0 43 00 04 20 00 ... CS F7)
    Worker->>Worker: recv None → conn.close()
    GUI->>GUI: out_tx = None<br/>sysex_out_flash = now
```

---

## SysEx フォーマット早見表

| 項目 | 1-voice | 32-voice |
|------|---------|---------|
| Fetch リクエスト | `F0 43 20 03 F7` | `F0 43 20 04 F7` |
| 受信バイト数 | 101 bytes | 4104 bytes (5チャンク) |
| DX100 応答ヘッダ | `F0 43 04 03 00 5D` | `F0 43 04 04 20 00` |
| 送信ヘッダ (device=0) | `F0 43 00 03 00 5D` | `F0 43 00 04 20 00` |
| Fetch タイムアウト | 5s | 30s |
