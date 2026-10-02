//! Annex-B Access Unit Assembler and Parameter Cache Micro-Block
//!
//! Blueprint Reference: Blueprint 27 (Flow-Oriented Micro-Block Architecture)
//!
//! Single Responsibility:
//! Ingests arbitrary, fragmented byte stream chunks (from USB Bulk, pipes, or network streams),
//! normalizes all NAL unit delimiters to 4-byte Annex-B start codes (`0x00 0x00 0x00 0x01`),
//! continuously caches Sequence, Picture, and Video Parameter Sets (SPS, PPS, VPS),
//! and strictly demarcates Access Unit (frame) boundaries on IDR / Keyframe transitions.
//!
//! Why this matters:
//! Hardware VPU decoders (specifically Broadcom VideoCore IV / V4L2 M2M on Raspberry Pi Zero)
//! require complete, contiguous Access Units prepended with standard 4-byte start codes.
//! Missing parameter sets during stream reconnections cause decoder stalls; caching them
//! enables instant recovery and mid-stream client attach.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

/// Standard Annex-B 4-byte start code delimiter
pub const START_CODE_4BYTES: [u8; 4] = [0x00, 0x00, 0x00, 0x01];

/// Assembled Annex-B Access Unit ready for V4L2 M2M hardware decoding
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnexBFrame {
    /// Raw bytes of the complete frame, with 4-byte start codes for all NAL units
    pub data: Vec<u8>,
    /// Indicates whether this Access Unit is an instantaneous decoder refresh (IDR / Keyframe)
    pub is_keyframe: bool,
    /// Whether this Access Unit includes an SPS
    pub has_sps: bool,
    /// Whether this Access Unit includes a PPS
    pub has_pps: bool,
    /// Whether this Access Unit includes a VPS (HEVC)
    pub has_vps: bool,
}

/// NAL unit classification result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NalClassification {
    H264Sps,
    H264Pps,
    H264Idr,
    H264NonIdr,
    HevcVps,
    HevcSps,
    HevcPps,
    HevcIdr,
    HevcNonIdr,
    Other,
}

impl NalClassification {
    /// Classifies a NAL unit based on its header bytes
    pub fn classify(nal_body: &[u8]) -> Self {
        if nal_body.is_empty() {
            return NalClassification::Other;
        }

        // H.264: 1-byte header: forbidden_zero_bit (bit 7), nal_ref_idc (bits 5-6), nal_unit_type (bits 0-4)
        let h264_type = nal_body[0] & 0x1F;
        match h264_type {
            7 => NalClassification::H264Sps,
            8 => NalClassification::H264Pps,
            5 => NalClassification::H264Idr,
            1 => NalClassification::H264NonIdr,
            _ => {
                // Check if it matches HEVC 2-byte header:
                // Byte 0: forbidden (bit 7), nal_unit_type (bits 1-6), nuh_layer_id (bit 0)
                if nal_body.len() >= 2 {
                    let hevc_type = (nal_body[0] >> 1) & 0x3F;
                    match hevc_type {
                        32 => NalClassification::HevcVps,
                        33 => NalClassification::HevcSps,
                        34 => NalClassification::HevcPps,
                        16..=21 => NalClassification::HevcIdr,
                        0..=9 => NalClassification::HevcNonIdr,
                        _ => NalClassification::Other,
                    }
                } else {
                    NalClassification::Other
                }
            }
        }
    }

    pub fn is_keyframe(&self) -> bool {
        matches!(self, NalClassification::H264Idr | NalClassification::HevcIdr)
    }

    pub fn is_parameter_set(&self) -> bool {
        matches!(
            self,
            NalClassification::H264Sps
                | NalClassification::H264Pps
                | NalClassification::HevcVps
                | NalClassification::HevcSps
                | NalClassification::HevcPps
        )
    }
}

/// Annex-B Stream Assembler and Parameter Cache
pub struct AnnexBAssembler {
    accumulator: Vec<u8>,
    current_au: Vec<u8>,
    current_is_keyframe: bool,
    current_has_sps: bool,
    current_has_pps: bool,
    current_has_vps: bool,
    current_has_slice: bool,

    cached_sps: Option<Vec<u8>>,
    cached_pps: Option<Vec<u8>>,
    cached_vps: Option<Vec<u8>>,
}

impl Default for AnnexBAssembler {
    fn default() -> Self {
        Self::new()
    }
}

impl AnnexBAssembler {
    /// Creates a new assembler pre-allocated for video frames
    pub fn new() -> Self {
        Self {
            accumulator: Vec::with_capacity(256 * 1024),
            current_au: Vec::with_capacity(256 * 1024),
            current_is_keyframe: false,
            current_has_sps: false,
            current_has_pps: false,
            current_has_vps: false,
            current_has_slice: false,

            cached_sps: None,
            cached_pps: None,
            cached_vps: None,
        }
    }

    /// Resets all buffers and cached parameters
    pub fn reset(&mut self) {
        self.accumulator.clear();
        self.current_au.clear();
        self.current_is_keyframe = false;
        self.current_has_sps = false;
        self.current_has_pps = false;
        self.current_has_vps = false;
        self.current_has_slice = false;
    }

    /// Returns the cached Sequence Parameter Set (SPS), if any
    pub fn cached_sps(&self) -> Option<&[u8]> {
        self.cached_sps.as_deref()
    }

    /// Returns the cached Picture Parameter Set (PPS), if any
    pub fn cached_pps(&self) -> Option<&[u8]> {
        self.cached_pps.as_deref()
    }

    /// Returns the cached Video Parameter Set (VPS, for HEVC), if any
    pub fn cached_vps(&self) -> Option<&[u8]> {
        self.cached_vps.as_deref()
    }

    /// Finds the next Annex-B start code in `data` starting at index `start_idx`.
    /// Returns `Some((offset, prefix_length))` where prefix_length is 3 or 4.
    pub fn find_start_code(data: &[u8], start_idx: usize) -> Option<(usize, usize)> {
        if data.len() < start_idx + 3 {
            return None;
        }

        let mut i = start_idx;
        while i + 2 < data.len() {
            if data[i] == 0 && data[i + 1] == 0 {
                if data[i + 2] == 1 {
                    // Check if it's preceded by an extra zero (4-byte start code)
                    if i > 0 && data[i - 1] == 0 {
                        return Some((i - 1, 4));
                    } else {
                        return Some((i, 3));
                    }
                } else if i + 3 < data.len() && data[i + 2] == 0 && data[i + 3] == 1 {
                    return Some((i, 4));
                }
            }
            i += 1;
        }
        None
    }

    /// Ingests a raw byte chunk from the transport, parsing complete NAL units
    /// and emitting any complete Access Units.
    pub fn push_chunk(&mut self, chunk: &[u8]) -> Vec<AnnexBFrame> {
        if chunk.is_empty() {
            return Vec::new();
        }

        self.accumulator.extend_from_slice(chunk);
        let mut frames_out = Vec::new();

        self.extract_nal_units(&mut frames_out);
        frames_out
    }

    /// Flushes any pending Access Unit trapped in the buffer
    pub fn flush(&mut self) -> Option<AnnexBFrame> {
        // If there is trailing data in accumulator that forms a NAL unit, process it
        if let Some((start_offset, prefix_len)) = Self::find_start_code(&self.accumulator, 0) {
            let nal_body = self.accumulator[start_offset + prefix_len..].to_vec();
            if !nal_body.is_empty() {
                self.process_nal_body(&nal_body, &mut Vec::new());
            }
        }
        self.accumulator.clear();

        if self.current_has_slice && !self.current_au.is_empty() {
            let data = std::mem::take(&mut self.current_au);
            self.current_au.reserve(256 * 1024);

            let frame = AnnexBFrame {
                data,
                is_keyframe: self.current_is_keyframe,
                has_sps: self.current_has_sps,
                has_pps: self.current_has_pps,
                has_vps: self.current_has_vps,
            };

            self.current_is_keyframe = false;
            self.current_has_sps = false;
            self.current_has_pps = false;
            self.current_has_vps = false;
            self.current_has_slice = false;

            Some(frame)
        } else {
            None
        }
    }

    fn extract_nal_units(&mut self, frames_out: &mut Vec<AnnexBFrame>) {
        // Drop any garbage before the first start code
        if let Some((first_offset, _)) = Self::find_start_code(&self.accumulator, 0) {
            if first_offset > 0 {
                self.accumulator.drain(0..first_offset);
            }
        } else {
            // Keep at most 3 bytes in case a start code is split across chunks
            if self.accumulator.len() > 3 {
                let keep = self.accumulator.len() - 3;
                self.accumulator.drain(0..keep);
            }
            return;
        }

        loop {
            if self.accumulator.len() < 4 {
                break;
            }

            let (start_offset, prefix_len) = match Self::find_start_code(&self.accumulator, 0) {
                Some((0, plen)) => (0, plen),
                Some((off, _)) => {
                    self.accumulator.drain(0..off);
                    continue;
                }
                None => break,
            };

            // Search for the next start code to delineate the boundary of this NAL unit
            let next_start = Self::find_start_code(&self.accumulator, start_offset + prefix_len);

            match next_start {
                Some((next_offset, _)) => {
                    let nal_body = self.accumulator[start_offset + prefix_len..next_offset].to_vec();
                    self.accumulator.drain(0..next_offset);

                    if !nal_body.is_empty() {
                        self.process_nal_body(&nal_body, frames_out);
                    }
                }
                None => {
                    // Need more data to determine the end of this NAL unit
                    break;
                }
            }
        }
    }

    fn process_nal_body(&mut self, nal_body: &[u8], frames_out: &mut Vec<AnnexBFrame>) {
        let classification = NalClassification::classify(nal_body);

        // Update parameter set caches
        match classification {
            NalClassification::H264Sps | NalClassification::HevcSps => {
                self.cached_sps = Some(nal_body.to_vec());
                self.current_has_sps = true;
            }
            NalClassification::H264Pps | NalClassification::HevcPps => {
                self.cached_pps = Some(nal_body.to_vec());
                self.current_has_pps = true;
            }
            NalClassification::HevcVps => {
                self.cached_vps = Some(nal_body.to_vec());
                self.current_has_vps = true;
            }
            NalClassification::H264Idr | NalClassification::HevcIdr => {
                // If we already have video slices in the current AU, the arrival of an IDR
                // slice begins a new Access Unit. Emit the previous AU immediately!
                if self.current_has_slice {
                    let prev_frame = AnnexBFrame {
                        data: std::mem::take(&mut self.current_au),
                        is_keyframe: self.current_is_keyframe,
                        has_sps: self.current_has_sps,
                        has_pps: self.current_has_pps,
                        has_vps: self.current_has_vps,
                    };
                    self.current_au.reserve(256 * 1024);
                    frames_out.push(prev_frame);

                    self.current_is_keyframe = false;
                    self.current_has_sps = false;
                    self.current_has_pps = false;
                    self.current_has_vps = false;
                    self.current_has_slice = false;
                }

                self.current_is_keyframe = true;
                self.current_has_slice = true;
            }
            NalClassification::H264NonIdr | NalClassification::HevcNonIdr => {
                self.current_has_slice = true;
            }
            NalClassification::Other => {}
        }

        // Standardize: prepend 4-byte start code
        self.current_au.extend_from_slice(&START_CODE_4BYTES);
        self.current_au.extend_from_slice(nal_body);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_start_code() {
        // 3-byte start code
        let data3 = [0xAA, 0xBB, 0x00, 0x00, 0x01, 0x67, 0x42];
        let (off3, len3) = AnnexBAssembler::find_start_code(&data3, 0).expect("found 3-byte");
        assert_eq!(off3, 2);
        assert_eq!(len3, 3);

        // 4-byte start code
        let data4 = [0xAA, 0xBB, 0x00, 0x00, 0x00, 0x01, 0x67, 0x42];
        let (off4, len4) = AnnexBAssembler::find_start_code(&data4, 0).expect("found 4-byte");
        assert_eq!(off4, 2);
        assert_eq!(len4, 4);

        // No start code
        let empty = [0x00, 0x00, 0x02, 0x03];
        assert_eq!(AnnexBAssembler::find_start_code(&empty, 0), None);
    }

    #[test]
    fn test_normalize_start_codes_and_cache_parameters() {
        let mut assembler = AnnexBAssembler::new();

        // Feed an SPS with 3-byte start code [0,0,1]
        let sps_nal = [0x67, 0x42, 0x00, 0x1E, 0x9A];
        let mut chunk = Vec::new();
        chunk.extend_from_slice(&[0x00, 0x00, 0x01]);
        chunk.extend_from_slice(&sps_nal);

        // Feed a PPS with 4-byte start code [0,0,0,1]
        let pps_nal = [0x68, 0xCE, 0x38, 0x80];
        chunk.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
        chunk.extend_from_slice(&pps_nal);

        // Trailing start code so assembler knows PPS ended
        chunk.extend_from_slice(&[0x00, 0x00, 0x01]);

        assembler.push_chunk(&chunk);

        // Verify SPS and PPS were cached correctly
        assert_eq!(assembler.cached_sps(), Some(&sps_nal[..]));
        assert_eq!(assembler.cached_pps(), Some(&pps_nal[..]));
        assert_eq!(assembler.cached_vps(), None);
    }

    #[test]
    fn test_hevc_vps_sps_pps_cache() {
        let mut assembler = AnnexBAssembler::new();

        // HEVC VPS: Type 32 => (32 << 1) = 0x40
        let vps_nal = [0x40, 0x01, 0x0C, 0x01];
        // HEVC SPS: Type 33 => (33 << 1) = 0x42
        let sps_nal = [0x42, 0x01, 0x01];
        // HEVC PPS: Type 34 => (34 << 1) = 0x44
        let pps_nal = [0x44, 0x01, 0x02];

        let mut stream = Vec::new();
        stream.extend_from_slice(&[0x00, 0x00, 0x01]);
        stream.extend_from_slice(&vps_nal);
        stream.extend_from_slice(&[0x00, 0x00, 0x01]);
        stream.extend_from_slice(&sps_nal);
        stream.extend_from_slice(&[0x00, 0x00, 0x01]);
        stream.extend_from_slice(&pps_nal);
        stream.extend_from_slice(&[0x00, 0x00, 0x01]); // trailing delimiter

        assembler.push_chunk(&stream);

        assert_eq!(assembler.cached_vps(), Some(&vps_nal[..]));
        assert_eq!(assembler.cached_sps(), Some(&sps_nal[..]));
        assert_eq!(assembler.cached_pps(), Some(&pps_nal[..]));
    }

    #[test]
    fn test_idr_boundary_detection() {
        let mut assembler = AnnexBAssembler::new();

        // Frame 1: Non-IDR Slice (H.264 NAL 1)
        let slice1 = [0x41, 0x9A, 0x01, 0x02];
        let mut stream = Vec::new();
        stream.extend_from_slice(&[0x00, 0x00, 0x01]);
        stream.extend_from_slice(&slice1);

        // Frame 2: IDR Slice (H.264 NAL 5)
        let idr_slice = [0x65, 0x88, 0x84, 0x10];
        stream.extend_from_slice(&[0x00, 0x00, 0x01]);
        stream.extend_from_slice(&idr_slice);

        // Trailing start code
        stream.extend_from_slice(&[0x00, 0x00, 0x01]);

        let frames = assembler.push_chunk(&stream);
        // Frame 1 was emitted upon encounter of IDR slice
        assert_eq!(frames.len(), 1);
        assert!(!frames[0].is_keyframe);
        // Verify 4-byte start code was inserted
        assert_eq!(&frames[0].data[0..4], &[0, 0, 0, 1]);
        assert_eq!(&frames[0].data[4..], &slice1);

        // Flush frame 2 (the IDR frame)
        let frame2 = assembler.flush().expect("frame 2 should flush");
        assert!(frame2.is_keyframe);
        assert_eq!(&frame2.data[0..4], &[0, 0, 0, 1]);
        assert_eq!(&frame2.data[4..], &idr_slice);
    }

    #[test]
    fn test_split_across_chunks() {
        let mut assembler = AnnexBAssembler::new();

        let nal = [0x65, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06];
        // Split chunk 1: half of start code + half of NAL
        let chunk1 = [0x00, 0x00];
        let chunk2 = [0x01, 0x65, 0x01, 0x02];
        let chunk3 = [0x03, 0x04, 0x05, 0x06, 0x00, 0x00, 0x01]; // ends with next start code

        assert!(assembler.push_chunk(&chunk1).is_empty());
        assert!(assembler.push_chunk(&chunk2).is_empty());
        assembler.push_chunk(&chunk3);

        let frame = assembler.flush().expect("flushed frame");
        assert!(frame.is_keyframe);
        assert_eq!(&frame.data[0..4], &[0, 0, 0, 1]);
        assert_eq!(&frame.data[4..], &nal);
    }
}
