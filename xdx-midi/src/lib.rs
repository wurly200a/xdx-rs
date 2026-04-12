use midir::{Ignore, MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use std::sync::mpsc::{self, Receiver};

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct MidiError(pub String);

impl std::fmt::Display for MidiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

// ── Events ────────────────────────────────────────────────────────────────────

pub enum MidiEvent {
    SysEx(Vec<u8>),
    Other(Vec<u8>),
}

// ── MidiManager ───────────────────────────────────────────────────────────────

pub struct MidiManager {
    in_conn:  Option<MidiInputConnection<()>>,
    out_conn: Option<MidiOutputConnection>,
    rx:       Option<Receiver<MidiEvent>>,
    pub in_port_name:  Option<String>,
    pub out_port_name: Option<String>,
}

impl MidiManager {
    pub fn new() -> Self {
        Self {
            in_conn:       None,
            out_conn:      None,
            rx:            None,
            in_port_name:  None,
            out_port_name: None,
        }
    }

    /// Enumerate available MIDI IN port names.
    pub fn list_in_ports() -> Vec<String> {
        let Ok(mi) = MidiInput::new("xdx-list") else { return vec![] };
        mi.ports().iter().filter_map(|p| mi.port_name(p).ok()).collect()
    }

    /// Enumerate available MIDI OUT port names.
    pub fn list_out_ports() -> Vec<String> {
        let Ok(mo) = MidiOutput::new("xdx-list") else { return vec![] };
        mo.ports().iter().filter_map(|p| mo.port_name(p).ok()).collect()
    }

    /// Open MIDI IN connection. Received messages are queued for `try_recv`.
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
        let conn = mi.connect(&port, "xdx-in", move |_ts, msg, _| {
            let ev = if msg.first() == Some(&0xF0) {
                MidiEvent::SysEx(msg.to_vec())
            } else {
                MidiEvent::Other(msg.to_vec())
            };
            let _ = tx.send(ev);
        }, ()).map_err(|e| MidiError(e.to_string()))?;

        self.in_conn = Some(conn);
        self.rx = Some(rx);
        self.in_port_name = Some(port_name.to_string());
        Ok(())
    }

    pub fn close_in(&mut self) {
        if let Some(c) = self.in_conn.take() { c.close(); }
        self.rx = None;
        self.in_port_name = None;
    }

    /// Open MIDI OUT connection.
    pub fn open_out(&mut self, port_name: &str) -> Result<(), MidiError> {
        self.close_out();
        let mo = MidiOutput::new("xdx-out")
            .map_err(|e| MidiError(e.to_string()))?;

        let ports = mo.ports();
        let port = ports.iter()
            .find(|p| mo.port_name(p).ok().as_deref() == Some(port_name))
            .ok_or_else(|| MidiError(format!("OUT port not found: {port_name}")))?
            .clone();

        let conn = mo.connect(&port, "xdx-out")
            .map_err(|e| MidiError(e.to_string()))?;

        self.out_conn = Some(conn);
        self.out_port_name = Some(port_name.to_string());
        Ok(())
    }

    pub fn close_out(&mut self) {
        if let Some(c) = self.out_conn.take() { c.close(); }
        self.out_port_name = None;
    }

    /// Send raw MIDI bytes via the open OUT connection.
    pub fn send(&mut self, data: &[u8]) -> Result<(), MidiError> {
        self.out_conn.as_mut()
            .ok_or_else(|| MidiError("MIDI OUT not connected".to_string()))?
            .send(data)
            .map_err(|e| MidiError(e.to_string()))
    }

    /// Non-blocking receive. Returns the next queued MIDI event, if any.
    pub fn try_recv(&mut self) -> Option<MidiEvent> {
        self.rx.as_ref()?.try_recv().ok()
    }

    pub fn in_connected(&self)  -> bool { self.in_conn.is_some() }
    pub fn out_connected(&self) -> bool { self.out_conn.is_some() }
}
