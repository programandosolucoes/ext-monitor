//! Pure-Rust Time Synchronization Subsystem
//!
//! Synchronizes host laptop system time (UTC epoch seconds) with the Raspberry Pi Zero
//! appliance over the USB CDC-ECM / Wi-Fi network link.
//!
//! Why this is critical:
//! Raspberry Pi Zero has no hardware battery-backed RTC. On cold boot, its system clock
//! defaults to Unix epoch (1970). This causes all modern TLS certificate validations
//! (Google Cast, HTTPS, etc.) to immediately fail as 'certificate not yet valid'.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Performs an atomic time synchronization request to the receiver
pub fn sync_receiver_time(target_ip: &str, http_port: u16) -> Result<u64, Box<dyn std::error::Error>> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let addr_str = format!("{}:{}", target_ip, http_port);
    let socket_addr: SocketAddr = addr_str.parse()?;

    let mut stream = TcpStream::connect_timeout(&socket_addr, Duration::from_millis(1500))?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));

    let body = format!("{{\"unix_epoch_secs\":{}}}", now);
    let request = format!(
        "POST /api/time/sync HTTP/1.1\r\n\
         Host: {}\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\r\n\
         {}",
        addr_str,
        body.len(),
        body
    );

    stream.write_all(request.as_bytes())?;
    stream.flush()?;

    let mut response = [0u8; 1024];
    let n = stream.read(&mut response)?;
    let resp_str = String::from_utf8_lossy(&response[..n]);

    if resp_str.contains("200 OK") {
        Ok(now)
    } else {
        Err(format!("Appliance returned non-200 response: {}", resp_str.lines().next().unwrap_or("")).into())
    }
}

/// Spawns a background thread that ensures the Pi Zero clock remains synchronized with the host
pub fn start_time_sync_daemon(running: Arc<AtomicBool>, target_ip: String, http_port: u16) {
    thread::Builder::new()
        .name("host-time-syncer".to_string())
        .spawn(move || {
            let mut last_sync = Instant::now() - Duration::from_secs(120); // Force immediate first sync

            while running.load(Ordering::Relaxed) {
                if last_sync.elapsed() >= Duration::from_secs(300) {
                    match sync_receiver_time(&target_ip, http_port) {
                        Ok(epoch) => {
                            println!(
                                "\x1b[1;32m[time-sync]\x1b[0m Synchronized host time to appliance {} (Unix epoch: {})",
                                target_ip, epoch
                            );
                            last_sync = Instant::now();
                        }
                        Err(_e) => {
                            // If appliance is not ready yet, retry in 3 seconds
                            thread::sleep(Duration::from_secs(3));
                            continue;
                        }
                    }
                }
                thread::sleep(Duration::from_secs(1));
            }
        })
        .ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_time_epoch() {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        assert!(now > 1700000000); // Beyond late 2023
    }
}

