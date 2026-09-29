#!/bin/bash
# Show Onboarding / Welcome Splash Screen on Pi Zero display
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SPLASH_IMG="/opt/ext-monitor/splash.png"

if [ ! -f "$SPLASH_IMG" ]; then
    SPLASH_IMG="$(dirname "$SCRIPT_DIR")/splash.png"
fi

if [ -f "/opt/ext-monitor/splash.raw" ] && [ -w /dev/fb0 ]; then
    echo "[*] Blitting splash.raw instantly to /dev/fb0 (<0.3s)..."
    cat /opt/ext-monitor/splash.raw > /dev/fb0
elif [ -f "$SPLASH_IMG" ]; then
    echo "[*] Exibindo tela de boas-vindas / onboarding em $SPLASH_IMG via GStreamer..."
    gst-launch-1.0 filesrc location="$SPLASH_IMG" ! pngdec ! imagefreeze ! videoconvert ! kmssink sync=false
fi

