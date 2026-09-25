//! Direct USB Bulk Receiver Engine (Mode 2 via Linux FunctionFS) in 100% Pure Rust
//!
//! Enables zero-network-stack H.264 video streaming over USB 2.0 High-Speed Bulk endpoints.
//! Bypasses kernel TCP/IP, UDP, ARP, and network buffers completely.
//!
//! Linux FunctionFS (`f_fs`) Architecture:
//! - `ep0`: Control channel configuring USB Interface & Endpoint Descriptors.
//! - `ep1`: Bulk OUT Endpoint (Host -> Pi Zero) for raw H.264 NAL stream.
//! - `ep2`: Bulk IN Endpoint (Pi Zero -> Host) for telemetry and flow control.
//!
//! License: MIT
//! Author: Carlos Alberto <psncarlosalberto4ti@gmail.com>

use crate::pipeline::{PipelineKind, PipelineManager};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::io::IntoRawFd;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[allow(dead_code)]
pub const FFS_DIR: &str = "/dev/usb-ffs/display";
pub const FFS_EP0: &str = "/dev/usb-ffs/display/ep0";
pub const FFS_EP1: &str = "/dev/usb-ffs/display/ep1"; // Bulk OUT (Video)
#[allow(dead_code)]
pub const FFS_EP2: &str = "/dev/usb-ffs/display/ep2"; // Bulk IN (Telemetry)

/// Linux FunctionFS Constants (include/uapi/linux/usb/functionfs.h)
const FUNCTIONFS_DESCRIPTORS_MAGIC_V2: u32 = 3;
const FUNCTIONFS_STRINGS_MAGIC: u32 = 2;

const FUNCTIONFS_HAS_FS_DESC: u32 = 1;
const FUNCTIONFS_HAS_HS_DESC: u32 = 2;

/// USB Interface Descriptor (9 bytes)
#[repr(C, packed)]
#[derive(Clone, Copy)]
struct UsbInterfaceDescriptor {
    b_length: u8,
    b_descriptor_type: u8, // 0x04 (INTERFACE)
    b_interface_number: u8,
    b_alternate_setting: u8,
    b_num_endpoints: u8,
    b_interface_class: u8,    // 0xFF (Vendor Specific)
    b_interface_sub_class: u8, // 0x00
    b_interface_protocol: u8,  // 0x00
    i_interface: u8,           // String index
}

/// USB Endpoint Descriptor (7 bytes)
#[repr(C, packed)]
#[derive(Clone, Copy)]
struct UsbEndpointDescriptorNoAudio {
    b_length: u8,
    b_descriptor_type: u8, // 0x05 (ENDPOINT)
    b_endpoint_address: u8,
    bm_attributes: u8, // 0x02 (BULK)
    w_max_packet_size: u16,
    b_interval: u8,
}

/// FunctionFS Descriptors Header (V2)
#[repr(C, packed)]
struct FfsDescriptorsHeader {
    magic: u32,
    length: u32,
    flags: u32,
    fs_count: u32,
    hs_count: u32,
}

/// Initializes FunctionFS descriptors on `ep0`.
/// Must be called once `/dev/usb-ffs/display` is mounted.
pub fn init_functionfs_descriptors() -> std::io::Result<File> {
    println!("\x1b[1;34m[usb-bulk]\x1b[0m Opening FunctionFS control endpoint at {}...", FFS_EP0);

    let mut ep0 = OpenOptions::new()
        .read(true)
        .write(true)
        .open(FFS_EP0)?;

    let iface = UsbInterfaceDescriptor {
        b_length: 9,
        b_descriptor_type: 4,
        b_interface_number: 0,
        b_alternate_setting: 0,
        b_num_endpoints: 2,
        b_interface_class: 0xFF, // Vendor specific
        b_interface_sub_class: 0x00,
        b_interface_protocol: 0x00,
        i_interface: 1,
    };

    // EP1: Bulk OUT (Host to Pi Zero - H.264 video stream)
    let ep1_fs = UsbEndpointDescriptorNoAudio {
        b_length: 7,
        b_descriptor_type: 5,
        b_endpoint_address: 0x01, // OUT 1
        bm_attributes: 0x02,      // Bulk
        w_max_packet_size: 64,    // Full-Speed
        b_interval: 0,
    };

    let ep1_hs = UsbEndpointDescriptorNoAudio {
        b_length: 7,
        b_descriptor_type: 5,
        b_endpoint_address: 0x01, // OUT 1
        bm_attributes: 0x02,      // Bulk
        w_max_packet_size: 512,   // High-Speed (480 Mbps)
        b_interval: 0,
    };

    // EP2: Bulk IN (Pi Zero to Host - Telemetry/Stats)
    let ep2_fs = UsbEndpointDescriptorNoAudio {
        b_length: 7,
        b_descriptor_type: 5,
        b_endpoint_address: 0x82, // IN 2
        bm_attributes: 0x02,      // Bulk
        w_max_packet_size: 64,
        b_interval: 0,
    };

    let ep2_hs = UsbEndpointDescriptorNoAudio {
        b_length: 7,
        b_descriptor_type: 5,
        b_endpoint_address: 0x82, // IN 2
        bm_attributes: 0x02,      // Bulk
        w_max_packet_size: 512,
        b_interval: 0,
    };

    let mut desc_payload = Vec::new();
    let header_size = std::mem::size_of::<FfsDescriptorsHeader>();
    let entry_size = std::mem::size_of::<UsbInterfaceDescriptor>()
        + 2 * std::mem::size_of::<UsbEndpointDescriptorNoAudio>();
    let total_len = (header_size + entry_size * 2) as u32;

    let header = FfsDescriptorsHeader {
        magic: FUNCTIONFS_DESCRIPTORS_MAGIC_V2,
        length: total_len,
        flags: FUNCTIONFS_HAS_FS_DESC | FUNCTIONFS_HAS_HS_DESC,
        fs_count: 3, // 1 iface + 2 ep
        hs_count: 3,
    };

    unsafe {
        let h_slice = std::slice::from_raw_parts(
            &header as *const _ as *const u8,
            header_size,
        );
        desc_payload.extend_from_slice(h_slice);

        let iface_slice = std::slice::from_raw_parts(
            &iface as *const _ as *const u8,
            std::mem::size_of::<UsbInterfaceDescriptor>(),
        );
        desc_payload.extend_from_slice(iface_slice);

        let ep1_fs_slice = std::slice::from_raw_parts(
            &ep1_fs as *const _ as *const u8,
            std::mem::size_of::<UsbEndpointDescriptorNoAudio>(),
        );
        desc_payload.extend_from_slice(ep1_fs_slice);

        let ep2_fs_slice = std::slice::from_raw_parts(
            &ep2_fs as *const _ as *const u8,
            std::mem::size_of::<UsbEndpointDescriptorNoAudio>(),
        );
        desc_payload.extend_from_slice(ep2_fs_slice);

        // High-Speed descriptors
        desc_payload.extend_from_slice(iface_slice);

        let ep1_hs_slice = std::slice::from_raw_parts(
            &ep1_hs as *const _ as *const u8,
            std::mem::size_of::<UsbEndpointDescriptorNoAudio>(),
        );
        desc_payload.extend_from_slice(ep1_hs_slice);

        let ep2_hs_slice = std::slice::from_raw_parts(
            &ep2_hs as *const _ as *const u8,
            std::mem::size_of::<UsbEndpointDescriptorNoAudio>(),
        );
        desc_payload.extend_from_slice(ep2_hs_slice);
    }

    ep0.write_all(&desc_payload)?;
    ep0.flush()?;
    println!("\x1b[1;32m[usb-bulk]\x1b[0m FunctionFS descriptors written to ep0 successfully.");

    // Write String Descriptors (LangID 0x0409 en-US)
    let mut str_payload = Vec::new();
    let str_val = b"VideoCore IV USB Display\0";
    let str_table_len = (16 + 2 + str_val.len()) as u32;

    str_payload.extend_from_slice(&FUNCTIONFS_STRINGS_MAGIC.to_ne_bytes());
    str_payload.extend_from_slice(&str_table_len.to_ne_bytes());
    str_payload.extend_from_slice(&1u32.to_ne_bytes()); // 1 string
    str_payload.extend_from_slice(&1u32.to_ne_bytes()); // 1 language
    str_payload.extend_from_slice(&0x0409u16.to_ne_bytes()); // LangID en-US
    str_payload.extend_from_slice(str_val);

    ep0.write_all(&str_payload)?;
    ep0.flush()?;
    println!("\x1b[1;32m[usb-bulk]\x1b[0m String descriptors registered successfully.");

    Ok(ep0)
}

/// Main loop for Mode 2 USB Bulk Direct receiver
pub fn run_usb_bulk_receiver(
    running: Arc<AtomicBool>,
    pipeline_mgr: Arc<PipelineManager>,
) -> std::io::Result<()> {
    if !Path::new(FFS_EP0).exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Endpoint {} not found. FunctionFS gadget is not mounted.", FFS_EP0),
        ));
    }

    let ep0 = init_functionfs_descriptors()?;

    let run_ep0 = running.clone();
    thread::spawn(move || {
        let mut ep0_file = ep0;
        let mut event_buf = [0u8; 512];
        while run_ep0.load(Ordering::SeqCst) {
            match ep0_file.read(&mut event_buf) {
                Ok(n) if n > 0 => {
                    // UDC events
                }
                _ => {
                    thread::sleep(Duration::from_millis(100));
                }
            }
        }
    });

    println!("\x1b[1;34m[usb-bulk]\x1b[0m Opening Bulk OUT data endpoint at {}...", FFS_EP1);
    let ep1 = OpenOptions::new()
        .read(true)
        .open(FFS_EP1)?;

    let raw_fd = ep1.into_raw_fd();

    println!("\x1b[1;32m[usb-bulk]\x1b[0m Connecting Bulk OUT endpoint directly to VideoCore IV decoder...");
    pipeline_mgr.start(PipelineKind::UsbBulkPipe { fd: raw_fd })?;

    while running.load(Ordering::SeqCst) {
        if pipeline_mgr.has_exited() {
            println!("\x1b[1;33m[usb-bulk]\x1b[0m Restarting USB Bulk decode pipeline...");
            let _ = pipeline_mgr.start(PipelineKind::UsbBulkPipe { fd: raw_fd });
        }
        thread::sleep(Duration::from_millis(500));
    }

    pipeline_mgr.stop();
    Ok(())
}
