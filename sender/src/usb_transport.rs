//! Direct USB Bulk Transport Module (Mode 2) for Linux Host in 100% Pure Rust
//!
//! Uses the `rusb` crate (bindings to `libusb-1.0`) to write hardware-encoded
//! VA-API H.264 video frames directly into Endpoint 1 Bulk OUT on Raspberry Pi Zero.
//! Completely bypasses kernel network sockets, IP, UDP, and ARP tables.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use rusb::{Context, DeviceHandle, UsbContext};
use std::os::unix::io::RawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub const USB_VENDOR_ID_1: u16 = 0x1d6b;  // Linux Foundation
pub const USB_PRODUCT_ID_1: u16 = 0x0104; // Multifunction Gadget
pub const USB_VENDOR_ID_2: u16 = 0x1d50;  // OpenMoko
pub const USB_PRODUCT_ID_2: u16 = 0x614d; // ExtMonitor Composite

#[allow(dead_code)]
pub const USB_DEFAULT_INTERFACE: u8 = 0;
pub const USB_DEFAULT_ENDPOINT_OUT: u8 = 0x01;  // Bulk OUT

/// Checks whether a given USB VID and PID match ExtMonitor hardware configurations
#[inline]
pub fn is_ext_monitor_vid_pid(vid: u16, pid: u16) -> bool {
    (vid == USB_VENDOR_ID_1 && pid == USB_PRODUCT_ID_1)
        || (vid == USB_VENDOR_ID_2 && pid == USB_PRODUCT_ID_2)
}

/// USB High-Speed Bulk transfer rule: if transfer size is a multiple of 512,
/// a Zero-Length Packet (ZLP) must follow to signal end-of-transfer to hardware FIFO.
#[inline]
pub fn needs_zlp(transfer_len: usize) -> bool {
    transfer_len > 0 && transfer_len % 512 == 0
}

/// Locates and opens the Pi Zero USB Display Gadget on the host USB bus
pub fn open_usb_display_device() -> Result<(DeviceHandle<Context>, u8, u8), String> {
    let context = Context::new().map_err(|e| format!("Failed to initialize libusb context: {}", e))?;
    let devices = context.devices().map_err(|e| format!("Failed to enumerate USB devices: {}", e))?;

    for device in devices.iter() {
        if let Ok(desc) = device.device_descriptor() {
            let is_match = is_ext_monitor_vid_pid(desc.vendor_id(), desc.product_id());

            if is_match {
                println!(
                    "\x1b[1;32m[usb-transport]\x1b[0m Found Pi Zero USB Display (VID: {:04x}, PID: {:04x}): Bus {:03} Device {:03}",
                    desc.vendor_id(), desc.product_id(),
                    device.bus_number(),
                    device.address()
                );
                let is_debug = std::env::var("EXT_DEBUG").map(|v| v == "1").unwrap_or(false);
                if is_debug {
                    println!("\x1b[1;35m[DEBUG] [usb-transport] Device identified as target. Scanning for interfaces...\x1b[0m");
                }

                // Dynamically discover Bulk OUT interface and endpoint
                // Prioritize Vendor-Specific (Class 0xFF) FunctionFS display interface
                // to prevent accidentally claiming CDC ACM (Serial) or CDC ECM (Network)
                let mut target_iface = None;
                let mut target_ep = None;

                if let Ok(config_desc) = device.active_config_descriptor() {
                    for iface in config_desc.interfaces() {
                        for iface_desc in iface.descriptors() {
                            if iface_desc.class_code() == 0xFF {
                                for ep_desc in iface_desc.endpoint_descriptors() {
                                    if ep_desc.transfer_type() == rusb::TransferType::Bulk
                                        && ep_desc.direction() == rusb::Direction::Out
                                    {
                                        target_iface = Some(iface_desc.interface_number());
                                        target_ep = Some(ep_desc.address());
                                        let is_debug = std::env::var("EXT_DEBUG").map(|v| v == "1").unwrap_or(false);
                                        if is_debug {
                                            println!("\x1b[1;35m[DEBUG] [usb-transport] Found matching Bulk OUT Interface: {}, Endpoint Address: 0x{:02x}\x1b[0m", iface_desc.interface_number(), ep_desc.address());
                                        }
                                        break;
                                    }
                                }
                            }
                            if target_iface.is_some() {
                                break;
                            }
                        }
                        if target_iface.is_some() {
                            break;
                        }
                    }
                }

                let iface_num = match target_iface {
                    Some(i) => i,
                    None => {
                        return Err(format!(
                            "Pi Zero USB device found (VID: {:04x}, PID: {:04x}), but FunctionFS Display Interface (Class 0xFF) is not active. Ensure ext-receiver has USB Bulk mode enabled.",
                            desc.vendor_id(), desc.product_id()
                        ));
                    }
                };
                let ep_out = target_ep.unwrap_or(USB_DEFAULT_ENDPOINT_OUT);

                let handle = device
                    .open()
                    .map_err(|e| format!("Failed to open USB device: {}. Check udev permissions.", e))?;

                // Detach active kernel driver if bound to interface
                let _ = handle.detach_kernel_driver(iface_num);

                // Retry claim_interface up to 5 times with 100ms backoff in case previous release is finalizing
                let mut claim_res = handle.claim_interface(iface_num);
                for _ in 0..5 {
                    if claim_res.is_ok() {
                        break;
                    }
                    thread::sleep(Duration::from_millis(100));
                    let _ = handle.detach_kernel_driver(iface_num);
                    claim_res = handle.claim_interface(iface_num);
                }
                claim_res.map_err(|e| format!("Failed to claim USB Interface {}: {}", iface_num, e))?;

                println!(
                    "\x1b[1;32m[usb-transport]\x1b[0m USB Interface {} (Bulk OUT 0x{:02x}) claimed successfully.",
                    iface_num, ep_out
                );
                return Ok((handle, iface_num, ep_out));
            }
        }
    }

    Err(format!(
        "Pi Zero USB Display Device (VID: {:04x}/{:04x}, PID: {:04x}/{:04x}) not found on USB bus.",
        USB_VENDOR_ID_1, USB_VENDOR_ID_2, USB_PRODUCT_ID_1, USB_PRODUCT_ID_2
    ))
}

#[allow(dead_code)]
pub const USB_MAGIC: [u8; 4] = *b"EXMO";
#[allow(dead_code)]
pub const FLAG_EOF: u8 = 0x01; // End of Frame marker bit

/// Spawns background worker thread pumping framed H.264 data into the USB Bulk OUT endpoint
pub fn spawn_usb_bulk_writer(
    handle: DeviceHandle<Context>,
    pipe_read_fd: RawFd,
    running: Arc<AtomicBool>,
    writer_alive: Arc<AtomicBool>,
    stop_flag: Arc<AtomicBool>,
    iface_num: u8,
    ep_out: u8,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        writer_alive.store(true, Ordering::SeqCst);
        /// Represents Guard configuration and operational state.
        struct Guard {
            alive: Arc<AtomicBool>,
            fd: RawFd,
        }
        impl Drop for Guard {
            /// Custom destructor releasing allocated kernel resources, file descriptors, and hardware handles.
            fn drop(&mut self) {
                self.alive.store(false, Ordering::SeqCst);
                unsafe { libc::close(self.fd); }
            }
        }
        let _guard = Guard {
            alive: writer_alive,
            fd: pipe_read_fd,
        };

        println!(
            "\x1b[1;32m[usb-transport]\x1b[0m USB Bulk streaming thread active on Endpoint 0x{:02x} (RFC 4571 Framed RTP + ZLP)...",
            ep_out
        );

        let mut buffer = [0u8; 65536]; // 64 KB chunk size matching OS pipe capacity
        let mut total_bytes = 0u64;
        let mut last_log = std::time::Instant::now();
        let is_debug = std::env::var("EXT_DEBUG").map(|v| v == "1").unwrap_or(false);
        let mut stalls = 0;
        let mut last_bytes = 0u64;

        while running.load(Ordering::SeqCst) && !stop_flag.load(Ordering::SeqCst) {
            let mut pfd = libc::pollfd {
                fd: pipe_read_fd,
                events: libc::POLLIN,
                revents: 0,
            };
            let poll_res = unsafe { libc::poll(&mut pfd, 1, 100) };
            if poll_res < 0 {
                let err = std::io::Error::last_os_error();
                if err.kind() == std::io::ErrorKind::Interrupted {
                    continue;
                }
                eprintln!("\x1b[1;31m[usb-transport]\x1b[0m Poll error on video pipe: {}. Exiting.", err);
                break;
            } else if poll_res == 0 {
                // Timeout (100ms) - continue loop to check running/stop_flag
                continue;
            }

            if (pfd.revents & libc::POLLIN) != 0 {
                let n = unsafe { libc::read(pipe_read_fd, buffer.as_mut_ptr() as *mut libc::c_void, buffer.len()) };
                if n <= 0 {
                    if n == 0 {
                        println!("\x1b[1;33m[usb-transport]\x1b[0m Video pipe closed (EOF). Exiting USB bulk writer.");
                    } else {
                        let err = std::io::Error::last_os_error();
                        eprintln!("\x1b[1;31m[usb-transport]\x1b[0m Pipe read error: {}. Exiting.", err);
                    }
                    break;
                }
                let n = n as usize;
                let mut offset = 0;
                let mut retries = 0;
                while offset < n && running.load(Ordering::SeqCst) && !stop_flag.load(Ordering::SeqCst) {
                    let slice = &buffer[offset..n];
                    match handle.write_bulk(ep_out, slice, Duration::from_millis(500)) {
                        Ok(written) => {
                            offset += written;
                            total_bytes += written as u64;
                            retries = 0;
                        }
                        Err(rusb::Error::Pipe) => {
                            stalls += 1;
                            eprintln!("\x1b[1;33m[usb-transport]\x1b[0m Endpoint halted (stall), clearing halt...");
                            let _ = handle.clear_halt(ep_out);
                            retries += 1;
                            if retries > 3 {
                                break; // Drop remaining to unblock video pipe
                            }
                            thread::sleep(Duration::from_millis(5));
                        }
                        Err(rusb::Error::Timeout) => {
                            retries += 1;
                            if retries > 2 {
                                break;
                            }
                            thread::sleep(Duration::from_millis(5));
                        }
                        Err(e) => {
                            eprintln!("\x1b[1;31m[usb-transport]\x1b[0m USB write error: {}. Exiting writer thread.", e);
                            return;
                        }
                    }
                }

                // Critical USB Bulk Protocol Rule: If transfer is exact multiple of 512, send ZLP
                if needs_zlp(n) && offset == n {
                    let _ = handle.write_bulk(ep_out, &[], Duration::from_millis(50));
                }

                let elapsed = last_log.elapsed();
                if elapsed >= Duration::from_secs(5) {
                    let mb = (total_bytes as f64) / (1024.0 * 1024.0);
                    println!(
                        "\x1b[1;34m[usb-transport]\x1b[0m Total transmitted via USB Bulk: {:.2} MB (Zero-Network)",
                        mb
                    );
                    if is_debug {
                        let bytes_sec = (total_bytes - last_bytes) as f64 / elapsed.as_secs_f64();
                        println!("\x1b[1;35m[DEBUG] [usb-transport] Avg Rate: {:.2} KB/s | Stalls in window: {}\x1b[0m", bytes_sec / 1024.0, stalls);
                        stalls = 0;
                        last_bytes = total_bytes;
                    }
                    use std::io::Write;
                    let _ = std::io::stdout().flush();
                    last_log = std::time::Instant::now();
                }
            } else if (pfd.revents & (libc::POLLHUP | libc::POLLERR)) != 0 {
                println!("\x1b[1;33m[usb-transport]\x1b[0m Pipe hangup detected (POLLHUP). Exiting USB bulk writer.");
                break;
            }
        }

        // Wake up any pending read on receiver with a zero-length packet (ZLP)
        let _ = handle.write_bulk(ep_out, &[], Duration::from_millis(50));
        let _ = handle.release_interface(iface_num);
        println!("\x1b[1;32m[usb-transport]\x1b[0m USB Bulk transport released cleanly.");
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zlp_boundary_conditions() {
        assert!(!needs_zlp(0));
        assert!(!needs_zlp(511));
        assert!(needs_zlp(512));
        assert!(!needs_zlp(513));
        assert!(needs_zlp(1024));
        assert!(needs_zlp(65536));
        assert!(!needs_zlp(65535));
    }

    #[test]
    fn test_ext_monitor_vid_pid_matching() {
        assert!(is_ext_monitor_vid_pid(0x1d6b, 0x0104));
        assert!(is_ext_monitor_vid_pid(0x1d50, 0x614d));
        assert!(!is_ext_monitor_vid_pid(0x1d6b, 0x0105));
        assert!(!is_ext_monitor_vid_pid(0x1234, 0x5678));
    }
}

