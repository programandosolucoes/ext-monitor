//! MPEG-2 Transport Stream (MPEG-TS) Demuxer in Pure Rust
//!
//! Extracts elementary H.264 video streams from MPEG-TS packets (RFC 2250 / Wi-Fi Display).
//! Parses 188-byte TS packets and PES headers without allocating intermediate buffers.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use crate::stream::AnnexBAssembler;

pub struct TsDemuxer {
    video_pid: Option<u16>,
    assembler: AnnexBAssembler,
}

impl TsDemuxer {
    pub fn new() -> Self {
        Self {
            video_pid: None,
            assembler: AnnexBAssembler::new(),
        }
    }

    /// Ingests a raw UDP packet (which may contain an optional 12-byte RTP header
    /// followed by multiple 188-byte TS packets)
    pub fn push_udp_packet(&mut self, data: &[u8], frames_out: &mut Vec<Vec<u8>>) {
        let mut offset = 0;

        // Check if packet has an RTP header (payload type 33 for MP2T or byte 0 == 0x80)
        if data.len() > 12 && data[0] == 0x80 && (data[1] & 0x7F) == 33 {
            offset = 12;
        } else if data.len() > 12 && data[0] == 0x80 && data[12] == 0x47 {
            offset = 12;
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
    }

    fn parse_ts_packet(&mut self, pkt: &[u8], frames_out: &mut Vec<Vec<u8>>) {
        let tei = (pkt[1] & 0x80) != 0;
        if tei {
            return;
        }

        let pusi = (pkt[1] & 0x40) != 0;
        let pid = (((pkt[1] & 0x1F) as u16) << 8) | (pkt[2] as u16);

        // Ignore PAT, CAT, Null packets
        if pid == 0x0000 || pid == 0x0001 || pid == 0x1FFF {
            return;
        }

        let afc = (pkt[3] >> 4) & 0x03;
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
                    let pes_header_data_len = payload[8] as usize;
                    let es_offset = 9 + pes_header_data_len;
                    if es_offset < payload.len() {
                        self.assembler.push(&payload[es_offset..], frames_out);
                    }
                }
            }
        } else if let Some(vpid) = self.video_pid {
            if pid == vpid {
                self.assembler.push(payload, frames_out);
            }
        }
    }

    pub fn flush(&mut self, frames_out: &mut Vec<Vec<u8>>) {
        self.assembler.flush(frames_out);
    }
}
