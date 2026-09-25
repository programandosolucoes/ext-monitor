//! Direct USB Bulk Transport Module (Mode 2) for Linux Host in 100% Pure Rust
//!
//! Uses the `rusb` crate (bindings to `libusb-1.0`) to write hardware-encoded
//! VA-API H.264 video frames directly into Endpoint 1 Bulk OUT on Raspberry Pi Zero.
//! Completely bypasses kernel network sockets, IP, UDP, and ARP tables.
//!
//! License: MIT
//! Author: Carlos Alberto <psncarlosalberto4ti@gmail.com>

use rusb::{Context, DeviceHandle, UsbContext};
use std::io::Read;
use std::os::unix::io::{FromRawFd, RawFd};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub const USB_VENDOR_ID: u16 = 0x1d6b;  // Linux Foundation
pub const USB_PRODUCT_ID: u16 = 0x0104; // Multifunction Gadget
pub const USB_INTERFACE_NUM: u8 = 0;
pub const USB_ENDPOINT_OUT: u8 = 0x01;  // Bulk OUT

/// Locates and opens the Pi Zero USB Display Gadget on the host USB bus
pub fn open_usb_display_device() -> Result<DeviceHandle<Context>, String> {
    let context = Context::new().map_err(|e| format!("Failed to initialize libusb context: {}", e))?;
    let devices = context.devices().map_err(|e| format!("Failed to enumerate USB devices: {}", e))?;

    for device in devices.iter() {
        if let Ok(desc) = device.device_descriptor() {
            if desc.vendor_id() == USB_VENDOR_ID && desc.product_id() == USB_PRODUCT_ID {
                println!(
                    "\x1b[1;32m[usb-transport]\x1b[0m Found Pi Zero USB Display: Bus {:03} Device {:03}",
                    device.bus_number(),
                    device.address()
                );
                let handle = device
                    .open()
                    .map_err(|e| format!("Failed to open USB device: {}. Check udev permissions.", e))?;

                // Detach active kernel driver if bound to interface
                if let Ok(active) = handle.kernel_driver_active(USB_INTERFACE_NUM) {
                    if active {
                        let _ = handle.detach_kernel_driver(USB_INTERFACE_NUM);
                    }
                }

                handle
                    .claim_interface(USB_INTERFACE_NUM)
                    .map_err(|e| format!("Failed to claim USB Interface {}: {}", USB_INTERFACE_NUM, e))?;

                println!("\x1b[1;32m[usb-transport]\x1b[0m USB Interface {} claimed successfully.", USB_INTERFACE_NUM);
                return Ok(handle);
            }
        }
    }

    Err(format!(
        "Pi Zero USB Display Device (VID: {:04x}, PID: {:04x}) not found on USB bus.",
        USB_VENDOR_ID, USB_PRODUCT_ID
    ))
}

/// Spawns background worker thread pumping raw H.264 data into the USB Bulk OUT endpoint
pub fn spawn_usb_bulk_writer(
    handle: DeviceHandle<Context>,
    pipe_read_fd: RawFd,
    running: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        println!("\x1b[1;32m[usb-transport]\x1b[0m USB Bulk streaming thread active on Endpoint 0x{:02x}...", USB_ENDPOINT_OUT);

        let mut file = unsafe { std::fs::File::from_raw_fd(pipe_read_fd) };
        let mut buffer = [0u8; 16384]; // 16 KB chunks for high throughput on USB 2.0 High-Speed bus

        let mut total_bytes = 0u64;
        let mut last_log = std::time::Instant::now();

        while running.load(Ordering::SeqCst) {
            match file.read(&mut buffer) {
                Ok(0) => {
                    println!("\x1b[1;33m[usb-transport]\x1b[0m Video pipe closed. Exiting USB bulk writer.");
                    break;
                }
                Ok(n) => {
                    let mut offset = 0;
                    while offset < n && running.load(Ordering::SeqCst) {
                        let slice = &buffer[offset..n];
                        match handle.write_bulk(USB_ENDPOINT_OUT, slice, Duration::from_millis(500)) {
                            Ok(written) => {
                                offset += written;
                                total_bytes += written as u64;
                            }
                            Err(e) => {
                                eprintln!("\x1b[1;31m[usb-transport]\x1b[0m write_bulk USB error: {}", e);
                                thread::sleep(Duration::from_millis(50));
                                break;
                            }
                        }
                    }

                    if last_log.elapsed() >= Duration::from_secs(5) {
                        let mb = (total_bytes as f64) / (1024.0 * 1024.0);
                        println!("\x1b[1;34m[usb-transport]\x1b[0m Total transmitted via USB Bulk: {:.2} MB (Zero-Network)", mb);
                        last_log = std::time::Instant::now();
                    }
                }
                Err(e) => {
                    eprintln!("\x1b[1;31m[usb-transport]\x1b[0m Error reading from video pipe: {}", e);
                    break;
                }
            }
        }

        let _ = handle.release_interface(USB_INTERFACE_NUM);
        println!("\x1b[1;32m[usb-transport]\x1b[0m USB Bulk transport released cleanly.");
    })
}
