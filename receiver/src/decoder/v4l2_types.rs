//! Linux V4L2 Memory-to-Memory (M2M) C FFI Types & Constants
//!
//! Provides ABI-compatible C structures and ioctl constants for `/dev/video10`
//! (bcm2835-codec hardware VideoCore IV video decoder) on 32-bit ARM (ARMv6/v7).
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

// ARM 32-bit ioctl constants (verified against Linux kernel videodev2.h)
pub const VIDIOC_ENUM_FMT: libc::c_ulong = 0xC0405602;
pub const VIDIOC_REQBUFS: libc::c_ulong = 0xC0145608;
pub const VIDIOC_QUERYBUF: libc::c_ulong = 0xC0445609;
pub const VIDIOC_QBUF: libc::c_ulong = 0xC044560F;
pub const VIDIOC_DQBUF: libc::c_ulong = 0xC0445611;
pub const VIDIOC_STREAMON: libc::c_ulong = 0x40045612;
pub const VIDIOC_STREAMOFF: libc::c_ulong = 0x40045613;
pub const VIDIOC_S_FMT: libc::c_ulong = 0xC0CC5605;
pub const VIDIOC_EXPBUF: libc::c_ulong = 0xC0405610;
#[allow(dead_code)]
pub const VIDIOC_G_FMT: libc::c_ulong = 0xC0CC5604;

pub const V4L2_BUF_TYPE_VIDEO_CAPTURE_MPLANE: u32 = 9;  // Decoded raw output frames (RGB565)
pub const V4L2_BUF_TYPE_VIDEO_OUTPUT_MPLANE: u32 = 10; // Input compressed stream (H.264)
pub const V4L2_MEMORY_MMAP: u32 = 1;

pub const V4L2_BUF_FLAG_KEYFRAME: u32 = 0x00000008;
pub const V4L2_BUF_FLAG_TIMESTAMP_MONOTONIC: u32 = 0x00002000;

pub const V4L2_PIX_FMT_H264: u32 = 0x34363248; // 'H264'
pub const V4L2_PIX_FMT_RGB565: u32 = 0x50424752; // 'RGBP' (RGB565 Little Endian)
pub const V4L2_PIX_FMT_YUV420: u32 = 0x32315559; // 'YU12' (Planar YUV 4:2:0)
pub const V4L2_PIX_FMT_NV12: u32 = 0x3231564E;   // 'NV12' (Semi-Planar Y + UV)
pub const V4L2_PIX_FMT_YUV420M: u32 = 0x32314D59; // 'YM12'
pub const V4L2_PIX_FMT_NV12M: u32 = 0x32314D4E;   // 'NM12'

#[repr(C)]
#[derive(Default)]
pub struct V4l2FmtDesc {
    pub index: u32,
    pub buf_type: u32,
    pub flags: u32,
    pub description: [u8; 32],
    pub pixelformat: u32,
    pub reserved: [u32; 4],
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct V4l2PlanePixFormat {
    pub sizeimage: u32,
    pub bytesperline: u32,
    pub reserved: [u16; 6],
}

#[repr(C)]
pub struct V4l2PixFormatMplane {
    pub width: u32,
    pub height: u32,
    pub pixelformat: u32,
    pub field: u32,
    pub colorspace: u32,
    pub plane_fmt: [V4l2PlanePixFormat; 8],
    pub num_planes: u8,
    pub flags: u8,
    pub ycbcr_enc: u8,
    pub quantization: u8,
    pub xfer_func: u8,
    pub reserved: [u8; 7],
}

#[repr(C)]
pub struct V4l2Format {
    pub buf_type: u32,
    pub fmt: [u8; 200],
}

#[repr(C)]
#[derive(Default)]
pub struct V4l2RequestBuffers {
    pub count: u32,
    pub buf_type: u32,
    pub memory: u32,
    pub capabilities: u32,
    pub flags: u8,
    pub reserved: [u8; 3],
}

#[repr(C)]
#[derive(Default)]
pub struct V4l2Plane {
    pub bytesused: u32,
    pub length: u32,
    pub mem_offset: u32,
    pub data_offset: u32,
    pub reserved: [u32; 11],
}

/// Linux 32-bit ARM struct timeval ABI: two 32-bit integers (tv_sec: i32, tv_usec: i32) = 8 bytes
#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct V4l2Timeval {
    pub tv_sec: i32,
    pub tv_usec: i32,
}

/// Linux 32-bit ARM struct v4l2_buffer ABI: exactly 68 bytes (0x44)
#[repr(C)]
pub struct V4l2Buffer {
    pub index: u32,
    pub buf_type: u32,
    pub bytesused: u32,
    pub flags: u32,
    pub field: u32,
    pub timestamp: V4l2Timeval,
    pub timecode: [u8; 16],
    pub sequence: u32,
    pub memory: u32,
    pub planes_ptr: u32, // Pointer address (32-bit on ARM)
    pub length: u32,     // Number of planes
    pub reserved2: u32,
    pub request_fd: i32,
}

impl Default for V4l2Buffer {
    fn default() -> Self {
        Self {
            index: 0,
            buf_type: 0,
            bytesused: 0,
            flags: 0,
            field: 0,
            timestamp: V4l2Timeval::default(),
            timecode: [0u8; 16],
            sequence: 0,
            memory: 0,
            planes_ptr: 0,
            length: 0,
            reserved2: 0,
            request_fd: 0,
        }
    }
}

#[repr(C)]
pub struct V4l2ExportBuffer {
    pub buf_type: u32,
    pub index: u32,
    pub plane: u32,
    pub flags: u32,
    pub fd: i32,
    pub reserved: [u32; 11],
}

#[repr(C)]
pub struct V4l2Capability {
    pub driver: [u8; 16],
    pub card: [u8; 32],
    pub bus_info: [u8; 32],
    pub version: u32,
    pub capabilities: u32,
    pub device_caps: u32,
    pub reserved: [u32; 3],
}

// Compile-time ABI validation matching Linux kernel ioctl sizes
const _: () = assert!(std::mem::size_of::<V4l2Buffer>() == 68);
const _: () = assert!(std::mem::size_of::<V4l2Plane>() == 60);
const _: () = assert!(std::mem::size_of::<V4l2RequestBuffers>() == 20);
const _: () = assert!(std::mem::size_of::<V4l2Format>() == 204);
const _: () = assert!(std::mem::size_of::<V4l2FmtDesc>() == 64);
const _: () = assert!(std::mem::size_of::<V4l2Capability>() == 104);
const _: () = assert!(std::mem::size_of::<V4l2ExportBuffer>() == 64);
