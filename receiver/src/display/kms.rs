//! DRM/KMS plane scanout for decoded frames.
//!
//! Same path the frozen receiver used with `v4l2h264dec` + `kmssink
//! sync=false skip-vsync=true`: the VideoCore buffer is imported as a
//! DMA-BUF and placed on a KMS plane. No CPU color conversion.
//!
//! A frame that cannot be queued (EBUSY) is dropped. The two buffers
//! most recently shown stay out of the decoder until the next one
//! replaces them, so the scanout is not overwritten mid-frame.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use std::fs::OpenOptions;
use std::io;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::{AsRawFd, RawFd};

const DRM_IOCTL_SET_MASTER: libc::c_ulong = 0x641e;
const DRM_IOCTL_GEM_CLOSE: libc::c_ulong = 0x4008_6409;
const DRM_IOCTL_PRIME_FD_TO_HANDLE: libc::c_ulong = 0xc00c_642e;
const DRM_IOCTL_MODE_GETRESOURCES: libc::c_ulong = 0xc040_64a0;
const DRM_IOCTL_MODE_GETCRTC: libc::c_ulong = 0xc068_64a1;
const DRM_IOCTL_MODE_GETENCODER: libc::c_ulong = 0xc014_64a6;
const DRM_IOCTL_MODE_GETCONNECTOR: libc::c_ulong = 0xc050_64a7;
const DRM_IOCTL_MODE_RMFB: libc::c_ulong = 0xc004_64af;
const DRM_IOCTL_MODE_GETPLANERESOURCES: libc::c_ulong = 0xc010_64b5;
const DRM_IOCTL_MODE_GETPLANE: libc::c_ulong = 0xc020_64b6;
const DRM_IOCTL_MODE_SETPLANE: libc::c_ulong = 0xc030_64b7;
const DRM_IOCTL_MODE_ADDFB2: libc::c_ulong = 0xc068_64b8;
const DRM_IOCTL_SET_CLIENT_CAP: libc::c_ulong = 0x4010_640d;
const DRM_CLIENT_CAP_UNIVERSAL_PLANES: u64 = 2;

#[repr(C)]
struct DrmSetClientCap {
    capability: u64,
    value: u64,
}

const DRM_MODE_CONNECTED: u32 = 1;
const NV12: u32 = 0x3231_564e;
const YU12: u32 = 0x3231_5559;

#[repr(C)]
struct DrmModeCardRes {
    fb_id_ptr: u64,
    crtc_id_ptr: u64,
    connector_id_ptr: u64,
    encoder_id_ptr: u64,
    count_fbs: u32,
    count_crtcs: u32,
    count_connectors: u32,
    count_encoders: u32,
    min_width: u32,
    max_width: u32,
    min_height: u32,
    max_height: u32,
}

#[derive(Clone, Copy)]
#[repr(C)]
struct DrmModeModeinfo {
    clock: u32,
    hdisplay: u16,
    hsync_start: u16,
    hsync_end: u16,
    htotal: u16,
    hskew: u16,
    vdisplay: u16,
    vsync_start: u16,
    vsync_end: u16,
    vtotal: u16,
    vscan: u16,
    vrefresh: u32,
    flags: u32,
    ty: u32,
    name: [u8; 32],
}

#[repr(C)]
struct DrmModeCrtc {
    set_connectors_ptr: u64,
    count_connectors: u32,
    crtc_id: u32,
    fb_id: u32,
    x: u32,
    y: u32,
    gamma_size: u32,
    mode_valid: u32,
    mode: DrmModeModeinfo,
}

#[repr(C)]
struct DrmModeGetEncoder {
    encoder_id: u32,
    encoder_type: u32,
    crtc_id: u32,
    possible_crtcs: u32,
    possible_clones: u32,
}

#[repr(C)]
struct DrmModeGetConnector {
    encoders_ptr: u64,
    modes_ptr: u64,
    props_ptr: u64,
    prop_values_ptr: u64,
    count_modes: u32,
    count_props: u32,
    count_encoders: u32,
    encoder_id: u32,
    connector_id: u32,
    connector_type: u32,
    connector_type_id: u32,
    connection: u32,
    mm_width: u32,
    mm_height: u32,
    subpixel: u32,
    pad: u32,
}

#[repr(C)]
struct DrmModeGetPlaneRes {
    plane_id_ptr: u64,
    count_planes: u32,
}

#[repr(C)]
struct DrmModeGetPlane {
    plane_id: u32,
    crtc_id: u32,
    fb_id: u32,
    possible_crtcs: u32,
    gamma_size: u32,
    count_format_types: u32,
    format_type_ptr: u64,
}

#[repr(C)]
struct DrmModeFbCmd2 {
    fb_id: u32,
    width: u32,
    height: u32,
    pixel_format: u32,
    flags: u32,
    handles: [u32; 4],
    pitches: [u32; 4],
    offsets: [u32; 4],
    modifier: [u64; 4],
}

#[repr(C)]
struct DrmModeSetPlane {
    plane_id: u32,
    crtc_id: u32,
    fb_id: u32,
    flags: u32,
    crtc_x: i32,
    crtc_y: i32,
    crtc_w: u32,
    crtc_h: u32,
    src_x: u32,
    src_y: u32,
    src_h: u32,
    src_w: u32,
}

#[repr(C)]
struct DrmPrimeHandle {
    handle: u32,
    flags: u32,
    fd: i32,
}

#[repr(C)]
struct DrmGemClose {
    handle: u32,
    pad: u32,
}

struct Imported {
    fb_id: u32,
    handle: u32,
}

pub struct KmsPlaneSink {
    _file: std::fs::File,
    fd: RawFd,
    plane_id: u32,
    crtc_id: u32,
    crtc_w: u32,
    crtc_h: u32,
    fourcc: u32,
    width: u32,
    height: u32,
    stride: u32,
    imported: Vec<Option<Imported>>,
}

const _: () = assert!(std::mem::size_of::<DrmModeSetPlane>() == 48);
const _: () = assert!(std::mem::size_of::<DrmModeFbCmd2>() == 104);
const _: () = assert!(std::mem::size_of::<DrmModeGetConnector>() == 80);
const _: () = assert!(std::mem::size_of::<DrmModeCrtc>() == 104);
const _: () = assert!(std::mem::size_of::<DrmModeModeinfo>() == 68);

impl KmsPlaneSink {
    /// Opens a DRM card and picks a plane that can scan out `fourcc` (NV12 or YU12).
    pub fn open(fourcc: u32, width: u32, height: u32, stride: u32) -> io::Result<Self> {
        if fourcc != NV12 && fourcc != YU12 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "plane format"));
        }
        let mut last = io::Error::new(io::ErrorKind::NotFound, "no drm card");
        for path in ["/dev/dri/card0", "/dev/dri/card1"] {
            match Self::open_card(path, fourcc, width, height, stride.max(width)) {
                Ok(sink) => return Ok(sink),
                Err(e) => {
                    if std::path::Path::new(path).exists() {
                        eprintln!("\x1b[1;33m[kms]\x1b[0m {path}: {e}");
                        return Err(e);
                    }
                    last = e;
                }
            }
        }
        Err(last)
    }

    fn open_card(path: &str, fourcc: u32, width: u32, height: u32, stride: u32) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_CLOEXEC)
            .open(path)?;
        let fd = file.as_raw_fd();
        unsafe { libc::ioctl(fd, DRM_IOCTL_SET_MASTER, 0); }

        // Enable Universal Planes so KMS exposes primary display planes
        let mut cap = DrmSetClientCap {
            capability: DRM_CLIENT_CAP_UNIVERSAL_PLANES,
            value: 1,
        };
        unsafe {
            let _ = libc::ioctl(fd, DRM_IOCTL_SET_CLIENT_CAP, &mut cap);
        }

        let (_connector, encoder_id) = find_connector(fd)?;
        let crtc_id = find_crtc(fd, encoder_id)?;
        let (crtc_w, crtc_h) = crtc_size(fd, crtc_id, width, height);
        let plane_id = find_plane(fd, crtc_id, fourcc)?;

        println!(
            "\x1b[1;32m[kms]\x1b[0m Plane {} on CRTC {} via {} ({}x{} -> {}x{}, {}).",
            plane_id,
            crtc_id,
            path,
            width,
            height,
            crtc_w,
            crtc_h,
            if fourcc == NV12 { "NV12" } else { "YU12" }
        );
        Ok(Self {
            _file: file,
            fd,
            plane_id,
            crtc_id,
            crtc_w,
            crtc_h,
            fourcc,
            width,
            height,
            stride,
            imported: Vec::new(),
        })
    }

    /// Imports one V4L2 capture dma-buf. The fd may be closed by the caller afterwards.
    pub fn import(&mut self, index: usize, dmabuf_fd: RawFd) -> io::Result<()> {
        if index >= 32 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "buffer index"));
        }
        let mut prime = DrmPrimeHandle { handle: 0, flags: 0, fd: dmabuf_fd };
        if unsafe { libc::ioctl(self.fd, DRM_IOCTL_PRIME_FD_TO_HANDLE, &mut prime) } != 0 || prime.handle == 0 {
            return Err(io::Error::last_os_error());
        }
        let handle = prime.handle;
        let mut cmd = DrmModeFbCmd2 {
            fb_id: 0,
            width: self.width,
            height: self.height,
            pixel_format: self.fourcc,
            flags: 0,
            handles: [0; 4],
            pitches: [0; 4],
            offsets: [0; 4],
            modifier: [0; 4],
        };
        if self.fourcc == NV12 {
            cmd.handles[0] = handle;
            cmd.handles[1] = handle;
            cmd.pitches[0] = self.stride;
            cmd.pitches[1] = self.stride;
            cmd.offsets[1] = self.stride.saturating_mul(self.height);
        } else {
            let uv = self.stride / 2;
            let y_size = self.stride.saturating_mul(self.height);
            let u_size = uv.saturating_mul(self.height / 2);
            cmd.handles = [handle, handle, handle, 0];
            cmd.pitches = [self.stride, uv, uv, 0];
            cmd.offsets = [0, y_size, y_size.saturating_add(u_size), 0];
        }
        if unsafe { libc::ioctl(self.fd, DRM_IOCTL_MODE_ADDFB2, &mut cmd) } != 0 || cmd.fb_id == 0 {
            let err = io::Error::last_os_error();
            let mut close = DrmGemClose { handle, pad: 0 };
            unsafe { libc::ioctl(self.fd, DRM_IOCTL_GEM_CLOSE, &mut close); }
            return Err(err);
        }
        if self.imported.len() <= index {
            self.imported.resize_with(index + 1, || None);
        }
        self.imported[index] = Some(Imported { fb_id: cmd.fb_id, handle });
        Ok(())
    }

    /// Shows buffer `index` immediately. `Ok(false)` means the plane was busy and the frame was not queued.
    pub fn present(&mut self, index: usize) -> io::Result<bool> {
        let fb_id = self.imported.get(index).and_then(|s| s.as_ref()).map(|s| s.fb_id).unwrap_or(0);
        if fb_id == 0 {
            return Err(io::Error::new(io::ErrorKind::NotFound, "fb"));
        }
        let mut req = DrmModeSetPlane {
            plane_id: self.plane_id,
            crtc_id: self.crtc_id,
            fb_id,
            flags: 0,
            crtc_x: 0,
            crtc_y: 0,
            crtc_w: self.crtc_w,
            crtc_h: self.crtc_h,
            src_x: 0,
            src_y: 0,
            src_h: self.height << 16,
            src_w: self.width << 16,
        };
        if unsafe { libc::ioctl(self.fd, DRM_IOCTL_MODE_SETPLANE, &mut req) } == 0 {
            return Ok(true);
        }
        let err = io::Error::last_os_error();
        if err.raw_os_error() == Some(libc::EBUSY) {
            return Ok(false);
        }
        Err(err)
    }
}

impl Drop for KmsPlaneSink {
    fn drop(&mut self) {
        let mut clear = DrmModeSetPlane {
            plane_id: self.plane_id,
            crtc_id: self.crtc_id,
            fb_id: 0,
            flags: 0,
            crtc_x: 0,
            crtc_y: 0,
            crtc_w: 0,
            crtc_h: 0,
            src_x: 0,
            src_y: 0,
            src_h: 0,
            src_w: 0,
        };
        unsafe { libc::ioctl(self.fd, DRM_IOCTL_MODE_SETPLANE, &mut clear); }
        for slot in self.imported.drain(..) {
            if let Some(imp) = slot {
                let mut id = imp.fb_id;
                unsafe { libc::ioctl(self.fd, DRM_IOCTL_MODE_RMFB, &mut id); }
                let mut gem = DrmGemClose { handle: imp.handle, pad: 0 };
                unsafe { libc::ioctl(self.fd, DRM_IOCTL_GEM_CLOSE, &mut gem); }
            }
        }
    }
}

struct CardResources {
    #[allow(dead_code)]
    fbs: Vec<u32>,
    crtcs: Vec<u32>,
    connectors: Vec<u32>,
    encoders: Vec<u32>,
}

fn get_card_resources(fd: RawFd) -> io::Result<CardResources> {
    let mut res = unsafe { std::mem::zeroed::<DrmModeCardRes>() };
    if unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_GETRESOURCES, &mut res) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let mut fbs = vec![0u32; res.count_fbs as usize];
    let mut crtcs = vec![0u32; res.count_crtcs as usize];
    let mut connectors = vec![0u32; res.count_connectors as usize];
    let mut encoders = vec![0u32; res.count_encoders as usize];
    res.fb_id_ptr = fbs.as_mut_ptr() as u64;
    res.crtc_id_ptr = crtcs.as_mut_ptr() as u64;
    res.connector_id_ptr = connectors.as_mut_ptr() as u64;
    res.encoder_id_ptr = encoders.as_mut_ptr() as u64;
    if unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_GETRESOURCES, &mut res) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(CardResources { fbs, crtcs, connectors, encoders })
}

fn find_connector(fd: RawFd) -> io::Result<(u32, u32)> {
    let card = get_card_resources(fd)?;
    let mut fallback = None;
    for &id in &card.connectors {
        let mut conn = unsafe { std::mem::zeroed::<DrmModeGetConnector>() };
        conn.connector_id = id;
        if unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_GETCONNECTOR, &mut conn) } != 0 {
            continue;
        }

        let mut modes = vec![unsafe { std::mem::zeroed::<DrmModeModeinfo>() }; conn.count_modes as usize];
        let mut props = vec![0u32; conn.count_props as usize];
        let mut prop_values = vec![0u64; conn.count_props as usize];
        let mut encoders = vec![0u32; conn.count_encoders as usize];
        conn.modes_ptr = modes.as_mut_ptr() as u64;
        conn.props_ptr = props.as_mut_ptr() as u64;
        conn.prop_values_ptr = prop_values.as_mut_ptr() as u64;
        conn.encoders_ptr = encoders.as_mut_ptr() as u64;
        if unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_GETCONNECTOR, &mut conn) } != 0 {
            continue;
        }

        let mut enc_id = conn.encoder_id;
        if enc_id == 0 && !encoders.is_empty() {
            enc_id = encoders[0];
        }
        if enc_id == 0 && !card.encoders.is_empty() {
            enc_id = card.encoders[0];
        }

        if enc_id == 0 {
            continue;
        }

        if conn.connection == DRM_MODE_CONNECTED {
            return Ok((id, enc_id));
        }
        fallback = Some((id, enc_id));
    }
    fallback.ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "connector"))
}

fn find_crtc(fd: RawFd, encoder_id: u32) -> io::Result<u32> {
    let mut enc = DrmModeGetEncoder {
        encoder_id,
        encoder_type: 0,
        crtc_id: 0,
        possible_crtcs: 0,
        possible_clones: 0,
    };
    if unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_GETENCODER, &mut enc) } != 0 {
        return Err(io::Error::last_os_error());
    }
    if enc.crtc_id != 0 {
        return Ok(enc.crtc_id);
    }
    let card = get_card_resources(fd)?;
    for (i, &crtc) in card.crtcs.iter().enumerate() {
        if (enc.possible_crtcs & (1 << i)) != 0 {
            return Ok(crtc);
        }
    }
    if let Some(&first) = card.crtcs.first() {
        return Ok(first);
    }
    Err(io::Error::new(io::ErrorKind::NotFound, "crtc"))
}

fn crtc_size(fd: RawFd, crtc_id: u32, fallback_w: u32, fallback_h: u32) -> (u32, u32) {
    let mut crtc = unsafe { std::mem::zeroed::<DrmModeCrtc>() };
    crtc.crtc_id = crtc_id;
    if unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_GETCRTC, &mut crtc) } == 0
        && crtc.mode_valid != 0
        && crtc.mode.hdisplay > 0
        && crtc.mode.vdisplay > 0
    {
        return (crtc.mode.hdisplay as u32, crtc.mode.vdisplay as u32);
    }
    (fallback_w, fallback_h)
}

fn find_plane(fd: RawFd, crtc_id: u32, fourcc: u32) -> io::Result<u32> {
    let crtc_index = crtc_bit(fd, crtc_id)?;
    let mut res = DrmModeGetPlaneRes { plane_id_ptr: 0, count_planes: 0 };
    if unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_GETPLANERESOURCES, &mut res) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let mut ids = vec![0u32; res.count_planes as usize];
    res.plane_id_ptr = ids.as_mut_ptr() as u64;
    if unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_GETPLANERESOURCES, &mut res) } != 0 {
        return Err(io::Error::last_os_error());
    }
    for &plane_id in &ids {
        let mut plane = DrmModeGetPlane {
            plane_id,
            crtc_id: 0,
            fb_id: 0,
            possible_crtcs: 0,
            gamma_size: 0,
            count_format_types: 0,
            format_type_ptr: 0,
        };
        if unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_GETPLANE, &mut plane) } != 0 {
            continue;
        }
        if plane.possible_crtcs & (1 << crtc_index) == 0 {
            continue;
        }
        let n = (plane.count_format_types as usize).min(64);
        let mut formats = vec![0u32; n];
        plane.count_format_types = n as u32;
        plane.format_type_ptr = formats.as_mut_ptr() as u64;
        if unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_GETPLANE, &mut plane) } != 0 {
            continue;
        }
        if formats.iter().any(|&f| f == fourcc) {
            return Ok(plane_id);
        }
        let names: Vec<String> = formats.iter().map(|f| fourcc_name(*f)).collect();
        eprintln!(
            "\x1b[1;33m[kms]\x1b[0m plane {plane_id} crtcs {:#x} skipped: {}",
            plane.possible_crtcs,
            names.join(",")
        );
    }
    Err(io::Error::new(io::ErrorKind::NotFound, "plane"))
}

fn fourcc_name(f: u32) -> String {
    let b = f.to_le_bytes();
    String::from_utf8_lossy(&b).chars().map(|c| if c.is_ascii_graphic() { c } else { '.' }).collect()
}

fn crtc_bit(fd: RawFd, crtc_id: u32) -> io::Result<u32> {
    let card = get_card_resources(fd)?;
    card.crtcs
        .iter()
        .position(|&id| id == crtc_id)
        .map(|i| i as u32)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "crtc index"))
}
