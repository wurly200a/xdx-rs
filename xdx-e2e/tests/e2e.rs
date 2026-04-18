//! Hardware E2E tests — require a real DX100 (or compatible) connected via MIDI.
//!
//! All tests are `#[ignore]` by default so `cargo test` skips them.
//!
//! # How to run
//!
//! **Must use `--test-threads=1`** — tests share a physical MIDI device and must
//! not run in parallel.
//!
//! ```sh
//! XDX_MIDI_IN="<port name>" XDX_MIDI_OUT="<port name>" \
//!   cargo test -p xdx-e2e -- --ignored --nocapture --test-threads=1
//! ```
//!
//! Discover available port names with:
//! ```sh
//! cargo run -p xdx-e2e --example list_ports
//! ```
//!
//! # Preconditions
//!
//! `test_voice_roundtrip`: the synth must not have IvoryEbony loaded as its
//! current edit-buffer voice, otherwise the `assert_ne((1),(2'))` check fails.

use std::time::{Duration, Instant};
use xdx_core::sysex::{
    dx100_decode_1voice, dx100_encode_1voice,
    dx100_decode_32voice, dx100_encode_32voice,
};
use xdx_midi::{MidiEvent, MidiManager};

// DX100 "Parameter Change Bulk Dump Request" for 1-voice edit buffer
const FETCH_1VOICE:  &[u8] = &[0xF0, 0x43, 0x20, 0x03, 0xF7];
// DX100 "Parameter Change Bulk Dump Request" for 32-voice bank
const FETCH_32VOICE: &[u8] = &[0xF0, 0x43, 0x20, 0x04, 0xF7];

const RECV_TIMEOUT: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(10);

static IVORY_EBONY_SYX: &[u8] =
    include_bytes!("../../testdata/syx/IvoryEbony.syx");
static VIBRABELLE_SYX: &[u8] =
    include_bytes!("../../testdata/syx/Vibrabelle.syx");
static ALL_VOICES_SYX: &[u8] =
    include_bytes!("../../testdata/syx/all_voices.syx");

// ── helpers ───────────────────────────────────────────────────────────────────

fn midi_ports() -> (String, String) {
    let in_port  = std::env::var("XDX_MIDI_IN")
        .expect("set XDX_MIDI_IN to the MIDI input port name");
    let out_port = std::env::var("XDX_MIDI_OUT")
        .expect("set XDX_MIDI_OUT to the MIDI output port name");
    (in_port, out_port)
}

/// Poll until a SysEx message arrives or the timeout elapses.
fn recv_sysex(mm: &mut MidiManager, timeout: Duration) -> Option<Vec<u8>> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        match mm.try_recv() {
            Some(MidiEvent::SysEx(data)) => {
                println!("  [IN] SysEx ({} bytes): {:02X?}", data.len(), &data[..data.len().min(16)]);
                return Some(data);
            }
            Some(MidiEvent::Other(data)) if data != [0xFE] => {
                println!("  [IN] Other: {:02X?}", data);
            }
            _ => {}
        }
        std::thread::sleep(POLL_INTERVAL);
    }
    None
}

/// Discard any events already queued (e.g. stale responses from earlier sends).
fn drain_recv(mm: &mut MidiManager) {
    while mm.try_recv().is_some() {}
}

/// Send the 1-voice dump request and return the raw SysEx response bytes.
fn fetch_1voice(mm: &mut MidiManager) -> Vec<u8> {
    drain_recv(mm);
    println!("  [OUT] Fetch 1-voice request");
    mm.send(FETCH_1VOICE).expect("send fetch request");
    recv_sysex(mm, RECV_TIMEOUT).expect("timed out waiting for SysEx response from synth")
}

/// Send the 32-voice bank dump request and return the raw SysEx response bytes.
fn fetch_32voice(mm: &mut MidiManager) -> Vec<u8> {
    drain_recv(mm);
    println!("  [OUT] Fetch 32-voice request");
    mm.send(FETCH_32VOICE).expect("send fetch 32-voice request");
    // 4104 bytes at 31250 bps ≈ 1s transmission; allow 30s total.
    recv_sysex(mm, Duration::from_secs(30))
        .expect("timed out waiting for 32-voice SysEx response from synth")
}

// ── tests ─────────────────────────────────────────────────────────────────────

/// Verify that MIDI IN and OUT ports can be opened and closed without error.
#[test]
#[ignore = "requires real MIDI hardware — set XDX_MIDI_IN and XDX_MIDI_OUT"]
fn test_midi_open_close() {
    let (in_port, out_port) = midi_ports();
    let mut mm = MidiManager::new();

    mm.open_in(&in_port).expect("open MIDI IN");
    assert!(mm.in_connected(), "MIDI IN should be connected after open_in");
    mm.close_in();
    assert!(!mm.in_connected(), "MIDI IN should be disconnected after close_in");

    mm.open_out(&out_port).expect("open MIDI OUT");
    assert!(mm.out_connected(), "MIDI OUT should be connected after open_out");
    mm.close_out();
    assert!(!mm.out_connected(), "MIDI OUT should be disconnected after close_out");
}

/// Diagnostic: send fetch request and print every MIDI event received.
/// Does not assert — use this to check whether the synth responds at all.
#[test]
#[ignore = "requires real MIDI hardware — set XDX_MIDI_IN and XDX_MIDI_OUT"]
fn test_midi_diag_fetch() {
    let (in_port, out_port) = midi_ports();
    let mut mm = MidiManager::new();

    println!("Opening OUT: {out_port}");
    mm.open_out(&out_port).expect("open MIDI OUT");
    println!("Opening IN:  {in_port}");
    mm.open_in(&in_port).expect("open MIDI IN");

    println!("Sending fetch request: {:02X?}", FETCH_1VOICE);
    mm.send(FETCH_1VOICE).expect("send fetch request");

    println!("Waiting {}s for any MIDI IN events...", RECV_TIMEOUT.as_secs());
    let deadline = Instant::now() + RECV_TIMEOUT;
    let mut count = 0usize;
    while Instant::now() < deadline {
        match mm.try_recv() {
            Some(MidiEvent::SysEx(data)) => {
                count += 1;
                println!("  event {count}: SysEx ({} bytes): {:02X?}", data.len(), data);
            }
            Some(MidiEvent::Other(data)) => {
                count += 1;
                println!("  event {count}: Other: {:02X?}", data);
            }
            None => {}
        }
        std::thread::sleep(POLL_INTERVAL);
    }

    mm.close_out();
    mm.close_in();
    println!("Total events received: {count}");
}

/// Diagnostic: send 32-voice fetch request and log every raw event received.
/// Does not assert — use to verify whether the synth responds at all.
#[test]
#[ignore = "requires real MIDI hardware — set XDX_MIDI_IN and XDX_MIDI_OUT"]
fn test_diag_fetch_32voice() {
    let (in_port, out_port) = midi_ports();
    let mut mm = MidiManager::new();

    mm.open_out(&out_port).expect("open MIDI OUT");
    mm.open_in(&in_port).expect("open MIDI IN");

    println!("Sending FETCH_32VOICE: {:02X?}", FETCH_32VOICE);
    mm.send(FETCH_32VOICE).expect("send fetch 32-voice request");

    println!("Waiting 15s for MIDI IN events...");
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut count = 0usize;
    let mut sysex_total = 0usize;
    while Instant::now() < deadline {
        match mm.try_recv() {
            Some(MidiEvent::SysEx(data)) => {
                count += 1;
                sysex_total += data.len();
                println!("  event {count}: SysEx {} bytes, first 32: {:02X?}", data.len(), &data[..data.len().min(32)]);
            }
            Some(MidiEvent::Other(data)) => {
                count += 1;
                if data != [0xFE] {  // suppress Active Sensing spam
                    println!("  event {count}: Other: {:02X?}", data);
                }
            }
            None => {}
        }
        std::thread::sleep(POLL_INTERVAL);
    }
    println!("Total events: {count}, total SysEx bytes: {sysex_total}");
    mm.close_out();
    mm.close_in();
}

/// Full voice roundtrip:
///
/// Setup: send IvoryEbony to put the synth in a known state.
/// Fetch (1) — should be IvoryEbony.
/// Load Vibrabelle as (2), send to synth.
/// Fetch (2') — assert (2) == (2')  and  (1) != (2').
///
/// This test is self-contained: it does not depend on the synth's prior state.
#[test]
#[ignore = "requires real MIDI hardware — set XDX_MIDI_IN and XDX_MIDI_OUT"]
fn test_voice_roundtrip() {
    let (in_port, out_port) = midi_ports();
    let mut mm = MidiManager::new();

    mm.open_out(&out_port).expect("open MIDI OUT");
    mm.open_in(&in_port).expect("open MIDI IN");

    // Setup: load IvoryEbony into the synth to establish a known initial state
    let voice_init = dx100_decode_1voice(IVORY_EBONY_SYX).expect("decode IvoryEbony");
    let syx_init = dx100_encode_1voice(&voice_init, 0);
    println!("  [OUT] Setup: sending IvoryEbony ({} bytes)", syx_init.len());
    mm.send(&syx_init).expect("setup send IvoryEbony");
    std::thread::sleep(Duration::from_millis(200));

    // (1) — capture current synth edit-buffer (should be IvoryEbony)
    let syx1 = fetch_1voice(&mut mm);
    let voice1 = dx100_decode_1voice(&syx1).expect("decode voice (1)");
    println!("(1) synth voice: {}", voice1.name_str());

    // (2) — load Vibrabelle from testdata (a different voice)
    let voice2 = dx100_decode_1voice(VIBRABELLE_SYX).expect("decode Vibrabelle");
    println!("(2) file voice:  {}", voice2.name_str());
    let syx2 = dx100_encode_1voice(&voice2, 0);

    // Send (2) to synth
    println!("  [OUT] Sending voice (2) ({} bytes)", syx2.len());
    mm.send(&syx2).expect("send voice (2) to synth");
    std::thread::sleep(Duration::from_millis(200));

    // (2') — fetch back from synth
    let syx2_prime = fetch_1voice(&mut mm);
    let voice2_prime = dx100_decode_1voice(&syx2_prime).expect("decode voice (2')");
    println!("(2') fetched:    {}", voice2_prime.name_str());

    mm.close_out();
    mm.close_in();

    // (2) and (2') must represent the same voice
    assert_eq!(
        voice2, voice2_prime,
        "(2') fetched from synth should match (2) that was sent"
    );

    // (1) and (2') must differ — IvoryEbony != Vibrabelle
    assert_ne!(
        voice1, voice2_prime,
        "(1) IvoryEbony should differ from (2') Vibrabelle"
    );
}

/// 32-voice bank roundtrip:
///
/// Load all_voices.syx as bank (2) → Send to synth → Fetch bank (2') →
/// assert each voice (2)[i] == (2')[i] for i in 0..24.
///
/// DX100 has 24 internal voice slots; voices 24-31 in the VMEM bank are
/// unused/padding and may not round-trip exactly, so only slots 0-23 are checked.
///
/// If there is a bug in the VMEM encode/decode codec, this test will fail and
/// print which voice slot and field differ.
#[test]
#[ignore = "requires real MIDI hardware — set XDX_MIDI_IN and XDX_MIDI_OUT"]
fn test_bank_roundtrip() {
    let (in_port, out_port) = midi_ports();
    let mut mm = MidiManager::new();

    mm.open_out(&out_port).expect("open MIDI OUT");
    mm.open_in(&in_port).expect("open MIDI IN");

    // (2) — decode all_voices.syx from testdata
    let bank2 = dx100_decode_32voice(ALL_VOICES_SYX).expect("decode all_voices.syx");
    println!("(2) bank loaded: {} voices", bank2.len());
    for (i, v) in bank2.iter().enumerate().take(24) {
        println!("  [{i:02}] {}", v.name_str());
    }

    // Send (2) to synth.
    // DX100 responds with device=4 but accepts bulk dumps on device=0.
    let syx2 = dx100_encode_32voice(&bank2, 0);
    println!("  [OUT] Sending bank (2) ({} bytes)", syx2.len());
    mm.send(&syx2).expect("send bank (2) to synth");
    // Allow time for the synth to receive and store the bank internally.
    std::thread::sleep(Duration::from_secs(3));

    // (2') — fetch back from synth
    let syx2_prime = fetch_32voice(&mut mm);
    let bank2_prime = dx100_decode_32voice(&syx2_prime).expect("decode fetched bank (2')");

    mm.close_out();
    mm.close_in();

    // Compare each voice slot 0-23
    let mut all_ok = true;
    for i in 0..24usize {
        if bank2[i] != bank2_prime[i] {
            all_ok = false;
            println!("MISMATCH at voice slot {i} \"{}\":", bank2[i].name_str());
            println!("  sent:    {:?}", bank2[i]);
            println!("  fetched: {:?}", bank2_prime[i]);
        }
    }
    assert!(all_ok, "one or more voice slots did not round-trip correctly (see output above)");
}
