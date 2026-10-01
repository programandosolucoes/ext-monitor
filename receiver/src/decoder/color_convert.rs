//! High-Performance Color Space Conversion Subsystem
//!
//! Provides ultra-fast, branchless SIMD-friendly color space conversion from
//! hardware decoder YUV formats (YUV420 Planar / NV12 Semi-Planar) to HDMI
//! Framebuffer RGB565 Little-Endian format.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

/// Converts a planar YUV420 (I420 / YU12) buffer to RGB565 in-place into `out`.
///
/// Designed with 2x2 chroma subsampling locality: U and V values and chroma
/// deltas are calculated once and shared across all four pixels in each quad.
#[inline]
pub fn yuv420_to_rgb565(yuv: &[u8], out: &mut [u8], width: usize, height: usize) {
    let y_size = width * height;
    let uv_size = (width / 2) * (height / 2);
    if yuv.len() < y_size + uv_size * 2 || out.len() < y_size * 2 {
        return;
    }

    let y_plane = &yuv[0..y_size];
    let u_plane = &yuv[y_size..y_size + uv_size];
    let v_plane = &yuv[y_size + uv_size..y_size + uv_size * 2];

    let half_w = width / 2;

    for y in (0..height).step_by(2) {
        let y0_row = y * width;
        let y1_row = (y + 1) * width;
        let uv_row = (y / 2) * half_w;

        for x in (0..width).step_by(2) {
            let uv_idx = uv_row + (x / 2);
            let u = u_plane[uv_idx] as i32 - 128;
            let v = v_plane[uv_idx] as i32 - 128;

            let c_v_r = (359 * v) >> 8;
            let c_uv_g = (-88 * u - 183 * v) >> 8;
            let c_u_b = (454 * u) >> 8;

            // Pixel (x, y)
            let y00 = y_plane[y0_row + x] as i32;
            let r00 = (y00 + c_v_r).clamp(0, 255) as u16;
            let g00 = (y00 + c_uv_g).clamp(0, 255) as u16;
            let b00 = (y00 + c_u_b).clamp(0, 255) as u16;
            let p00 = ((r00 >> 3) << 11) | ((g00 >> 2) << 5) | (b00 >> 3);

            // Pixel (x + 1, y)
            let y01 = y_plane[y0_row + x + 1] as i32;
            let r01 = (y01 + c_v_r).clamp(0, 255) as u16;
            let g01 = (y01 + c_uv_g).clamp(0, 255) as u16;
            let b01 = (y01 + c_u_b).clamp(0, 255) as u16;
            let p01 = ((r01 >> 3) << 11) | ((g01 >> 2) << 5) | (b01 >> 3);

            // Pixel (x, y + 1)
            let y10 = y_plane[y1_row + x] as i32;
            let r10 = (y10 + c_v_r).clamp(0, 255) as u16;
            let g10 = (y10 + c_uv_g).clamp(0, 255) as u16;
            let b10 = (y10 + c_u_b).clamp(0, 255) as u16;
            let p10 = ((r10 >> 3) << 11) | ((g10 >> 2) << 5) | (b10 >> 3);

            // Pixel (x + 1, y + 1)
            let y11 = y_plane[y1_row + x + 1] as i32;
            let r11 = (y11 + c_v_r).clamp(0, 255) as u16;
            let g11 = (y11 + c_uv_g).clamp(0, 255) as u16;
            let b11 = (y11 + c_u_b).clamp(0, 255) as u16;
            let p11 = ((r11 >> 3) << 11) | ((g11 >> 2) << 5) | (b11 >> 3);

            let out0_idx = (y0_row + x) * 2;
            let out1_idx = (y1_row + x) * 2;

            let b00_bytes = p00.to_le_bytes();
            let b01_bytes = p01.to_le_bytes();
            out[out0_idx] = b00_bytes[0];
            out[out0_idx + 1] = b00_bytes[1];
            out[out0_idx + 2] = b01_bytes[0];
            out[out0_idx + 3] = b01_bytes[1];

            let b10_bytes = p10.to_le_bytes();
            let b11_bytes = p11.to_le_bytes();
            out[out1_idx] = b10_bytes[0];
            out[out1_idx + 1] = b10_bytes[1];
            out[out1_idx + 2] = b11_bytes[0];
            out[out1_idx + 3] = b11_bytes[1];
        }
    }
}

/// Converts a semi-planar NV12 buffer (Y plane + interleaved UV) to RGB565 into `out`.
#[allow(dead_code)]
#[inline]
pub fn nv12_to_rgb565(nv12: &[u8], out: &mut [u8], width: usize, height: usize) {
    nv12_to_rgb565_strided(nv12, out, width, height, width, height);
}

/// Converts a strided semi-planar NV12 buffer (with buffer_height alignment) to RGB565 into `out`.
#[inline]
pub fn nv12_to_rgb565_strided(
    nv12: &[u8],
    out: &mut [u8],
    width: usize,
    height: usize,
    stride: usize,
    buffer_height: usize,
) {
    let y_stride = stride.max(width);
    let uv_stride = y_stride;
    let uv_offset = y_stride * buffer_height.max(height);
    let uv_needed = uv_offset + uv_stride * (height / 2);

    if nv12.len() < uv_needed || out.len() < width * height * 2 {
        return;
    }

    let y_plane = &nv12[0..y_stride * height];
    let uv_plane = &nv12[uv_offset..];

    for y in (0..height).step_by(2) {
        let y0_row = y * y_stride;
        let y1_row = (y + 1) * y_stride;
        let uv_row = (y / 2) * uv_stride;

        for x in (0..width).step_by(2) {
            let uv_idx = uv_row + x;
            let u = uv_plane[uv_idx] as i32 - 128;
            let v = uv_plane[uv_idx + 1] as i32 - 128;

            let c_v_r = (359 * v) >> 8;
            let c_uv_g = (-88 * u - 183 * v) >> 8;
            let c_u_b = (454 * u) >> 8;

            // Pixel (x, y)
            let y00 = y_plane[y0_row + x] as i32;
            let r00 = (y00 + c_v_r).clamp(0, 255) as u16;
            let g00 = (y00 + c_uv_g).clamp(0, 255) as u16;
            let b00 = (y00 + c_u_b).clamp(0, 255) as u16;
            let p00 = ((r00 >> 3) << 11) | ((g00 >> 2) << 5) | (b00 >> 3);

            // Pixel (x + 1, y)
            let y01 = y_plane[y0_row + x + 1] as i32;
            let r01 = (y01 + c_v_r).clamp(0, 255) as u16;
            let g01 = (y01 + c_uv_g).clamp(0, 255) as u16;
            let b01 = (y01 + c_u_b).clamp(0, 255) as u16;
            let p01 = ((r01 >> 3) << 11) | ((g01 >> 2) << 5) | (b01 >> 3);

            // Pixel (x, y + 1)
            let y10 = y_plane[y1_row + x] as i32;
            let r10 = (y10 + c_v_r).clamp(0, 255) as u16;
            let g10 = (y10 + c_uv_g).clamp(0, 255) as u16;
            let b10 = (y10 + c_u_b).clamp(0, 255) as u16;
            let p10 = ((r10 >> 3) << 11) | ((g10 >> 2) << 5) | (b10 >> 3);

            // Pixel (x + 1, y + 1)
            let y11 = y_plane[y1_row + x + 1] as i32;
            let r11 = (y11 + c_v_r).clamp(0, 255) as u16;
            let g11 = (y11 + c_uv_g).clamp(0, 255) as u16;
            let b11 = (y11 + c_u_b).clamp(0, 255) as u16;
            let p11 = ((r11 >> 3) << 11) | ((g11 >> 2) << 5) | (b11 >> 3);

            let out0_idx = (y * width + x) * 2;
            let out1_idx = ((y + 1) * width + x) * 2;

            let b00_bytes = p00.to_le_bytes();
            let b01_bytes = p01.to_le_bytes();
            out[out0_idx] = b00_bytes[0];
            out[out0_idx + 1] = b00_bytes[1];
            out[out0_idx + 2] = b01_bytes[0];
            out[out0_idx + 3] = b01_bytes[1];

            let b10_bytes = p10.to_le_bytes();
            let b11_bytes = p11.to_le_bytes();
            out[out1_idx] = b10_bytes[0];
            out[out1_idx + 1] = b10_bytes[1];
            out[out1_idx + 2] = b11_bytes[0];
            out[out1_idx + 3] = b11_bytes[1];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yuv420_black_conversion() {
        // Y = 0, U = 128, V = 128 (Black)
        let w = 4;
        let h = 4;
        let mut yuv = vec![0u8; w * h + (w / 2) * (h / 2) * 2];
        for i in (w * h)..yuv.len() {
            yuv[i] = 128;
        }

        let mut out = vec![0xFFu8; w * h * 2];
        yuv420_to_rgb565(&yuv, &mut out, w, h);

        for chunk in out.chunks(2) {
            let pixel = u16::from_le_bytes([chunk[0], chunk[1]]);
            assert_eq!(pixel, 0, "Black YUV must yield RGB565 0");
        }
    }

    #[test]
    fn test_nv12_white_conversion() {
        // Y = 255, U = 128, V = 128 (White)
        let w = 4;
        let h = 4;
        let mut nv12 = vec![255u8; w * h + w * (h / 2)];
        for i in (w * h)..nv12.len() {
            nv12[i] = 128;
        }

        let mut out = vec![0u8; w * h * 2];
        nv12_to_rgb565(&nv12, &mut out, w, h);

        for chunk in out.chunks(2) {
            let pixel = u16::from_le_bytes([chunk[0], chunk[1]]);
            assert_eq!(pixel, 0xFFFF, "White NV12 must yield RGB565 0xFFFF");
        }
    }
}
