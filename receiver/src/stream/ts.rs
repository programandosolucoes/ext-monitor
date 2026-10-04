//! MPEG-2 Transport Stream (MPEG-TS) Demuxer in Pure Rust
//!
//! Extracts elementary H.264 video streams from MPEG-TS packets (RFC 2250 / Wi-Fi Display).
//! Parses 188-byte TS packets and PES headers directly, assembling complete Access Units (AUs)
//! without intermediate guessing heuristics or extra memory copies.
//!
//! Guaranteed Invariants:
//! 1. Strictly demarcates Access Units via PES packet boundaries (PUSI=1) and RTP Marker bit (M=1).
//! 2. Zero-latency: emits completed frames instantly as soon as the last packet arrives.
//! 3. Checks TS continuity counter to discard corrupted frames caused by packet loss.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

/// Represents Tsdemuxer configuration and operational state.
pub struct TsDemuxer {
    video_pid: Option<u16>,
    current_au: Vec<u8>,
    last_cc: Option<u8>,
    frame_corrupted: bool,
}

impl TsDemuxer {
    /// Constructs and initializes a new `new` instance with default or provided parameters.
    pub fn new() -> Self {
        Self {
            video_pid: None,
            current_au: Vec::with_capacity(128 * 1024),
            last_cc: None,
            frame_corrupted: false,
        }
    }

    /// Ingests a raw UDP packet (which may contain an optional 12-byte RTP header
    /// followed by multiple 188-byte TS packets)
    pub fn push_udp_packet(&mut self, data: &[u8], frames_out: &mut Vec<Vec<u8>>) {
        let mut offset = 0;
        let mut rtp_marker = false;

        // Check if packet has an RTP header (payload type 33 for MP2T or byte 0 == 0x80)
        if data.len() > 12 && data[0] == 0x80 && (data[1] & 0x7F) == 33 {
            offset = 12;
            rtp_marker = (data[1] & 0x80) != 0;
        } else if data.len() > 12 && data[0] == 0x80 && data[12] == 0x47 {
            offset = 12;
            rtp_marker = (data[1] & 0x80) != 0;
        }

        let ts_data = &data[offset..];
        let mut pos = 0;

        while pos + 188 <= ts_data.len() {
            // Synchronize on 0x47 sync byte
            if ts_data[pos] != 0x47 {
                if let Some(sync_pos) = ts_data[pos..].iter().position(|&b| b == 0x47) {
                    pos += sync_pos;
                    if pos + 188 > ts_data.len() {
                        break;
                    }
                } else {
                    break;
                }
            }

            let pkt = &ts_data[pos..pos + 188];
            pos += 188;

            self.parse_ts_packet(pkt, frames_out);
        }

        // If RTP packet signaled end of Access Unit (Marker bit = 1), emit completed frame immediately
        if rtp_marker && !self.frame_corrupted && !self.current_au.is_empty() {
            let completed = std::mem::take(&mut self.current_au);
            self.current_au.reserve(128 * 1024);
            frames_out.push(completed);
        }
    }

    /// Executes `parse_ts_packet` operational routine.
    fn parse_ts_packet(&mut self, pkt: &[u8], frames_out: &mut Vec<Vec<u8>>) {
        let tei = (pkt[1] & 0x80) != 0;
        if tei {
            self.frame_corrupted = true;
            return;
        }

        let pusi = (pkt[1] & 0x40) != 0;
        let pid = (((pkt[1] & 0x1F) as u16) << 8) | (pkt[2] as u16);

        // Ignore PAT, CAT, Null packets
        if pid == 0x0000 || pid == 0x0001 || pid == 0x1FFF {
            return;
        }

        let afc = (pkt[3] >> 4) & 0x03;
        let cc = pkt[3] & 0x0F;
        let mut payload_offset = 4;

        match afc {
            0b01 => {
                // Payload only
            }
            0b11 => {
                // Adaptation field followed by payload
                let adaptation_len = pkt[4] as usize;
                payload_offset = 5 + adaptation_len;
                if payload_offset > 188 {
                    return;
                }
            }
            _ => {
                // No payload (reserved or adaptation only)
                return;
            }
        }

        let payload = &pkt[payload_offset..188];
        if payload.is_empty() {
            return;
        }

        if pusi {
            // Payload Unit Start Indicator: starts with PES header
            if payload.len() >= 9 && payload[0] == 0x00 && payload[1] == 0x00 && payload[2] == 0x01 {
                let stream_id = payload[3];
                // Video stream IDs are in range 0xE0..=0xEF
                if stream_id >= 0xE0 && stream_id <= 0xEF {
                    self.video_pid = Some(pid);

                    // A new PES packet with PUSI=1 strictly demarcates the end of the previous
                    // video Access Unit (WFA WFD Spec Sec 5.3.3: one video AU per PES packet).
                    if !self.frame_corrupted && !self.current_au.is_empty() {
                        let completed = std::mem::take(&mut self.current_au);
                        self.current_au.reserve(128 * 1024);
                        frames_out.push(completed);
                    } else {
                        self.current_au.clear();
                    }
                    self.frame_corrupted = false;
                    self.last_cc = Some(cc);

                    let pes_header_data_len = payload[8] as usize;
                    let es_offset = 9 + pes_header_data_len;
                    if es_offset < payload.len() {
                        self.current_au.extend_from_slice(&payload[es_offset..]);
                    }
                }
            }
        } else if let Some(vpid) = self.video_pid {
            if pid == vpid {
                // Continuity counter check for video payload packets
                if let Some(prev_cc) = self.last_cc {
                    let expected_cc = (prev_cc + 1) & 0x0F;
                    if cc == prev_cc {
                        // Duplicate TS packet: discard to avoid stream corruption
                        return;
                    } else if cc != expected_cc {
                        // Discontinuity / packet drop detected!
                        self.frame_corrupted = true;
                    }
                }
                self.last_cc = Some(cc);

                if !self.frame_corrupted {
                    self.current_au.extend_from_slice(payload);
                }
            }
        }
    }

    /// Flushes pending decoded frames retained inside hardware pipeline buffers.
    pub fn flush(&mut self, frames_out: &mut Vec<Vec<u8>>) {
        if !self.frame_corrupted && !self.current_au.is_empty() {
            let completed = std::mem::take(&mut self.current_au);
            self.current_au.reserve(128 * 1024);
            frames_out.push(completed);
        }
        self.frame_corrupted = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ts_demuxer_pes_demarcation() {
        let mut demuxer = TsDemuxer::new();
        let mut frames_out = Vec::new();

        // 1. Build TS packet 1 with PUSI=1 on video PID 0x100
        let mut pkt1 = vec![0x47, 0x41, 0x00, 0x10]; // sync, pusi=1, pid=0x100, payload only, cc=0
        // PES Header: 00 00 01 E0, len=00 00, flags=80 00 00 (header len = 0)
        let pes_hdr = [0x00, 0x00, 0x01, 0xE0, 0x00, 0x00, 0x80, 0x00, 0x00];
        pkt1.extend_from_slice(&pes_hdr);
        // H.264 slice data
        let h264_payload1 = [0x00, 0x00, 0x00, 0x01, 0x67, 0x42, 0x00, 0x1F];
        pkt1.extend_from_slice(&h264_payload1);
        pkt1.resize(188, 0xAA);

        demuxer.push_udp_packet(&pkt1, &mut frames_out);
        assert_eq!(frames_out.len(), 0); // Not finished yet

        // 2. Build TS packet 2 with PUSI=0 on video PID 0x100
        let mut pkt2 = vec![0x47, 0x01, 0x00, 0x11]; // sync, pusi=0, pid=0x100, payload only, cc=1
        let h264_payload2 = [0xBB; 50];
        pkt2.extend_from_slice(&h264_payload2);
        pkt2.resize(188, 0xAA);

        demuxer.push_udp_packet(&pkt2, &mut frames_out);
        assert_eq!(frames_out.len(), 0);

        // 3. Build TS packet 3 with PUSI=1 (starts Frame 2) -> Frame 1 emitted!
        let mut pkt3 = vec![0x47, 0x41, 0x00, 0x12]; // sync, pusi=1, pid=0x100, cc=2
        pkt3.extend_from_slice(&pes_hdr);
        pkt3.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x65]);
        pkt3.resize(188, 0xCC);

        demuxer.push_udp_packet(&pkt3, &mut frames_out);
        assert_eq!(frames_out.len(), 1);
        assert!(frames_out[0].starts_with(&[0x00, 0x00, 0x00, 0x01, 0x67]));

        // 4. Flush emits Frame 2
        demuxer.flush(&mut frames_out);
        assert_eq!(frames_out.len(), 2);
        assert!(frames_out[1].starts_with(&[0x00, 0x00, 0x00, 0x01, 0x65]));
    }
}
