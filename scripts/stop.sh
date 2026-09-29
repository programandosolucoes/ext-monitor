#!/bin/bash
# ==============================================================================
# ext-monitor: Clean Shutdown Script
# Terminates ext-sender, wayland-damage-pacer, and any orphan GStreamer pipelines
# ==============================================================================

echo -e "\x1b[1;33m[*] Parando ext-sender, audio streamers e damage pacers...\x1b[0m"
pkill -f "ext-sender" 2>/dev/null || true
pkill -f "wayland-damage-pacer" 2>/dev/null || true
pkill -f "ximagesrc" 2>/dev/null || true
killall -9 gst-launch-1.0 2>/dev/null || true
echo -e "\x1b[1;32m[+] ext-monitor parado com sucesso.\x1b[0m"
