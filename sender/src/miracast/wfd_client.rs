//! Wi-Fi Display (WFD 1.0 / MS-MICE) RTSP 1.0 Source State Machine in 100% Pure Rust
//!
//! Implements the Wi-Fi Display Specification (WFD 1.0) and Microsoft Miracast over
//! Infrastructure (MS-MICE) client (transmitter / source) protocol.
//!
//! Features:
//! - Pure Rust with zero external C library dependencies.
//! - Standard RTSP port 7236 and MS-MICE port 7250 support.
//! - Full M1-M7 sequence:
//!   - M1: Source sends OPTIONS with Require: org.wfa.wfd1.0
//!   - M2: Handles incoming reverse OPTIONS from Sink
//!   - M3: Source sends GET_PARAMETER querying video formats and client RTP ports
//!   - M4: Source sends SET_PARAMETER configuring video formats & presentation URL
//!   - M5: Source sends SET_PARAMETER triggering SETUP
//!   - M6: Source sends SETUP (and handles incoming reverse SETUP from Pi Zero sinks)
//!   - M7: Source sends PLAY (and handles incoming reverse PLAY from Pi Zero sinks)
//!   - TEARDOWN: Clean RTSP TEARDOWN with socket shutdown (< 100ms)
//! - Socket timeouts (5s default) and graceful disconnection.

use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::net::{Shutdown, TcpStream, ToSocketAddrs};
use std::time::Duration;

/// Standard RTSP port for Wi-Fi Display / Miracast
pub const WFD_RTSP_PORT: u16 = 7236;

/// Standard MS-MICE signaling port (Miracast over Infrastructure)
pub const WFD_MICE_PORT: u16 = 7250;

/// Default UDP RTP port for MPEG-TS streaming
pub const DEFAULT_RTP_PORT: u16 = 5002;

/// Standard 720p60 CEA index 6 format descriptor for VideoCore IV and generic Miracast sinks
pub const WFD_VIDEO_FORMAT_720P60: &str =
    "30 00 01 02 00000069 00000000 00000000 00 0000 0000 00 none none";

/// Parsed RTSP message (either Request or Response)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RtspMessage {
    Response {
        version: String,
        status_code: u16,
        reason: String,
        headers: HashMap<String, String>,
        body: String,
    },
    Request {
        method: String,
        uri: String,
        version: String,
        headers: HashMap<String, String>,
        body: String,
    },
}

impl RtspMessage {
    /// Returns the sequence number (CSeq) from headers if present.
    pub fn cseq(&self) -> Option<u32> {
        match self {
            RtspMessage::Response { headers, .. } | RtspMessage::Request { headers, .. } => {
                headers.get("cseq").and_then(|v| v.parse().ok())
            }
        }
    }

    /// Returns the response status code (e.g. 200) or None if this is a request.
    pub fn status_code(&self) -> Option<u16> {
        match self {
            RtspMessage::Response { status_code, .. } => Some(*status_code),
            RtspMessage::Request { .. } => None,
        }
    }

    /// Checks if response status code is 2xx.
    pub fn is_success(&self) -> bool {
        match self {
            RtspMessage::Response { status_code, .. } => *status_code >= 200 && *status_code < 300,
            RtspMessage::Request { .. } => false,
        }
    }

    /// Returns a header value by name (case-insensitive).
    pub fn header(&self, name: &str) -> Option<&str> {
        let key = name.to_lowercase();
        match self {
            RtspMessage::Response { headers, .. } | RtspMessage::Request { headers, .. } => {
                headers.get(&key).map(|s| s.as_str())
            }
        }
    }

    /// Returns the message body.
    pub fn body(&self) -> &str {
        match self {
            RtspMessage::Response { body, .. } | RtspMessage::Request { body, .. } => body,
        }
    }
}

/// Wi-Fi Display (WFD) RTSP 1.0 Source Client State Machine
pub struct WfdClient {
    stream: TcpStream,
    target_ip: String,
    target_port: u16,
    cseq: u32,
    session_id: Option<String>,
    presentation_url: String,
    negotiated_rtp_port: Option<u16>,
    read_buf: Vec<u8>,
    is_teared_down: bool,
}

impl WfdClient {
    /// Creates a new `WfdClient` wrapping an already-connected `TcpStream`.
    pub fn from_stream(stream: TcpStream, target_ip: &str, target_port: u16) -> Self {
        Self {
            stream,
            target_ip: target_ip.to_string(),
            target_port,
            cseq: 1,
            session_id: None,
            presentation_url: String::new(),
            negotiated_rtp_port: None,
            read_buf: Vec::with_capacity(4096),
            is_teared_down: false,
        }
    }

    /// Connects to a Miracast sink at `(target_ip, target_port)` with a default 5-second timeout.
    /// If connecting to MS-MICE port 7250, automatically performs the MS-MICE handshake.
    pub fn connect(target_ip: &str, target_port: u16) -> Result<Self, io::Error> {
        Self::connect_timeout(target_ip, target_port, Duration::from_secs(5))
    }

    /// Connects to a Miracast sink with a custom timeout.
    pub fn connect_timeout(
        target_ip: &str,
        target_port: u16,
        timeout: Duration,
    ) -> Result<Self, io::Error> {
        let addrs = (target_ip, target_port).to_socket_addrs()?;
        let mut last_err = None;
        let mut connected = None;

        for addr in addrs {
            match TcpStream::connect_timeout(&addr, timeout) {
                Ok(s) => {
                    connected = Some(s);
                    break;
                }
                Err(e) => last_err = Some(e),
            }
        }

        let stream = connected.ok_or_else(|| {
            last_err.unwrap_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotConnected,
                    format!("Failed to connect to sink at {}:{}", target_ip, target_port),
                )
            })
        })?;

        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;
        stream.set_nodelay(true)?;

        let mut client = Self::from_stream(stream, target_ip, target_port);

        if target_port == WFD_MICE_PORT {
            client.perform_mice_handshake()?;
        }

        Ok(client)
    }

    /// Performs MS-MICE (Miracast over Infrastructure) initial handshake on port 7250:
    /// sends SOURCE_READY TLV/text packet and waits for acknowledgment.
    pub fn perform_mice_handshake(&mut self) -> Result<(), io::Error> {
        let pkt = Self::build_mice_source_ready(WFD_RTSP_PORT);
        self.stream.write_all(&pkt)?;
        self.stream.flush()?;

        let mut buf = [0u8; 512];
        let mut total = 0;
        while total < 4 {
            let n = self.stream.read(&mut buf[total..])?;
            if n == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "MS-MICE sink closed connection before sending ACK",
                ));
            }
            total += n;
            if total >= 4 {
                break;
            }
        }

        self.read_buf.clear();
        Ok(())
    }

    /// Negotiates the full WFD RTSP session (M1-M7 sequence).
    /// Returns the negotiated destination UDP RTP port on success.
    pub fn negotiate_session(&mut self) -> Result<u16, io::Error> {
        let presentation_url = format!("rtsp://{}/wfd1.0/streamid=0", self.target_ip);
        self.presentation_url = presentation_url.clone();

        // -------------------------------------------------------------
        // M1: OPTIONS * RTSP/1.0
        // -------------------------------------------------------------
        let m1 = Self::build_options_request(&self.target_ip, self.target_port, self.cseq);
        let m1_cseq = self.cseq;
        self.cseq += 1;
        self.stream.write_all(m1.as_bytes())?;
        self.stream.flush()?;

        let resp1 = self.read_response_handling_requests(m1_cseq)?;
        if !resp1.is_success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("M1 OPTIONS rejected by sink: status {:?}", resp1.status_code()),
            ));
        }

        // -------------------------------------------------------------
        // M2: Check for incoming reverse OPTIONS from Sink
        // -------------------------------------------------------------
        self.process_pending_requests()?;

        // If not already buffered, wait briefly (up to 100ms) for reverse OPTIONS
        let _ = self.stream.set_read_timeout(Some(Duration::from_millis(100)));
        let mut temp = [0u8; 1024];
        match self.stream.read(&mut temp) {
            Ok(0) => {}
            Ok(n) => {
                self.read_buf.extend_from_slice(&temp[..n]);
                self.process_pending_requests()?;
            }
            Err(_) => {} // Timeout or WouldBlock is normal if sink does not send M2
        }
        let _ = self.stream.set_read_timeout(Some(Duration::from_secs(5)));

        // -------------------------------------------------------------
        // M3: GET_PARAMETER (query video formats, audio codecs, ports)
        // -------------------------------------------------------------
        let m3_params = [
            "wfd_video_formats",
            "wfd_audio_codecs",
            "wfd_client_rtpports",
            "wfd_uibc_capability",
        ];
        let m3 = Self::build_get_parameter_request(&presentation_url, self.cseq, &m3_params);
        let m3_cseq = self.cseq;
        self.cseq += 1;
        self.stream.write_all(m3.as_bytes())?;
        self.stream.flush()?;

        let resp3 = self.read_response_handling_requests(m3_cseq)?;
        if !resp3.is_success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("M3 GET_PARAMETER rejected: status {:?}", resp3.status_code()),
            ));
        }
        if let Some(port) = Self::parse_rtp_port_from_transport(resp3.body()) {
            self.negotiated_rtp_port = Some(port);
        }

        // -------------------------------------------------------------
        // M4: SET_PARAMETER (video formats & presentation URL)
        // -------------------------------------------------------------
        let m4 = Self::build_set_parameter_video_formats(
            &presentation_url,
            self.cseq,
            WFD_VIDEO_FORMAT_720P60,
            &self.target_ip,
        );
        let m4_cseq = self.cseq;
        self.cseq += 1;
        self.stream.write_all(m4.as_bytes())?;
        self.stream.flush()?;

        let resp4 = self.read_response_handling_requests(m4_cseq)?;
        if !resp4.is_success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("M4 SET_PARAMETER rejected: status {:?}", resp4.status_code()),
            ));
        }

        // -------------------------------------------------------------
        // M5: SET_PARAMETER (trigger SETUP)
        // -------------------------------------------------------------
        let m5 = Self::build_set_parameter_trigger_setup(&presentation_url, self.cseq);
        let m5_cseq = self.cseq;
        self.cseq += 1;
        self.stream.write_all(m5.as_bytes())?;
        self.stream.flush()?;

        let resp5 = self.read_response_handling_requests(m5_cseq)?;
        if !resp5.is_success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("M5 trigger SETUP rejected: status {:?}", resp5.status_code()),
            ));
        }

        // Process any reverse SETUP already buffered or arriving within 100ms
        self.process_pending_requests()?;
        let _ = self.stream.set_read_timeout(Some(Duration::from_millis(100)));
        let mut temp = [0u8; 1024];
        match self.stream.read(&mut temp) {
            Ok(0) => {}
            Ok(n) => {
                self.read_buf.extend_from_slice(&temp[..n]);
                self.process_pending_requests()?;
            }
            Err(_) => {}
        }
        let _ = self.stream.set_read_timeout(Some(Duration::from_secs(5)));

        // -------------------------------------------------------------
        // M6: SETUP (configure RTP transport)
        // -------------------------------------------------------------
        let client_rtp = self.negotiated_rtp_port.unwrap_or(DEFAULT_RTP_PORT);
        let m6 = Self::build_setup_request(&presentation_url, self.cseq, client_rtp);
        let m6_cseq = self.cseq;
        self.cseq += 1;
        self.stream.write_all(m6.as_bytes())?;
        self.stream.flush()?;

        let resp6 = self.read_response_handling_requests(m6_cseq)?;
        if !resp6.is_success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("M6 SETUP rejected: status {:?}", resp6.status_code()),
            ));
        }
        if let Some(session_hdr) = resp6.header("session") {
            let sid = session_hdr.split(';').next().unwrap_or("").trim();
            if !sid.is_empty() {
                self.session_id = Some(sid.to_string());
            }
        }
        if let Some(transport) = resp6.header("transport") {
            if let Some(port) = Self::parse_rtp_port_from_transport(transport) {
                self.negotiated_rtp_port = Some(port);
            }
        }

        // Process any reverse PLAY from Sink before or during M7
        self.process_pending_requests()?;

        // -------------------------------------------------------------
        // M7: PLAY (start streaming)
        // -------------------------------------------------------------
        let sid = self.session_id.as_deref().unwrap_or("12345678");
        let m7 = Self::build_play_request(&presentation_url, self.cseq, sid);
        let m7_cseq = self.cseq;
        self.cseq += 1;
        self.stream.write_all(m7.as_bytes())?;
        self.stream.flush()?;

        let resp7 = self.read_response_handling_requests(m7_cseq)?;
        if !resp7.is_success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("M7 PLAY rejected: status {:?}", resp7.status_code()),
            ));
        }

        let final_port = self.negotiated_rtp_port.unwrap_or(DEFAULT_RTP_PORT);
        Ok(final_port)
    }

    /// Gracefully tears down the session with RTSP TEARDOWN and shuts down the TCP socket.
    pub fn teardown(&mut self) -> Result<(), io::Error> {
        if self.is_teared_down {
            return Ok(());
        }
        self.is_teared_down = true;

        if !self.presentation_url.is_empty() {
            let msg = Self::build_teardown_request(
                &self.presentation_url,
                self.cseq,
                self.session_id.as_deref(),
            );
            self.cseq += 1;
            let _ = self.stream.write_all(msg.as_bytes());
            let _ = self.stream.flush();
        }

        let _ = self.stream.shutdown(Shutdown::Both);
        Ok(())
    }

    /// Target sink IP address.
    pub fn target_ip(&self) -> &str {
        &self.target_ip
    }

    /// Target sink port.
    pub fn target_port(&self) -> u16 {
        self.target_port
    }

    /// Negotiated destination UDP RTP port if known.
    pub fn rtp_port(&self) -> Option<u16> {
        self.negotiated_rtp_port
    }

    /// Established session ID if available.
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// Presentation URL negotiated with the sink.
    pub fn presentation_url(&self) -> &str {
        &self.presentation_url
    }

    // =========================================================================
    // Internal Message Reading & Request Dispatching
    // =========================================================================

    /// Processes any pending incoming requests already present in `read_buf`.
    fn process_pending_requests(&mut self) -> Result<(), io::Error> {
        loop {
            if let Some(pos) = find_subsequence(&self.read_buf, b"\r\n\r\n") {
                let header_bytes = &self.read_buf[..pos];
                let header_str = String::from_utf8_lossy(header_bytes);
                let content_len = find_content_length(&header_str);
                let total_len = pos + 4 + content_len;

                if self.read_buf.len() >= total_len {
                    let msg_bytes = self.read_buf[..total_len].to_vec();
                    let raw_str = String::from_utf8_lossy(&msg_bytes);
                    if let Some(msg @ RtspMessage::Request { .. }) = Self::parse_rtsp_message(&raw_str) {
                        self.read_buf.drain(..total_len);
                        self.handle_incoming_request(msg)?;
                        continue;
                    }
                }
            }
            break;
        }
        Ok(())
    }

    /// Handles an incoming request from the Sink and writes the appropriate RTSP response.
    fn handle_incoming_request(&mut self, msg: RtspMessage) -> Result<(), io::Error> {
        if let RtspMessage::Request {
            ref method,
            ref headers,
            ..
        } = msg
        {
            let req_cseq: u32 = headers
                .get("cseq")
                .and_then(|c| c.parse().ok())
                .unwrap_or(1);

            match method.as_str() {
                "OPTIONS" => {
                    // M2: Sink sent reverse OPTIONS to Source
                    let resp = Self::build_options_response(req_cseq);
                    self.stream.write_all(resp.as_bytes())?;
                    self.stream.flush()?;
                }
                "SETUP" => {
                    // Sink sent reverse SETUP to Source (e.g. Pi Zero sink)
                    if let Some(transport) = headers.get("transport") {
                        if let Some(port) = Self::parse_rtp_port_from_transport(transport) {
                            self.negotiated_rtp_port = Some(port);
                        }
                    }
                    let sid = self
                        .session_id
                        .get_or_insert_with(|| "12345678".to_string())
                        .clone();
                    let rtp_port = self.negotiated_rtp_port.unwrap_or(DEFAULT_RTP_PORT);
                    let transport_hdr = format!(
                        "RTP/AVP/UDP;unicast;client_port={}-{};server_port={}",
                        rtp_port,
                        rtp_port + 1,
                        rtp_port
                    );
                    let resp = Self::build_setup_response(req_cseq, &sid, &transport_hdr);
                    self.stream.write_all(resp.as_bytes())?;
                    self.stream.flush()?;
                }
                "PLAY" => {
                    // Sink sent reverse PLAY to Source (e.g. Pi Zero sink)
                    let sid = self.session_id.as_deref().unwrap_or("12345678");
                    let resp =
                        Self::build_response(200, "OK", req_cseq, &[("Session", sid)], "");
                    self.stream.write_all(resp.as_bytes())?;
                    self.stream.flush()?;
                }
                _ => {
                    // Acknowledge other requests
                    let resp = Self::build_response(200, "OK", req_cseq, &[], "");
                    self.stream.write_all(resp.as_bytes())?;
                    self.stream.flush()?;
                }
            }
        }
        Ok(())
    }

    /// Reads next message from socket, handling and replying to incoming requests from Sink
    /// (e.g. M2 reverse OPTIONS, M6 reverse SETUP, M7 reverse PLAY) until the expected response arrives.
    fn read_response_handling_requests(&mut self, expected_cseq: u32) -> Result<RtspMessage, io::Error> {
        loop {
            let msg = self.read_next_message()?;
            match msg {
                req @ RtspMessage::Request { .. } => {
                    self.handle_incoming_request(req)?;
                }
                RtspMessage::Response { ref headers, .. } => {
                    if let Some(c) = headers.get("cseq").and_then(|c| c.parse::<u32>().ok()) {
                        if c == expected_cseq {
                            return Ok(msg);
                        } else if c < expected_cseq {
                            // Delayed response to earlier request, skip
                            continue;
                        } else {
                            return Ok(msg);
                        }
                    } else {
                        return Ok(msg);
                    }
                }
            }
        }
    }

    /// Reads bytes from stream until a full RTSP message is available in `read_buf`.
    fn read_next_message(&mut self) -> Result<RtspMessage, io::Error> {
        let mut temp = [0u8; 2048];
        loop {
            if let Some(pos) = find_subsequence(&self.read_buf, b"\r\n\r\n") {
                let header_bytes = &self.read_buf[..pos];
                let header_str = String::from_utf8_lossy(header_bytes);
                let content_len = find_content_length(&header_str);
                let total_len = pos + 4 + content_len;

                if self.read_buf.len() >= total_len {
                    let msg_bytes = self.read_buf[..total_len].to_vec();
                    self.read_buf.drain(..total_len);
                    let raw_str = String::from_utf8_lossy(&msg_bytes);
                    if let Some(msg) = Self::parse_rtsp_message(&raw_str) {
                        return Ok(msg);
                    } else {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("Failed to parse RTSP message: {}", raw_str),
                        ));
                    }
                }
            }

            match self.stream.read(&mut temp) {
                Ok(0) => {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "RTSP connection closed unexpectedly by sink",
                    ));
                }
                Ok(n) => {
                    self.read_buf.extend_from_slice(&temp[..n]);
                }
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }
    }

    // =========================================================================
    // Message Builders
    // =========================================================================

    /// Builds an M1 `OPTIONS * RTSP/1.0` request.
    pub fn build_options_request(_sink_ip: &str, _sink_port: u16, cseq: u32) -> String {
        format!(
            "OPTIONS * RTSP/1.0\r\n\
             CSeq: {}\r\n\
             Require: org.wfa.wfd1.0\r\n\
             \r\n",
            cseq
        )
    }

    /// Builds a response to an M2 reverse `OPTIONS` request.
    pub fn build_options_response(cseq: u32) -> String {
        Self::build_response(
            200,
            "OK",
            cseq,
            &[("Public", "org.wfa.wfd1.0, GET_PARAMETER, SET_PARAMETER")],
            "",
        )
    }

    /// Builds an M3 `GET_PARAMETER` request querying specified parameters.
    pub fn build_get_parameter_request(presentation_url: &str, cseq: u32, params: &[&str]) -> String {
        let mut body = params.join("\r\n");
        if !body.is_empty() {
            body.push_str("\r\n");
        }
        format!(
            "GET_PARAMETER {} RTSP/1.0\r\n\
             CSeq: {}\r\n\
             Content-Type: text/parameters\r\n\
             Content-Length: {}\r\n\
             \r\n\
             {}",
            presentation_url,
            cseq,
            body.len(),
            body
        )
    }

    /// Builds a generic `SET_PARAMETER` request with a text body.
    pub fn build_set_parameter_request(presentation_url: &str, cseq: u32, body: &str) -> String {
        format!(
            "SET_PARAMETER {} RTSP/1.0\r\n\
             CSeq: {}\r\n\
             Content-Type: text/parameters\r\n\
             Content-Length: {}\r\n\
             \r\n\
             {}",
            presentation_url,
            cseq,
            body.len(),
            body
        )
    }

    /// Builds an M4 `SET_PARAMETER` selecting video formats and presentation URL.
    pub fn build_set_parameter_video_formats(
        presentation_url: &str,
        cseq: u32,
        video_formats: &str,
        sink_ip: &str,
    ) -> String {
        let body = format!(
            "wfd_video_formats: {}\r\nwfd_presentation_URL: rtsp://{}/wfd1.0/streamid=0 none\r\n",
            video_formats, sink_ip
        );
        Self::build_set_parameter_request(presentation_url, cseq, &body)
    }

    /// Builds an M5 `SET_PARAMETER` with `wfd_trigger_method: SETUP`.
    pub fn build_set_parameter_trigger_setup(presentation_url: &str, cseq: u32) -> String {
        Self::build_set_parameter_request(presentation_url, cseq, "wfd_trigger_method: SETUP\r\n")
    }

    /// Builds an M6 `SETUP` request with Transport header.
    pub fn build_setup_request(presentation_url: &str, cseq: u32, client_port: u16) -> String {
        format!(
            "SETUP {} RTSP/1.0\r\n\
             CSeq: {}\r\n\
             Transport: RTP/AVP/UDP;unicast;client_port={}\r\n\
             \r\n",
            presentation_url, cseq, client_port
        )
    }

    /// Builds a 200 OK response for an incoming reverse `SETUP` request.
    pub fn build_setup_response(cseq: u32, session_id: &str, transport: &str) -> String {
        Self::build_response(
            200,
            "OK",
            cseq,
            &[("Session", session_id), ("Transport", transport)],
            "",
        )
    }

    /// Builds an M7 `PLAY` request.
    pub fn build_play_request(presentation_url: &str, cseq: u32, session_id: &str) -> String {
        format!(
            "PLAY {} RTSP/1.0\r\n\
             CSeq: {}\r\n\
             Session: {}\r\n\
             \r\n",
            presentation_url, cseq, session_id
        )
    }

    /// Builds a `TEARDOWN` request.
    pub fn build_teardown_request(
        presentation_url: &str,
        cseq: u32,
        session_id: Option<&str>,
    ) -> String {
        let session_header = match session_id {
            Some(sid) => format!("Session: {}\r\n", sid),
            None => String::new(),
        };
        format!(
            "TEARDOWN {} RTSP/1.0\r\n\
             CSeq: {}\r\n\
             {}\r\n",
            presentation_url, cseq, session_header
        )
    }

    /// Builds a generic RTSP response string.
    pub fn build_response(
        status_code: u16,
        reason: &str,
        cseq: u32,
        extra_headers: &[(&str, &str)],
        body: &str,
    ) -> String {
        let mut resp = format!("RTSP/1.0 {} {}\r\nCSeq: {}\r\n", status_code, reason, cseq);
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
        resp
    }

    /// Builds an MS-MICE SOURCE_READY packet with binary TLV tag 0x02 (RTSP Port) and text payload.
    pub fn build_mice_source_ready(rtsp_port: u16) -> Vec<u8> {
        let mut pkt = Vec::new();
        // 4-byte header: [0x00, 0x00, 0x01, 0x01] (satisfies req[2] == 0x01 && req[3] == 0x01 in wfd.rs)
        pkt.extend_from_slice(&[0x00, 0x00, 0x01, 0x01]);
        // TLV: Tag 0x02 (RTSP Port), Length 2, port bytes
        pkt.push(0x02);
        pkt.extend_from_slice(&(2u16).to_be_bytes());
        pkt.extend_from_slice(&rtsp_port.to_be_bytes());
        // Append text identifier for compatibility
        pkt.extend_from_slice(b"SOURCE_READY\r\n\r\n");
        pkt
    }

    // =========================================================================
    // Parsing Utilities
    // =========================================================================

    /// Extracts destination UDP RTP port from a Transport header, response string, or GET_PARAMETER body.
    pub fn parse_rtp_port_from_transport(raw: &str) -> Option<u16> {
        // 1. Check for server_port=
        if let Some(pos) = raw.find("server_port=") {
            let remainder = &raw[pos + "server_port=".len()..];
            if let Some(port) = extract_first_port_number(remainder) {
                return Some(port);
            }
        }
        // 2. Check for client_port=
        if let Some(pos) = raw.find("client_port=") {
            let remainder = &raw[pos + "client_port=".len()..];
            if let Some(port) = extract_first_port_number(remainder) {
                return Some(port);
            }
        }
        // 3. Check for space-separated "unicast " ports (wfd_client_rtpports format)
        if let Some(pos) = raw.find("unicast ") {
            let remainder = &raw[pos + "unicast ".len()..];
            if let Some(port) = extract_first_port_number(remainder) {
                return Some(port);
            }
        }
        None
    }

    /// Parses an RTSP message string into an `RtspMessage` enum.
    pub fn parse_rtsp_message(raw: &str) -> Option<RtspMessage> {
        let (header_part, body) = match raw.find("\r\n\r\n") {
            Some(pos) => (&raw[..pos], &raw[pos + 4..]),
            None => (raw, ""),
        };

        let mut lines = header_part.lines();
        let first_line = lines.next()?.trim();
        if first_line.is_empty() {
            return None;
        }

        let mut headers = HashMap::new();
        for line in lines {
            if let Some(idx) = line.find(':') {
                let k = line[..idx].trim().to_lowercase();
                let v = line[idx + 1..].trim().to_string();
                headers.insert(k, v);
            }
        }

        if first_line.starts_with("RTSP/") {
            // Response: "RTSP/1.0 200 OK"
            let parts: Vec<&str> = first_line.splitn(3, ' ').collect();
            if parts.len() < 2 {
                return None;
            }
            let version = parts[0].to_string();
            let status_code: u16 = parts[1].parse().ok()?;
            let reason = if parts.len() >= 3 {
                parts[2].to_string()
            } else {
                String::new()
            };
            Some(RtspMessage::Response {
                version,
                status_code,
                reason,
                headers,
                body: body.to_string(),
            })
        } else {
            // Request: "OPTIONS * RTSP/1.0"
            let parts: Vec<&str> = first_line.split_whitespace().collect();
            if parts.len() < 3 {
                return None;
            }
            let method = parts[0].to_string();
            let uri = parts[1].to_string();
            let version = parts[2].to_string();
            Some(RtspMessage::Request {
                method,
                uri,
                version,
                headers,
                body: body.to_string(),
            })
        }
    }

    /// Parses status code from an RTSP response string.
    pub fn parse_status_code(response: &str) -> Option<u16> {
        let first_line = response.lines().next()?.trim();
        if !first_line.starts_with("RTSP/") {
            return None;
        }
        let parts: Vec<&str> = first_line.split_whitespace().collect();
        parts.get(1)?.parse().ok()
    }

    /// Parses CSeq number from an RTSP message string.
    pub fn parse_cseq(message: &str) -> Option<u32> {
        for line in message.lines() {
            if let Some(idx) = line.find(':') {
                let key = line[..idx].trim();
                if key.eq_ignore_ascii_case("cseq") {
                    return line[idx + 1..].trim().parse().ok();
                }
            }
        }
        None
    }

    /// Parses session ID from an RTSP message string.
    pub fn parse_session_id(message: &str) -> Option<String> {
        for line in message.lines() {
            if let Some(idx) = line.find(':') {
                let key = line[..idx].trim();
                if key.eq_ignore_ascii_case("session") {
                    let val = line[idx + 1..].trim();
                    let sid = val.split(';').next()?.trim();
                    if !sid.is_empty() {
                        return Some(sid.to_string());
                    }
                }
            }
        }
        None
    }
}

impl Drop for WfdClient {
    fn drop(&mut self) {
        let _ = self.teardown();
    }
}

/// Helper extracting digits of the first port number in a string slice.
fn extract_first_port_number(s: &str) -> Option<u16> {
    let digits: String = s
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse::<u16>().ok()
}

/// Helper parsing Content-Length from header string.
fn find_content_length(header_str: &str) -> usize {
    for line in header_str.lines() {
        if let Some(idx) = line.find(':') {
            let key = line[..idx].trim();
            if key.eq_ignore_ascii_case("content-length") {
                if let Ok(len) = line[idx + 1..].trim().parse::<usize>() {
                    return len;
                }
            }
        }
    }
    0
}

/// Helper finding byte subsequence in a haystack.
fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn test_rtsp_options_builder() {
        let msg = WfdClient::build_options_request("192.168.7.2", 7236, 1);
        assert!(msg.starts_with("OPTIONS * RTSP/1.0\r\n"));
        assert!(msg.contains("CSeq: 1\r\n"));
        assert!(msg.contains("Require: org.wfa.wfd1.0\r\n"));
    }

    #[test]
    fn test_parse_transport_server_port() {
        let response = "RTSP/1.0 200 OK\r\nCSeq: 4\r\nTransport: RTP/AVP/UDP;unicast;client_port=5002;server_port=5002\r\n\r\n";
        let port = WfdClient::parse_rtp_port_from_transport(response);
        assert_eq!(port, Some(5002));
    }

    #[test]
    fn test_parse_transport_variations() {
        assert_eq!(
            WfdClient::parse_rtp_port_from_transport(
                "Transport: RTP/AVP/UDP;unicast;client_port=5002-5003"
            ),
            Some(5002)
        );
        assert_eq!(
            WfdClient::parse_rtp_port_from_transport(
                "Transport: RTP/AVP/UDP;unicast;server_port=5004-5005"
            ),
            Some(5004)
        );
        assert_eq!(
            WfdClient::parse_rtp_port_from_transport(
                "wfd_client_rtpports: RTP/AVP/UDP;unicast 5002 5003 mode=play"
            ),
            Some(5002)
        );
        assert_eq!(
            WfdClient::parse_rtp_port_from_transport("Invalid header without ports"),
            None
        );
    }

    #[test]
    fn test_rtsp_get_parameter_builder() {
        let params = ["wfd_video_formats", "wfd_client_rtpports"];
        let msg = WfdClient::build_get_parameter_request("rtsp://192.168.7.2/wfd1.0/streamid=0", 2, &params);
        assert!(msg.starts_with("GET_PARAMETER rtsp://192.168.7.2/wfd1.0/streamid=0 RTSP/1.0\r\n"));
        assert!(msg.contains("CSeq: 2\r\n"));
        assert!(msg.contains("Content-Type: text/parameters\r\n"));
        assert!(msg.contains("wfd_video_formats\r\nwfd_client_rtpports\r\n"));
    }

    #[test]
    fn test_rtsp_set_parameter_builder() {
        let msg = WfdClient::build_set_parameter_video_formats(
            "rtsp://192.168.7.2/wfd1.0/streamid=0",
            3,
            WFD_VIDEO_FORMAT_720P60,
            "192.168.7.2",
        );
        assert!(msg.starts_with("SET_PARAMETER rtsp://192.168.7.2/wfd1.0/streamid=0 RTSP/1.0\r\n"));
        assert!(msg.contains("CSeq: 3\r\n"));
        assert!(msg.contains("wfd_video_formats: 30 00 01 02"));
        assert!(msg.contains("wfd_presentation_URL: rtsp://192.168.7.2/wfd1.0/streamid=0 none"));

        let trigger = WfdClient::build_set_parameter_trigger_setup("rtsp://192.168.7.2/wfd1.0/streamid=0", 4);
        assert!(trigger.contains("wfd_trigger_method: SETUP"));
    }

    #[test]
    fn test_rtsp_setup_and_play_builder() {
        let setup = WfdClient::build_setup_request("rtsp://192.168.7.2/wfd1.0/streamid=0", 5, 5002);
        assert!(setup.starts_with("SETUP rtsp://192.168.7.2/wfd1.0/streamid=0 RTSP/1.0\r\n"));
        assert!(setup.contains("Transport: RTP/AVP/UDP;unicast;client_port=5002\r\n"));

        let play = WfdClient::build_play_request("rtsp://192.168.7.2/wfd1.0/streamid=0", 6, "98765432");
        assert!(play.starts_with("PLAY rtsp://192.168.7.2/wfd1.0/streamid=0 RTSP/1.0\r\n"));
        assert!(play.contains("Session: 98765432\r\n"));
    }

    #[test]
    fn test_rtsp_teardown_builder() {
        let td = WfdClient::build_teardown_request("rtsp://192.168.7.2/wfd1.0/streamid=0", 7, Some("98765432"));
        assert!(td.starts_with("TEARDOWN rtsp://192.168.7.2/wfd1.0/streamid=0 RTSP/1.0\r\n"));
        assert!(td.contains("Session: 98765432\r\n"));

        let td_nosid = WfdClient::build_teardown_request("rtsp://192.168.7.2/wfd1.0/streamid=0", 8, None);
        assert!(!td_nosid.contains("Session:"));
    }

    #[test]
    fn test_rtsp_response_builder() {
        let opt_resp = WfdClient::build_options_response(1);
        assert!(opt_resp.starts_with("RTSP/1.0 200 OK\r\n"));
        assert!(opt_resp.contains("Public: org.wfa.wfd1.0, GET_PARAMETER, SET_PARAMETER\r\n"));

        let setup_resp = WfdClient::build_setup_response(
            2,
            "12345678",
            "RTP/AVP/UDP;unicast;client_port=5002-5003;server_port=5002",
        );
        assert!(setup_resp.contains("Session: 12345678\r\n"));
        assert!(setup_resp.contains("Transport: RTP/AVP/UDP;unicast;client_port=5002-5003;server_port=5002\r\n"));
    }

    #[test]
    fn test_ms_mice_source_ready_packet() {
        let pkt = WfdClient::build_mice_source_ready(7236);
        assert!(pkt.len() >= 9);
        assert_eq!(pkt[0..4], [0x00, 0x00, 0x01, 0x01]);
        assert_eq!(pkt[4], 0x02); // TLV tag 0x02 (RTSP port)
        assert_eq!(&pkt[5..7], &[0x00, 0x02]); // length = 2
        assert_eq!(u16::from_be_bytes([pkt[7], pkt[8]]), 7236);
        let s = String::from_utf8_lossy(&pkt);
        assert!(s.contains("SOURCE_READY"));
    }

    #[test]
    fn test_parse_status_code_cseq_and_session() {
        let resp = "RTSP/1.0 200 OK\r\nCSeq: 10\r\nSession: test_sess_123;timeout=30\r\n\r\n";
        assert_eq!(WfdClient::parse_status_code(resp), Some(200));
        assert_eq!(WfdClient::parse_cseq(resp), Some(10));
        assert_eq!(
            WfdClient::parse_session_id(resp),
            Some("test_sess_123".to_string())
        );

        let err_resp = "RTSP/1.0 404 Not Found\r\nCSeq: 11\r\n\r\n";
        assert_eq!(WfdClient::parse_status_code(err_resp), Some(404));
        assert_eq!(WfdClient::parse_cseq(err_resp), Some(11));
        assert_eq!(WfdClient::parse_session_id(err_resp), None);
    }

    #[test]
    fn test_mock_wfd_handshake_standard_sink() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind test listener");
        let port = listener.local_addr().unwrap().port();

        let server_thread = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("Accept failed");
            let mut buf = [0u8; 4096];

            // 1. Read M1 OPTIONS
            let n = stream.read(&mut buf).unwrap();
            let req1 = String::from_utf8_lossy(&buf[..n]);
            assert!(req1.starts_with("OPTIONS"));
            // Reply 200 OK
            let resp1 = "RTSP/1.0 200 OK\r\nCSeq: 1\r\nPublic: org.wfa.wfd1.0, GET_PARAMETER, SET_PARAMETER\r\n\r\n";
            stream.write_all(resp1.as_bytes()).unwrap();

            // 2. Read M3 GET_PARAMETER
            let n = stream.read(&mut buf).unwrap();
            let req3 = String::from_utf8_lossy(&buf[..n]);
            assert!(req3.starts_with("GET_PARAMETER"));
            let body3 = "wfd_video_formats: 30 00 01 02 00000069 00000000 00000000 00 0000 0000 00 none none\r\nwfd_client_rtpports: RTP/AVP/UDP;unicast 5002 5003 mode=play\r\n";
            let resp3 = format!(
                "RTSP/1.0 200 OK\r\nCSeq: 2\r\nContent-Type: text/parameters\r\nContent-Length: {}\r\n\r\n{}",
                body3.len(),
                body3
            );
            stream.write_all(resp3.as_bytes()).unwrap();

            // 3. Read M4 SET_PARAMETER
            let n = stream.read(&mut buf).unwrap();
            let req4 = String::from_utf8_lossy(&buf[..n]);
            assert!(req4.starts_with("SET_PARAMETER"));
            let resp4 = "RTSP/1.0 200 OK\r\nCSeq: 3\r\n\r\n";
            stream.write_all(resp4.as_bytes()).unwrap();

            // 4. Read M5 SET_PARAMETER (trigger SETUP)
            let n = stream.read(&mut buf).unwrap();
            let req5 = String::from_utf8_lossy(&buf[..n]);
            assert!(req5.starts_with("SET_PARAMETER"));
            let resp5 = "RTSP/1.0 200 OK\r\nCSeq: 4\r\n\r\n";
            stream.write_all(resp5.as_bytes()).unwrap();

            // 5. Read M6 SETUP
            let n = stream.read(&mut buf).unwrap();
            let req6 = String::from_utf8_lossy(&buf[..n]);
            assert!(req6.starts_with("SETUP"));
            let resp6 = "RTSP/1.0 200 OK\r\nCSeq: 5\r\nSession: 88776655;timeout=30\r\nTransport: RTP/AVP/UDP;unicast;client_port=5002;server_port=5002\r\n\r\n";
            stream.write_all(resp6.as_bytes()).unwrap();

            // 6. Read M7 PLAY
            let n = stream.read(&mut buf).unwrap();
            let req7 = String::from_utf8_lossy(&buf[..n]);
            assert!(req7.starts_with("PLAY"));
            let resp7 = "RTSP/1.0 200 OK\r\nCSeq: 6\r\nSession: 88776655\r\n\r\n";
            stream.write_all(resp7.as_bytes()).unwrap();

            // 7. Read TEARDOWN
            let n = stream.read(&mut buf).unwrap();
            let req_td = String::from_utf8_lossy(&buf[..n]);
            assert!(req_td.starts_with("TEARDOWN"));
            let resp_td = "RTSP/1.0 200 OK\r\nCSeq: 7\r\n\r\n";
            stream.write_all(resp_td.as_bytes()).unwrap();
        });

        let mut client = WfdClient::connect("127.0.0.1", port).expect("Client connect failed");
        let rtp_port = client.negotiate_session().expect("Negotiation failed");
        assert_eq!(rtp_port, 5002);
        assert_eq!(client.session_id(), Some("88776655"));
        client.teardown().expect("Teardown failed");

        server_thread.join().expect("Server thread panicked");
    }

    #[test]
    fn test_mock_wfd_handshake_pizero_style_sink() {
        // Simulates Pi Zero receiver/src/wfd.rs:
        // Sink responds to M1, sends reverse M2 OPTIONS, responds to M3/M4/M5,
        // and upon M5 triggers reverse M6 SETUP and M7 PLAY!
        let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind test listener");
        let port = listener.local_addr().unwrap().port();

        let server_thread = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("Accept failed");
            let mut buf = [0u8; 4096];

            // 1. Read M1 OPTIONS
            let n = stream.read(&mut buf).unwrap();
            let req1 = String::from_utf8_lossy(&buf[..n]);
            assert!(req1.starts_with("OPTIONS"));

            // Sink sends M1 200 OK AND immediately sends M2 reverse OPTIONS!
            let m1_resp_and_m2_req =
                "RTSP/1.0 200 OK\r\nCSeq: 1\r\nPublic: org.wfa.wfd1.0\r\n\r\n\
                 OPTIONS * RTSP/1.0\r\nCSeq: 1\r\nRequire: org.wfa.wfd1.0\r\n\r\n";
            stream.write_all(m1_resp_and_m2_req.as_bytes()).unwrap();

            // Sink reads M2 reverse OPTIONS response from Source
            let n = stream.read(&mut buf).unwrap();
            let m2_resp = String::from_utf8_lossy(&buf[..n]);
            assert!(m2_resp.contains("RTSP/1.0 200 OK"));
            assert!(m2_resp.contains("CSeq: 1"));

            // 2. Read M3 GET_PARAMETER
            let n = stream.read(&mut buf).unwrap();
            let req3 = String::from_utf8_lossy(&buf[..n]);
            assert!(req3.starts_with("GET_PARAMETER"));
            let body3 = "wfd_client_rtpports: RTP/AVP/UDP;unicast 5002 5003 mode=play\r\n";
            let resp3 = format!(
                "RTSP/1.0 200 OK\r\nCSeq: 2\r\nContent-Type: text/parameters\r\nContent-Length: {}\r\n\r\n{}",
                body3.len(),
                body3
            );
            stream.write_all(resp3.as_bytes()).unwrap();

            // 3. Read M4 SET_PARAMETER
            let n = stream.read(&mut buf).unwrap();
            let req4 = String::from_utf8_lossy(&buf[..n]);
            assert!(req4.starts_with("SET_PARAMETER"));
            let resp4 = "RTSP/1.0 200 OK\r\nCSeq: 3\r\n\r\n";
            stream.write_all(resp4.as_bytes()).unwrap();

            // 4. Read M5 SET_PARAMETER (trigger SETUP)
            let n = stream.read(&mut buf).unwrap();
            let req5 = String::from_utf8_lossy(&buf[..n]);
            assert!(req5.starts_with("SET_PARAMETER"));

            // Sink replies 200 OK to M5, AND sends reverse M6 SETUP to Source!
            let resp5_and_m6_req =
                "RTSP/1.0 200 OK\r\nCSeq: 4\r\n\r\n\
                 SETUP rtsp://127.0.0.1/wfd1.0/streamid=0 RTSP/1.0\r\nCSeq: 2\r\nTransport: RTP/AVP/UDP;unicast;client_port=5002-5003\r\n\r\n";
            stream.write_all(resp5_and_m6_req.as_bytes()).unwrap();

            // Sink reads 200 OK from Source for reverse SETUP
            let n = stream.read(&mut buf).unwrap();
            let m6_sink_resp = String::from_utf8_lossy(&buf[..n]);
            assert!(m6_sink_resp.contains("RTSP/1.0 200 OK"));
            assert!(m6_sink_resp.contains("Session:"));

            // Read Source's M6 SETUP request and reply 200 OK
            let n = stream.read(&mut buf).unwrap();
            let m6_src_req = String::from_utf8_lossy(&buf[..n]);
            assert!(m6_src_req.starts_with("SETUP"));
            let resp6 = "RTSP/1.0 200 OK\r\nCSeq: 5\r\nSession: 12345678\r\nTransport: RTP/AVP/UDP;unicast;client_port=5002;server_port=5002\r\n\r\n";
            stream.write_all(resp6.as_bytes()).unwrap();

            // Read Source's M7 PLAY request and reply 200 OK
            let n = stream.read(&mut buf).unwrap();
            let m7_src_req = String::from_utf8_lossy(&buf[..n]);
            assert!(m7_src_req.starts_with("PLAY"));
            let resp7 = "RTSP/1.0 200 OK\r\nCSeq: 6\r\nSession: 12345678\r\n\r\n";
            stream.write_all(resp7.as_bytes()).unwrap();
        });

        let mut client = WfdClient::connect("127.0.0.1", port).expect("Client connect failed");
        let rtp_port = client.negotiate_session().expect("Negotiation failed");
        assert_eq!(rtp_port, 5002);

        server_thread.join().expect("Server thread panicked");
    }

    #[test]
    fn test_mock_ms_mice_handshake() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind test listener");
        let port = listener.local_addr().unwrap().port();

        let server_thread = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("Accept failed");
            let mut buf = [0u8; 512];
            let n = stream.read(&mut buf).unwrap();
            assert!(n >= 9);
            // Verify SOURCE_READY
            let is_ready = (buf[2] == 0x01 && buf[3] == 0x01)
                || String::from_utf8_lossy(&buf[..n]).contains("SOURCE_READY");
            assert!(is_ready);

            // Send MS-MICE ACK
            let ack = [0x00, 0x04, 0x01, 0x02];
            stream.write_all(&ack).unwrap();
            stream.write_all(b"OK\r\n\r\n").unwrap();
            stream.flush().unwrap();
        });

        let stream = TcpStream::connect(format!("127.0.0.1:{}", port)).expect("Connect failed");
        let mut client = WfdClient::from_stream(stream, "127.0.0.1", port);
        client
            .perform_mice_handshake()
            .expect("MICE handshake failed");

        server_thread.join().expect("Server thread panicked");
    }

    #[test]
    fn test_teardown_idempotent() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        let _server = thread::spawn(move || {
            let _ = listener.accept();
        });

        let mut client = WfdClient::connect("127.0.0.1", port).unwrap();
        assert!(client.teardown().is_ok());
        assert!(client.teardown().is_ok());
    }
}
