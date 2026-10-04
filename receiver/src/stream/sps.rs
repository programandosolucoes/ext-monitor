//! H.264 Sequence Parameter Set (SPS) Parser in Pure Rust
//!
//! Extracts video dimensions (width x height) from H.264 SPS NAL units (RFC 6184 / ITU-T H.264)
//! with zero allocations. Used to dynamically configure hardware V4L2 M2M decoders to match
//! incoming streams (e.g. 1920x1080 from GNOME Network Displays / Windows Miracast vs 1280x720).
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

/// Represents Bitreader configuration and operational state.
struct BitReader<'a> {
    data: &'a [u8],
    bit_pos: usize,
}

impl<'a> BitReader<'a> {
    /// Constructs and initializes a new `new` instance with default or provided parameters.
    fn new(data: &'a [u8]) -> Self {
        Self { data, bit_pos: 0 }
    }

    /// Executes `read_bit` operational routine.
    fn read_bit(&mut self) -> Option<u32> {
        let byte_pos = self.bit_pos / 8;
        if byte_pos >= self.data.len() {
            return None;
        }
        let bit_idx = 7 - (self.bit_pos % 8);
        self.bit_pos += 1;
        Some(((self.data[byte_pos] >> bit_idx) & 1) as u32)
    }

    /// Executes `read_bits` operational routine.
    fn read_bits(&mut self, n: usize) -> Option<u32> {
        let mut val = 0u32;
        for _ in 0..n {
            val = (val << 1) | self.read_bit()?;
        }
        Some(val)
    }

    /// Executes `read_ue` operational routine.
    fn read_ue(&mut self) -> Option<u32> {
        let mut zeros = 0usize;
        while self.read_bit()? == 0 {
            zeros += 1;
            if zeros > 31 {
                return None;
            }
        }
        if zeros == 0 {
            return Some(0);
        }
        let info = self.read_bits(zeros)?;
        Some((1u32 << zeros) - 1 + info)
    }

    /// Executes `read_se` operational routine.
    fn read_se(&mut self) -> Option<i32> {
        let code = self.read_ue()?;
        if code == 0 {
            Some(0)
        } else if code % 2 == 1 {
            Some(((code + 1) / 2) as i32)
        } else {
            Some(-((code / 2) as i32))
        }
    }
}

/// Unescapes H.264 emulation prevention bytes (`0x00 0x00 0x03` -> `0x00 0x00`) into a fixed buffer
fn unescape_rbsp(src: &[u8], dst: &mut [u8]) -> usize {
    let mut s = 0;
    let mut d = 0;
    while s < src.len() && d < dst.len() {
        if s + 2 < src.len() && src[s] == 0 && src[s + 1] == 0 && src[s + 2] == 3 {
            dst[d] = 0;
            dst[d + 1] = 0;
            d += 2;
            s += 3;
        } else {
            dst[d] = src[s];
            d += 1;
            s += 1;
        }
    }
    d
}

/// Scans an Access Unit or H.264 byte buffer for an SPS NAL unit and parses width & height
pub fn parse_sps_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    if data.len() < 5 {
        return None;
    }

    // 1. Locate start code followed by SPS NAL (NAL type 7)
    let mut i = 0;
    let limit = data.len() - 4;
    let mut sps_offset = None;

    while i < limit {
        if data[i] == 0 && data[i + 1] == 0 {
            let (start_len, nal_idx) = if data[i + 2] == 1 {
                (3, i + 3)
            } else if i + 3 < data.len() && data[i + 2] == 0 && data[i + 3] == 1 {
                (4, i + 4)
            } else {
                i += 1;
                continue;
            };

            if (data[nal_idx] & 0x1F) == 7 {
                sps_offset = Some(nal_idx);
                break;
            }
            i += start_len;
        } else {
            i += 1;
        }
    }

    let nal_start = sps_offset?;
    // Look for next start code to find end of SPS
    let mut nal_end = data.len();
    for j in nal_start + 1..data.len() - 3 {
        if data[j] == 0 && data[j + 1] == 0 && (data[j + 2] == 1 || (data[j + 2] == 0 && data[j + 3] == 1)) {
            nal_end = j;
            break;
        }
    }

    let sps_raw = &data[nal_start..nal_end];
    if sps_raw.len() < 4 {
        return None;
    }

    // Skip NAL header (1 byte), unescape RBSP
    let mut rbsp = [0u8; 128];
    let rbsp_len = unescape_rbsp(&sps_raw[1..], &mut rbsp);
    if rbsp_len < 3 {
        return None;
    }

    let mut br = BitReader::new(&rbsp[..rbsp_len]);
    let profile_idc = br.read_bits(8)?;
    let _constraint_flags = br.read_bits(8)?;
    let _level_idc = br.read_bits(8)?;
    let _sps_id = br.read_ue()?;

    if matches!(profile_idc, 100 | 110 | 122 | 244 | 44 | 83 | 86 | 118 | 128) {
        let chroma_format_idc = br.read_ue()?;
        if chroma_format_idc == 3 {
            let _separate_colour_plane = br.read_bit()?;
        }
        let _bit_depth_luma_minus8 = br.read_ue()?;
        let _bit_depth_chroma_minus8 = br.read_ue()?;
        let _qpprime_y_zero = br.read_bit()?;
        let seq_scaling_matrix_present = br.read_bit()?;
        if seq_scaling_matrix_present != 0 {
            let count = if chroma_format_idc != 3 { 8 } else { 12 };
            for _ in 0..count {
                if br.read_bit()? != 0 {
                    // skip scaling list
                    let size = 16;
                    let mut last = 8;
                    let mut next = 8;
                    for _ in 0..size {
                        if next != 0 {
                            let delta = br.read_se()?;
                            next = (last + delta + 256) % 256;
                        }
                        last = if next == 0 { last } else { next };
                    }
                }
            }
        }
    }

    let _log2_max_frame_num_minus4 = br.read_ue()?;
    let pic_order_cnt_type = br.read_ue()?;
    if pic_order_cnt_type == 0 {
        let _log2_max_pic_order_cnt_lsb_minus4 = br.read_ue()?;
    } else if pic_order_cnt_type == 1 {
        let _delta_pic_order_always_zero_flag = br.read_bit()?;
        let _offset_for_non_ref_pic = br.read_se()?;
        let _offset_for_top_to_bottom_field = br.read_se()?;
        let num_ref_frames = br.read_ue()?;
        for _ in 0..num_ref_frames {
            let _offset = br.read_se()?;
        }
    }

    let _max_num_ref_frames = br.read_ue()?;
    let _gaps_in_frame_num = br.read_bit()?;
    let pic_width_in_mbs_minus1 = br.read_ue()?;
    let pic_height_in_map_units_minus1 = br.read_ue()?;
    let frame_mbs_only_flag = br.read_bit()?;

    if frame_mbs_only_flag == 0 {
        let _mb_adaptive_frame_field = br.read_bit()?;
    }
    let _direct_8x8_inference = br.read_bit()?;
    let frame_cropping_flag = br.read_bit()?;

    let (crop_l, crop_r, crop_t, crop_b) = if frame_cropping_flag != 0 {
        (
            br.read_ue()?,
            br.read_ue()?,
            br.read_ue()?,
            br.read_ue()?,
        )
    } else {
        (0, 0, 0, 0)
    };

    let mut width = (pic_width_in_mbs_minus1 + 1) * 16;
    let vert_mult = if frame_mbs_only_flag != 0 { 1 } else { 2 };
    let mut height = (pic_height_in_map_units_minus1 + 1) * 16 * vert_mult;

    width = width.saturating_sub((crop_l + crop_r) * 2);
    height = height.saturating_sub((crop_t + crop_b) * 2 * vert_mult);

    if width > 0 && height > 0 {
        Some((width, height))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_gnome_1080p_sps() {
        // Captured SPS from GNOME Network Displays 1080p High Profile
        let sps_au = [
            0x00, 0x00, 0x00, 0x01, 0x27, 0x64, 0x00, 0x28, 0xac, 0x56, 0x80, 0x78, 0x02,
            0x27, 0xe5, 0xc0, 0x44, 0x00, 0x00, 0x03, 0x00, 0x04, 0x00, 0x00, 0x03, 0x00,
            0xf0, 0x3c, 0x60, 0xc6, 0x58, 0x00, 0x00, 0x00, 0x01, 0x28,
        ];
        let dims = parse_sps_dimensions(&sps_au);
        assert_eq!(dims, Some((1920, 1080)));
    }
}
