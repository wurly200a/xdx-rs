// ── Common types ──────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct MidiError(pub String);

impl std::fmt::Display for MidiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

pub enum MidiEvent {
    SysEx(Vec<u8>),
    Other(Vec<u8>),
}

// ── Real MIDI backend (midir / WinMM / CoreMIDI / ALSA) ──────────────────────

#[cfg(not(feature = "virtual-midi"))]
mod real {
    use super::{MidiError, MidiEvent};
    use midir::{Ignore, MidiInput, MidiInputConnection, MidiOutput};
    use std::sync::mpsc::{self, Receiver, Sender};

    pub struct MidiManager {
        in_conn:    Option<MidiInputConnection<()>>,
        // OUT is owned by a worker thread; we communicate via a channel.
        // Some(data) → send the bytes.  None → send done, close port and exit.
        // Dropping out_tx also causes the worker to exit and close the port.
        out_tx:     Option<Sender<Option<Vec<u8>>>>,
        rx:         Option<Receiver<MidiEvent>>,
        pub in_port_name:  Option<String>,
        pub out_port_name: Option<String>,
    }

    impl MidiManager {
        pub fn new() -> Self {
            Self {
                in_conn:       None,
                out_tx:        None,
                rx:            None,
                in_port_name:  None,
                out_port_name: None,
            }
        }

        pub fn list_in_ports() -> Vec<String> {
            let Ok(mi) = MidiInput::new("xdx-list") else { return vec![] };
            mi.ports().iter().filter_map(|p| mi.port_name(p).ok()).collect()
        }

        pub fn list_out_ports() -> Vec<String> {
            let Ok(mo) = MidiOutput::new("xdx-list") else { return vec![] };
            mo.ports().iter().filter_map(|p| mo.port_name(p).ok()).collect()
        }

        pub fn open_in(&mut self, port_name: &str) -> Result<(), MidiError> {
            self.close_in();
            let mut mi = MidiInput::new("xdx-in")
                .map_err(|e| MidiError(e.to_string()))?;
            mi.ignore(Ignore::None); // receive SysEx

            let ports = mi.ports();
            let port = ports.iter()
                .find(|p| mi.port_name(p).ok().as_deref() == Some(port_name))
                .ok_or_else(|| MidiError(format!("IN port not found: {port_name}")))?
                .clone();

            let (tx, rx) = mpsc::channel();
            let mut sysex_buf: Vec<u8> = Vec::new();
            let conn = mi.connect(&port, "xdx-in", move |_ts, msg, _| {
                if msg.is_empty() {
                    return;
                }
                // Real-Time messages (0xF8-0xFF: clock, active sensing, reset…)
                // are single-byte and may arrive at any time, including during a
                // multi-packet SysEx.  Deliver them as Other without interrupting
                // the SysEx accumulator.
                if msg[0] >= 0xF8 {
                    let _ = tx.send(MidiEvent::Other(msg.to_vec()));
                    return;
                }
                if msg[0] == 0xF0 {
                    // Start of a new SysEx (may be complete or first chunk)
                    sysex_buf.clear();
                    sysex_buf.extend_from_slice(msg);
                } else if !sysex_buf.is_empty() {
                    // Continuation chunk of a multi-packet SysEx
                    sysex_buf.extend_from_slice(msg);
                } else {
                    // Regular (non-SysEx) MIDI message
                    let _ = tx.send(MidiEvent::Other(msg.to_vec()));
                    return;
                }
                // Deliver only when the complete SysEx (ending F7) has arrived
                if sysex_buf.last() == Some(&0xF7) {
                    let _ = tx.send(MidiEvent::SysEx(sysex_buf.clone()));
                    sysex_buf.clear();
                }
            }, ()).map_err(|e| MidiError(e.to_string()))?;

            self.in_conn = Some(conn);
            self.rx = Some(rx);
            self.in_port_name = Some(port_name.to_string());
            Ok(())
        }

        pub fn close_in(&mut self) {
            self.rx = None;
            self.in_port_name = None;
            if let Some(c) = self.in_conn.take() {
                // Close on a background thread: on Windows/WinMM the USB MIDI
                // class driver may block midiInReset() while cancelling pending
                // read URBs.  The 8 s timeout lets the caller proceed even if
                // the close thread is still running.
                let (tx, rx) = std::sync::mpsc::channel::<()>();
                std::thread::spawn(move || {
                    c.close();
                    let _ = tx.send(());
                });
                let _ = rx.recv_timeout(std::time::Duration::from_secs(8));
            }
        }

        pub fn open_out(&mut self, port_name: &str) -> Result<(), MidiError> {
            self.close_out();
            let mo = MidiOutput::new("xdx-out")
                .map_err(|e| MidiError(e.to_string()))?;

            let ports = mo.ports();
            let port = ports.iter()
                .find(|p| mo.port_name(p).ok().as_deref() == Some(port_name))
                .ok_or_else(|| MidiError(format!("OUT port not found: {port_name}")))?
                .clone();

            let mut conn = mo.connect(&port, "xdx-out")
                .map_err(|e| MidiError(e.to_string()))?;

            // Worker thread owns the connection.  The GUI thread sends byte
            // buffers through `tx`; the worker forwards them to MIDI OUT.
            // Dropping `tx` (via close_out) causes recv() to return Err,
            // which cleanly exits the worker and closes the port.
            let (tx, rx) = mpsc::channel::<Option<Vec<u8>>>();
            std::thread::spawn(move || {
                while let Ok(Some(data)) = rx.recv() {
                    conn.send(&data).ok();
                }
                // Received None sentinel or sender dropped — close port cleanly.
                conn.close();
            });

            self.out_tx = Some(tx);
            self.out_port_name = Some(port_name.to_string());
            Ok(())
        }

        pub fn close_out(&mut self) {
            self.out_tx = None;  // drops Sender → worker thread exits
            self.out_port_name = None;
        }

        pub fn send(&mut self, data: &[u8]) -> Result<(), MidiError> {
            self.out_tx.as_ref()
                .ok_or_else(|| MidiError("MIDI OUT not connected".to_string()))?
                .send(Some(data.to_vec()))
                .map_err(|e| MidiError(e.to_string()))
        }

        /// Queue data then close OUT after the worker finishes sending.
        /// The indicator goes gray immediately; the port closes in the background
        /// once the last byte is transmitted.
        pub fn send_then_close(&mut self, data: &[u8]) -> Result<(), MidiError> {
            let tx = self.out_tx.as_ref()
                .ok_or_else(|| MidiError("MIDI OUT not connected".to_string()))?;
            tx.send(Some(data.to_vec())).map_err(|e| MidiError(e.to_string()))?;
            tx.send(None).map_err(|e| MidiError(e.to_string()))?;
            self.out_tx = None;
            self.out_port_name = None;
            Ok(())
        }

        pub fn try_recv(&mut self) -> Option<MidiEvent> {
            self.rx.as_ref()?.try_recv().ok()
        }

        pub fn in_connected(&self)  -> bool { self.in_conn.is_some() }
        pub fn out_connected(&self) -> bool { self.out_tx.is_some() }
    }

    impl Drop for MidiManager {
        fn drop(&mut self) {
            // Close OUT first: signal the worker thread to stop, so no more
            // messages are queued before we tear down the IN connection.
            self.close_out();
            self.close_in();
        }
    }
}

#[cfg(not(feature = "virtual-midi"))]
pub use real::MidiManager;

// ── Virtual MIDI backend (stub — no system deps) ──────────────────────────────

#[cfg(feature = "virtual-midi")]
mod stub {
    use super::{MidiError, MidiEvent};

    pub struct MidiManager {
        in_open:  bool,
        out_open: bool,
        pub in_port_name:  Option<String>,
        pub out_port_name: Option<String>,
    }

    impl MidiManager {
        pub fn new() -> Self {
            Self { in_open: false, out_open: false, in_port_name: None, out_port_name: None }
        }

        pub fn list_in_ports() -> Vec<String> {
            vec!["Virtual MIDI IN (stub)".to_string()]
        }

        pub fn list_out_ports() -> Vec<String> {
            vec!["Virtual MIDI OUT (stub)".to_string()]
        }

        pub fn open_in(&mut self, port_name: &str) -> Result<(), MidiError> {
            self.in_open = true;
            self.in_port_name = Some(port_name.to_string());
            Ok(())
        }

        pub fn close_in(&mut self) {
            self.in_open = false;
            self.in_port_name = None;
        }

        pub fn open_out(&mut self, port_name: &str) -> Result<(), MidiError> {
            self.out_open = true;
            self.out_port_name = Some(port_name.to_string());
            Ok(())
        }

        pub fn close_out(&mut self) {
            self.out_open = false;
            self.out_port_name = None;
        }

        pub fn send(&mut self, _data: &[u8]) -> Result<(), MidiError> { Ok(()) }

        pub fn try_recv(&mut self) -> Option<MidiEvent> { None }

        pub fn in_connected(&self)  -> bool { self.in_open }
        pub fn out_connected(&self) -> bool { self.out_open }
    }
}

#[cfg(feature = "virtual-midi")]
pub use stub::MidiManager;
