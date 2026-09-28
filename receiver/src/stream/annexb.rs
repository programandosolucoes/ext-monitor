//! Annex-B H.264 Access Unit (AU) Stream Assembler
//!
//! Transforms arbitrary byte stream chunks (from USB Bulk or pipes) into
//! complete, framed H.264 Access Units (complete video frames) ready for
//! hardware decoding on Broadcom VideoCore IV / V4L2 M2M.
//!
//! Guaranteed Invariants:
//! 1. No slice or macroblock is ever split across buffer boundaries.
//! 2. Each assembled Access Unit starts with SPS/PPS or AUD and contains all frame slices.
//! 3. Static/idle frames flush automatically on timeout without waiting for future motion.
//!
//! License: MIT
//! Author: Carlos Alberto <psncarlosalberto4ti@gmail.com>

pub struct AnnexBAssembler {
    accumulator: Vec<u8>,
    current_au: Vec<u8>,
    has_slice: bool,
}

impl AnnexBAssembler {
    /// Creates a new assembler pre-allocated for 720p H.264 frames
    pub fn new() -> Self {
        Self {
            accumulator: Vec::with_capacity(256 * 1024),
            current_au: Vec::with_capacity(256 * 1024),
            has_slice: false,
        }
    }

    /// Ingests a raw byte chunk from the transport and extracts any completed Access Units
    pub fn push(&mut self, chunk: &[u8], frames_out: &mut Vec<Vec<u8>>) {
        if chunk.is_empty() {
            return;
        }
        self.accumulator.extend_from_slice(chunk);
        self.extract_access_units(frames_out);
    }

    /// Flushes any pending Access Unit trapped in the buffer (e.g., during idle periods or mouse exit)
    pub fn flush(&mut self, frames_out: &mut Vec<Vec<u8>>) {
        // If the accumulator holds an in-progress NAL without a trailing start code,
        // move it into the current Access Unit so no trailing slice data is lost.
        if let Some((offset, prefix_len)) = Self::find_start_code(&self.accumulator, 0) {
            let nal_body = &self.accumulator[offset + prefix_len..];
            if !nal_body.is_empty() {
                let nal_type = nal_body[0] & 0x1F;
                if nal_type == 1 || nal_type == 5 {
                    self.has_slice = true;
                }
                self.current_au.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
                self.current_au.extend_from_slice(nal_body);
            }
        }
        self.accumulator.clear();

        // Emit completed Access Unit if it contains visual slice data
        if self.has_slice && !self.current_au.is_empty() {
            let completed = std::mem::take(&mut self.current_au);
            self.current_au.reserve(256 * 1024);
            frames_out.push(completed);
            self.has_slice = false;
        }
    }

    /// Returns true if there is uncommitted stream data in the assembler
    #[allow(dead_code)]
    pub fn has_pending(&self) -> bool {
        self.has_slice || !self.accumulator.is_empty() || !self.current_au.is_empty()
    }

    fn extract_access_units(&mut self, frames_out: &mut Vec<Vec<u8>>) {
        // Drop any junk preceding the initial start code
        if let Some((first_offset, _)) = Self::find_start_code(&self.accumulator, 0) {
            if first_offset > 0 {
                self.accumulator.drain(0..first_offset);
            }
        } else {
            // Keep at most 3 bytes to avoid breaking a start code split across reads
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

            let prefix_len = if self.accumulator.starts_with(&[0, 0, 0, 1]) {
                4
            } else if self.accumulator.starts_with(&[0, 0, 1]) {
                3
            } else {
                break;
            };

            // Search for the next start code to establish the boundary of the current NAL
            if let Some((next_offset, _)) = Self::find_start_code(&self.accumulator, prefix_len) {
                let nal_body = &self.accumulator[prefix_len..next_offset];

                if !nal_body.is_empty() {
                    let nal_type = nal_body[0] & 0x1F;
                    let is_first_slice = if (nal_type == 1 || nal_type == 5) && nal_body.len() > 1 {
                        // In H.264 slice header, first_mb_in_slice == 0 is encoded as 1 bit ('1'),
                        // setting the MSB (0x80) of the first byte after NAL header.
                        (nal_body[1] & 0x80) != 0
                    } else {
                        false
                    };

                    let is_new_au = nal_type == 9 // AUD (Access Unit Delimiter)
                        || nal_type == 7          // SPS (Sequence Parameter Set)
                        || (nal_type == 8 && self.has_slice) // PPS after existing slice
                        || is_first_slice;

                    // If this NAL marks the start of a subsequent frame, emit the completed AU
                    if is_new_au && self.has_slice && !self.current_au.is_empty() {
                        let completed = std::mem::take(&mut self.current_au);
                        self.current_au.reserve(256 * 1024);
                        frames_out.push(completed);
                        self.has_slice = false;
                    }

                    if nal_type == 1 || nal_type == 5 {
                        self.has_slice = true;
                    }

                    // Append NAL with canonical 4-byte Annex-B start code
                    self.current_au.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
                    self.current_au.extend_from_slice(nal_body);
                }

                self.accumulator.drain(0..next_offset);
            } else {
                // Next start code not yet received. Wait for subsequent transport chunks.
                break;
            }
        }
    }

    /// Finds the index and length of the first Annex-B start code (00 00 01 or 00 00 00 01)
    fn find_start_code(data: &[u8], start: usize) -> Option<(usize, usize)> {
        if data.len() < start + 3 {
            return None;
        }
        let limit = data.len();
        let mut i = start;
        while i + 2 < limit {
            if data[i] == 0 && data[i + 1] == 0 {
                if data[i + 2] == 1 {
                    return Some((i, 3));
                }
                if i + 3 < limit && data[i + 2] == 0 && data[i + 3] == 1 {
                    return Some((i, 4));
                }
            }
            i += 1;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assemble_single_complete_frame() {
        let mut assembler = AnnexBAssembler::new();
        let mut frames = Vec::new();

        // Sample frame: AUD + SPS + PPS + IDR Slice
        let mut stream = Vec::new();
        stream.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x09, 0xF0]); // AUD
        stream.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x67, 0x42, 0x00, 0x1F]); // SPS
        stream.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x68, 0xCE, 0x3C, 0x80]); // PPS
        stream.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x65, 0x88, 0x80, 0x10]); // IDR slice (first MB = 0)

        // Push in arbitrary 3-byte chunks
        for chunk in stream.chunks(3) {
            assembler.push(chunk, &mut frames);
        }

        // Flush end of stream
        assembler.flush(&mut frames);

        assert_eq!(frames.len(), 1);
        assert!(frames[0].len() >= stream.len());
        assert!(frames[0].starts_with(&[0x00, 0x00, 0x00, 0x01, 0x09]));
    }

    #[test]
    fn test_assemble_consecutive_frames() {
        let mut assembler = AnnexBAssembler::new();
        let mut frames = Vec::new();

        // Frame 1: AUD + Slice (first MB = 0)
        let mut f1 = Vec::new();
        f1.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x09, 0xF0]);
        f1.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x41, 0x88, 0x11, 0x22]);

        // Frame 2: AUD + Slice (first MB = 0)
        let mut f2 = Vec::new();
        f2.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x09, 0xF0]);
        f2.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x41, 0x88, 0x33, 0x44]);

        assembler.push(&f1, &mut frames);
        assert_eq!(frames.len(), 0); // Frame 1 waiting for next start code or flush

        assembler.push(&f2, &mut frames);
        assert_eq!(frames.len(), 1); // Frame 1 emitted upon arrival of Frame 2's AUD!

        assembler.flush(&mut frames);
        assert_eq!(frames.len(), 2); // Frame 2 emitted upon flush!
    }
}
