//! Pure-Rust Web Screen & Tab Cast Subsystem (Option C)
//!
//! Provides zero-dependency, universal in-browser tab, window, and full-screen
//! mirroring to the Raspberry Pi Zero HDMI display via standard HTML5 Screen
//! Capture API (getDisplayMedia) and WebSockets over port 8080.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use crate::pipeline::{PipelineKind, PipelineManager};
use std::io::{Read, Write};
use std::net::{TcpStream, UdpSocket};
use std::sync::Arc;
use std::time::Duration;

/// SHA-1 Implementation in Pure Rust (RFC 3174) for WebSocket Handshake
pub fn sha1(input: &[u8]) -> [u8; 20] {
    let mut h0: u32 = 0x67452301;
    let mut h1: u32 = 0xEFCDAB89;
    let mut h2: u32 = 0x98BADCFE;
    let mut h3: u32 = 0x10325476;
    let mut h4: u32 = 0xC3D2E1F0;

    let ml = (input.len() as u64) * 8;
    let mut msg = input.to_vec();
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&ml.to_be_bytes());

    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 80];
        for i in 0..16 {
            w[i] = u32::from_be_bytes(chunk[i * 4..(i + 1) * 4].try_into().unwrap());
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }

        let mut a = h0;
        let mut b = h1;
        let mut c = h2;
        let mut d = h3;
        let mut e = h4;

        for i in 0..80 {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5A827999),
                20..=39 => (b ^ c ^ d, 0x6ED9EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDC),
                _ => (b ^ c ^ d, 0xCA62C1D6),
            };
            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(w[i]);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }

        h0 = h0.wrapping_add(a);
        h1 = h1.wrapping_add(b);
        h2 = h2.wrapping_add(c);
        h3 = h3.wrapping_add(d);
        h4 = h4.wrapping_add(e);
    }

    let mut out = [0u8; 20];
    out[0..4].copy_from_slice(&h0.to_be_bytes());
    out[4..8].copy_from_slice(&h1.to_be_bytes());
    out[8..12].copy_from_slice(&h2.to_be_bytes());
    out[12..16].copy_from_slice(&h3.to_be_bytes());
    out[16..20].copy_from_slice(&h4.to_be_bytes());
    out
}

/// Standard Base64 Encoder in Pure Rust
pub fn base64_encode(input: &[u8]) -> String {
    const CHARSET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((input.len() + 2) / 3 * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0];
        let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };

        result.push(CHARSET[(b0 >> 2) as usize] as char);
        result.push(CHARSET[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARSET[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARSET[(b2 & 0x3f) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

/// Generates RFC 6455 Sec-WebSocket-Accept token from Sec-WebSocket-Key
pub fn generate_websocket_accept(key: &str) -> String {
    let mut combined = key.trim().to_string();
    combined.push_str("258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
    let digest = sha1(combined.as_bytes());
    base64_encode(&digest)
}

/// Handles incoming WebSocket connection for `/api/stream/ws`
pub fn handle_websocket_stream(
    mut stream: TcpStream,
    req_str: &str,
    pipe: Arc<PipelineManager>,
    default_udp_port: u16,
) {
    // 1. Extract Sec-WebSocket-Key
    let key = req_str
        .lines()
        .find(|line| line.to_ascii_lowercase().starts_with("sec-websocket-key:"))
        .and_then(|line| line.split(':').nth(1))
        .map(|k| k.trim())
        .unwrap_or("");

    if key.is_empty() {
        let _ = stream.write_all(b"HTTP/1.1 400 Bad Request\r\n\r\nMissing Sec-WebSocket-Key");
        return;
    }

    let accept_token = generate_websocket_accept(key);
    let handshake_resp = format!(
        "HTTP/1.1 101 Switching Protocols\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Accept: {}\r\n\r\n",
        accept_token
    );

    if stream.write_all(handshake_resp.as_bytes()).is_err() {
        return;
    }
    let _ = stream.flush();

    println!("\x1b[1;32m[web-cast]\x1b[0m WebSocket client connected! Activating HDMI hardware decoder on port {}...", default_udp_port);

    // 2. Activate hardware decode pipeline
    let default_kind = PipelineKind::RawH264Rtp {
        port: default_udp_port,
    };
    let _ = pipe.resume(default_kind);

    // 3. Bind UDP socket to forward RTP packets to local decoder
    let udp_sock = match UdpSocket::bind("127.0.0.1:0") {
        Ok(s) => s,
        Err(e) => {
            eprintln!("\x1b[1;31m[web-cast]\x1b[0m Failed to bind loopback UDP: {}", e);
            return;
        }
    };
    let target_addr = format!("127.0.0.1:{}", default_udp_port);

    // Increase read timeout for streaming
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));

    let mut seq: u16 = 0;
    let mut ts: u32 = 0;
    let ssrc: u32 = 0x4558544D; // "EXTM"

    loop {
        // Read 2-byte WebSocket header
        let mut hdr = [0u8; 2];
        if stream.read_exact(&mut hdr).is_err() {
            break;
        }

        let opcode = hdr[0] & 0x0F;
        let is_masked = (hdr[1] & 0x80) != 0;
        let mut payload_len = (hdr[1] & 0x7F) as u64;

        if opcode == 0x08 {
            // Close frame
            println!("\x1b[1;33m[web-cast]\x1b[0m Client requested WebSocket close.");
            break;
        }

        if payload_len == 126 {
            let mut ext = [0u8; 2];
            if stream.read_exact(&mut ext).is_err() {
                break;
            }
            payload_len = u16::from_be_bytes(ext) as u64;
        } else if payload_len == 127 {
            let mut ext = [0u8; 8];
            if stream.read_exact(&mut ext).is_err() {
                break;
            }
            payload_len = u64::from_be_bytes(ext);
        }

        let mut mask_key = [0u8; 4];
        if is_masked {
            if stream.read_exact(&mut mask_key).is_err() {
                break;
            }
        }

        // Limit maximum frame size to 2MB to prevent memory exhaustion
        if payload_len > 2 * 1024 * 1024 {
            eprintln!("\x1b[1;31m[web-cast]\x1b[0m Payload too large: {} bytes", payload_len);
            break;
        }

        let mut payload = vec![0u8; payload_len as usize];
        if stream.read_exact(&mut payload).is_err() {
            break;
        }

        if is_masked {
            for i in 0..payload.len() {
                payload[i] ^= mask_key[i % 4];
            }
        }

        if opcode == 0x09 {
            // Ping frame -> Send Pong
            let mut pong = vec![0x8A, 0x00];
            let _ = stream.write_all(&mut pong);
            continue;
        }

        if opcode != 0x02 && opcode != 0x00 {
            // Only process binary or continuation frames
            continue;
        }

        // Check if payload is already RTP or raw Annex-B H.264
        if payload.len() > 12 && (payload[0] & 0xC0) == 0x80 && (payload[1] & 0x7F) == 96 {
            // Already RTP packet from client
            let _ = udp_sock.send_to(&payload, &target_addr);
        } else {
            // Wrap NALU / Annex-B into RFC 6184 RTP packet
            // Remove leading 0x00 0x00 0x00 0x01 or 0x00 0x00 0x01 if present
            let mut nalu_slice = payload.as_slice();
            while nalu_slice.starts_with(&[0, 0, 0, 1]) {
                nalu_slice = &nalu_slice[4..];
            }
            while nalu_slice.starts_with(&[0, 0, 1]) {
                nalu_slice = &nalu_slice[3..];
            }

            if nalu_slice.is_empty() {
                continue;
            }

            // Fragment NAL units larger than MTU (1300 bytes) into FU-A
            const MAX_RTP_PAYLOAD: usize = 1200;
            if nalu_slice.len() <= MAX_RTP_PAYLOAD {
                let mut packet = Vec::with_capacity(12 + nalu_slice.len());
                packet.push(0x80); // Version 2
                packet.push(0x60 | 0x80); // Payload type 96, Marker = 1
                packet.extend_from_slice(&seq.to_be_bytes());
                packet.extend_from_slice(&ts.to_be_bytes());
                packet.extend_from_slice(&ssrc.to_be_bytes());
                packet.extend_from_slice(nalu_slice);
                let _ = udp_sock.send_to(&packet, &target_addr);
                seq = seq.wrapping_add(1);
            } else {
                let nal_header = nalu_slice[0];
                let nal_type = nal_header & 0x1F;
                let nri = nal_header & 0x60;
                let mut data = &nalu_slice[1..];
                let mut is_start = true;

                while !data.is_empty() {
                    let chunk_size = data.len().min(MAX_RTP_PAYLOAD);
                    let is_end = chunk_size == data.len();

                    let mut packet = Vec::with_capacity(14 + chunk_size);
                    packet.push(0x80);
                    packet.push(0x60 | if is_end { 0x80 } else { 0x00 });
                    packet.extend_from_slice(&seq.to_be_bytes());
                    packet.extend_from_slice(&ts.to_be_bytes());
                    packet.extend_from_slice(&ssrc.to_be_bytes());

                    // FU indicator
                    packet.push(nri | 28);
                    // FU header: S | E | R | Type
                    let fu_hdr = (if is_start { 0x80 } else { 0x00 })
                        | (if is_end { 0x40 } else { 0x00 })
                        | nal_type;
                    packet.push(fu_hdr);
                    packet.extend_from_slice(&data[..chunk_size]);

                    let _ = udp_sock.send_to(&packet, &target_addr);
                    seq = seq.wrapping_add(1);
                    data = &data[chunk_size..];
                    is_start = false;
                }
            }
            ts = ts.wrapping_add(3000); // 90kHz / 30fps = 3000 ticks
        }
    }

    println!("\x1b[1;33m[web-cast]\x1b[0m Web Cast ended. Restoring display...");
    pipe.pause();
    crate::display::SplashEngine::show_ready();
}

/// Dedicated Web Cast Single-Page Application (HTML, CSS, JS)
pub const CAST_HTML: &str = r##"<!DOCTYPE html>
<html lang="pt-BR">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Ext-Monitor • Web Screen & Tab Cast</title>
    <style>
        :root {
            --bg-base: #0a0e17;
            --bg-card: rgba(16, 23, 38, 0.88);
            --border: rgba(0, 229, 255, 0.22);
            --accent: #00e5ff;
            --accent-green: #00ff66;
            --accent-red: #ff5252;
            --text-main: #f0f6fc;
            --text-muted: #94a3b8;
            --radius: 14px;
        }
        * { margin: 0; padding: 0; box-sizing: border-box; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; }
        body { background: var(--bg-base); color: var(--text-main); min-height: 100vh; display: flex; flex-direction: column; align-items: center; justify-content: center; padding: 1.5rem; }
        .cast-container { width: 100%; max-width: 680px; background: var(--bg-card); border: 1px solid var(--border); border-radius: var(--radius); padding: 2.2rem; box-shadow: 0 10px 40px rgba(0,0,0,0.6), 0 0 25px rgba(0,229,255,0.12); backdrop-filter: blur(16px); }
        .cast-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 1.8rem; border-bottom: 1px solid rgba(255,255,255,0.08); padding-bottom: 1rem; }
        .logo { display: flex; align-items: center; gap: 0.8rem; font-size: 1.35rem; font-weight: 700; color: #fff; }
        .logo span { color: var(--accent); }
        .badge { background: rgba(0,229,255,0.15); color: var(--accent); padding: 0.25rem 0.65rem; border-radius: 20px; font-size: 0.75rem; font-weight: 600; text-transform: uppercase; border: 1px solid rgba(0,229,255,0.3); }
        .badge.live { background: rgba(0,255,102,0.15); color: var(--accent-green); border-color: rgba(0,255,102,0.4); animation: pulse 1.5s infinite; }
        @keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.6; } }
        .info-box { background: rgba(0,229,255,0.06); border-left: 4px solid var(--accent); padding: 1rem; border-radius: 8px; font-size: 0.9rem; color: var(--text-muted); line-height: 1.5; margin-bottom: 1.8rem; }
        .info-box strong { color: #fff; }
        .controls-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 1rem; margin-bottom: 1.8rem; }
        .control-group { display: flex; flex-direction: column; gap: 0.4rem; }
        label { font-size: 0.82rem; color: var(--text-muted); font-weight: 600; text-transform: uppercase; letter-spacing: 0.5px; }
        select, input[type="range"] { background: rgba(255,255,255,0.05); border: 1px solid rgba(255,255,255,0.12); color: #fff; padding: 0.65rem 0.8rem; border-radius: 8px; font-size: 0.95rem; outline: none; transition: 0.2s; }
        select:focus { border-color: var(--accent); box-shadow: 0 0 10px rgba(0,229,255,0.3); }
        .btn-action { width: 100%; padding: 1rem 1.5rem; font-size: 1.1rem; font-weight: 700; border: none; border-radius: 10px; cursor: pointer; display: flex; align-items: center; justify-content: center; gap: 0.6rem; transition: all 0.25s ease; }
        .btn-start { background: linear-gradient(135deg, var(--accent) 0%, #0091ea 100%); color: #000; box-shadow: 0 4px 20px rgba(0,229,255,0.35); }
        .btn-start:hover { transform: translateY(-2px); box-shadow: 0 6px 25px rgba(0,229,255,0.5); }
        .btn-stop { background: linear-gradient(135deg, var(--accent-red) 0%, #d50000 100%); color: #fff; box-shadow: 0 4px 20px rgba(255,82,82,0.35); display: none; }
        .btn-stop:hover { transform: translateY(-2px); box-shadow: 0 6px 25px rgba(255,82,82,0.5); }
        .preview-box { margin-top: 1.5rem; background: #000; border-radius: 10px; overflow: hidden; position: relative; border: 1px solid rgba(255,255,255,0.1); display: none; }
        video { width: 100%; height: auto; display: block; }
        .stats-bar { display: flex; justify-content: space-around; background: rgba(0,0,0,0.5); padding: 0.75rem; border-top: 1px solid rgba(255,255,255,0.08); font-size: 0.85rem; color: var(--text-muted); }
        .stats-bar span strong { color: var(--accent); }
        .back-link { display: inline-flex; align-items: center; gap: 0.4rem; color: var(--text-muted); text-decoration: none; font-size: 0.85rem; margin-top: 1.5rem; transition: 0.2s; }
        .back-link:hover { color: var(--accent); }
    </style>
</head>
<body>
    <div class="cast-container">
        <div class="cast-header">
            <div class="logo">
                <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="2" y="3" width="20" height="14" rx="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/></svg>
                Ext-Monitor <span>Web Cast</span>
            </div>
            <span class="badge" id="statusBadge">Pronto</span>
        </div>

        <div class="info-box">
            <strong>Transmissão Direta do Navegador:</strong> Transmita qualquer aba do Chrome, janela de aplicativo ou a tela inteira sem restrições de certificados proprietários. O fluxo é codificado e exibido diretamente no Raspberry Pi Zero via HDMI.
        </div>

        <div class="controls-grid">
            <div class="control-group">
                <label>Resolução & Pacing</label>
                <select id="resSelect">
                    <option value="720p60" selected>1280x720 @ 60 FPS (Nativa Pi Zero)</option>
                    <option value="1080p30">1920x1080 @ 30 FPS (Full HD)</option>
                    <option value="720p30">1280x720 @ 30 FPS (Econômica)</option>
                    <option value="480p30">854x480 @ 30 FPS (Ultra Leve)</option>
                </select>
            </div>
            <div class="control-group">
                <label>Taxa de Bits (Bitrate)</label>
                <select id="bitrateSelect">
                    <option value="2500000" selected>2.5 Mbps (Balanceado)</option>
                    <option value="4000000">4.0 Mbps (Alta Nitidez)</option>
                    <option value="1500000">1.5 Mbps (Baixa Latência)</option>
                </select>
            </div>
        </div>

        <button class="btn-action btn-start" id="startBtn">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z"/></svg>
            Transmitir Guia / Janela / Tela
        </button>

        <button class="btn-action btn-stop" id="stopBtn">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor"><rect x="6" y="6" width="12" height="12"/></svg>
            Parar Transmissão
        </button>

        <div class="preview-box" id="previewBox">
            <video id="previewVideo" autoplay playsinline muted></video>
            <div class="stats-bar">
                <span>Status: <strong id="statState">Transmitindo</strong></span>
                <span>FPS: <strong id="statFps">60</strong></span>
                <span>Resolução: <strong id="statRes">1280x720</strong></span>
                <span>Tempo: <strong id="statTimer">00:00</strong></span>
            </div>
        </div>

        <a href="/" class="back-link">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="19" y1="12" x2="5" y2="12"/><polyline points="12 19 5 12 12 5"/></svg>
            Voltar ao Dashboard Principal
        </a>
    </div>

    <script>
        let activeStream = null;
        let activeWs = null;
        let activeEncoder = null;
        let timerInterval = null;
        let secondsElapsed = 0;

        const startBtn = document.getElementById('startBtn');
        const stopBtn = document.getElementById('stopBtn');
        const previewBox = document.getElementById('previewBox');
        const previewVideo = document.getElementById('previewVideo');
        const statusBadge = document.getElementById('statusBadge');
        const resSelect = document.getElementById('resSelect');
        const bitrateSelect = document.getElementById('bitrateSelect');

        function updateTimer() {
            secondsElapsed++;
            const mins = String(Math.floor(secondsElapsed / 60)).padStart(2, '0');
            const secs = String(secondsElapsed % 60).padStart(2, '0');
            document.getElementById('statTimer').innerText = `${mins}:${secs}`;
        }

        startBtn.addEventListener('click', async () => {
            try {
                const resMode = resSelect.value;
                let targetWidth = 1280, targetHeight = 720, targetFps = 60;
                if (resMode === '1080p30') { targetWidth = 1920; targetHeight = 1080; targetFps = 30; }
                else if (resMode === '720p30') { targetWidth = 1280; targetHeight = 720; targetFps = 30; }
                else if (resMode === '480p30') { targetWidth = 854; targetHeight = 480; targetFps = 30; }

                const targetBitrate = parseInt(bitrateSelect.value, 10);

                // Open native browser screen / tab picker
                const stream = await navigator.mediaDevices.getDisplayMedia({
                    video: {
                        width: { ideal: targetWidth, max: targetWidth },
                        height: { ideal: targetHeight, max: targetHeight },
                        frameRate: { ideal: targetFps, max: targetFps },
                        cursor: "always"
                    },
                    audio: true
                });

                activeStream = stream;
                previewVideo.srcObject = stream;
                previewBox.style.display = 'block';

                document.getElementById('statRes').innerText = `${targetWidth}x${targetHeight}`;
                document.getElementById('statFps').innerText = `${targetFps}`;

                // Handle stop sharing from browser UI
                stream.getVideoTracks()[0].onended = () => stopCasting();

                // Open WebSocket to Pi Zero
                const wsProto = location.protocol === 'https:' ? 'wss:' : 'ws:';
                const wsUrl = `${wsProto}//${location.host}/api/stream/ws`;
                const ws = new WebSocket(wsUrl);
                ws.binaryType = 'arraybuffer';
                activeWs = ws;

                ws.onopen = async () => {
                    statusBadge.innerText = 'AO VIVO';
                    statusBadge.className = 'badge live';
                    startBtn.style.display = 'none';
                    stopBtn.style.display = 'flex';
                    secondsElapsed = 0;
                    timerInterval = setInterval(updateTimer, 1000);

                    // Use WebCodecs VideoEncoder for ultra low latency hardware encoding
                    if ('VideoEncoder' in window) {
                        try {
                            const videoTrack = stream.getVideoTracks()[0];
                            const trackProcessor = new MediaStreamTrackProcessor({ track: videoTrack });
                            const reader = trackProcessor.readable.getReader();

                            const encoder = new VideoEncoder({
                                output: (chunk) => {
                                    if (ws.readyState === WebSocket.OPEN) {
                                        const buf = new Uint8Array(chunk.byteLength);
                                        chunk.copyTo(buf);
                                        ws.send(buf);
                                    }
                                },
                                error: (e) => console.error('[WebCodecs]', e)
                            });

                            encoder.configure({
                                codec: 'avc1.42001f', // Baseline 3.1
                                width: targetWidth,
                                height: targetHeight,
                                bitrate: targetBitrate,
                                framerate: targetFps,
                                avc: { format: 'annexb' }
                            });
                            activeEncoder = encoder;

                            let frameIndex = 0;
                            while (activeStream) {
                                const { done, value: videoFrame } = await reader.read();
                                if (done || !videoFrame) break;
                                const keyFrame = (frameIndex % 30 === 0);
                                encoder.encode(videoFrame, { keyFrame });
                                videoFrame.close();
                                frameIndex++;
                            }
                        } catch (err) {
                            console.warn('[WebCodecs fallback to MediaRecorder]', err);
                            fallbackMediaRecorder(stream, ws);
                        }
                    } else {
                        fallbackMediaRecorder(stream, ws);
                    }
                };

                ws.onerror = (e) => {
                    console.error('[WS Error]', e);
                    stopCasting();
                };

                ws.onclose = () => {
                    stopCasting();
                };

            } catch (err) {
                console.error('[getDisplayMedia Error]', err);
                alert('Seleção de tela/aba cancelada ou não suportada.');
            }
        });

        function fallbackMediaRecorder(stream, ws) {
            let mime = 'video/webm; codecs=h264';
            if (!MediaRecorder.isTypeSupported(mime)) mime = 'video/webm';
            const recorder = new MediaRecorder(stream, { mimeType: mime, videoBitsPerSecond: 2500000 });
            recorder.ondataavailable = async (e) => {
                if (e.data.size > 0 && ws.readyState === WebSocket.OPEN) {
                    const buf = await e.data.arrayBuffer();
                    ws.send(buf);
                }
            };
            recorder.start(40);
        }

        stopBtn.addEventListener('click', () => {
            stopCasting();
        });

        function stopCasting() {
            if (timerInterval) {
                clearInterval(timerInterval);
                timerInterval = null;
            }
            if (activeEncoder) {
                try { activeEncoder.close(); } catch(e){}
                activeEncoder = null;
            }
            if (activeWs) {
                try { activeWs.close(); } catch(e){}
                activeWs = null;
            }
            if (activeStream) {
                activeStream.getTracks().forEach(t => t.stop());
                activeStream = null;
            }
            previewVideo.srcObject = null;
            previewBox.style.display = 'none';
            startBtn.style.display = 'flex';
            stopBtn.style.display = 'none';
            statusBadge.innerText = 'Pronto';
            statusBadge.className = 'badge';
        }
    </script>
</body>
</html>
"##;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha1_known_vector() {
        // Standard FIPS 180-1 test vector "abc"
        // SHA1("abc") = a9993e364706816aba3e25717850c26c9cd0d89d
        let digest = sha1(b"abc");
        let hex: String = digest.iter().map(|b| format!("{:02x}", b)).collect();
        assert_eq!(hex, "a9993e364706816aba3e25717850c26c9cd0d89d");
    }

    #[test]
    fn test_base64_known_vector() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
    }

    #[test]
    fn test_rfc6455_websocket_accept() {
        // RFC 6455 Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==
        // Expected Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=
        let key = "dGhlIHNhbXBsZSBub25jZQ==";
        let accept = generate_websocket_accept(key);
        assert_eq!(accept, "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
    }
}

