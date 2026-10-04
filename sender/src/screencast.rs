//! GNOME Mutter ScreenCast D-Bus Session Manager
//!
//! Encapsulates D-Bus IPC with `org.gnome.Mutter.ScreenCast` to capture virtual
//! monitors and acquire PipeWire node IDs cleanly with RAII session cleanup.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use std::collections::HashMap;
use std::error::Error;
use zbus::blocking::{Connection, Proxy};
use zbus::zvariant::{OwnedObjectPath, Value};

pub struct MutterScreenCastSession {
    conn: Connection,
    session_path: OwnedObjectPath,
    pub node_id: u32,
    #[allow(dead_code)]
    pub monitor: String,
}

impl MutterScreenCastSession {
    /// Creates and starts a new GNOME Mutter ScreenCast session for the target monitor
    pub fn create_and_start(monitor: &str) -> Result<Self, Box<dyn Error>> {
        let conn = Connection::session()?;

        // 1. Create Mutter ScreenCast Session
        let session_props: HashMap<&str, Value> = HashMap::new();
        let session_reply = conn.call_method(
            Some("org.gnome.Mutter.ScreenCast"),
            "/org/gnome/Mutter/ScreenCast",
            Some("org.gnome.Mutter.ScreenCast"),
            "CreateSession",
            &(session_props,),
        )?;

        let session_path: OwnedObjectPath = session_reply.body().deserialize()?;
        println!("\x1b[1;32m[+] Mutter Session created:\x1b[0m {}", session_path);

        // 2. Request monitor recording with cursor embedded (mode 1)
        let is_virtual = monitor.to_lowercase() == "virtual"
            || monitor.to_lowercase() == "auto";
        let target_connector = if monitor.to_lowercase() == "clone" {
            "eDP-1"
        } else if monitor.to_lowercase() == "extend" {
            "HDMI-1"
        } else {
            monitor
        };

        let (stream_path, active_monitor) = {
            let actual_connector = if is_virtual {
                "eDP-1"
            } else {
                target_connector
            };
            let mut monitor_props: HashMap<&str, Value> = HashMap::new();
            monitor_props.insert("cursor-mode", Value::from(1u32));
            match conn.call_method(
                Some("org.gnome.Mutter.ScreenCast"),
                session_path.as_str(),
                Some("org.gnome.Mutter.ScreenCast.Session"),
                "RecordMonitor",
                &(actual_connector, monitor_props),
            ) {
                Ok(stream_reply) => {
                    let sp: OwnedObjectPath = stream_reply.body().deserialize()?;
                    println!("\x1b[1;32m[+] {} ScreenCast Stream created:\x1b[0m {}", actual_connector, sp);
                    (sp, actual_connector.to_string())
                }
                Err(err) => {
                    eprintln!(
                        "\x1b[1;31m[!] RecordMonitor('{}') failed: {}. (RecordVirtual disabled to prevent libmutter SIGSEGV)\x1b[0m",
                        actual_connector, err
                    );
                    return Err(format!("RecordMonitor('{}') failed: {}", actual_connector, err).into());
                }
            }
        };


        // 3. Subscribe to PipeWireStreamAdded signal BEFORE calling Start()
        let stream_proxy = Proxy::new(
            &conn,
            "org.gnome.Mutter.ScreenCast",
            stream_path.as_str(),
            "org.gnome.Mutter.ScreenCast.Stream",
        )?;
        let mut signal_iter = stream_proxy.receive_signal("PipeWireStreamAdded")?;

        // 4. Start the ScreenCast Session
        conn.call_method(
            Some("org.gnome.Mutter.ScreenCast"),
            session_path.as_str(),
            Some("org.gnome.Mutter.ScreenCast.Session"),
            "Start",
            &(),
        )?;

        // 5. Receive PipeWire Node ID
        let node_id = match signal_iter.next() {
            Some(sig) => {
                let (id,): (u32,) = sig.body().deserialize()?;
                id
            }
            None => {
                return Err("Did not receive PipeWireStreamAdded signal from Mutter".into());
            }
        };

        println!("\x1b[1;32m[+] PipeWire Node ID for {}:\x1b[0m {}", monitor, node_id);

        Ok(Self {
            conn,
            session_path,
            node_id,
            monitor: active_monitor,
        })
    }

    /// Stops the ScreenCast session explicitly
    pub fn stop(&self) {
        let _ = self.conn.call_method(
            Some("org.gnome.Mutter.ScreenCast"),
            self.session_path.as_str(),
            Some("org.gnome.Mutter.ScreenCast.Session"),
            "Stop",
            &(),
        );
    }
}

impl Drop for MutterScreenCastSession {
    fn drop(&mut self) {
        self.stop();
    }
}

/// GNOME Session Inhibitor to prevent screensaver, display sleep and idle lock
pub struct GnomeSessionInhibitor {
    conn: Connection,
    cookie: u32,
}

impl GnomeSessionInhibitor {
    pub fn inhibit(app_id: &str, reason: &str) -> Option<Self> {
        let conn = Connection::session().ok()?;
        // Flags: 4 = Inhibit Suspend, 8 = Inhibit Idle/ScreenSaver (4 | 8 = 12)
        let reply = conn.call_method(
            Some("org.gnome.SessionManager"),
            "/org/gnome/SessionManager",
            Some("org.gnome.SessionManager"),
            "Inhibit",
            &(app_id, 0u32, reason, 12u32),
        ).ok()?;
        let cookie: u32 = reply.body().deserialize().ok()?;
        println!("\x1b[1;32m[+] GNOME Session Inhibit Active (Cookie: {}):\x1b[0m Bloqueio de tela e suspensão inibidos durante transmissão", cookie);
        Some(Self { conn, cookie })
    }
}

impl Drop for GnomeSessionInhibitor {
    fn drop(&mut self) {
        let _ = self.conn.call_method(
            Some("org.gnome.SessionManager"),
            "/org/gnome/SessionManager",
            Some("org.gnome.SessionManager"),
            "Uninhibit",
            &(self.cookie,),
        );
        println!("\x1b[1;33m[*] GNOME Session Inhibit liberado.\x1b[0m");
    }
}

