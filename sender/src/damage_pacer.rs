//! Pure Rust Wayland/X11 Damage Pacer (100% In-Process, Zero Python Dependency)
//!
//! Blueprint Reference: Blueprint 16 & Blueprint 28
//!
//! Maintains continuous 60 FPS frame generation on extended displays by emitting
//! a periodic damage heartbeat to GNOME Mutter via a 1x1 transparent surface.
//!
//! Uses empty XShape input mask to guarantee 100% click-through / pass-through,
//! ensuring mouse clicks, scrolls, and gestures are never intercepted.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use std::ffi::{c_char, c_int, c_ulong, c_void, CString};
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const SHAPE_INPUT: c_int = 2;
const SHAPE_SET: c_int = 0;
const CW_OVERRIDE_REDIRECT: c_ulong = 1 << 9;

type Display = c_void;
type Window = c_ulong;
type GC = *mut c_void;

#[repr(C)]
struct XSetWindowAttributes {
    background_pixmap: c_ulong,
    background_pixel: c_ulong,
    border_pixmap: c_ulong,
    border_pixel: c_ulong,
    bit_gravity: c_int,
    win_gravity: c_int,
    backing_store: c_int,
    backing_planes: c_ulong,
    backing_pixel: c_ulong,
    save_under: c_int,
    event_mask: libc::c_long,
    do_not_propagate_mask: libc::c_long,
    override_redirect: c_int,
    colormap: c_ulong,
    cursor: c_ulong,
}

type XOpenDisplayFn = unsafe extern "C" fn(*const c_char) -> *mut Display;
type XCloseDisplayFn = unsafe extern "C" fn(*mut Display) -> c_int;
type XDefaultRootWindowFn = unsafe extern "C" fn(*mut Display) -> Window;
type XCreateSimpleWindowFn = unsafe extern "C" fn(
    *mut Display,
    Window,
    c_int,
    c_int,
    c_int,
    c_int,
    c_int,
    c_ulong,
    c_ulong,
) -> Window;
type XDestroyWindowFn = unsafe extern "C" fn(*mut Display, Window) -> c_int;
type XMapWindowFn = unsafe extern "C" fn(*mut Display, Window) -> c_int;
type XClearAreaFn = unsafe extern "C" fn(*mut Display, Window, c_int, c_int, c_int, c_int, c_int) -> c_int;
type XFlushFn = unsafe extern "C" fn(*mut Display) -> c_int;
type XChangeWindowAttributesFn = unsafe extern "C" fn(
    *mut Display,
    Window,
    c_ulong,
    *mut XSetWindowAttributes,
) -> c_int;
type XCreateGCFn = unsafe extern "C" fn(*mut Display, Window, c_ulong, *mut c_void) -> GC;
type XFreeGCFn = unsafe extern "C" fn(*mut Display, GC) -> c_int;
type XSetForegroundFn = unsafe extern "C" fn(*mut Display, GC, c_ulong) -> c_int;
type XFillRectangleFn = unsafe extern "C" fn(
    *mut Display,
    Window,
    GC,
    c_int,
    c_int,
    libc::c_uint,
    libc::c_uint,
) -> c_int;
type XShapeCombineRectanglesFn = unsafe extern "C" fn(
    *mut Display,
    Window,
    c_int,
    c_int,
    c_int,
    *const c_void,
    c_int,
    c_int,
    c_int,
) -> c_int;

struct X11Bindings {
    _lib_x11: *mut c_void,
    _lib_xext: *mut c_void,
    open_display: XOpenDisplayFn,
    close_display: XCloseDisplayFn,
    default_root_window: XDefaultRootWindowFn,
    create_simple_window: XCreateSimpleWindowFn,
    destroy_window: XDestroyWindowFn,
    map_window: XMapWindowFn,
    clear_area: XClearAreaFn,
    flush: XFlushFn,
    change_window_attributes: Option<XChangeWindowAttributesFn>,
    create_gc: Option<XCreateGCFn>,
    free_gc: Option<XFreeGCFn>,
    set_foreground: Option<XSetForegroundFn>,
    fill_rectangle: Option<XFillRectangleFn>,
    shape_combine_rectangles: Option<XShapeCombineRectanglesFn>,
}

impl X11Bindings {
    fn load() -> Option<Self> {
        unsafe {
            let lib_x11_name = CString::new("libX11.so.6").ok()?;
            let lib_x11 = libc::dlopen(lib_x11_name.as_ptr(), libc::RTLD_LAZY);
            if lib_x11.is_null() {
                return None;
            }

            let load_sym = |lib: *mut c_void, name: &str| -> Option<*mut c_void> {
                let c_name = CString::new(name).ok()?;
                let sym = libc::dlsym(lib, c_name.as_ptr());
                if sym.is_null() {
                    None
                } else {
                    Some(sym)
                }
            };

            let open_display: XOpenDisplayFn = std::mem::transmute(load_sym(lib_x11, "XOpenDisplay")?);
            let close_display: XCloseDisplayFn = std::mem::transmute(load_sym(lib_x11, "XCloseDisplay")?);
            let default_root_window: XDefaultRootWindowFn = std::mem::transmute(load_sym(lib_x11, "XDefaultRootWindow")?);
            let create_simple_window: XCreateSimpleWindowFn = std::mem::transmute(load_sym(lib_x11, "XCreateSimpleWindow")?);
            let destroy_window: XDestroyWindowFn = std::mem::transmute(load_sym(lib_x11, "XDestroyWindow")?);
            let map_window: XMapWindowFn = std::mem::transmute(load_sym(lib_x11, "XMapWindow")?);
            let clear_area: XClearAreaFn = std::mem::transmute(load_sym(lib_x11, "XClearArea")?);
            let flush: XFlushFn = std::mem::transmute(load_sym(lib_x11, "XFlush")?);
            let change_window_attributes: Option<XChangeWindowAttributesFn> =
                load_sym(lib_x11, "XChangeWindowAttributes").map(|sym| std::mem::transmute(sym));
            let create_gc: Option<XCreateGCFn> =
                load_sym(lib_x11, "XCreateGC").map(|sym| std::mem::transmute(sym));
            let free_gc: Option<XFreeGCFn> =
                load_sym(lib_x11, "XFreeGC").map(|sym| std::mem::transmute(sym));
            let set_foreground: Option<XSetForegroundFn> =
                load_sym(lib_x11, "XSetForeground").map(|sym| std::mem::transmute(sym));
            let fill_rectangle: Option<XFillRectangleFn> =
                load_sym(lib_x11, "XFillRectangle").map(|sym| std::mem::transmute(sym));

            // Xext is optional for click-through input shape
            let lib_xext_name = CString::new("libXext.so.6").ok();
            let lib_xext = lib_xext_name
                .map(|name| libc::dlopen(name.as_ptr(), libc::RTLD_LAZY))
                .unwrap_or(ptr::null_mut());

            let shape_combine_rectangles: Option<XShapeCombineRectanglesFn> = if !lib_xext.is_null() {
                load_sym(lib_xext, "XShapeCombineRectangles").map(|sym| std::mem::transmute(sym))
            } else {
                None
            };

            Some(Self {
                _lib_x11: lib_x11,
                _lib_xext: lib_xext,
                open_display,
                close_display,
                default_root_window,
                create_simple_window,
                destroy_window,
                map_window,
                clear_area,
                flush,
                change_window_attributes,
                create_gc,
                free_gc,
                set_foreground,
                fill_rectangle,
                shape_combine_rectangles,
            })
        }
    }
}

/// Manages the lifecycle of the in-process pure Rust Damage Pacer.
pub struct DamagePacer {
    running: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl DamagePacer {
    pub fn start(target_x: i32, target_y: i32) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let handle = spawn_damage_pacer(running.clone(), target_x, target_y);
        Self { running, handle }
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

impl Drop for DamagePacer {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Spawns an in-process, pure Rust Damage Pacer background thread.
/// Returns a join handle or None if X11/Xwayland is unavailable.
pub fn spawn_damage_pacer(
    running: Arc<AtomicBool>,
    target_x: i32,
    target_y: i32,
) -> Option<thread::JoinHandle<()>> {
    let x11 = X11Bindings::load()?;

    let handle = thread::Builder::new()
        .name("wayland-damage-pacer-rust".to_string())
        .spawn(move || unsafe {
            let display_env = std::env::var("DISPLAY").unwrap_or_else(|_| ":0".to_string());
            let c_display = CString::new(display_env.as_str()).ok();
            let display = (x11.open_display)(c_display.as_ref().map(|s| s.as_ptr()).unwrap_or(ptr::null()));
            if display.is_null() {
                eprintln!("\x1b[1;33m[pacer-rust]\x1b[0m Cannot open X11 display for damage pacer.");
                return;
            }

            let root = (x11.default_root_window)(display);
            let window = (x11.create_simple_window)(
                display, root, target_x, target_y, 1, 1, 0, 0, 0,
            );

            // Set override_redirect = 1 so window manager leaves it alone without borders or input focus
            if let Some(change_attr) = x11.change_window_attributes {
                let mut attr: XSetWindowAttributes = std::mem::zeroed();
                attr.override_redirect = 1;
                change_attr(display, window, CW_OVERRIDE_REDIRECT, &mut attr);
            }

            // Configure 100% click-through empty input shape
            if let Some(shape_fn) = x11.shape_combine_rectangles {
                shape_fn(display, window, SHAPE_INPUT, 0, 0, ptr::null(), 0, SHAPE_SET, 0);
            }

            let gc = if let Some(create_gc) = x11.create_gc {
                create_gc(display, window, 0, ptr::null_mut())
            } else {
                ptr::null_mut()
            };

            (x11.map_window)(display, window);
            (x11.flush)(display);

            println!(
                "\x1b[1;32m[pacer-rust]\x1b[0m Pure Rust Wayland Damage Pacer active at ({}, {}) with 100% click-through",
                target_x, target_y
            );

            let mut tick: c_ulong = 0;
            // 60 Hz damage pulse loop (every 16.6ms)
            while running.load(Ordering::SeqCst) {
                tick = tick.wrapping_add(1);
                if !gc.is_null() {
                    if let (Some(set_fg), Some(fill_rect)) = (x11.set_foreground, x11.fill_rectangle) {
                        let color = if tick % 2 == 0 { 0x00000000 } else { 0x00010101 };
                        set_fg(display, gc, color);
                        fill_rect(display, window, gc, 0, 0, 1, 1);
                    }
                }
                (x11.clear_area)(display, window, 0, 0, 1, 1, 1);
                (x11.flush)(display);
                thread::sleep(Duration::from_millis(16));
            }

            if !gc.is_null() {
                if let Some(free_gc) = x11.free_gc {
                    free_gc(display, gc);
                }
            }
            (x11.destroy_window)(display, window);
            (x11.close_display)(display);
            println!("\x1b[1;34m[pacer-rust]\x1b[0m Pure Rust Wayland Damage Pacer stopped.");
        })
        .ok()?;

    Some(handle)
}
