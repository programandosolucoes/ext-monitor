#!/usr/bin/env python3
"""
Pi Zero GPU Monitor Web Control & Telemetry Server
Lightweight daemon running on Raspberry Pi Zero (Python standard library only).
Port: 8080
"""

import http.server
import json
import os
import socketserver
import subprocess
import threading
import time
import urllib.parse

PORT = 8080
CONFIG_FILE = "/opt/ext-monitor/config.json"
WEB_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "static")

# Shared state
DEFAULT_CONFIG = {
    "fps": 12,
    "color": "256",
    "bitrate": 2500,
    "hud": True,
    "hud_auto_hide": True,
    "hud_timer_secs": 60,
    "last_hud_trigger": time.time(),
}

def load_config():
    if os.path.exists(CONFIG_FILE):
        try:
            with open(CONFIG_FILE, "r") as f:
                cfg = json.load(f)
                return {**DEFAULT_CONFIG, **cfg}
        except Exception:
            pass
    return DEFAULT_CONFIG.copy()

def save_config(cfg):
    try:
        os.makedirs(os.path.dirname(CONFIG_FILE), exist_ok=True)
        with open(CONFIG_FILE, "w") as f:
            json.dump(cfg, f, indent=2)
    except Exception as e:
        print(f"[!] Error saving config: {e}")

CURRENT_CONFIG = load_config()

# Helper to read system metrics without external packages
def get_system_metrics():
    metrics = {
        "cpu_percent": 0.0,
        "vpu_freq_mhz": 500,
        "vpu_voltage": "1.25V",
        "temp_c": 50.0,
        "throttled": "0x0",
        "ram_total_mb": 363.5,
        "ram_used_mb": 140.0,
        "ram_free_mb": 223.5,
        "rx_kbps": 0.0,
        "tx_kbps": 0.0,
        "current_config": CURRENT_CONFIG,
        "timestamp": time.time(),
    }
    
    # 1. Temperature & Throttled via vcgencmd
    try:
        out = subprocess.check_output(["vcgencmd", "measure_temp"], text=True)
        # temp=50.2'C
        val = out.replace("temp=", "").replace("'C\n", "").strip()
        metrics["temp_c"] = float(val)
    except Exception:
        pass

    try:
        out = subprocess.check_output(["vcgencmd", "get_throttled"], text=True)
        # throttled=0x0
        metrics["throttled"] = out.replace("throttled=", "").strip()
    except Exception:
        pass

    try:
        out = subprocess.check_output(["vcgencmd", "measure_clock", "core"], text=True)
        # frequency(1)=500000000
        parts = out.strip().split("=")
        if len(parts) == 2:
            metrics["vpu_freq_mhz"] = round(int(parts[1]) / 1000000)
    except Exception:
        pass

    # 2. RAM via /proc/meminfo
    try:
        with open("/proc/meminfo", "r") as f:
            mem = {}
            for line in f:
                parts = line.split(":")
                if len(parts) == 2:
                    k = parts[0].strip()
                    v = parts[1].strip().split()[0]
                    mem[k] = int(v)
            total = mem.get("MemTotal", 372224) / 1024.0
            avail = mem.get("MemAvailable", 228000) / 1024.0
            metrics["ram_total_mb"] = round(total, 1)
            metrics["ram_used_mb"] = round(total - avail, 1)
            metrics["ram_free_mb"] = round(avail, 1)
    except Exception:
        pass

    # 3. CPU Load via /proc/loadavg or stat
    try:
        with open("/proc/loadavg", "r") as f:
            load_1m = float(f.read().split()[0])
            # Normalize to single-core percentage (Pi Zero has 1 core)
            metrics["cpu_percent"] = min(100.0, round(load_1m * 100.0, 1))
    except Exception:
        pass

    # 4. Network rx/tx via /proc/net/dev on usb0
    try:
        with open("/proc/net/dev", "r") as f:
            for line in f:
                if "usb0:" in line:
                    parts = line.split(":")
                    if len(parts) == 2:
                        cols = parts[1].split()
                        rx_bytes = int(cols[0])
                        tx_bytes = int(cols[8])
                        # We store raw bytes or calculate delta
                        metrics["rx_bytes"] = rx_bytes
                        metrics["tx_bytes"] = tx_bytes
                        break
    except Exception:
        pass

    return metrics

# HTTP Request Handler
class WebControlHandler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=WEB_DIR, **kwargs)

    def do_GET(self):
        parsed = urllib.parse.urlparse(self.path)
        
        if parsed.path == "/api/status":
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Access-Control-Allow-Origin", "*")
            self.end_headers()
            data = get_system_metrics()
            self.wfile.write(json.dumps(data).encode("utf-8"))
            return

        elif parsed.path == "/api/config":
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Access-Control-Allow-Origin", "*")
            self.end_headers()
            self.wfile.write(json.dumps(CURRENT_CONFIG).encode("utf-8"))
            return

        return super().do_GET()

    def do_POST(self):
        parsed = urllib.parse.urlparse(self.path)
        
        if parsed.path == "/api/config":
            length = int(self.headers.get("Content-Length", 0))
            body = self.rfile.read(length)
            try:
                data = json.loads(body.decode("utf-8"))
                for k, v in data.items():
                    if k in CURRENT_CONFIG:
                        CURRENT_CONFIG[k] = v
                save_config(CURRENT_CONFIG)
                
                # Signal sender via UDP control broadcast or state file
                notify_control_change(CURRENT_CONFIG)
                
                self.send_response(200)
                self.send_header("Content-Type", "application/json")
                self.send_header("Access-Control-Allow-Origin", "*")
                self.end_headers()
                self.wfile.write(json.dumps({"success": True, "config": CURRENT_CONFIG}).encode("utf-8"))
            except Exception as e:
                self.send_response(400)
                self.end_headers()
                self.wfile.write(json.dumps({"error": str(e)}).encode("utf-8"))
            return

        elif parsed.path == "/api/hud/trigger":
            CURRENT_CONFIG["last_hud_trigger"] = time.time()
            CURRENT_CONFIG["hud_visible"] = True
            save_config(CURRENT_CONFIG)
            notify_control_change({"action": "trigger_hud", "duration": 60})
            
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Access-Control-Allow-Origin", "*")
            self.end_headers()
            self.wfile.write(json.dumps({"success": True, "hud_visible": True, "duration": 60}).encode("utf-8"))
            return

        self.send_response(404)
        self.end_headers()

def notify_control_change(payload):
    # Send UDP control notification to host on 192.168.7.1:5001
    try:
        import socket
        sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        msg = json.dumps(payload).encode("utf-8")
        sock.sendto(msg, ("192.168.7.1", 5001))
        # Also broadcast locally
        sock.sendto(msg, ("127.0.0.1", 5001))
    except Exception:
        pass

def run():
    os.makedirs(WEB_DIR, exist_ok=True)
    server_address = ("", PORT)
    with socketserver.ThreadingTCPServer(server_address, WebControlHandler) as httpd:
        httpd.allow_reuse_address = True
        print(f"[*] Pi Zero Web Control Server running on port {PORT}...")
        httpd.serve_forever()

if __name__ == "__main__":
    run()
