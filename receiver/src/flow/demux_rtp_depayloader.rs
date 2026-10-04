//! RTP Video Depayloader Micro-Block (RFC 6184 H.264 & RFC 7798 HEVC/H.265)
//!
//! Blueprint Reference: Blueprint 27 (Flow-Oriented Micro-Block Architecture & Generic Codec Support)
//!
//! Single Responsibility:
//! Decapsulates RTP network datagrams into standard Annex-B Access Units (AUs) with 4-byte
//! start codes (`0x00 0x00 0x00 0x01`), fully supporting:
//! - H.264 / AVC (RFC 6184):
//!   * Single NAL Unit Packets (types 1..=23)
//!   * STAP-A Aggregation Packets (type 24, e.g. combined SPS + PPS)
//!   * FU-A Fragmentation Units (type 28, slices spanning multiple MTU datagrams)
//! - HEVC / H.265 (RFC 7798):
//!   * Single NAL Unit Packets (types 0..=47, including VPS=32, SPS=33, PPS=34, IDR=19..20)
//!   * AP Aggregation Packets (type 48, combined VPS + SPS + PPS)
//!   * FU Fragmentation Units (type 49, multi-packet fragmented slices with 2-byte NAL header reconstruction)
//!
//! Boundary Isolation Rules:
//! 1. Zero V4L2 / KMS coupling: Emits clean Annex-B byte buffers directly into decoder/assembler queues.
//! 2. Strict RTP Marker Handling: Emits completed frames instantly when the RTP marker bit (M=1) is asserted.
//! 3. Autonomous recovery: Corrupted fragmentation sequences are discarded without halting stream flow.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use super::codec_types::CodecKind;
use std::fmt;

/// Standard Annex-B 4-byte start code prefix
pub const ANNEX_B_START_CODE: [u8; 4] = [0x00, 0x00, 0x00, 0x01];

/// Demuxer error conditions during RTP packet validation and depayloading
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DemuxError {
    /// Packet is too small to contain a valid RTP header (< 12 bytes)
    PacketTooShort { length: usize, minimum: usize },
    /// RTP version field is invalid (must be 2)
    InvalidRtpVersion(u8),
    /// Truncated packet where header length exceeds packet size
    HeaderTruncated { header_len: usize, packet_len: usize },
    /// Corrupted payload or invalid aggregation length
    CorruptedPayload(String),
    /// Unsupported or unrecognized RTP payload type
    UnsupportedPayloadType(u8),
    /// Requested codec is not supported by this depayloader
    UnsupportedCodec(String),
}

impl fmt::Display for DemuxError {
    /// Formats the instance using the provided formatter for display and debugging.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DemuxError::PacketTooShort { length, minimum } => {
                write!(
                    f,
                    "RTP packet too short: {} bytes (minimum {} bytes required)",
                    length, minimum
                )
            }
            DemuxError::InvalidRtpVersion(v) => write!(f, "Invalid RTP version: expected 2, got {}", v),
            DemuxError::HeaderTruncated { header_len, packet_len } => {
                write!(
                    f,
                    "RTP header claims {} bytes, but packet is only {} bytes",
                    header_len, packet_len
                )
            }
            DemuxError::CorruptedPayload(msg) => write!(f, "Corrupted RTP payload: {}", msg),
            DemuxError::UnsupportedPayloadType(pt) => write!(f, "Unsupported RTP payload type: {}", pt),
            DemuxError::UnsupportedCodec(c) => write!(f, "Unsupported codec for RTP depayload: {}", c),
        }
    }
}

impl std::error::Error for DemuxError {}

/// Parsed RTP Header information
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RtpHeader {
    pub version: u8,
    pub padding: bool,
    pub extension: bool,
    pub csrc_count: usize,
    pub marker: bool,
    pub payload_type: u8,
    pub sequence_number: u16,
    pub timestamp: u32,
    pub ssrc: u32,
    pub header_length: usize,
}

impl RtpHeader {
    /// Parses fixed 12-byte RTP header and optional CSRC / extension headers
    pub fn parse(packet: &[u8]) -> Result<Self, DemuxError> {
        if packet.len() < 12 {
            return Err(DemuxError::PacketTooShort {
                length: packet.len(),
                minimum: 12,
            });
        }

        let version = (packet[0] >> 6) & 0x03;
        if version != 2 {
            return Err(DemuxError::InvalidRtpVersion(version));
        }

        let padding = (packet[0] & 0x20) != 0;
        let extension = (packet[0] & 0x10) != 0;
        let csrc_count = (packet[0] & 0x0F) as usize;

        let marker = (packet[1] & 0x80) != 0;
        let payload_type = packet[1] & 0x7F;

        let sequence_number = u16::from_be_bytes([packet[2], packet[3]]);
        let timestamp = u32::from_be_bytes([packet[4], packet[5], packet[6], packet[7]]);
        let ssrc = u32::from_be_bytes([packet[8], packet[9], packet[10], packet[11]]);

        let mut header_length = 12 + (csrc_count * 4);

        if header_length > packet.len() {
            return Err(DemuxError::HeaderTruncated {
                header_len: header_length,
                packet_len: packet.len(),
            });
        }

        if extension {
            if packet.len() < header_length + 4 {
                return Err(DemuxError::HeaderTruncated {
                    header_len: header_length + 4,
                    packet_len: packet.len(),
                });
            }
            let ext_len_words = u16::from_be_bytes([
                packet[header_length + 2],
                packet[header_length + 3],
            ]) as usize;
            header_length += 4 + (ext_len_words * 4);

            if header_length > packet.len() {
                return Err(DemuxError::HeaderTruncated {
                    header_len: header_length,
                    packet_len: packet.len(),
                });
            }
        }

        Ok(Self {
            version,
            padding,
            extension,
            csrc_count,
            marker,
            payload_type,
            sequence_number,
            timestamp,
            ssrc,
            header_length,
        })
    }
}

/// RTP Video Depayloader Micro-Block supporting H.264 and HEVC/H.265
pub struct RtpDepayloader {
    fu_accumulator: Vec<u8>,
    current_au: Vec<u8>,
    last_seq: Option<u16>,
    total_frames_completed: u64,
}

impl Default for RtpDepayloader {
    /// Returns default configuration parameters.
    fn default() -> Self {
        Self::new()
    }
}

impl RtpDepayloader {
    /// Creates a new RTP depayloader with pre-allocated accumulators
    pub fn new() -> Self {
        Self {
            fu_accumulator: Vec::with_capacity(65536),
            current_au: Vec::with_capacity(128 * 1024),
            last_seq: None,
            total_frames_completed: 0,
        }
    }

    /// Resets the internal state and accumulators
    pub fn reset(&mut self) {
        self.fu_accumulator.clear();
        self.current_au.clear();
        self.last_seq = None;
    }

    /// Flushes any pending Access Unit trapped in the buffer
    pub fn flush(&mut self) -> Option<Vec<u8>> {
        if !self.current_au.is_empty() {
            let frame = std::mem::take(&mut self.current_au);
            self.current_au.reserve(128 * 1024);
            self.total_frames_completed += 1;
            Some(frame)
        } else {
            None
        }
    }

    /// Depayloads an RTP packet for the specified video codec.
    ///
    /// If the RTP Marker bit (M=1) is present, the completed Access Unit is returned as `Ok(Some(frame))`.
    /// Otherwise, `Ok(None)` is returned as slices continue to accumulate.
    pub fn depayload(&mut self, packet: &[u8], codec: CodecKind) -> Result<Option<Vec<u8>>, DemuxError> {
        match codec {
            CodecKind::H264 => self.depayload_h264(packet),
            CodecKind::HevcH265 => self.depayload_hevc(packet),
            CodecKind::Av1 => Err(DemuxError::UnsupportedCodec("AV1 RTP depayload not implemented".into())),
        }
    }

    /// Depayloads an H.264 RTP packet conforming to RFC 6184
    pub fn depayload_h264(&mut self, packet: &[u8]) -> Result<Option<Vec<u8>>, DemuxError> {
        let header = RtpHeader::parse(packet)?;
        self.last_seq = Some(header.sequence_number);

        let mut payload_len = packet.len() - header.header_length;
        if header.padding && payload_len > 0 {
            let padding_len = packet[packet.len() - 1] as usize;
            if padding_len <= payload_len {
                payload_len -= padding_len;
            }
        }

        let payload = &packet[header.header_length..header.header_length + payload_len];
        if payload.is_empty() {
            return Ok(None);
        }

        let nal_type = payload[0] & 0x1F;

        match nal_type {
            1..=23 => {
                // Single NAL Unit Packet (RFC 6184 Section 5.6)
                self.current_au.extend_from_slice(&ANNEX_B_START_CODE);
                self.current_au.extend_from_slice(payload);
            }
            24 => {
                // STAP-A Aggregation Packet (RFC 6184 Section 5.7)
                let mut offset = 1;
                while offset + 2 <= payload.len() {
                    let nalu_size = u16::from_be_bytes([payload[offset], payload[offset + 1]]) as usize;
                    offset += 2;
                    if offset + nalu_size <= payload.len() {
                        let nalu = &payload[offset..offset + nalu_size];
                        self.current_au.extend_from_slice(&ANNEX_B_START_CODE);
                        self.current_au.extend_from_slice(nalu);
                        offset += nalu_size;
                    } else {
                        return Err(DemuxError::CorruptedPayload(format!(
                            "STAP-A truncated: size {} exceeds remaining bytes",
                            nalu_size
                        )));
                    }
                }
            }
            28 => {
                // FU-A Fragmentation Unit (RFC 6184 Section 5.8)
                if payload.len() < 2 {
                    return Err(DemuxError::CorruptedPayload("FU-A header too short".into()));
                }
                let fu_indicator = payload[0];
                let fu_header = payload[1];
                let start_bit = (fu_header & 0x80) != 0;
                let end_bit = (fu_header & 0x40) != 0;
                let original_nal_type = fu_header & 0x1F;
                let reconstructed_header = (fu_indicator & 0xE0) | original_nal_type;

                if start_bit {
                    self.fu_accumulator.clear();
                    self.fu_accumulator.push(reconstructed_header);
                    self.fu_accumulator.extend_from_slice(&payload[2..]);
                } else if !self.fu_accumulator.is_empty() {
                    self.fu_accumulator.extend_from_slice(&payload[2..]);
                }

                if end_bit && !self.fu_accumulator.is_empty() {
                    self.current_au.extend_from_slice(&ANNEX_B_START_CODE);
                    self.current_au.extend_from_slice(&self.fu_accumulator);
                    self.fu_accumulator.clear();
                }
            }
            _ => {
                // Non-standard or unsupported NAL types (e.g. STAP-B, MTAP)
                return Err(DemuxError::UnsupportedPayloadType(nal_type));
            }
        }

        if header.marker && !self.current_au.is_empty() {
            let frame = std::mem::take(&mut self.current_au);
            self.current_au.reserve(128 * 1024);
            self.total_frames_completed += 1;
            Ok(Some(frame))
        } else {
            Ok(None)
        }
    }

    /// Depayloads an HEVC / H.265 RTP packet conforming to RFC 7798
    pub fn depayload_hevc(&mut self, packet: &[u8]) -> Result<Option<Vec<u8>>, DemuxError> {
        let header = RtpHeader::parse(packet)?;
        self.last_seq = Some(header.sequence_number);

        let mut payload_len = packet.len() - header.header_length;
        if header.padding && payload_len > 0 {
            let padding_len = packet[packet.len() - 1] as usize;
            if padding_len <= payload_len {
                payload_len -= padding_len;
            }
        }

        let payload = &packet[header.header_length..header.header_length + payload_len];
        if payload.len() < 2 {
            return Err(DemuxError::CorruptedPayload(
                "HEVC payload shorter than 2-byte NAL header".into(),
            ));
        }

        // HEVC NAL unit type: bits 1-6 of payload byte 0: (byte0 >> 1) & 0x3F
        let nal_unit_type = (payload[0] >> 1) & 0x3F;

        match nal_unit_type {
            0..=47 => {
                // Single NAL Unit Packet (RFC 7798 Section 4.4.1)
                self.current_au.extend_from_slice(&ANNEX_B_START_CODE);
                self.current_au.extend_from_slice(payload);
            }
            48 => {
                // AP (Aggregation Packet) - HEVC equivalent of STAP-A (RFC 7798 Section 4.4.2)
                let mut offset = 2; // Skip 2-byte AP payload header
                while offset + 2 <= payload.len() {
                    let nalu_size = u16::from_be_bytes([payload[offset], payload[offset + 1]]) as usize;
                    offset += 2;
                    if offset + nalu_size <= payload.len() {
                        let nalu = &payload[offset..offset + nalu_size];
                        self.current_au.extend_from_slice(&ANNEX_B_START_CODE);
                        self.current_au.extend_from_slice(nalu);
                        offset += nalu_size;
                    } else {
                        return Err(DemuxError::CorruptedPayload(format!(
                            "HEVC AP truncated: size {} exceeds remaining bytes",
                            nalu_size
                        )));
                    }
                }
            }
            49 => {
                // FU (Fragmentation Unit) - HEVC equivalent of FU-A (RFC 7798 Section 4.4.3)
                if payload.len() < 3 {
                    return Err(DemuxError::CorruptedPayload(
                        "HEVC FU payload shorter than 3-byte header".into(),
                    ));
                }
                let fu_header = payload[2];
                let start_bit = (fu_header & 0x80) != 0;
                let end_bit = (fu_header & 0x40) != 0;
                let original_nal_type = fu_header & 0x3F;

                // Reconstruct the 2-byte HEVC NAL header:
                // Byte 0: forbidden_zero_bit (bit 7) + original_nal_type (bits 1-6) + nuh_layer_id MSB (bit 0)
                let hdr0 = (payload[0] & 0x81) | ((original_nal_type & 0x3F) << 1);
                // Byte 1: nuh_layer_id remaining bits + nuh_temporal_id_plus1
                let hdr1 = payload[1];

                if start_bit {
                    self.fu_accumulator.clear();
                    self.fu_accumulator.push(hdr0);
                    self.fu_accumulator.push(hdr1);
                    self.fu_accumulator.extend_from_slice(&payload[3..]);
                } else if !self.fu_accumulator.is_empty() {
                    self.fu_accumulator.extend_from_slice(&payload[3..]);
                }

                if end_bit && !self.fu_accumulator.is_empty() {
                    self.current_au.extend_from_slice(&ANNEX_B_START_CODE);
                    self.current_au.extend_from_slice(&self.fu_accumulator);
                    self.fu_accumulator.clear();
                }
            }
            _ => {
                return Err(DemuxError::UnsupportedPayloadType(nal_unit_type));
            }
        }

        if header.marker && !self.current_au.is_empty() {
            let frame = std::mem::take(&mut self.current_au);
            self.current_au.reserve(128 * 1024);
            self.total_frames_completed += 1;
            Ok(Some(frame))
        } else {
            Ok(None)
        }
    }

    /// Total count of frames completed and emitted
    pub fn total_frames_completed(&self) -> u64 {
        self.total_frames_completed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to synthesize a valid 12-byte RTP header
    fn make_rtp_header(marker: bool, pt: u8, seq: u16, ts: u32, ssrc: u32) -> [u8; 12] {
        let mut hdr = [0u8; 12];
        hdr[0] = 0x80; // V=2, P=0, X=0, CC=0
        hdr[1] = (if marker { 0x80 } else { 0x00 }) | (pt & 0x7F);
        hdr[2..4].copy_from_slice(&seq.to_be_bytes());
        hdr[4..8].copy_from_slice(&ts.to_be_bytes());
        hdr[8..12].copy_from_slice(&ssrc.to_be_bytes());
        hdr
    }

    #[test]
    fn test_h264_single_nal_depayload() {
        let mut depayloader = RtpDepayloader::new();
        let rtp_hdr = make_rtp_header(true, 96, 100, 90000, 0x12345678);

        // H.264 IDR Slice: NAL type 5 (0x65 = forbidden 0 | ref 3 | type 5)
        let nal_data = [0x65, 0x88, 0x84, 0x00, 0x10];
        let mut packet = Vec::new();
        packet.extend_from_slice(&rtp_hdr);
        packet.extend_from_slice(&nal_data);

        let result = depayloader.depayload_h264(&packet).expect("depayload single NAL");
        let frame = result.expect("should return completed frame due to marker bit");

        // Verify Annex-B start code [0,0,0,1] followed by NAL data
        assert_eq!(&frame[0..4], &[0x00, 0x00, 0x00, 0x01]);
        assert_eq!(&frame[4..], &nal_data);
        assert_eq!(depayloader.total_frames_completed(), 1);
    }

    #[test]
    fn test_h264_stap_a_depayload() {
        let mut depayloader = RtpDepayloader::new();
        let rtp_hdr = make_rtp_header(true, 96, 101, 90000, 0x12345678);

        // STAP-A aggregation packet containing SPS (type 7) and PPS (type 8)
        let sps = [0x67, 0x42, 0x00, 0x1E, 0x9A];
        let pps = [0x68, 0xCE, 0x38, 0x80];

        let mut payload = Vec::new();
        payload.push(24); // STAP-A indicator
        payload.extend_from_slice(&(sps.len() as u16).to_be_bytes());
        payload.extend_from_slice(&sps);
        payload.extend_from_slice(&(pps.len() as u16).to_be_bytes());
        payload.extend_from_slice(&pps);

        let mut packet = Vec::new();
        packet.extend_from_slice(&rtp_hdr);
        packet.extend_from_slice(&payload);

        let result = depayloader.depayload_h264(&packet).expect("depayload STAP-A");
        let frame = result.expect("should return frame");

        // Should have 2 NALUs, each with start codes
        let mut expected = Vec::new();
        expected.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
        expected.extend_from_slice(&sps);
        expected.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
        expected.extend_from_slice(&pps);

        assert_eq!(frame, expected);
    }

    #[test]
    fn test_h264_fu_a_depayload() {
        let mut depayloader = RtpDepayloader::new();

        // Packet 1: FU-A Start (M=0)
        let rtp_hdr1 = make_rtp_header(false, 96, 201, 90000, 0x12345678);
        // Original NAL: 0x65 (IDR, NRI=3, Type=5)
        // FU indicator: (0x65 & 0xE0) | 28 = 0x60 | 28 = 0x7C
        // FU header: Start (0x80) | Type 5 = 0x85
        let fu_ind = 0x7C;
        let fu_hdr_start = 0x85;
        let chunk1 = [1, 2, 3, 4, 5];

        let mut pkt1 = Vec::new();
        pkt1.extend_from_slice(&rtp_hdr1);
        pkt1.push(fu_ind);
        pkt1.push(fu_hdr_start);
        pkt1.extend_from_slice(&chunk1);

        let res1 = depayloader.depayload_h264(&pkt1).expect("pkt 1");
        assert_eq!(res1, None, "No frame yet, M=0");

        // Packet 2: FU-A End (M=1)
        let rtp_hdr2 = make_rtp_header(true, 96, 202, 90000, 0x12345678);
        // FU header: End (0x40) | Type 5 = 0x45
        let fu_hdr_end = 0x45;
        let chunk2 = [6, 7, 8, 9, 10];

        let mut pkt2 = Vec::new();
        pkt2.extend_from_slice(&rtp_hdr2);
        pkt2.push(fu_ind);
        pkt2.push(fu_hdr_end);
        pkt2.extend_from_slice(&chunk2);

        let res2 = depayloader.depayload_h264(&pkt2).expect("pkt 2");
        let frame = res2.expect("frame emitted upon M=1");

        let mut expected = Vec::new();
        expected.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
        expected.push(0x65); // Reconstructed header
        expected.extend_from_slice(&chunk1);
        expected.extend_from_slice(&chunk2);

        assert_eq!(frame, expected);
    }

    #[test]
    fn test_hevc_single_nal_depayload() {
        let mut depayloader = RtpDepayloader::new();
        let rtp_hdr = make_rtp_header(true, 98, 300, 90000, 0xAABBCCDD);

        // HEVC VPS NAL: Type 32 (0x40 = (32 << 1))
        let hevc_vps = [0x40, 0x01, 0x0C, 0x01, 0xFF, 0xFF];
        let mut packet = Vec::new();
        packet.extend_from_slice(&rtp_hdr);
        packet.extend_from_slice(&hevc_vps);

        let result = depayloader.depayload_hevc(&packet).expect("depayload HEVC VPS");
        let frame = result.expect("frame complete");

        assert_eq!(&frame[0..4], &[0x00, 0x00, 0x00, 0x01]);
        assert_eq!(&frame[4..], &hevc_vps);
    }

    #[test]
    fn test_hevc_ap_depayload() {
        let mut depayloader = RtpDepayloader::new();
        let rtp_hdr = make_rtp_header(true, 98, 301, 90000, 0xAABBCCDD);

        // HEVC AP: NAL type 48 (0x60 = (48 << 1))
        let vps = [0x40, 0x01, 0xAA];
        let sps = [0x42, 0x01, 0xBB];

        let mut payload = Vec::new();
        payload.extend_from_slice(&[0x60, 0x01]); // 2-byte AP header
        payload.extend_from_slice(&(vps.len() as u16).to_be_bytes());
        payload.extend_from_slice(&vps);
        payload.extend_from_slice(&(sps.len() as u16).to_be_bytes());
        payload.extend_from_slice(&sps);

        let mut packet = Vec::new();
        packet.extend_from_slice(&rtp_hdr);
        packet.extend_from_slice(&payload);

        let result = depayloader.depayload(&packet, CodecKind::HevcH265).expect("depayload AP");
        let frame = result.expect("frame complete");

        let mut expected = Vec::new();
        expected.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
        expected.extend_from_slice(&vps);
        expected.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
        expected.extend_from_slice(&sps);

        assert_eq!(frame, expected);
    }

    #[test]
    fn test_hevc_fu_depayload() {
        let mut depayloader = RtpDepayloader::new();

        // Fragmented HEVC IDR slice: Type 19 (IDR_W_RADL)
        // Original 2-byte header: (19 << 1) = 38 (0x26), nuh_layer_id=0, nuh_temporal_id_plus1=1 => [0x26, 0x01]
        // Payload Header: Type 49 (FU) => (49 << 1) = 98 (0x62) => [0x62, 0x01]
        let payload_hdr = [0x62, 0x01];

        // Packet 1: Start (M=0)
        let rtp_hdr1 = make_rtp_header(false, 98, 401, 90000, 0xAABBCCDD);
        let fu_hdr_start = 0x80 | 19; // S=1, Type=19
        let chunk1 = [0xDE, 0xAD, 0xBE, 0xEF];

        let mut pkt1 = Vec::new();
        pkt1.extend_from_slice(&rtp_hdr1);
        pkt1.extend_from_slice(&payload_hdr);
        pkt1.push(fu_hdr_start);
        pkt1.extend_from_slice(&chunk1);

        let res1 = depayloader.depayload_hevc(&pkt1).expect("pkt 1");
        assert_eq!(res1, None);

        // Packet 2: End (M=1)
        let rtp_hdr2 = make_rtp_header(true, 98, 402, 90000, 0xAABBCCDD);
        let fu_hdr_end = 0x40 | 19; // E=1, Type=19
        let chunk2 = [0xCA, 0xFE, 0xBA, 0xBE];

        let mut pkt2 = Vec::new();
        pkt2.extend_from_slice(&rtp_hdr2);
        pkt2.extend_from_slice(&payload_hdr);
        pkt2.push(fu_hdr_end);
        pkt2.extend_from_slice(&chunk2);

        let res2 = depayloader.depayload_hevc(&pkt2).expect("pkt 2");
        let frame = res2.expect("frame complete on M=1");

        let mut expected = Vec::new();
        expected.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
        expected.extend_from_slice(&[0x26, 0x01]); // Reconstructed 2-byte header
        expected.extend_from_slice(&chunk1);
        expected.extend_from_slice(&chunk2);

        assert_eq!(frame, expected);
    }
}
