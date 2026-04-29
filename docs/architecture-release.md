<!-- AUTO-GENERATED — do not edit manually.
     Regenerate: python tools/rust-mermaid.py --preset release --symbols struct,enum,trait,fn,type --out docs/architecture-release.md -->

# Crate Architecture — Release

```mermaid
classDiagram
    class xdx_core {
        +Dx100Operator [S]
        +Dx100Voice [S]
        +Dx7Operator [S]
        +Dx7Voice [S]
        +SysExError [E]
        +dx100_decode_1voice [F]
        +dx100_decode_32voice [F]
        +dx100_encode_1voice [F]
        +dx100_encode_32voice [F]
        +dx100_to_dx7 [F]
        +dx7_decode_1voice [F]
        +name_str [F]
    }
    class xdx_gui {
        +AudioHandle [S]
        +note_off [F]
        +note_on [F]
        +set_voice [F]
        +start [F]
    }
    class xdx_midi {
        +MidiError [S]
        +MidiEvent [E]
    }
    class xdx_synth {
        +FmEngine [S]
        +new [F]
        +note_off [F]
        +note_on [F]
        +render [F]
        +render_lfo [F]
        +set_voice [F]
    }

    xdx_gui ..> xdx_core : uses
    xdx_gui ..> xdx_midi : uses
    xdx_gui ..> xdx_synth : uses
    xdx_synth ..> xdx_core : uses
```
