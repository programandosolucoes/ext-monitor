#!/bin/bash
# ==============================================================================
# USB Serial Console & Remote Reconfiguration Tool for Raspberry Pi Zero
#
# Connects directly to the Pi Zero via USB CDC-ACM (/dev/ttyACM0)
# ZERO IP / ZERO NETWORK REQUIRED - 100% Hardware USB Serial
#
# Usage:
#   ./scripts/serial-console.sh              # Interactive Serial Console Shell
#   ./scripts/serial-console.sh "command"    # Execute command directly on Pi Zero
#
# License: MIT
# Author: Carlos Alberto <psncarlosalberto4ti@gmail.com>
# ==============================================================================

set -e

PORT="${SERIAL_PORT:-/dev/ttyACM0}"
BAUD="${SERIAL_BAUD:-115200}"

if [ ! -c "$PORT" ]; then
    # Try finding any ttyACM or ttyUSB device
    FOUND_PORT=$(ls /dev/ttyACM* 2>/dev/null | head -n 1)
    if [ -n "$FOUND_PORT" ]; then
        PORT="$FOUND_PORT"
    else
        echo -e "\x1b[1;31m[!] Error: Pi Zero USB Serial Port not found ($PORT).\x1b[0m"
        echo -e "    Ensure the micro-USB cable is connected to the center OTG port."
        exit 1
    fi
fi

# Ensure user has read/write permissions
if [ ! -r "$PORT" ] || [ ! -w "$PORT" ]; then
    sudo chmod 666 "$PORT" 2>/dev/null || true
fi

# If command argument provided, execute non-interactively over USB serial
if [ -n "$1" ]; then
    python3 -c "
import serial, time, sys
port, baud, cmd = sys.argv[1], int(sys.argv[2]), sys.argv[3]
try:
    s = serial.Serial(port, baud, timeout=2)
    s.write(b'\n' + cmd.encode() + b'\n')
    time.sleep(0.4)
    out = b''
    while s.in_waiting:
        out += s.read(s.in_waiting)
        time.sleep(0.05)
    print(out.decode(errors='replace').strip())
    s.close()
except Exception as e:
    sys.stderr.write(f'Serial error: {e}\n')
    sys.exit(1)
" "$PORT" "$BAUD" "$*"
    exit 0
fi

echo -e "\x1b[1;32m========================================================================\x1b[0m"
echo -e "\x1b[1;32m  Raspberry Pi Zero USB Serial Console (Hardware OTG CDC-ACM)          \x1b[0m"
echo -e "\x1b[1;34m  Port: $PORT | Baud: $BAUD baud | Zero IP / Zero Network Required     \x1b[0m"
echo -e "\x1b[1;33m  To exit: Press Ctrl+] or Ctrl+A then K                               \x1b[0m"
echo -e "\x1b[1;32m========================================================================\x1b[0m\n"

# Check for terminal tools
if command -v picocom >/dev/null 2>&1; then
    exec picocom -b "$BAUD" -q -l "$PORT"
elif command -v screen >/dev/null 2>&1; then
    exec screen "$PORT" "$BAUD"
elif command -v minicom >/dev/null 2>&1; then
    exec minicom -D "$PORT" -b "$BAUD"
else
    # Built-in zero-dependency Python interactive serial terminal with raw mode
    python3 -c "
import serial, sys, tty, termios, select, os

port = '$PORT'
baud = $BAUD

try:
    s = serial.Serial(port, baud, timeout=0.1)
except Exception as e:
    print(f'Error opening {port}: {e}')
    sys.exit(1)

old_settings = termios.tcgetattr(sys.stdin)
try:
    tty.setraw(sys.stdin.fileno())
    s.write(b'\n')
    while True:
        r, _, _ = select.select([sys.stdin, s], [], [])
        if sys.stdin in r:
            ch = os.read(sys.stdin.fileno(), 1024)
            if b'\x1d' in ch: # Ctrl + ]
                break
            s.write(ch)
        if s in r:
            data = s.read(s.in_waiting or 1)
            os.write(sys.stdout.fileno(), data)
finally:
    termios.tcsetattr(sys.stdin, termios.TCSADRAIN, old_settings)
    s.close()
    print('\n[+] USB Serial connection closed.')
"
fi
