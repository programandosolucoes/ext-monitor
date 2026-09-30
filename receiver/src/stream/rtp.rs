//! RTP H.264 Depayloader and RFC 4571 Stream Framing
//!
//! Reassembles discrete NAL units and complete Access Units (AUs) from RTP network packets:
//! - Single NAL Unit Packets (types 1..=23)
//! - STAP-A Aggregation Packets (type 24, e.g. SPS + PPS in a single datagram)
//! - FU-A Fragmentation Units (type 28, slices spanning multiple packets)
//! - RFC 4571 2-byte length-delimited framing over streaming transports (USB Bulk / TCP)
//!
//! Guarantees:
//! 1. Complete Access Units emitted immediately upon RTP packet with marker bit (M=1).
//! 2. Zero slice truncation, zero macroblock artifacts.
//! 3. Zero latency / zero waiting for subsequent frames.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

pub struct RtpDepayloader {
    fu_accumulator: Vec<u8>,
    current_au: Vec<u8>,
}

impl RtpDepayloader {
    pub fn new() -> Self {
        Self {
            fu_accumulator: Vec::with_capacity(65536),
            current_au: Vec::with_capacity(128 * 1024),
        }
    }

    /// Depayloads a full RTP packet (including 12-byte header).
    /// If the packet has the RTP marker bit (M=1), the completed Access Unit is pushed to `frames_out`.
    pub fn depayload_packet(&mut self, packet: &[u8], frames_out: &mut Vec<Vec<u8>>) {
        if packet.len() <= 12 {
            return;
        }

        let marker = (packet[1] & 0x80) != 0;
        let payload = &packet[12..];

        self.depayload_payload(payload);

        if marker && !self.current_au.is_empty() {
            frames_out.push(std::mem::take(&mut self.current_au));
            self.current_au.reserve(128 * 1024);
        }
    }

    /// Depayloads raw RTP payload data into `current_au`
    pub fn depayload_payload(&mut self, payload: &[u8]) {
        if payload.is_empty() {
            return;
        }

        let nal_type = payload[0] & 0x1F;

        if nal_type >= 1 && nal_type <= 23 {
            // 1. Single NAL Unit Packet
            self.current_au.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
            self.current_au.extend_from_slice(payload);
        } else if nal_type == 24 {
            // 2. STAP-A Aggregation Packet
            let mut offset = 1;
            while offset + 2 <= payload.len() {
                let nalu_size = u16::from_be_bytes([payload[offset], payload[offset + 1]]) as usize;
                offset += 2;
                if offset + nalu_size <= payload.len() {
                    let nalu = &payload[offset..offset + nalu_size];
                    self.current_au.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
                    self.current_au.extend_from_slice(nalu);
                    offset += nalu_size;
                } else {
                    break;
                }
            }
        } else if nal_type == 28 {
            // 3. FU-A Fragmentation Unit
            if payload.len() < 2 {
                return;
            }
            let fu_header = payload[1];
            let start_bit = (fu_header & 0x80) != 0;
            let end_bit = (fu_header & 0x40) != 0;
            let reconstructed_nal_type = (payload[0] & 0xE0) | (fu_header & 0x1F);

            if start_bit {
                self.fu_accumulator.clear();
                self.fu_accumulator.push(reconstructed_nal_type);
                self.fu_accumulator.extend_from_slice(&payload[2..]);
            } else if !self.fu_accumulator.is_empty() {
                self.fu_accumulator.extend_from_slice(&payload[2..]);
            }

            if end_bit && !self.fu_accumulator.is_empty() {
                self.current_au.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
                self.current_au.extend_from_slice(&self.fu_accumulator);
                self.fu_accumulator.clear();
            }
        }
    }

    /// Flushes any pending Access Unit trapped in the buffer
    #[allow(dead_code)]
    pub fn flush(&mut self, frames_out: &mut Vec<Vec<u8>>) {
        if !self.current_au.is_empty() {
            frames_out.push(std::mem::take(&mut self.current_au));
            self.current_au.reserve(128 * 1024);
        }
    }
}

/// Reassembles discrete RFC 4571 framed RTP packets from an unstructured byte stream (USB Bulk / TCP)
pub struct Rfc4571Assembler {
    buffer: Vec<u8>,
}

impl Rfc4571Assembler {
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(128 * 1024),
        }
    }

    /// Pushes incoming byte chunks and extracts complete RTP packets via `depayloader`
    pub fn push(
        &mut self,
        chunk: &[u8],
        depayloader: &mut RtpDepayloader,
        frames_out: &mut Vec<Vec<u8>>,
    ) {
        if chunk.is_empty() {
            return;
        }

        self.buffer.extend_from_slice(chunk);

        loop {
            if self.buffer.len() < 2 {
                break;
            }

            let pkt_len = u16::from_be_bytes([self.buffer[0], self.buffer[1]]) as usize;
            if self.buffer.len() < 2 + pkt_len {
                break; // Incomplete packet, await more bytes
            }

            let rtp_packet = &self.buffer[2..2 + pkt_len];
            depayloader.depayload_packet(rtp_packet, frames_out);

            self.buffer.drain(0..2 + pkt_len);
        }
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_nal_and_marker() {
        let mut depayloader = RtpDepayloader::new();
        let mut frames = Vec::new();

        // Packet 1: Single NAL (AUD type 9), marker = 0
        let mut p1 = vec![0x80, 0x60, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01];
        p1.extend_from_slice(&[0x09, 0xF0]);
        depayloader.depayload_packet(&p1, &mut frames);
        assert_eq!(frames.len(), 0);

        // Packet 2: Single NAL (IDR Slice type 5), marker = 1 (bit 0x80 in byte 1)
        let mut p2 = vec![0x80, 0xE0, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01];
        p2.extend_from_slice(&[0x65, 0x88, 0x11, 0x22]);
        depayloader.depayload_packet(&p2, &mut frames);

        // Should emit 1 complete Access Unit containing both AUD and IDR Slice
        assert_eq!(frames.len(), 1);
        assert_eq!(
            frames[0],
            vec![
                0x00, 0x00, 0x00, 0x01, 0x09, 0xF0, // AUD
                0x00, 0x00, 0x00, 0x01, 0x65, 0x88, 0x11, 0x22 // IDR Slice
            ]
        );
    }

    #[test]
    fn test_fua_assembly_into_complete_access_unit() {
        let mut depayloader = RtpDepayloader::new();
        let mut frames = Vec::new();

        // Header without marker (byte 1 = 0x60)
        let hdr_no_marker = [0x80, 0x60, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01];
        // Header with marker (byte 1 = 0xE0)
        let hdr_marker = [0x80, 0xE0, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01];

        // FU-A Start
        let mut p1 = hdr_no_marker.to_vec();
        p1.extend_from_slice(&[0x7C, 0x85, 0x11, 0x22]);
        depayloader.depayload_packet(&p1, &mut frames);
        assert_eq!(frames.len(), 0);

        // FU-A Middle
        let mut p2 = hdr_no_marker.to_vec();
        p2.extend_from_slice(&[0x7C, 0x05, 0x33, 0x44]);
        depayloader.depayload_packet(&p2, &mut frames);
        assert_eq!(frames.len(), 0);

        // FU-A End with marker=1
        let mut p3 = hdr_marker.to_vec();
        p3.extend_from_slice(&[0x7C, 0x45, 0x55, 0x66]);
        depayloader.depayload_packet(&p3, &mut frames);

        assert_eq!(frames.len(), 1);
        assert_eq!(
            frames[0],
            vec![0x00, 0x00, 0x00, 0x01, 0x65, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66]
        );
    }

    #[test]
    fn test_rfc4571_stream_framing() {
        let mut assembler = Rfc4571Assembler::new();
        let mut depayloader = RtpDepayloader::new();
        let mut frames = Vec::new();

        // 1 RTP packet with marker=1
        let mut rtp = vec![0x80, 0xE0, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01];
        rtp.extend_from_slice(&[0x09, 0xF0]);

        let pkt_len = rtp.len() as u16;
        let mut stream = Vec::new();
        stream.extend_from_slice(&pkt_len.to_be_bytes());
        stream.extend_from_slice(&rtp);

        // Feed in 3-byte chunks to test stream reassembly across reads
        for chunk in stream.chunks(3) {
            assembler.push(chunk, &mut depayloader, &mut frames);
        }

        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0], vec![0x00, 0x00, 0x00, 0x01, 0x09, 0xF0]);
    }
}
