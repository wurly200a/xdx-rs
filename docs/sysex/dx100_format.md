# DX100 SysEx Format

## 1-Voice Bulk Dump (101 bytes)

```
F0 43 0n 03 00 49 <73 bytes payload> <checksum> F7
```

- `n` = MIDI channel (0-15)
- Byte count field: `00 49` = 73 (0x49)
- Checksum: two's complement of payload sum, masked to 7 bits

### Payload layout (73 bytes)

#### Operators (offset 0–51): OP4, OP3, OP2, OP1 — 13 bytes each

| Offset | Bits  | Parameter              | Range |
|--------|-------|------------------------|-------|
| +0     | 7-0   | EG Rate 1              | 0-31  |
| +1     | 7-0   | EG Rate 2              | 0-31  |
| +2     | 7-0   | EG Rate 3              | 0-31  |
| +3     | 7-0   | EG Rate 4              | 0-31  |
| +4     | 7-0   | EG Level 1             | 0-15  |
| +5     | 7-0   | EG Level 2             | 0-15  |
| +6     | 7-0   | EG Level 3             | 0-15  |
| +7     | 7-0   | EG Level 4             | 0-15  |
| +8     | 5-0   | Kbd Level Scaling      | 0-63  |
| +9     | 2-0   | EG Bias Sensitivity    | 0-7   |
| +9     | 4-3   | Kbd Rate Scaling       | 0-3   |
| +10    | 1-0   | Amp Mod Sensitivity    | 0-3   |
| +10    | 4-2   | Key Vel Sensitivity    | 0-7   |
| +11    | 6-0   | Output Level           | 0-99  |
| +12    | 0     | Osc Mode (0=ratio)     | 0-1   |
| +12    | 5-1   | Osc Freq Coarse        | 0-31  |

#### Osc Freq Fine (offset 52–55): one byte per operator (OP4..OP1)
- Range: 0-63

#### Osc Detune (offset 56–59): one byte per operator (OP4..OP1)
- Range: 0-6 (center = 3, display: -3..+3)

#### Global parameters (offset 60–72)

| Offset | Bits  | Parameter              | Range |
|--------|-------|------------------------|-------|
| 60     | 2-0   | Algorithm              | 0-7 (display 1-8) |
| 60     | 5-3   | Feedback               | 0-7   |
| 61     | 6-0   | LFO Speed              | 0-99  |
| 62     | 6-0   | LFO Delay              | 0-99  |
| 63     | 6-0   | LFO Pitch Mod Depth    | 0-99  |
| 64     | 6-0   | LFO Amp Mod Depth      | 0-99  |
| 65     | 0     | LFO Sync               | 0-1   |
| 65     | 2-1   | LFO Wave               | 0-3   |
| 65     | 6-4   | Pitch Mod Sensitivity  | 0-7   |
| 65     | 7     | Amp Mod Sensitivity    | 0-1   |
| 66     | 5-0   | Transpose              | 0-48 (center=24) |
| 67     | 0     | Poly/Mono (1=mono)     | 0-1   |
| 67     | 4-1   | Pitch Bend Range       | 0-12  |
| 68     | 0     | Portamento Mode        | 0-1   |
| 69     | 6-0   | Portamento Time        | 0-99  |
| 70     | 6-0   | FC Volume              | 0-99  |
| 71     | 0     | Sustain                | 0-1   |
| 71     | 1     | Portamento             | 0-1   |
| 71     | 2     | Chorus                 | 0-1   |
| 72     | 6-0   | Voice name char 1      | ASCII |
| ...    |       | (10 chars total)       |       |

## 32-Voice Bulk Dump (4104 bytes)

```
F0 43 0n 04 20 00 <4096 bytes payload> <checksum> F7
```

- Byte count: `20 00` = 4096 (128 bytes × 32 voices)
- Each packed voice is 128 bytes (different packing from 1-voice dump)

## Checksum Formula

```
checksum = ((~sum(payload)) + 1) & 0x7F
```

Where `sum` is the byte sum of all payload bytes (wrapping u8 arithmetic).
