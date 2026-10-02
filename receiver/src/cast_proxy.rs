//! Pure-Rust TCP Forwarder for Google Cast V2 (Port 8009)
//!
//! Listens on 0.0.0.0:8009 on the Raspberry Pi Zero appliance and proxies
//! incoming Cast V2 TLS connections directly to the host laptop (192.168.7.1:8009)
//! where the cryptographic Cast V2 server and developer certificate engine run.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use std::io;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub const CAST_PORT: u16 = 8009;

/// Starts the Cast V2 TCP forwarder daemon in a background thread
pub fn start_cast_forwarder(running: Arc<AtomicBool>, target_host: &'static str, target_port: u16) {
    let r = running.clone();
    thread::Builder::new()
        .name("cast-proxy-8009".to_string())
        .spawn(move || {
            let bind_addr: SocketAddr = ([0, 0, 0, 0], CAST_PORT).into();
            let listener = match TcpListener::bind(bind_addr) {
                Ok(l) => {
                    println!("\x1b[1;32m[cast-proxy]\x1b[0m Google Cast V2 TCP forwarder listening on port {} -> {}:{}", CAST_PORT, target_host, target_port);
                    let _ = l.set_nonblocking(true);
                    l
                }
                Err(e) => {
                    eprintln!("\x1b[1;33m[cast-proxy]\x1b[0m Could not bind port {}: {}", CAST_PORT, e);
                    return;
                }
            };

            while r.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((client_stream, peer_addr)) => {
                        let target_addr = format!("{}:{}", target_host, target_port);
                        let r_conn = r.clone();
                        thread::spawn(move || {
                            handle_cast_connection(client_stream, peer_addr, &target_addr, r_conn);
                        });
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(50));
                    }
                    Err(_) => {
                        thread::sleep(Duration::from_millis(100));
                    }
                }
            }
        })
        .expect("Failed to spawn cast-proxy thread");
}

fn handle_cast_connection(
    mut client: TcpStream,
    peer: SocketAddr,
    target_addr: &str,
    running: Arc<AtomicBool>,
) {
    let _ = client.set_nodelay(true);
    let _ = client.set_read_timeout(Some(Duration::from_secs(10)));
    let _ = client.set_write_timeout(Some(Duration::from_secs(10)));

    let mut target = match TcpStream::connect(target_addr) {
        Ok(t) => {
            let _ = t.set_nodelay(true);
            let _ = t.set_read_timeout(Some(Duration::from_secs(10)));
            let _ = t.set_write_timeout(Some(Duration::from_secs(10)));
            t
        }
        Err(e) => {
            eprintln!("\x1b[1;33m[cast-proxy]\x1b[0m Connection from {} dropped (target {} unreachable: {})", peer, target_addr, e);
            return;
        }
    };

    let mut client_clone = match client.try_clone() {
        Ok(c) => c,
        Err(_) => return,
    };
    let mut target_clone = match target.try_clone() {
        Ok(t) => t,
        Err(_) => return,
    };

    let r_up = running.clone();
    let up_thread = thread::spawn(move || {
        let mut buf = [0u8; 8192];
        while r_up.load(Ordering::SeqCst) {
            match io::Read::read(&mut client, &mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if io::Write::write_all(&mut target, &buf[..n]).is_err() {
                        break;
                    }
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock || e.kind() == io::ErrorKind::TimedOut => {
                    continue;
                }
                Err(_) => break,
            }
        }
    });

    let mut buf = [0u8; 8192];
    while running.load(Ordering::SeqCst) {
        match io::Read::read(&mut target_clone, &mut buf) {
            Ok(0) => break,
            Ok(n) => {
                if io::Write::write_all(&mut client_clone, &buf[..n]).is_err() {
                    break;
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock || e.kind() == io::ErrorKind::TimedOut => {
                continue;
            }
            Err(_) => break,
        }
    }

    let _ = up_thread.join();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cast_port_constant() {
        assert_eq!(CAST_PORT, 8009);
    }
}
