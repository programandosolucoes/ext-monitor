#!/usr/bin/env python3
"""
Wayland Damage Pacer for ext-monitor
Forces GNOME Mutter/Clutter to maintain a continuous 60 FPS frame clock on HDMI-1
by invalidating a tiny 1x1 transparent sub-surface.
Eliminates quiescence/sleep state when mouse is stationary or outside the screen.
"""

import os
import sys

# Force X11 backend so Xwayland assigns exact multi-monitor coordinates
os.environ['GDK_BACKEND'] = 'x11'

import gi
gi.require_version('Gtk', '3.0')
from gi.repository import Gtk, Gdk, GLib

class DamagePacer(Gtk.Window):
    def __init__(self, target_x=None, target_y=None):
        super().__init__(type=Gtk.WindowType.TOPLEVEL)
        self.set_title("ext-monitor-pacer")
        self.set_decorated(False)
        self.set_resizable(False)
        self.set_app_paintable(True)
        self.set_default_size(1, 1)
        self.set_skip_taskbar_hint(True)
        self.set_skip_pager_hint(True)
        self.set_keep_above(True)
        self.set_accept_focus(False)

        # Make window transparent
        screen = self.get_screen()
        visual = screen.get_rgba_visual()
        if visual and screen.is_composited():
            self.set_visual(visual)

        self.connect("draw", self.on_draw)

        # Auto-detect target monitor if coordinates not provided
        if target_x is None or target_y is None:
            display = Gdk.Display.get_default()
            target_x = 1920
            target_y = 0
            if display:
                for i in range(display.get_n_monitors()):
                    m = display.get_monitor(i)
                    geom = m.get_geometry()
                    # Find extended monitor (x > 0 or model matching HDMI/Pi)
                    if geom.x > 0:
                        target_x = geom.x
                        target_y = geom.y
                        break

        self.target_x = target_x
        self.target_y = target_y
        self.tick_count = 0

    def on_draw(self, widget, cr):
        # 1x1 100% transparent pixel
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.0)
        cr.paint()
        return False

    def start(self):
        self.show_all()
        # Move window directly into target monitor coordinate space
        self.move(self.target_x, self.target_y)
        # 60 FPS heartbeat = 16 ms
        GLib.timeout_add(16, self.pulse)

    def pulse(self):
        self.tick_count += 1
        self.queue_draw()
        return True

if __name__ == "__main__":
    x = int(sys.argv[1]) if len(sys.argv) > 1 else None
    y = int(sys.argv[2]) if len(sys.argv) > 2 else None
    pacer = DamagePacer(target_x=x, target_y=y)
    pacer.start()
    print(f"[*] Damage Pacer active on ({pacer.target_x}, {pacer.target_y}) @ 60 FPS continuous")
    sys.stdout.flush()
    try:
        Gtk.main()
    except KeyboardInterrupt:
        pass
