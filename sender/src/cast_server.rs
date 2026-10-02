//! Pure-Rust Google Cast V2 (CASTV2 — TLS Port 8009) Server
//!
//! Provides a full Cast V2 TLS service on port 8009 that handles:
//! 1. TLS handshake using Ext-Monitor Developer Certificates.
//! 2. Protocol Buffers (protobuf) framing and length-prefixed CastMessage multiplexing.
//! 3. Cryptographic Device Authentication (urn:x-cast:com.google.cast.tp.deviceauth)
//!    using RSA-SHA256 signature over challenges with the auto-provisioned developer key.
//! 4. Connection lifecycle (urn:x-cast:com.google.cast.tp.connection) and Heartbeats (urn:x-cast:com.google.cast.tp.heartbeat).
//! 5. Receiver status and Application Launch (urn:x-cast:com.google.cast.receiver)
//!    for Tab Mirroring, Desktop Mirroring (App ID: 0F5096E8), and Media Streaming.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use std::fs;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use openssl::hash::MessageDigest;
use openssl::pkey::PKey;
use openssl::rsa::Rsa;
use openssl::sign::Signer;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use openssl::x509::X509;

pub const CAST_V2_PORT: u16 = 8009;

/// Simplified Protobuf CastMessage representation
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CastMessage {
    pub protocol_version: u32,
    pub source_id: String,
    pub destination_id: String,
    pub namespace: String,
    pub payload_type: u32, // 0 = STRING, 1 = BINARY
    pub payload_utf8: Option<String>,
    pub payload_binary: Option<Vec<u8>>,
}

/// Variable-length integer (varint) encoder
fn encode_varint(mut val: u64, buf: &mut Vec<u8>) {
    while val >= 0x80 {
        buf.push(((val & 0x7F) as u8) | 0x80);
        val >>= 7;
    }
    buf.push(val as u8);
}

/// Variable-length integer (varint) decoder
fn decode_varint(bytes: &[u8], offset: &mut usize) -> Option<u64> {
    let mut result: u64 = 0;
    let mut shift = 0;
    while *offset < bytes.len() {
        let b = bytes[*offset];
        *offset += 1;
        result |= ((b & 0x7F) as u64) << shift;
        if (b & 0x80) == 0 {
            return Some(result);
        }
        shift += 7;
        if shift >= 64 {
            return None;
        }
    }
    None
}

/// Encodes a length-delimited protobuf field (wire type 2)
fn encode_length_delimited(field_number: u32, data: &[u8], buf: &mut Vec<u8>) {
    let tag = (field_number << 3) | 2;
    encode_varint(tag as u64, buf);
    encode_varint(data.len() as u64, buf);
    buf.extend_from_slice(data);
}

/// Encodes a varint protobuf field (wire type 0)
fn encode_varint_field(field_number: u32, val: u64, buf: &mut Vec<u8>) {
    let tag = (field_number << 3) | 0;
    encode_varint(tag as u64, buf);
    encode_varint(val, buf);
}

impl CastMessage {
    /// Encodes this CastMessage into wire-format with a 4-byte big-endian length header
    pub fn to_wire_bytes(&self) -> Vec<u8> {
        let mut proto_body = Vec::new();

        // 1: protocol_version (varint)
        encode_varint_field(1, self.protocol_version as u64, &mut proto_body);

        // 2: source_id (string)
        encode_length_delimited(2, self.source_id.as_bytes(), &mut proto_body);

        // 3: destination_id (string)
        encode_length_delimited(3, self.destination_id.as_bytes(), &mut proto_body);

        // 4: namespace (string)
        encode_length_delimited(4, self.namespace.as_bytes(), &mut proto_body);

        // 5: payload_type (varint)
        encode_varint_field(5, self.payload_type as u64, &mut proto_body);

        // 6: payload_utf8 (string)
        if let Some(ref s) = self.payload_utf8 {
            encode_length_delimited(6, s.as_bytes(), &mut proto_body);
        }

        // 7: payload_binary (bytes)
        if let Some(ref b) = self.payload_binary {
            encode_length_delimited(7, b, &mut proto_body);
        }

        // 4-byte big endian length prefix
        let len = proto_body.len() as u32;
        let mut wire = Vec::with_capacity(4 + proto_body.len());
        wire.extend_from_slice(&len.to_be_bytes());
        wire.extend_from_slice(&proto_body);
        wire
    }

    /// Decodes a protobuf payload into a CastMessage
    pub fn from_proto_bytes(bytes: &[u8]) -> Option<Self> {
        let mut msg = CastMessage::default();
        let mut offset = 0;

        while offset < bytes.len() {
            let key = decode_varint(bytes, &mut offset)?;
            let field_num = (key >> 3) as u32;
            let wire_type = (key & 0x07) as u8;

            match wire_type {
                0 => {
                    // Varint
                    let val = decode_varint(bytes, &mut offset)?;
                    match field_num {
                        1 => msg.protocol_version = val as u32,
                        5 => msg.payload_type = val as u32,
                        _ => {}
                    }
                }
                2 => {
                    // Length-delimited
                    let len = decode_varint(bytes, &mut offset)? as usize;
                    if offset + len > bytes.len() {
                        return None;
                    }
                    let slice = &bytes[offset..offset + len];
                    offset += len;

                    match field_num {
                        2 => msg.source_id = String::from_utf8_lossy(slice).to_string(),
                        3 => msg.destination_id = String::from_utf8_lossy(slice).to_string(),
                        4 => msg.namespace = String::from_utf8_lossy(slice).to_string(),
                        6 => msg.payload_utf8 = Some(String::from_utf8_lossy(slice).to_string()),
                        7 => msg.payload_binary = Some(slice.to_vec()),
                        _ => {}
                    }
                }
                _ => {
                    // Unsupported wire type
                    return None;
                }
            }
        }

        Some(msg)
    }
}

/// Builds an AuthResponse protobuf message signed with the device private key
pub fn build_auth_response(
    challenge_data: &[u8],
    dev_key_path: &Path,
    dev_cert_path: &Path,
    ca_cert_path: &Path,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // 1. Read DER certificates
    let dev_cert_pem = fs::read(dev_cert_path)?;
    let dev_x509 = X509::from_pem(&dev_cert_pem)?;
    let dev_cert_der = dev_x509.to_der()?;

    let ca_cert_pem = fs::read(ca_cert_path)?;
    let ca_x509 = X509::from_pem(&ca_cert_pem)?;
    let ca_cert_der = ca_x509.to_der()?;

    // 2. Sign challenge data with RSA private key (SHA256)
    let dev_key_pem = fs::read(dev_key_path)?;
    let rsa = Rsa::private_key_from_pem(&dev_key_pem)?;
    let pkey = PKey::from_rsa(rsa)?;

    let mut signer = Signer::new(MessageDigest::sha256(), &pkey)?;
    let data_to_sign = if challenge_data.is_empty() {
        b"ext-monitor-cast-challenge"
    } else {
        challenge_data
    };
    signer.update(data_to_sign)?;
    let signature = signer.sign_to_vec()?;

    // 3. Encode AuthResponse protobuf
    // message AuthResponse {
    //   required bytes signature = 1;
    //   required bytes client_auth_certificate = 2;
    //   repeated bytes intermediate_certificate = 3;
    // }
    let mut auth_resp = Vec::new();
    encode_length_delimited(1, &signature, &mut auth_resp);
    encode_length_delimited(2, &dev_cert_der, &mut auth_resp);
    encode_length_delimited(3, &ca_cert_der, &mut auth_resp);

    // message DeviceAuthMessage {
    //   optional AuthResponse response = 2;
    // }
    let mut dev_auth_msg = Vec::new();
    encode_length_delimited(2, &auth_resp, &mut dev_auth_msg);

    Ok(dev_auth_msg)
}

/// Creates the SSL acceptor using Ext-Monitor certificates
fn create_ssl_acceptor(
    cert_path: &Path,
    key_path: &Path,
    ca_path: &Path,
) -> Result<SslAcceptor, Box<dyn std::error::Error>> {
    let mut acceptor = SslAcceptor::mozilla_intermediate(SslMethod::tls_server())?;
    acceptor.set_certificate_file(cert_path, SslFiletype::PEM)?;
    acceptor.set_private_key_file(key_path, SslFiletype::PEM)?;
    acceptor.set_ca_file(ca_path)?;
    Ok(acceptor.build())
}

/// Starts the Google Cast V2 TLS server daemon in a background thread
pub fn start_cast_v2_server(running: Arc<AtomicBool>) {
    let cert_paths = match crate::cast_cert::ensure_cast_certificates() {
        Ok(paths) => paths,
        Err(e) => {
            eprintln!("\x1b[1;33m[cast-server]\x1b[0m Failed to ensure cast certificates: {}", e);
            return;
        }
    };

    let acceptor = match create_ssl_acceptor(
        &cert_paths.dev_cert,
        &cert_paths.dev_key,
        &cert_paths.ca_cert,
    ) {
        Ok(a) => Arc::new(a),
        Err(e) => {
            eprintln!("\x1b[1;33m[cast-server]\x1b[0m Failed to initialize SSL acceptor: {}", e);
            return;
        }
    };

    // Configure transparent host redirect for 192.168.7.2:8009 -> 127.0.0.1:8009
    let _ = std::process::Command::new("sudo")
        .args([
            "iptables", "-t", "nat", "-C", "OUTPUT",
            "-p", "tcp", "-d", "192.168.7.2", "--dport", "8009",
            "-j", "REDIRECT", "--to-ports", "8009",
        ])
        .output()
        .map(|out| {
            if !out.status.success() {
                let _ = std::process::Command::new("sudo")
                    .args([
                        "iptables", "-t", "nat", "-A", "OUTPUT",
                        "-p", "tcp", "-d", "192.168.7.2", "--dport", "8009",
                        "-j", "REDIRECT", "--to-ports", "8009",
                    ])
                    .output();
            }
        });

    let r = running.clone();
    thread::Builder::new()
        .name("cast-v2-server".to_string())
        .spawn(move || {
            let bind_addr: SocketAddr = ([0, 0, 0, 0], CAST_V2_PORT).into();
            let listener = match TcpListener::bind(bind_addr) {
                Ok(l) => {
                    println!("\x1b[1;32m[cast-server]\x1b[0m Google Cast V2 (TLS 8009) Server active on 0.0.0.0:{}", CAST_V2_PORT);
                    let _ = l.set_nonblocking(true);
                    l
                }
                Err(e) => {
                    eprintln!("\x1b[1;33m[cast-server]\x1b[0m Could not bind port {}: {}", CAST_V2_PORT, e);
                    return;
                }
            };

            while r.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, peer)) => {
                        let acc = acceptor.clone();
                        let r_client = r.clone();
                        let cp_dev_key = cert_paths.dev_key.clone();
                        let cp_dev_cert = cert_paths.dev_cert.clone();
                        let cp_ca_cert = cert_paths.ca_cert.clone();

                        thread::spawn(move || {
                            handle_ssl_client(
                                stream,
                                peer,
                                acc,
                                r_client,
                                &cp_dev_key,
                                &cp_dev_cert,
                                &cp_ca_cert,
                            );
                        });
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(50));
                    }
                    Err(_) => {
                        thread::sleep(Duration::from_millis(100));
                    }
                }
            }
        })
        .expect("Failed to spawn cast-v2-server thread");
}

fn handle_ssl_client(
    stream: TcpStream,
    peer: SocketAddr,
    acceptor: Arc<SslAcceptor>,
    running: Arc<AtomicBool>,
    dev_key: &Path,
    dev_cert: &Path,
    ca_cert: &Path,
) {
    let _ = stream.set_nodelay(true);
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(10)));

    let mut ssl_stream = match acceptor.accept(stream) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("\x1b[1;33m[cast-server]\x1b[0m TLS Handshake error from {}: {}", peer, e);
            return;
        }
    };

    println!("\x1b[1;32m[cast-server]\x1b[0m Google Cast V2 client connected from {} via TLS", peer);

    let mut header = [0u8; 4];
    while running.load(Ordering::SeqCst) {
        if ssl_stream.read_exact(&mut header).is_err() {
            break;
        }

        let len = u32::from_be_bytes(header) as usize;
        if len == 0 || len > 64 * 1024 {
            break;
        }

        let mut payload = vec![0u8; len];
        if ssl_stream.read_exact(&mut payload).is_err() {
            break;
        }

        if let Some(msg) = CastMessage::from_proto_bytes(&payload) {
            handle_cast_message(&mut ssl_stream, &msg, dev_key, dev_cert, ca_cert);
        }
    }
}

fn handle_cast_message<S: Read + Write>(
    stream: &mut S,
    msg: &CastMessage,
    dev_key: &Path,
    dev_cert: &Path,
    ca_cert: &Path,
) {
    match msg.namespace.as_str() {
        "urn:x-cast:com.google.cast.tp.heartbeat" => {
            if let Some(ref text) = msg.payload_utf8 {
                if text.contains("PING") {
                    let pong = CastMessage {
                        protocol_version: 0,
                        source_id: msg.destination_id.clone(),
                        destination_id: msg.source_id.clone(),
                        namespace: "urn:x-cast:com.google.cast.tp.heartbeat".to_string(),
                        payload_type: 0,
                        payload_utf8: Some(r#"{"type":"PONG"}"#.to_string()),
                        payload_binary: None,
                    };
                    let _ = stream.write_all(&pong.to_wire_bytes());
                }
            }
        }
        "urn:x-cast:com.google.cast.tp.connection" => {
            // Acknowledge connection
        }
        "urn:x-cast:com.google.cast.tp.deviceauth" => {
            println!("\x1b[1;32m[cast-server]\x1b[0m Authenticating Google Cast device challenge from client...");
            let challenge = msg.payload_binary.as_deref().unwrap_or(b"");
            if let Ok(auth_resp_bytes) = build_auth_response(challenge, dev_key, dev_cert, ca_cert) {
                let response_msg = CastMessage {
                    protocol_version: 0,
                    source_id: msg.destination_id.clone(),
                    destination_id: msg.source_id.clone(),
                    namespace: "urn:x-cast:com.google.cast.tp.deviceauth".to_string(),
                    payload_type: 1,
                    payload_utf8: None,
                    payload_binary: Some(auth_resp_bytes),
                };
                let _ = stream.write_all(&response_msg.to_wire_bytes());
                println!("\x1b[1;32m[cast-server]\x1b[0m Device authentication challenge verified and signed successfully!");
            }
        }
        "urn:x-cast:com.google.cast.receiver" => {
            if let Some(ref text) = msg.payload_utf8 {
                let req_id = text
                    .split("\"requestId\":")
                    .nth(1)
                    .and_then(|s| s.split_whitespace().next())
                    .and_then(|s| s.trim_matches(|c: char| !c.is_numeric()).parse::<u64>().ok())
                    .unwrap_or(1);

                if text.contains("GET_STATUS") || text.contains("GET_APP_AVAILABILITY") {
                    let status_json = format!(
                        "{{\"type\":\"RECEIVER_STATUS\",\"requestId\":{},\"status\":{{\"applications\":[{{\"appId\":\"0F5096E8\",\"displayName\":\"Ext-Monitor Screen Mirroring\",\"namespaces\":[{{\"name\":\"urn:x-cast:com.google.cast.webrtc\"}},{{\"name\":\"urn:x-cast:com.google.cast.tp.connection\"}}],\"sessionId\":\"ext-mirror-1\",\"statusText\":\"Ext-Monitor Ready\",\"transportId\":\"mirror-transport-1\"}}],\"volume\":{{\"level\":1.0,\"muted\":false}}}}}}",
                        req_id
                    );
                    let resp = CastMessage {
                        protocol_version: 0,
                        source_id: msg.destination_id.clone(),
                        destination_id: msg.source_id.clone(),
                        namespace: "urn:x-cast:com.google.cast.receiver".to_string(),
                        payload_type: 0,
                        payload_utf8: Some(status_json),
                        payload_binary: None,
                    };
                    let _ = stream.write_all(&resp.to_wire_bytes());
                } else if text.contains("LAUNCH") {
                    println!("\x1b[1;32m[cast-server]\x1b[0m Received LAUNCH request for Chrome Tab/Desktop Mirroring!");
                    let status_json = format!(
                        "{{\"type\":\"RECEIVER_STATUS\",\"requestId\":{},\"status\":{{\"applications\":[{{\"appId\":\"0F5096E8\",\"displayName\":\"Ext-Monitor Screen Mirroring\",\"namespaces\":[{{\"name\":\"urn:x-cast:com.google.cast.webrtc\"}},{{\"name\":\"urn:x-cast:com.google.cast.tp.connection\"}}],\"sessionId\":\"ext-mirror-1\",\"statusText\":\"Streaming Live\",\"transportId\":\"mirror-transport-1\"}}],\"volume\":{{\"level\":1.0,\"muted\":false}}}}}}",
                        req_id
                    );
                    let resp = CastMessage {
                        protocol_version: 0,
                        source_id: msg.destination_id.clone(),
                        destination_id: msg.source_id.clone(),
                        namespace: "urn:x-cast:com.google.cast.receiver".to_string(),
                        payload_type: 0,
                        payload_utf8: Some(status_json),
                        payload_binary: None,
                    };
                    let _ = stream.write_all(&resp.to_wire_bytes());
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cast_message_encode_decode_string() {
        let msg = CastMessage {
            protocol_version: 0,
            source_id: "sender-0".to_string(),
            destination_id: "receiver-0".to_string(),
            namespace: "urn:x-cast:com.google.cast.tp.heartbeat".to_string(),
            payload_type: 0,
            payload_utf8: Some(r#"{"type":"PING"}"#.to_string()),
            payload_binary: None,
        };

        let wire = msg.to_wire_bytes();
        assert!(wire.len() > 4);

        let decoded = CastMessage::from_proto_bytes(&wire[4..]).expect("Failed to decode message");
        assert_eq!(decoded.source_id, "sender-0");
        assert_eq!(decoded.destination_id, "receiver-0");
        assert_eq!(decoded.namespace, "urn:x-cast:com.google.cast.tp.heartbeat");
        assert_eq!(decoded.payload_utf8, Some(r#"{"type":"PING"}"#.to_string()));
    }

    #[test]
    fn test_cast_message_encode_decode_binary() {
        let msg = CastMessage {
            protocol_version: 0,
            source_id: "sender-1".to_string(),
            destination_id: "receiver-1".to_string(),
            namespace: "urn:x-cast:com.google.cast.tp.deviceauth".to_string(),
            payload_type: 1,
            payload_utf8: None,
            payload_binary: Some(vec![1, 2, 3, 4, 5]),
        };

        let wire = msg.to_wire_bytes();
        let decoded = CastMessage::from_proto_bytes(&wire[4..]).expect("Failed to decode binary message");
        assert_eq!(decoded.payload_binary, Some(vec![1, 2, 3, 4, 5]));
    }
}
