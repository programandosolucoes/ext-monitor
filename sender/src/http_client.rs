//! 100% Pure Rust Minimal HTTP Client
//!
//! Replaces all external `curl` process spawning with zero-dependency native TCP sockets.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

/// Sends a lightweight HTTP POST request with a JSON payload in pure Rust
pub fn post_json(url: &str, json_body: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let (host, port, path) = parse_url(url)?;
    let addr = format!("{}:{}", host, port);
    
    let mut stream = TcpStream::connect_timeout(
        &addr.parse()?,
        Duration::from_millis(800),
    )?;
    stream.set_read_timeout(Some(Duration::from_millis(800)))?;
    stream.set_write_timeout(Some(Duration::from_millis(800)))?;

    let req = format!(
        "POST {} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        path, host, json_body.len(), json_body
    );
    stream.write_all(req.as_bytes())?;

    let mut response = String::new();
    let _ = stream.read_to_string(&mut response);
    Ok(response)
}

/// Sends a lightweight HTTP POST request with zero payload (for triggers/stops) in pure Rust
pub fn post_empty(url: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (host, port, path) = parse_url(url)?;
    let addr = format!("{}:{}", host, port);
    
    let mut stream = TcpStream::connect_timeout(
        &addr.parse()?,
        Duration::from_millis(600),
    )?;
    stream.set_write_timeout(Some(Duration::from_millis(600)))?;

    let req = format!(
        "POST {} HTTP/1.1\r\nHost: {}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        path, host
    );
    stream.write_all(req.as_bytes())?;
    Ok(())
}

/// Sends a lightweight HTTP GET request in pure Rust
pub fn get(url: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let (host, port, path) = parse_url(url)?;
    let addr = format!("{}:{}", host, port);
    
    let mut stream = TcpStream::connect_timeout(
        &addr.parse()?,
        Duration::from_millis(800),
    )?;
    stream.set_read_timeout(Some(Duration::from_millis(800)))?;
    stream.set_write_timeout(Some(Duration::from_millis(800)))?;

    let req = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        path, host
    );
    stream.write_all(req.as_bytes())?;

    let mut response = String::new();
    let _ = stream.read_to_string(&mut response);
    
    // Extract body after HTTP headers (\r\n\r\n)
    if let Some(pos) = response.find("\r\n\r\n") {
        Ok(response[pos + 4..].to_string())
    } else {
        Ok(response)
    }
}

/// Helper to parse simple http://host:port/path URLs without external URL parser crates
fn parse_url(url: &str) -> Result<(String, u16, String), Box<dyn std::error::Error + Send + Sync>> {
    let stripped = url.strip_prefix("http://").unwrap_or(url);
    let (host_port, path) = match stripped.find('/') {
        Some(idx) => (&stripped[..idx], &stripped[idx..]),
        None => (stripped, "/"),
    };

    let (host, port) = match host_port.find(':') {
        Some(idx) => {
            let h = &host_port[..idx];
            let p: u16 = host_port[idx + 1..].parse()?;
            (h.to_string(), p)
        }
        None => (host_port.to_string(), 80),
    };

    Ok((host, port, path.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_url_variations() {
        let (host, port, path) = parse_url("http://192.168.7.2:8080/api/status").unwrap();
        assert_eq!(host, "192.168.7.2");
        assert_eq!(port, 8080);
        assert_eq!(path, "/api/status");

        let (host, port, path) = parse_url("http://127.0.0.1/stop").unwrap();
        assert_eq!(host, "127.0.0.1");
        assert_eq!(port, 80);
        assert_eq!(path, "/stop");
    }
}
