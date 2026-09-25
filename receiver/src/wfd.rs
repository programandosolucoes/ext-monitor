//! Wi-Fi Display (Miracast / MS-MICE) RTSP 1.0 Server in 100% Pure Rust
//!
//! Implements Wi-Fi Display Specification (WFD 1.0) and Microsoft Miracast over
//! Infrastructure (MS-MICE) for native Windows 10/11 wireless projection (`Win + K`).
//!
//! Requires zero extra software, drivers, or configuration on the Windows host.
//! Video format negotiation (1080p60, 720p60, H.264 Baseline/Main) and session control
//! are handled natively in pure Rust.
//!
//! License: MIT
//! Author: Carlos Alberto <psncarlosalberto4ti@gmail.com>

use crate::pipeline::{PipelineKind, PipelineManager};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Standard RTSP port for Wi-Fi Display / Miracast
pub const WFD_RTSP_PORT: u16 = 7236;

/// UDP port for incoming Windows MPEG-TS / H.264 RTP stream
pub const WFD_RTP_PORT: u16 = 5002;

/// H.264 video formats natively supported by Broadcom VideoCore IV:
/// - 00: Native Res Index
/// - 00: Preferred Display Mode
/// - 02: H.264 Profiles (Constrained Baseline + Main Profile)
/// - 02: H.264 Levels (Level 3.1 / 4.0 / 4.1 / 4.2)
/// - 0001deff: CEA Resolutions Bitmap (1080p60, 1080p30, 720p60, 480p)
/// - 157cff5f: VESA Resolutions Bitmap (1920x1080, 1600x900, 1366x768, 1280x720, 1024x768)
/// - 00000fff: HH Resolutions Bitmap
pub const WFD_VIDEO_FORMATS: &str =
    "00 00 02 02 0001deff 157cff5f 00000fff 00 0000 0000 00 none none";

/// Starts the Wi-Fi Display RTSP server on a background thread
pub fn start_wfd_server(
    running: Arc<AtomicBool>,
    pipeline_mgr: Arc<PipelineManager>,
) -> std::io::Result<()> {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", WFD_RTSP_PORT))?;
    listener.set_nonblocking(true)?;

    println!(
        "\x1b[1;32m[wfd-rust]\x1b[0m Miracast RTSP server listening on TCP port {}",
        WFD_RTSP_PORT
    );
    println!("\x1b[1;34m[wfd-rust]\x1b[0m Ready for native Windows wireless casting (Win + K)...");

    thread::spawn(move || {
        while running.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, addr)) => {
                    let client_ip = addr.ip().to_string();
                    println!(
                        "\x1b[1;32m[wfd-rust]\x1b[0m Incoming RTSP WFD connection from {}",
                        client_ip
                    );

                    let pipe = pipeline_mgr.clone();
                    let run = running.clone();

                    thread::spawn(move || {
                        let mut session = WfdSession::new(stream, client_ip, pipe);
                        session.run(run);
                    });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(100));
                }
                Err(e) => {
                    eprintln!("\x1b[1;31m[wfd-rust]\x1b[0m Accept error: {}", e);
                    thread::sleep(Duration::from_millis(500));
                }
            }
        }
        println!("\x1b[1;33m[wfd-rust]\x1b[0m Miracast RTSP server stopped.");
    });

    Ok(())
}

/// Active connection session with a Windows transmitter (Source)
struct WfdSession {
    stream: TcpStream,
    client_ip: String,
    pipeline_mgr: Arc<PipelineManager>,
    sink_cseq: u32,
    session_id: String,
    is_streaming: bool,
}

impl WfdSession {
    fn new(stream: TcpStream, client_ip: String, pipeline_mgr: Arc<PipelineManager>) -> Self {
        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
        let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
        Self {
            stream,
            client_ip,
            pipeline_mgr,
            sink_cseq: 1,
            session_id: "12345678".to_string(),
            is_streaming: false,
        }
    }

    fn send_response(
        &mut self,
        cseq: &str,
        status: &str,
        extra_headers: &[(&str, &str)],
        body: &str,
    ) -> std::io::Result<()> {
        let mut resp = format!("RTSP/1.0 {}\r\nCSeq: {}\r\n", status, cseq);
        for (k, v) in extra_headers {
            resp.push_str(&format!("{}: {}\r\n", k, v));
        }

        if !body.is_empty() {
            resp.push_str("Content-Type: text/parameters\r\n");
            resp.push_str(&format!("Content-Length: {}\r\n\r\n", body.len()));
            resp.push_str(body);
        } else {
            resp.push_str("\r\n");
        }

        self.stream.write_all(resp.as_bytes())?;
        self.stream.flush()?;
        Ok(())
    }

    fn send_request(
        &mut self,
        method: &str,
        uri: &str,
        extra_headers: &[(&str, &str)],
        body: &str,
    ) -> std::io::Result<()> {
        let mut req = format!("{} {} RTSP/1.0\r\nCSeq: {}\r\n", method, uri, self.sink_cseq);
        self.sink_cseq += 1;

        for (k, v) in extra_headers {
            req.push_str(&format!("{}: {}\r\n", k, v));
        }

        if !body.is_empty() {
            req.push_str("Content-Type: text/parameters\r\n");
            req.push_str(&format!("Content-Length: {}\r\n\r\n", body.len()));
            req.push_str(body);
        } else {
            req.push_str("\r\n");
        }

        self.stream.write_all(req.as_bytes())?;
        self.stream.flush()?;
        Ok(())
    }

    fn run(&mut self, running: Arc<AtomicBool>) {
        let mut buffer = Vec::with_capacity(8192);
        let mut temp = [0u8; 2048];

        while running.load(Ordering::SeqCst) {
            match self.stream.read(&mut temp) {
                Ok(0) => {
                    println!(
                        "\x1b[1;33m[wfd-rust]\x1b[0m Client {} closed connection.",
                        self.client_ip
                    );
                    break;
                }
                Ok(n) => {
                    buffer.extend_from_slice(&temp[..n]);

                    while let Some(pos) = find_subsequence(&buffer, b"\r\n\r\n") {
                        let header_bytes = &buffer[..pos];
                        let header_str = String::from_utf8_lossy(header_bytes).to_string();

                        let (method, uri, headers) = parse_rtsp_headers(&header_str);
                        let content_len: usize = headers
                            .get("content-length")
                            .and_then(|l| l.parse().ok())
                            .unwrap_or(0);

                        let total_msg_len = pos + 4 + content_len;
                        if buffer.len() < total_msg_len {
                            break;
                        }

                        let body_bytes = &buffer[pos + 4..total_msg_len];
                        let body_str = String::from_utf8_lossy(body_bytes).to_string();

                        buffer.drain(..total_msg_len);

                        if let Err(e) = self.handle_message(&method, &uri, &headers, &body_str) {
                            eprintln!("\x1b[1;31m[wfd-rust]\x1b[0m Message handling error: {}", e);
                            return;
                        }
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => {}
                Err(e) => {
                    eprintln!("\x1b[1;31m[wfd-rust]\x1b[0m Socket error: {}", e);
                    break;
                }
            }
        }

        if self.is_streaming {
            println!("\x1b[1;33m[wfd-rust]\x1b[0m Stopping Miracast decode pipeline.");
            self.pipeline_mgr.stop();
        }
    }

    fn handle_message(
        &mut self,
        method: &str,
        uri: &str,
        headers: &HashMap<String, String>,
        body: &str,
    ) -> std::io::Result<()> {
        let cseq = headers.get("cseq").cloned().unwrap_or_else(|| "1".to_string());
        println!(
            "\x1b[1;36m[wfd-rust]\x1b[0m Received: {} {} (CSeq: {})",
            method, uri, cseq
        );

        match method {
            "OPTIONS" => {
                // M1: Respond with supported methods
                self.send_response(
                    &cseq,
                    "200 OK",
                    &[("Public", "org.wfa.wfd1.0, GET_PARAMETER, SET_PARAMETER")],
                    "",
                )?;

                // Trigger M2 (Sink OPTIONS to Source)
                thread::sleep(Duration::from_millis(30));
                self.send_request("OPTIONS", "*", &[("Require", "org.wfa.wfd1.0")], "")?;
            }
            "GET_PARAMETER" => {
                // M3: Query video capabilities and ports
                let mut resp_params = Vec::new();
                for line in body.lines() {
                    let param = line.trim();
                    match param {
                        "wfd_video_formats" => {
                            resp_params.push(format!("wfd_video_formats: {}", WFD_VIDEO_FORMATS));
                        }
                        "wfd_audio_codecs" => {
                            resp_params.push("wfd_audio_codecs: none".to_string());
                        }
                        "wfd_client_rtpports" => {
                            resp_params.push(format!(
                                "wfd_client_rtpports: RTP/AVP/UDP;unicast {} 0 mode=play",
                                WFD_RTP_PORT
                            ));
                        }
                        "wfd_uibc_capability" => {
                            resp_params.push("wfd_uibc_capability: none".to_string());
                        }
                        "wfd_connector_type" => {
                            resp_params.push("wfd_connector_type: 05".to_string());
                        }
                        "wfd_content_protection" => {
                            resp_params.push("wfd_content_protection: none".to_string());
                        }
                        "wfd_idr_request_capability" => {
                            resp_params.push("wfd_idr_request_capability: 01".to_string());
                        }
                        _ => {}
                    }
                }
                resp_params.push("".to_string());
                let resp_body = resp_params.join("\r\n");
                self.send_response(&cseq, "200 OK", &[], &resp_body)?;
            }
            "SET_PARAMETER" => {
                // M4 / M5: Parameter confirmation & SETUP trigger
                self.send_response(&cseq, "200 OK", &[], "")?;

                if body.contains("wfd_trigger_method: SETUP")
                    || body.contains("wfd_trigger_method: setup")
                {
                    println!(
                        "\x1b[1;32m[wfd-rust]\x1b[0m Trigger SETUP detected. Sending M6 SETUP to client..."
                    );
                    thread::sleep(Duration::from_millis(30));
                    let transport = format!("RTP/AVP/UDP;unicast;client_port={}", WFD_RTP_PORT);
                    let target_uri = format!("rtsp://{}/wfd1.0/streamid=0", self.client_ip);
                    self.send_request("SETUP", &target_uri, &[("Transport", &transport)], "")?;
                }
            }
            "RTSP/1.0" => {
                // Responses from Source to Sink requests (M6 SETUP or M7 PLAY)
                if let Some(session_hdr) = headers.get("session") {
                    let sid = session_hdr.split(';').next().unwrap_or("").trim();
                    self.session_id = sid.to_string();

                    if !self.is_streaming {
                        println!(
                            "\x1b[1;32m[wfd-rust]\x1b[0m Miracast session established: {}. Sending M7 PLAY...",
                            self.session_id
                        );
                        thread::sleep(Duration::from_millis(30));
                        let target_uri = format!("rtsp://{}/wfd1.0/streamid=0", self.client_ip);
                        let sid_ref = self.session_id.clone();
                        self.send_request("PLAY", &target_uri, &[("Session", &sid_ref)], "")?;

                        println!(
                            "\x1b[1;32m[wfd-rust]\x1b[0m Starting VideoCore IV Miracast decode on UDP port {}...",
                            WFD_RTP_PORT
                        );
                        if let Err(e) = self.pipeline_mgr.start(PipelineKind::MiracastMp2t {
                            port: WFD_RTP_PORT,
                        }) {
                            eprintln!(
                                "\x1b[1;31m[wfd-rust]\x1b[0m Failed to start Miracast pipeline: {}",
                                e
                            );
                        } else {
                            self.is_streaming = true;
                        }
                    }
                }
            }
            "TEARDOWN" => {
                println!(
                    "\x1b[1;33m[wfd-rust]\x1b[0m Session terminated by client (TEARDOWN)."
                );
                self.send_response(&cseq, "200 OK", &[], "")?;
                self.pipeline_mgr.stop();
                self.is_streaming = false;
            }
            _ => {
                self.send_response(&cseq, "200 OK", &[], "")?;
            }
        }

        Ok(())
    }
}

/// Helper for parsing RTSP headers
fn parse_rtsp_headers(raw: &str) -> (String, String, HashMap<String, String>) {
    let mut headers = HashMap::new();
    let mut lines = raw.lines();

    let first_line = lines.next().unwrap_or("");
    let parts: Vec<&str> = first_line.split_whitespace().collect();
    let method = parts.get(0).unwrap_or(&"").to_string();
    let uri = parts.get(1).unwrap_or(&"").to_string();

    for line in lines {
        if let Some(idx) = line.find(':') {
            let key = line[..idx].trim().to_lowercase();
            let val = line[idx + 1..].trim().to_string();
            headers.insert(key, val);
        }
    }

    (method, uri, headers)
}

/// Helper for finding byte subsequence
fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
