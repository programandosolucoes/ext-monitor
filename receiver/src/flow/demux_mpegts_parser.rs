//! MPEG-2 Transport Stream (MPEG-TS) Demuxer Micro-Block
//!
//! Blueprint Reference: Blueprint 27 (Flow-Oriented Micro-Block Architecture)
//!
//! Single Responsibility:
//! Ingests standard 188-byte MPEG-TS packets (sync byte `0x47`), parses Program Association Tables (PAT)
//! and Program Map Tables (PMT), discovers elementary video stream PIDs (H.264 / HEVC), and extracts
//! Packetized Elementary Stream (PES) video payloads with Presentation Time Stamps (PTS).
//!
//! Key Invariants:
//! 1. Strictly synchronizes on `0x47` sync byte delimiters.
//! 2. Discards corrupted packets with Transport Error Indicator (TEI=1).
//! 3. Verifies 4-bit continuity counters to detect packet drops and avoid decoding artifacts.
//! 4. Extracts 33-bit MPEG-2 Presentation Time Stamps (PTS) for deterministic presentation pacing.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

/// Fixed size of a standard MPEG-TS packet
pub const TS_PACKET_SIZE: usize = 188;

/// MPEG-TS sync byte marker
pub const TS_SYNC_BYTE: u8 = 0x47;

/// Well-known PAT (Program Association Table) PID
pub const PAT_PID: u16 = 0x0000;

/// Stream types defined in ISO/IEC 13818-1 / ITU-T H.222.0
pub const STREAM_TYPE_H264: u8 = 0x1B;
pub const STREAM_TYPE_HEVC: u8 = 0x24;

/// Extracted Packetized Elementary Stream (PES) Video Payload
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PesPayload {
    /// PES stream ID (0xE0..=0xEF for video)
    pub stream_id: u8,
    /// Transport Stream Packet Identifier (PID)
    pub pid: u16,
    /// Extracted raw Elementary Stream (ES) video data
    pub data: Vec<u8>,
    /// Presentation Time Stamp (90 kHz clock ticks) if signaled in PES header
    pub pts: Option<u64>,
}

/// MPEG-TS Demuxer and Parser Micro-Block
pub struct MpegTsParser {
    pmt_pid: Option<u16>,
    video_pid: Option<u16>,
    video_stream_type: Option<u8>,

    current_stream_id: u8,
    current_pts: Option<u64>,
    current_pes_data: Vec<u8>,
    last_video_cc: Option<u8>,

    total_packets_parsed: u64,
    discontinuity_count: u64,
}

impl Default for MpegTsParser {
    /// Returns default configuration parameters.
    fn default() -> Self {
        Self::new()
    }
}

impl MpegTsParser {
    /// Creates a new MPEG-TS parser instance
    pub fn new() -> Self {
        Self {
            pmt_pid: None,
            video_pid: None,
            video_stream_type: None,

            current_stream_id: 0xE0,
            current_pts: None,
            current_pes_data: Vec::with_capacity(128 * 1024),
            last_video_cc: None,

            total_packets_parsed: 0,
            discontinuity_count: 0,
        }
    }

    /// Resets all internal tables and accumulators
    pub fn reset(&mut self) {
        self.pmt_pid = None;
        self.video_pid = None;
        self.video_stream_type = None;
        self.current_pts = None;
        self.current_pes_data.clear();
        self.last_video_cc = None;
    }

    /// Discovered PMT PID
    pub fn pmt_pid(&self) -> Option<u16> {
        self.pmt_pid
    }

    /// Discovered Video elementary PID
    pub fn video_pid(&self) -> Option<u16> {
        self.video_pid
    }

    /// Discovered Video Stream Type (e.g. 0x1B for H.264, 0x24 for HEVC)
    pub fn video_stream_type(&self) -> Option<u8> {
        self.video_stream_type
    }

    /// Total TS packets processed
    pub fn total_packets_parsed(&self) -> u64 {
        self.total_packets_parsed
    }

    /// Total continuity counter discontinuities detected
    pub fn discontinuity_count(&self) -> u64 {
        self.discontinuity_count
    }

    /// Parses a single 188-byte MPEG-TS packet.
    ///
    /// If the packet marks the completion of a PES video payload (e.g. PUSI=1 on the next PES packet),
    /// the accumulated `PesPayload` is returned.
    pub fn parse_ts_packet(&mut self, packet: &[u8; TS_PACKET_SIZE]) -> Option<PesPayload> {
        self.total_packets_parsed += 1;

        // Verify sync byte
        if packet[0] != TS_SYNC_BYTE {
            return None;
        }

        // Transport Error Indicator (TEI)
        let tei = (packet[1] & 0x80) != 0;
        if tei {
            return None;
        }

        let pusi = (packet[1] & 0x40) != 0;
        let pid = (((packet[1] & 0x1F) as u16) << 8) | (packet[2] as u16);

        // Adaptation field control and continuity counter
        let afc = (packet[3] >> 4) & 0x03;
        let cc = packet[3] & 0x0F;

        let payload_offset = match afc {
            0b01 => 4, // Payload only
            0b11 => {
                // Adaptation field followed by payload
                let adaptation_len = packet[4] as usize;
                let offset = 5 + adaptation_len;
                if offset > TS_PACKET_SIZE {
                    return None; // Corrupted adaptation field
                }
                offset
            }
            _ => return None, // 0b00 (reserved) or 0b10 (adaptation only, no payload)
        };

        let payload = &packet[payload_offset..TS_PACKET_SIZE];
        if payload.is_empty() {
            return None;
        }

        // Handle PAT (PID 0x0000)
        if pid == PAT_PID {
            self.parse_pat(payload, pusi);
            return None;
        }

        // Handle PMT (PID matches pmt_pid)
        if let Some(pmt_pid) = self.pmt_pid {
            if pid == pmt_pid {
                self.parse_pmt(payload, pusi);
                return None;
            }
        }

        // Handle Video PES (PID matches video_pid or default auto-detect on 0xE0..=0xEF)
        let mut completed_frame = None;

        if let Some(vpid) = self.video_pid {
            if pid == vpid {
                completed_frame = self.process_video_payload(payload, pusi, cc, pid);
            }
        } else if pusi && payload.len() >= 9 && payload[0] == 0 && payload[1] == 0 && payload[2] == 1 {
            let stream_id = payload[3];
            if (0xE0..=0xEF).contains(&stream_id) {
                // Auto-detect video PID if not yet announced by PMT
                self.video_pid = Some(pid);
                completed_frame = self.process_video_payload(payload, pusi, cc, pid);
            }
        }

        completed_frame
    }

    /// Parses an arbitrary buffer containing one or more 188-byte MPEG-TS packets,
    /// automatically resynchronizing on `0x47` sync bytes.
    pub fn parse_buffer(&mut self, buffer: &[u8]) -> Vec<PesPayload> {
        let mut results = Vec::new();
        let mut pos = 0;

        while pos + TS_PACKET_SIZE <= buffer.len() {
            if buffer[pos] != TS_SYNC_BYTE {
                if let Some(sync_pos) = buffer[pos..].iter().position(|&b| b == TS_SYNC_BYTE) {
                    pos += sync_pos;
                    if pos + TS_PACKET_SIZE > buffer.len() {
                        break;
                    }
                } else {
                    break;
                }
            }

            let mut pkt = [0u8; TS_PACKET_SIZE];
            pkt.copy_from_slice(&buffer[pos..pos + TS_PACKET_SIZE]);
            pos += TS_PACKET_SIZE;

            if let Some(pes) = self.parse_ts_packet(&pkt) {
                results.push(pes);
            }
        }

        results
    }

    /// Flushes any pending accumulated PES payload
    pub fn flush(&mut self) -> Option<PesPayload> {
        if !self.current_pes_data.is_empty() {
            let pid = self.video_pid.unwrap_or(0);
            let frame = PesPayload {
                stream_id: self.current_stream_id,
                pid,
                data: std::mem::take(&mut self.current_pes_data),
                pts: self.current_pts,
            };
            self.current_pes_data.reserve(128 * 1024);
            self.current_pts = None;
            Some(frame)
        } else {
            None
        }
    }

    /// Executes `process_video_payload` operational routine.
    fn process_video_payload(
        &mut self,
        payload: &[u8],
        pusi: bool,
        cc: u8,
        pid: u16,
    ) -> Option<PesPayload> {
        let mut completed = None;

        if pusi {
            // New PES packet starts!
            if !self.current_pes_data.is_empty() {
                completed = Some(PesPayload {
                    stream_id: self.current_stream_id,
                    pid,
                    data: std::mem::take(&mut self.current_pes_data),
                    pts: self.current_pts,
                });
                self.current_pes_data.reserve(128 * 1024);
                self.current_pts = None;
            }

            self.last_video_cc = Some(cc);

            // Parse PES Header
            if payload.len() >= 9 && payload[0] == 0 && payload[1] == 0 && payload[2] == 1 {
                self.current_stream_id = payload[3];
                let flags2 = payload[7];
                let pts_flag = (flags2 & 0x80) != 0;
                let header_data_len = payload[8] as usize;

                if pts_flag && payload.len() >= 14 {
                    // Extract 33-bit PTS
                    let p0 = (payload[9] as u64 & 0x0E) << 29;
                    let p1 = (payload[10] as u64 & 0xFF) << 22;
                    let p2 = (payload[11] as u64 & 0xFE) << 14;
                    let p3 = (payload[12] as u64 & 0xFF) << 7;
                    let p4 = (payload[13] as u64 & 0xFE) >> 1;
                    self.current_pts = Some(p0 | p1 | p2 | p3 | p4);
                }

                let es_offset = 9 + header_data_len;
                if es_offset < payload.len() {
                    self.current_pes_data.extend_from_slice(&payload[es_offset..]);
                }
            }
        } else {
            // Continuation packet
            if let Some(prev_cc) = self.last_video_cc {
                let expected_cc = (prev_cc + 1) & 0x0F;
                if cc != expected_cc && cc != prev_cc {
                    self.discontinuity_count += 1;
                }
            }
            self.last_video_cc = Some(cc);
            self.current_pes_data.extend_from_slice(payload);
        }

        completed
    }

    /// Executes `parse_pat` operational routine.
    fn parse_pat(&mut self, payload: &[u8], pusi: bool) {
        let mut offset = 0;
        if pusi {
            let pointer = payload[0] as usize;
            offset = 1 + pointer;
        }

        if offset + 8 > payload.len() {
            return;
        }

        let section = &payload[offset..];
        let table_id = section[0];
        if table_id != 0x00 {
            return;
        }

        let section_len = (((section[1] & 0x0F) as usize) << 8) | (section[2] as usize);
        if section.len() < section_len + 3 {
            return;
        }

        // Entries start at byte 8, each entry is 4 bytes, ending 4 bytes before section end (CRC32)
        let entries_end = section_len + 3 - 4;
        let mut i = 8;
        while i + 4 <= entries_end && i + 4 <= section.len() {
            let program_num = u16::from_be_bytes([section[i], section[i + 1]]);
            let pmt_pid = (((section[i + 2] & 0x1F) as u16) << 8) | (section[i + 3] as u16);
            if program_num != 0 {
                self.pmt_pid = Some(pmt_pid);
                break;
            }
            i += 4;
        }
    }

    /// Executes `parse_pmt` operational routine.
    fn parse_pmt(&mut self, payload: &[u8], pusi: bool) {
        let mut offset = 0;
        if pusi {
            let pointer = payload[0] as usize;
            offset = 1 + pointer;
        }

        if offset + 12 > payload.len() {
            return;
        }

        let section = &payload[offset..];
        let table_id = section[0];
        if table_id != 0x02 {
            return;
        }

        let section_len = (((section[1] & 0x0F) as usize) << 8) | (section[2] as usize);
        if section.len() < section_len + 3 {
            return;
        }

        let program_info_len = (((section[10] & 0x0F) as usize) << 8) | (section[11] as usize);
        let mut es_offset = 12 + program_info_len;
        let entries_end = section_len + 3 - 4;

        while es_offset + 5 <= entries_end && es_offset + 5 <= section.len() {
            let stream_type = section[es_offset];
            let elem_pid = (((section[es_offset + 1] & 0x1F) as u16) << 8) | (section[es_offset + 2] as u16);
            let es_info_len = (((section[es_offset + 3] & 0x0F) as usize) << 8) | (section[es_offset + 4] as usize);

            if stream_type == STREAM_TYPE_H264 || stream_type == STREAM_TYPE_HEVC {
                self.video_pid = Some(elem_pid);
                self.video_stream_type = Some(stream_type);
                break;
            }
            es_offset += 5 + es_info_len;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_byte_validation() {
        let mut parser = MpegTsParser::new();
        let invalid_packet = [0x00; 188];
        assert_eq!(parser.parse_ts_packet(&invalid_packet), None);
        assert_eq!(parser.total_packets_parsed(), 1);
    }

    #[test]
    fn test_pat_and_pmt_parsing() {
        let mut parser = MpegTsParser::new();

        // Synthesize PAT packet (PID 0x0000, Program 1 -> PMT PID 0x1000)
        let mut pat_packet = [0xFFu8; 188];
        pat_packet[0] = TS_SYNC_BYTE;
        pat_packet[1] = 0x40; // PUSI=1, PID=0
        pat_packet[2] = 0x00; // PID=0
        pat_packet[3] = 0x10; // AFC=01 (payload only), CC=0
        pat_packet[4] = 0x00; // Pointer field = 0

        // PAT Table Section
        let pat_section: &[u8] = &[
            0x00,       // table_id = 0 (PAT)
            0xB0, 0x0D, // section_syntax_indicator=1, section_length=13 (0x0D)
            0x00, 0x01, // transport_stream_id = 1
            0xC1,       // version=0, current_next=1
            0x00,       // section_number = 0
            0x00,       // last_section_number = 0
            0x00, 0x01, // program_number = 1
            0xE1, 0x00, // PMT PID = 0x0100 (256)
            0x00, 0x00, 0x00, 0x00, // CRC32 placeholder
        ];
        pat_packet[5..5 + pat_section.len()].copy_from_slice(pat_section);

        parser.parse_ts_packet(&pat_packet);
        assert_eq!(parser.pmt_pid(), Some(0x0100));

        // Synthesize PMT packet (PID 0x0100, Elementary Stream H.264 type 0x1B -> PID 0x0101)
        let mut pmt_packet = [0xFFu8; 188];
        pmt_packet[0] = TS_SYNC_BYTE;
        pmt_packet[1] = 0x41; // PUSI=1, PID high = 0x01
        pmt_packet[2] = 0x00; // PID low = 0x00 => 0x0100
        pmt_packet[3] = 0x10; // AFC=01, CC=0
        pmt_packet[4] = 0x00; // Pointer field = 0

        // PMT Table Section
        let pmt_section: &[u8] = &[
            0x02,       // table_id = 2 (PMT)
            0xB0, 0x12, // section_length = 18 (0x12)
            0x00, 0x01, // program_number = 1
            0xC1,       // version=0, current_next=1
            0x00,       // section_number = 0
            0x00,       // last_section_number = 0
            0xE1, 0x01, // PCR PID = 0x0101
            0xF0, 0x00, // program_info_length = 0
            0x1B,       // stream_type = 0x1B (H.264 Video)
            0xE1, 0x01, // elementary_PID = 0x0101
            0xF0, 0x00, // ES_info_length = 0
            0x00, 0x00, 0x00, 0x00, // CRC32
        ];
        pmt_packet[5..5 + pmt_section.len()].copy_from_slice(pmt_section);

        parser.parse_ts_packet(&pmt_packet);
        assert_eq!(parser.video_pid(), Some(0x0101));
        assert_eq!(parser.video_stream_type(), Some(STREAM_TYPE_H264));
    }

    #[test]
    fn test_pes_packet_extraction_with_pts() {
        let mut parser = MpegTsParser::new();

        // PES Header: prefix [0,0,1], stream_id=0xE0 (video), len=0 (unbounded), flags, PTS
        let mut pes_hdr = vec![
            0x00, 0x00, 0x01, // start prefix
            0xE0,             // stream_id = video 0
            0x00, 0x00,       // length = 0
            0x80,             // flags 1
            0x80,             // flags 2: PTS present (bit 7)
            0x05,             // PES header data length = 5 bytes
            // PTS encoding for pts = 90000 (1 second @ 90kHz):
            0x21, 0x00, 0x05, 0xBF, 0x21,
        ];
        // ES Video payload chunk 1:
        let video_data1 = [0x00, 0x00, 0x00, 0x01, 0x67, 0x42, 0x00];
        pes_hdr.extend_from_slice(&video_data1);

        // 1. Send first packet with PUSI=1 on video PID 0x0101 with adaptation stuffing (AFC=0b11)
        let mut pkt1 = [0xFFu8; 188];
        pkt1[0] = TS_SYNC_BYTE;
        pkt1[1] = 0x41; // PUSI=1, PID=0x0101
        pkt1[2] = 0x01;
        pkt1[3] = 0x30; // AFC=11 (adaptation + payload), CC=0
        let adapt_len1 = 188 - 5 - pes_hdr.len();
        pkt1[4] = adapt_len1 as u8;
        pkt1[5 + adapt_len1..188].copy_from_slice(&pes_hdr);

        // First packet initiates PES accumulation, should return None (not completed yet)
        assert_eq!(parser.parse_ts_packet(&pkt1), None);

        // 2. Send second packet with PUSI=0 (continuation) with adaptation stuffing (AFC=0b11)
        let video_data2 = [0x68, 0xCE, 0x38, 0x80];
        let mut pkt2 = [0xFFu8; 188];
        pkt2[0] = TS_SYNC_BYTE;
        pkt2[1] = 0x01; // PUSI=0, PID=0x0101
        pkt2[2] = 0x01;
        pkt2[3] = 0x31; // AFC=11, CC=1 (expected CC)
        let adapt_len2 = 188 - 5 - video_data2.len();
        pkt2[4] = adapt_len2 as u8;
        pkt2[5 + adapt_len2..188].copy_from_slice(&video_data2);

        assert_eq!(parser.parse_ts_packet(&pkt2), None);

        // 3. Flush to verify completed PES payload
        let pes = parser.flush().expect("should return accumulated PES");
        assert_eq!(pes.stream_id, 0xE0);
        assert_eq!(pes.pid, 0x0101);
        assert!(pes.pts.is_some());

        // Verify video payload matches chunks 1 and 2
        assert_eq!(pes.data.len(), video_data1.len() + video_data2.len());
        assert_eq!(&pes.data[0..video_data1.len()], &video_data1);
        assert_eq!(&pes.data[video_data1.len()..], &video_data2);
    }
}
