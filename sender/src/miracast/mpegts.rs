//! Pure-Rust MPEG-TS / PES Packetizer and RTP MP2T Encapsulator
//!
//! Complies with:
//! - ISO/IEC 13818-1 (MPEG-2 Systems: Transport Stream, PES, PAT, PMT, PCR)
//! - RFC 2250 (RTP Payload Format for MPEG1/MPEG2 Video)
//! - RFC 3550 (RTP: A Transport Protocol for Real-Time Applications)
//! - Wi-Fi Display (WFD) / Miracast Technical Specification

pub const TS_PACKET_SIZE: usize = 188;
pub const TS_SYNC_BYTE: u8 = 0x47;
pub const PAT_PID: u16 = 0x0000;
pub const PMT_PID: u16 = 0x1000;
pub const VIDEO_PID: u16 = 0x0100;
pub const STREAM_TYPE_H264: u8 = 0x1B;
pub const PES_STREAM_ID_VIDEO: u8 = 0xE0;
pub const RTP_HEADER_SIZE: usize = 12;
pub const RTP_PAYLOAD_TYPE_MP2T: u8 = 33;
pub const TS_PACKETS_PER_RTP: usize = 7;
pub const RTP_MAX_PAYLOAD_SIZE: usize = TS_PACKET_SIZE * TS_PACKETS_PER_RTP; // 1316 bytes

/// Precomputed 256-entry lookup table for MPEG-2 32-bit CRC calculation (polynomial 0x04C11DB7).
const fn make_crc_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0usize;
    while i < 256 {
        let mut crc = (i as u32) << 24;
        let mut j = 0;
        while j < 8 {
            if (crc & 0x8000_0000) != 0 {
                crc = (crc << 1) ^ 0x04C1_1DB7;
            } else {
                crc <<= 1;
            }
            j += 1;
        }
        table[i] = crc;
        i += 1;
    }
    table
}

static CRC_TABLE: [u32; 256] = make_crc_table();

/// Calculates standard MPEG-2 32-bit CRC (polynomial 0x04C11DB7, initial 0xFFFFFFFF, MSB-first).
#[inline]
pub fn mpeg2_crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        let idx = (((crc >> 24) ^ (byte as u32)) & 0xFF) as usize;
        crc = (crc << 8) ^ CRC_TABLE[idx];
    }
    crc
}

/// Pure-Rust MPEG-TS / PES Muxer and RTP Encapsulator.
#[derive(Debug, Clone)]
pub struct MpegTsMuxer {
    pat_cc: u8,
    pmt_cc: u8,
    video_cc: u8,
    rtp_seq: u16,
    ssrc: u32,
}

impl Default for MpegTsMuxer {
    fn default() -> Self {
        Self::new()
    }
}

impl MpegTsMuxer {
    /// Creates a new `MpegTsMuxer` with zeroed continuity counters and default SSRC.
    pub fn new() -> Self {
        Self {
            pat_cc: 0,
            pmt_cc: 0,
            video_cc: 0,
            rtp_seq: 1,
            ssrc: 0x1234_5678,
        }
    }

    /// Creates a new `MpegTsMuxer` with custom SSRC.
    pub fn with_ssrc(ssrc: u32) -> Self {
        Self {
            pat_cc: 0,
            pmt_cc: 0,
            video_cc: 0,
            rtp_seq: 1,
            ssrc,
        }
    }

    /// Resets all continuity counters back to 0.
    pub fn reset_cc(&mut self) {
        self.pat_cc = 0;
        self.pmt_cc = 0;
        self.video_cc = 0;
    }

    /// Generates a 188-byte PAT (Program Association Table) TS packet.
    /// Program 1 -> PMT PID 0x1000.
    pub fn create_pat(&mut self) -> [u8; TS_PACKET_SIZE] {
        let mut pkt = [0xFFu8; TS_PACKET_SIZE];

        // 4-byte TS Header
        pkt[0] = TS_SYNC_BYTE;
        pkt[1] = 0x40 | ((PAT_PID >> 8) & 0x1F) as u8; // PUSI=1, PID=0x0000
        pkt[2] = (PAT_PID & 0xFF) as u8;
        pkt[3] = 0x10 | (self.pat_cc & 0x0F); // AFC=01 (payload only), CC
        self.pat_cc = (self.pat_cc + 1) & 0x0F;

        // Pointer field: 0x00 (section starts immediately)
        pkt[4] = 0x00;

        // PAT Section (16 bytes):
        // table_id: 0x00
        // section_syntax_indicator=1, '0', reserved=11, section_length=13 (0x000D) -> 0xB0, 0x0D
        // transport_stream_id: 0x0001
        // reserved=11, version=0, current_next=1 -> 0xC1
        // section_number: 0x00
        // last_section_number: 0x00
        // Program 1: program_number=0x0001, reserved=111, pmt_pid=0x1000 -> 0xF0, 0x00
        // CRC32 (4 bytes)
        let mut section = [0u8; 16];
        section[0] = 0x00; // table_id
        section[1] = 0xB0;
        section[2] = 0x0D; // section_length = 13
        section[3] = 0x00; // transport_stream_id hi
        section[4] = 0x01; // transport_stream_id lo
        section[5] = 0xC1; // version 0, current
        section[6] = 0x00; // section_number
        section[7] = 0x00; // last_section_number
        section[8] = 0x00; // program_number hi
        section[9] = 0x01; // program_number lo
        section[10] = 0xE0 | ((PMT_PID >> 8) & 0x1F) as u8; // 0xF0
        section[11] = (PMT_PID & 0xFF) as u8;                 // 0x00

        let crc = mpeg2_crc32(&section[0..12]);
        section[12..16].copy_from_slice(&crc.to_be_bytes());

        pkt[5..21].copy_from_slice(&section);
        // Bytes 21..188 remain 0xFF padding
        pkt
    }

    /// Generates a 188-byte PMT (Program Map Table) TS packet.
    /// Program 1, PCR PID 0x0100, Stream 0x0100 = 0x1B (H.264 video).
    pub fn create_pmt(&mut self) -> [u8; TS_PACKET_SIZE] {
        let mut pkt = [0xFFu8; TS_PACKET_SIZE];

        // 4-byte TS Header
        pkt[0] = TS_SYNC_BYTE;
        pkt[1] = 0x40 | ((PMT_PID >> 8) & 0x1F) as u8; // PUSI=1, PID=0x1000
        pkt[2] = (PMT_PID & 0xFF) as u8;
        pkt[3] = 0x10 | (self.pmt_cc & 0x0F); // AFC=01 (payload only), CC
        self.pmt_cc = (self.pmt_cc + 1) & 0x0F;

        // Pointer field: 0x00
        pkt[4] = 0x00;

        // PMT Section (21 bytes):
        // table_id: 0x02
        // section_syntax_indicator=1, '0', reserved=11, section_length=18 (0x0012) -> 0xB0, 0x12
        // program_number: 0x0001
        // reserved=11, version=0, current_next=1 -> 0xC1
        // section_number: 0x00
        // last_section_number: 0x00
        // PCR_PID: 0x0100 -> reserved 111 | 0x0100 -> 0xE1, 0x00
        // program_info_length: 0 -> reserved 1111 | 0x0000 -> 0xF0, 0x00
        // ES descriptor:
        //   stream_type: 0x1B (AVC / H.264)
        //   elementary_PID: 0x0100 -> reserved 111 | 0x0100 -> 0xE1, 0x00
        //   ES_info_length: 0 -> reserved 1111 | 0x0000 -> 0xF0, 0x00
        // CRC32 (4 bytes)
        let mut section = [0u8; 21];
        section[0] = 0x02; // table_id
        section[1] = 0xB0;
        section[2] = 0x12; // section_length = 18
        section[3] = 0x00; // program_number hi
        section[4] = 0x01; // program_number lo
        section[5] = 0xC1; // version 0, current
        section[6] = 0x00; // section_number
        section[7] = 0x00; // last_section_number
        section[8] = 0xE0 | ((VIDEO_PID >> 8) & 0x1F) as u8; // PCR_PID hi (0xE1)
        section[9] = (VIDEO_PID & 0xFF) as u8;                 // PCR_PID lo (0x00)
        section[10] = 0xF0; // program_info_length hi
        section[11] = 0x00; // program_info_length lo
        // Elementary Stream:
        section[12] = STREAM_TYPE_H264; // 0x1B
        section[13] = 0xE0 | ((VIDEO_PID >> 8) & 0x1F) as u8; // elementary_PID hi (0xE1)
        section[14] = (VIDEO_PID & 0xFF) as u8;                 // elementary_PID lo (0x00)
        section[15] = 0xF0; // ES_info_length hi
        section[16] = 0x00; // ES_info_length lo

        let crc = mpeg2_crc32(&section[0..17]);
        section[17..21].copy_from_slice(&crc.to_be_bytes());

        pkt[5..26].copy_from_slice(&section);
        // Bytes 26..188 remain 0xFF padding
        pkt
    }

    /// Encodes a 14-byte PES header with 90 kHz PTS timestamp for video stream 0xE0.
    #[inline]
    pub fn build_pes_header(nalu_len: usize, pts_90khz: u64) -> [u8; 14] {
        let mut header = [0u8; 14];
        // Packet start code prefix (0x000001)
        header[0] = 0x00;
        header[1] = 0x00;
        header[2] = 0x01;
        // Stream ID (0xE0 for video stream 0)
        header[3] = PES_STREAM_ID_VIDEO;

        // PES packet length: 16-bit.
        // For video elementary stream in TS, length can be (nalu_len + 8) if <= 0xFFFF,
        // or 0 if unbounded / larger than 65535 bytes (ISO/IEC 13818-1).
        let pes_len = if nalu_len + 8 <= 0xFFFF {
            (nalu_len + 8) as u16
        } else {
            0
        };
        header[4..6].copy_from_slice(&pes_len.to_be_bytes());

        // PES flags:
        // '10' marker bits, data_alignment_indicator=1 -> 0x84
        header[6] = 0x84;
        // PTS_DTS_flags = 10 (PTS present), others 0 -> 0x80
        header[7] = 0x80;
        // PES header data length = 5 (PTS is 5 bytes)
        header[8] = 0x05;

        // PTS (33 bits in 5 bytes with marker bits)
        let pts = pts_90khz & 0x1_FFFF_FFFF;
        header[9] = 0x21 | ((((pts >> 30) & 0x07) as u8) << 1);
        header[10] = ((pts >> 22) & 0xFF) as u8;
        header[11] = 0x01 | ((((pts >> 15) & 0x7F) as u8) << 1);
        header[12] = ((pts >> 7) & 0xFF) as u8;
        header[13] = 0x01 | (((pts & 0x7F) as u8) << 1);

        header
    }

    /// Encodes a 6-byte PCR (Program Clock Reference) field from a 90 kHz timestamp.
    #[inline]
    pub fn encode_pcr(pts_90khz: u64) -> [u8; 6] {
        let pcr_base = pts_90khz & 0x1_FFFF_FFFF; // 33 bits
        let pcr_ext = 0u16; // 9 bits

        let b0 = (pcr_base >> 25) as u8;
        let b1 = (pcr_base >> 17) as u8;
        let b2 = (pcr_base >> 9) as u8;
        let b3 = (pcr_base >> 1) as u8;
        // bit 0 of pcr_base in bit 7, 6 reserved bits '111111' (0x7E), bit 8 of pcr_ext in bit 0
        let b4 = (((pcr_base & 1) as u8) << 7) | 0x7E | (((pcr_ext >> 8) & 1) as u8);
        let b5 = (pcr_ext & 0xFF) as u8;

        [b0, b1, b2, b3, b4, b5]
    }

    /// Muxes raw H.264 NALUs into a sequence of 188-byte MPEG-TS packets.
    ///
    /// If `is_keyframe` is true:
    /// - Prepends PAT (PID 0) and PMT (PID 0x1000) packets.
    /// - Injects an Adaptation Field with PCR and Random Access Indicator into the first video packet.
    ///
    /// Automatically handles stuffing (`0xFF`) and Continuity Counters (`cc & 0x0F`).
    pub fn mux_h264_nalus(
        &mut self,
        nalus: &[u8],
        is_keyframe: bool,
        pts_90khz: u64,
    ) -> Vec<[u8; TS_PACKET_SIZE]> {
        if nalus.is_empty() {
            return Vec::new();
        }

        let pes_header = Self::build_pes_header(nalus.len(), pts_90khz);
        let total_pes_len = pes_header.len() + nalus.len();

        // Estimate packet count to pre-allocate
        let estimated_packets = (total_pes_len + 183) / 184 + if is_keyframe { 3 } else { 0 };
        let mut packets = Vec::with_capacity(estimated_packets);

        if is_keyframe {
            packets.push(self.create_pat());
            packets.push(self.create_pmt());
        }

        // Zero-copy segmented reader over pes_header and nalus
        let mut reader = PesReader {
            header: &pes_header,
            nalus,
            offset: 0,
        };

        let mut first_packet = true;
        while reader.remaining() > 0 {
            let mut pkt = [0xFFu8; TS_PACKET_SIZE];
            let pusi = first_packet;
            let need_pcr = first_packet && is_keyframe;

            // 4-byte TS Header
            pkt[0] = TS_SYNC_BYTE;
            let pusi_bit = if pusi { 0x40 } else { 0x00 };
            pkt[1] = pusi_bit | ((VIDEO_PID >> 8) & 0x1F) as u8;
            pkt[2] = (VIDEO_PID & 0xFF) as u8;

            let rem = reader.remaining();

            if need_pcr {
                // Keyframe first packet: inject Adaptation Field with PCR.
                // TS Header is 4 bytes. Space remaining = 184 bytes.
                // PCR Adaptation Field requires at least 8 bytes:
                // 1 byte length + 1 byte flags (0x60) + 6 bytes PCR = 8 bytes.
                // Maximum video payload in this packet is 184 - 8 = 176 bytes.
                let pcr_bytes = Self::encode_pcr(pts_90khz);
                let payload_size = rem.min(176);
                let af_size = 184 - payload_size; // af_size includes length byte
                let af_len = (af_size - 1) as u8; // value written to length byte

                // AFC = 11 (Adaptation field + payload)
                pkt[3] = 0x30 | (self.video_cc & 0x0F);
                self.video_cc = (self.video_cc + 1) & 0x0F;

                pkt[4] = af_len;
                pkt[5] = 0x60; // Random Access Indicator (0x40) | PCR flag (0x20)
                pkt[6..12].copy_from_slice(&pcr_bytes);
                // Bytes 12..(4 + af_size) are stuffing padding (already 0xFF)

                // Copy payload at the end of the 188-byte packet
                let payload_start = TS_PACKET_SIZE - payload_size;
                reader.read(&mut pkt[payload_start..TS_PACKET_SIZE]);
            } else if rem >= 184 {
                // Full payload, no adaptation field needed
                pkt[3] = 0x10 | (self.video_cc & 0x0F); // AFC = 01 (payload only)
                self.video_cc = (self.video_cc + 1) & 0x0F;

                reader.read(&mut pkt[4..TS_PACKET_SIZE]);
            } else {
                // Payload < 184 bytes: pad remaining space with Adaptation Field
                let payload_size = rem;
                let af_size = 184 - payload_size;
                let af_len = (af_size - 1) as u8;

                pkt[3] = 0x30 | (self.video_cc & 0x0F); // AFC = 11 (AF + payload)
                self.video_cc = (self.video_cc + 1) & 0x0F;

                pkt[4] = af_len;
                if af_len > 0 {
                    pkt[5] = 0x00; // flags = 0 (no PCR/RAI, pure stuffing)
                    // Bytes 6..(4 + af_size) are stuffing padding (already 0xFF)
                }

                let payload_start = TS_PACKET_SIZE - payload_size;
                reader.read(&mut pkt[payload_start..TS_PACKET_SIZE]);
            }

            packets.push(pkt);
            first_packet = false;
        }

        packets
    }

    /// Packs MPEG-TS packets into RTP datagrams (RFC 2250 / RFC 3550).
    ///
    /// Packs up to 7 TS packets (1316 bytes payload) per datagram.
    /// Each datagram is prefixed with a 12-byte RTP header (Payload Type = 33 MP2T).
    pub fn wrap_rtp(ts_packets: &[[u8; TS_PACKET_SIZE]], start_seq: u16, timestamp: u32) -> Vec<Vec<u8>> {
        Self::wrap_rtp_with_ssrc(ts_packets, start_seq, timestamp, 0x1234_5678)
    }

    /// Packs MPEG-TS packets into RTP datagrams with custom SSRC.
    pub fn wrap_rtp_with_ssrc(
        ts_packets: &[[u8; TS_PACKET_SIZE]],
        start_seq: u16,
        timestamp: u32,
        ssrc: u32,
    ) -> Vec<Vec<u8>> {
        if ts_packets.is_empty() {
            return Vec::new();
        }

        let num_rtp = (ts_packets.len() + TS_PACKETS_PER_RTP - 1) / TS_PACKETS_PER_RTP;
        let mut rtp_packets = Vec::with_capacity(num_rtp);

        let mut seq = start_seq;

        for (idx, chunk) in ts_packets.chunks(TS_PACKETS_PER_RTP).enumerate() {
            let is_last_chunk = idx + 1 == num_rtp;
            let mut rtp = Vec::with_capacity(RTP_HEADER_SIZE + chunk.len() * TS_PACKET_SIZE);

            // 12-byte RTP Header:
            // Byte 0: V=2 (0b10), P=0, X=0, CC=0 -> 0x80
            rtp.push(0x80);
            // Byte 1: M bit (0x80 on last chunk of AU per WFD Spec 5.3.3) | PT=33 (MP2T) -> 0x21
            let marker_byte = if is_last_chunk {
                0x80 | RTP_PAYLOAD_TYPE_MP2T
            } else {
                RTP_PAYLOAD_TYPE_MP2T
            };
            rtp.push(marker_byte);
            // Bytes 2..3: Sequence Number (big endian)
            rtp.extend_from_slice(&seq.to_be_bytes());
            // Bytes 4..7: Timestamp (90 kHz, big endian)
            rtp.extend_from_slice(&timestamp.to_be_bytes());
            // Bytes 8..11: SSRC (big endian)
            rtp.extend_from_slice(&ssrc.to_be_bytes());

            // Payload: 1 to 7 MPEG-TS packets (188 bytes each)
            for ts_pkt in chunk {
                rtp.extend_from_slice(ts_pkt);
            }

            rtp_packets.push(rtp);
            seq = seq.wrapping_add(1);
        }

        rtp_packets
    }

    /// Statefully wraps TS packets into RTP datagrams, advancing the internal sequence counter.
    pub fn wrap_rtp_auto(&mut self, ts_packets: &[[u8; TS_PACKET_SIZE]], timestamp: u32) -> Vec<Vec<u8>> {
        let pkts = Self::wrap_rtp_with_ssrc(ts_packets, self.rtp_seq, timestamp, self.ssrc);
        self.rtp_seq = self.rtp_seq.wrapping_add(pkts.len() as u16);
        pkts
    }
}

/// Helper struct for streaming bytes sequentially across header and payload buffers without heap allocation.
struct PesReader<'a> {
    header: &'a [u8],
    nalus: &'a [u8],
    offset: usize,
}

impl<'a> PesReader<'a> {
    #[inline]
    fn remaining(&self) -> usize {
        (self.header.len() + self.nalus.len()).saturating_sub(self.offset)
    }

    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> usize {
        let mut written = 0;
        while written < buf.len() && self.remaining() > 0 {
            if self.offset < self.header.len() {
                let avail = self.header.len() - self.offset;
                let to_copy = avail.min(buf.len() - written);
                buf[written..written + to_copy].copy_from_slice(&self.header[self.offset..self.offset + to_copy]);
                self.offset += to_copy;
                written += to_copy;
            } else {
                let nalu_offset = self.offset - self.header.len();
                let avail = self.nalus.len() - nalu_offset;
                let to_copy = avail.min(buf.len() - written);
                buf[written..written + to_copy].copy_from_slice(&self.nalus[nalu_offset..nalu_offset + to_copy]);
                self.offset += to_copy;
                written += to_copy;
            }
        }
        written
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ts_packet_sync_byte() {
        let mut muxer = MpegTsMuxer::new();
        let dummy_nalu = vec![0x00, 0x00, 0x00, 0x01, 0x65, 0x88, 0x84, 0x00];
        let packets = muxer.mux_h264_nalus(&dummy_nalu, true, 90000);
        assert!(!packets.is_empty());
        for pkt in &packets {
            assert_eq!(pkt[0], 0x47, "MPEG-TS packet must begin with sync byte 0x47");
            assert_eq!(pkt.len(), 188, "MPEG-TS packet must be exactly 188 bytes");
        }
    }

    #[test]
    fn test_rtp_wrapping_chunking() {
        let dummy_ts = [[0x47u8; 188]; 14];
        let rtp_packets = MpegTsMuxer::wrap_rtp(&dummy_ts, 1, 90000);
        // 14 TS packets should be split into 2 RTP packets (7 TS packets each = 1316 bytes + 12-byte header)
        assert_eq!(rtp_packets.len(), 2);
        assert_eq!(rtp_packets[0].len(), 12 + 7 * 188);
        assert_eq!(rtp_packets[0][1] & 0x7F, 33, "RTP Payload Type must be 33 (MP2T)");
        assert_eq!(rtp_packets[0][1] & 0x80, 0x00, "First RTP packet must have Marker bit M=0");
        assert_eq!(rtp_packets[1].len(), 12 + 7 * 188);
        assert_eq!(rtp_packets[1][1] & 0x80, 0x80, "Last RTP packet must have Marker bit M=1 (WFD Spec 5.3.3)");

        // Verify sequence numbers
        let seq0 = u16::from_be_bytes([rtp_packets[0][2], rtp_packets[0][3]]);
        let seq1 = u16::from_be_bytes([rtp_packets[1][2], rtp_packets[1][3]]);
        assert_eq!(seq0, 1);
        assert_eq!(seq1, 2);

        // Verify timestamps
        let ts0 = u32::from_be_bytes([rtp_packets[0][4], rtp_packets[0][5], rtp_packets[0][6], rtp_packets[0][7]]);
        assert_eq!(ts0, 90000);
    }

    #[test]
    fn test_mpeg2_crc32_calculation() {
        // Standard MPEG-2 test vector: feed data followed by its CRC, resulting CRC must be 0x00000000
        let data = b"123456789";
        let crc = mpeg2_crc32(data);
        assert_ne!(crc, 0);

        let mut data_with_crc = data.to_vec();
        data_with_crc.extend_from_slice(&crc.to_be_bytes());
        let check_crc = mpeg2_crc32(&data_with_crc);
        assert_eq!(check_crc, 0x0000_0000, "MPEG-2 CRC32 over data+crc must equal 0");
    }

    #[test]
    fn test_pat_packet_structure_and_crc() {
        let mut muxer = MpegTsMuxer::new();
        let pat = muxer.create_pat();

        assert_eq!(pat[0], TS_SYNC_BYTE);
        // PUSI=1, PID=0x0000
        let pid = (((pat[1] & 0x1F) as u16) << 8) | (pat[2] as u16);
        assert_eq!(pid, PAT_PID);
        assert_eq!(pat[1] & 0x40, 0x40, "PUSI must be 1 for PAT start");

        // Pointer field
        assert_eq!(pat[4], 0x00);

        // Section bytes: 5..21 (16 bytes)
        let section = &pat[5..21];
        assert_eq!(section[0], 0x00, "PAT table_id must be 0x00");
        assert_eq!(section[1] & 0xF0, 0xB0); // section_syntax=1, reserved=11
        assert_eq!(section[2], 0x0D); // section_length=13

        // Program 1 -> PMT PID 0x1000
        let prog_num = u16::from_be_bytes([section[8], section[9]]);
        assert_eq!(prog_num, 1);
        let pmt_pid = (((section[10] & 0x1F) as u16) << 8) | (section[11] as u16);
        assert_eq!(pmt_pid, PMT_PID);

        // Check CRC: running CRC32 over the entire 16-byte section (including CRC) must yield 0
        assert_eq!(mpeg2_crc32(section), 0x0000_0000, "PAT section CRC32 must validate to 0");
    }

    #[test]
    fn test_pmt_packet_structure_and_crc() {
        let mut muxer = MpegTsMuxer::new();
        let pmt = muxer.create_pmt();

        assert_eq!(pmt[0], TS_SYNC_BYTE);
        let pid = (((pmt[1] & 0x1F) as u16) << 8) | (pmt[2] as u16);
        assert_eq!(pid, PMT_PID);
        assert_eq!(pmt[1] & 0x40, 0x40, "PUSI must be 1 for PMT start");

        assert_eq!(pmt[4], 0x00); // Pointer field

        // Section bytes: 5..26 (21 bytes)
        let section = &pmt[5..26];
        assert_eq!(section[0], 0x02, "PMT table_id must be 0x02");
        assert_eq!(section[2], 0x12, "section_length must be 18");

        // PCR PID 0x0100
        let pcr_pid = (((section[8] & 0x1F) as u16) << 8) | (section[9] as u16);
        assert_eq!(pcr_pid, VIDEO_PID);

        // Stream type 0x1B (H.264), elementary PID 0x0100
        assert_eq!(section[12], STREAM_TYPE_H264);
        let elem_pid = (((section[13] & 0x1F) as u16) << 8) | (section[14] as u16);
        assert_eq!(elem_pid, VIDEO_PID);

        // Check CRC
        assert_eq!(mpeg2_crc32(section), 0x0000_0000, "PMT section CRC32 must validate to 0");
    }

    #[test]
    fn test_pes_header_pts_math() {
        let test_pts_values = [0u64, 90000, 1000000, 0x1_FFFF_FFFF];
        for &expected_pts in &test_pts_values {
            let header = MpegTsMuxer::build_pes_header(100, expected_pts);

            assert_eq!(&header[0..3], &[0x00, 0x00, 0x01], "PES start code prefix");
            assert_eq!(header[3], PES_STREAM_ID_VIDEO);
            assert_eq!(header[6], 0x84, "PES flags: data alignment");
            assert_eq!(header[7], 0x80, "PTS flag set");
            assert_eq!(header[8], 0x05, "PTS length = 5");

            // Decode 33-bit PTS back from 5 bytes
            let b0 = header[9];
            let b1 = header[10];
            let b2 = header[11];
            let b3 = header[12];
            let b4 = header[13];

            // Verify marker bits
            assert_eq!(b0 & 0xF1, 0x21, "PTS prefix 0010 and marker bit");
            assert_eq!(b2 & 0x01, 0x01, "PTS marker bit");
            assert_eq!(b4 & 0x01, 0x01, "PTS marker bit");

            let decoded_pts = (((b0 as u64 >> 1) & 0x07) << 30)
                | ((b1 as u64) << 22)
                | (((b2 as u64 >> 1) & 0x7F) << 15)
                | ((b3 as u64) << 7)
                | (((b4 as u64 >> 1) & 0x7F));

            assert_eq!(decoded_pts, expected_pts, "Decoded PTS must match encoded PTS");
        }
    }

    #[test]
    fn test_pcr_encoding_math() {
        let test_pcr_values = [0u64, 90000, 2700000, 0x1_FFFF_FFFF];
        for &expected_pcr in &test_pcr_values {
            let pcr_bytes = MpegTsMuxer::encode_pcr(expected_pcr);
            assert_eq!(pcr_bytes.len(), 6);

            let b0 = pcr_bytes[0];
            let b1 = pcr_bytes[1];
            let b2 = pcr_bytes[2];
            let b3 = pcr_bytes[3];
            let b4 = pcr_bytes[4];
            let b5 = pcr_bytes[5];

            // Verify 6 reserved bits in b4 (01111110)
            assert_eq!(b4 & 0x7E, 0x7E, "PCR reserved bits must be all 1s");

            let decoded_base = ((b0 as u64) << 25)
                | ((b1 as u64) << 17)
                | ((b2 as u64) << 9)
                | ((b3 as u64) << 1)
                | ((b4 as u64 >> 7) & 1);
            let decoded_ext = (((b4 as u16 & 1) << 8) | (b5 as u16)) & 0x1FF;

            assert_eq!(decoded_base, expected_pcr, "Decoded PCR base must match");
            assert_eq!(decoded_ext, 0, "PCR extension should be 0");
        }
    }

    #[test]
    fn test_continuity_counter_wrapping() {
        let mut muxer = MpegTsMuxer::new();
        let dummy = vec![0x00, 0x00, 0x00, 0x01, 0x41, 0x00];

        // Mux 20 non-keyframe packets
        let mut prev_cc: Option<u8> = None;
        for _ in 0..20 {
            let pkts = muxer.mux_h264_nalus(&dummy, false, 90000);
            assert_eq!(pkts.len(), 1); // dummy is small enough to fit in 1 packet
            let cc = pkts[0][3] & 0x0F;
            if let Some(prev) = prev_cc {
                assert_eq!(cc, (prev + 1) & 0x0F, "CC must increment modulo 16");
            }
            prev_cc = Some(cc);
        }
    }

    #[test]
    fn test_large_nalu_fragmentation() {
        let mut muxer = MpegTsMuxer::new();
        // 50 KB dummy frame
        let dummy = vec![0xAAu8; 50_000];
        let pkts = muxer.mux_h264_nalus(&dummy, true, 180000);

        // Must include PAT, PMT, and multiple video packets
        assert!(pkts.len() > 250);
        assert_eq!(pkts[0][1] & 0x1F, 0x00); // PAT
        assert_eq!(pkts[1][1] & 0x1F, 0x10); // PMT (PID 0x1000)

        // First video packet must have PUSI=1 and PCR adaptation field
        let vid0 = &pkts[2];
        assert_eq!(vid0[0], TS_SYNC_BYTE);
        assert_eq!(vid0[1] & 0x40, 0x40, "First video packet must have PUSI=1");
        assert_eq!(vid0[3] & 0x20, 0x20, "First keyframe packet must have Adaptation Field");
        assert_eq!(vid0[5] & 0x20, 0x20, "AF must have PCR flag set");

        // Subsequent video packets must have PUSI=0
        for (i, p) in pkts[3..].iter().enumerate() {
            assert_eq!(p[0], TS_SYNC_BYTE);
            assert_eq!(p[1] & 0x40, 0x00, "Subsequent packets must have PUSI=0 at index {}", i);
            assert_eq!(p.len(), TS_PACKET_SIZE);
        }
    }

    #[test]
    fn test_small_nalu_padding_and_stuffing() {
        let mut muxer = MpegTsMuxer::new();
        let tiny_nalu = [0x00, 0x00, 0x00, 0x01, 0x67]; // 5 bytes
        let pkts = muxer.mux_h264_nalus(&tiny_nalu, false, 90000);

        assert_eq!(pkts.len(), 1);
        let pkt = &pkts[0];
        assert_eq!(pkt.len(), TS_PACKET_SIZE);
        assert_eq!(pkt[0], TS_SYNC_BYTE);
        // AFC should be 11 (AF + payload)
        assert_eq!(pkt[3] & 0x30, 0x30);
        let af_len = pkt[4] as usize;
        // Total PES is 14 (header) + 5 (nalu) = 19 bytes.
        // AF size should be 184 - 19 = 165 bytes.
        // af_len byte value is 165 - 1 = 164.
        assert_eq!(af_len, 164);
        // Check that stuffing bytes in AF are 0xFF
        for &b in &pkt[6..4 + 1 + af_len] {
            assert_eq!(b, 0xFF, "Stuffing byte in AF must be 0xFF");
        }
    }
}
