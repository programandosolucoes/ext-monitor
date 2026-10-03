//! Embedded Web Dashboard Frontend UI in 100% Pure Rust
//!
//! Stores the complete, self-contained HTML, CSS, JavaScript, and internationalization
//! dictionary directly in the binary's read-only data segment.
//!
//! Features:
//! - Tabbed Dashboard Interface:
//!   * Tab 1: 📊 Live Telemetry & Monitoring (SoC Temp, CPU Load, RAM, HDMI Display)
//!   * Tab 2: ⚙️ Stream Tuning & Controls (Bitrate 400-6000 kbps, FPS, Colors, Hot-Apply, Reboot)
//!   * Tab 3: 📥 Client Tools & Driver Downloads (One-liner curl install, tar.gz package, udev rules)
//!   * Tab 4: 💾 Micro-SD Card & RAM Upgrade (Physical SD mounting, firmware flash without card removal)
//!   * Tab 5: 📖 Comprehensive Manual & Operation Guides (Linux Wayland, Windows Win+K, USB Serial)
//! - Multilingual UI: English (en), Portuguese (pt), Italian (it), Chinese (zh)
//! - Interactive Reboot Modal & Hot-Apply commit actions
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

pub const DASHBOARD_HTML: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta http-equiv="Cache-Control" content="no-cache, no-store, must-revalidate, max-age=0">
    <meta http-equiv="Pragma" content="no-cache">
    <meta http-equiv="Expires" content="0">
    <title>Pi Zero Extended Monitor - Control Dashboard</title>
    <style>
        :root {
            --bg-primary: #0a0e17;
            --bg-surface: rgba(16, 23, 38, 0.85);
            --bg-surface-hover: rgba(22, 32, 54, 0.95);
            --bg-surface-border: rgba(0, 229, 255, 0.18);
            --accent-cyan: #00e5ff;
            --accent-emerald: #00ff66;
            --accent-purple: #b388ff;
            --accent-red: #ff5252;
            --accent-amber: #ffb300;
            --text-primary: #f0f6fc;
            --text-secondary: #94a3b8;
            --text-muted: #64748b;
            --radius-sm: 8px;
            --radius-md: 14px;
            --radius-lg: 20px;
            --transition: all 0.22s cubic-bezier(0.16, 1, 0.3, 1);
        }
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            background-color: var(--bg-primary);
            color: var(--text-primary);
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;
            min-height: 100vh;
            overflow-x: hidden;
            line-height: 1.5;
        }
        .glow-bg {
            position: fixed;
            top: -200px;
            left: 50%;
            transform: translateX(-50%);
            width: 900px;
            height: 450px;
            background: radial-gradient(circle, rgba(0, 229, 255, 0.08) 0%, rgba(179, 136, 255, 0.04) 50%, transparent 70%);
            pointer-events: none;
            z-index: 0;
        }
        .navbar {
            position: sticky;
            top: 0;
            z-index: 100;
            display: flex;
            justify-content: space-between;
            align-items: center;
            padding: 0.9rem 2rem;
            background: rgba(10, 14, 23, 0.92);
            backdrop-filter: blur(20px);
            border-bottom: 1px solid var(--bg-surface-border);
        }
        .brand { display: flex; align-items: center; gap: 1rem; }
        .logo-icon {
            width: 36px; height: 36px;
            background: rgba(0, 229, 255, 0.12);
            border: 1px solid var(--accent-cyan);
            border-radius: var(--radius-sm);
            display: flex; align-items: center; justify-content: center;
        }
        .logo-icon .dot {
            width: 10px; height: 10px;
            background: var(--accent-emerald);
            border-radius: 50%;
            box-shadow: 0 0 12px var(--accent-emerald);
            animation: pulse 2s infinite;
        }
        @keyframes pulse {
            0% { transform: scale(0.95); box-shadow: 0 0 0 0 rgba(0, 255, 102, 0.7); }
            70% { transform: scale(1.1); box-shadow: 0 0 0 8px rgba(0, 255, 102, 0); }
            100% { transform: scale(0.95); box-shadow: 0 0 0 0 rgba(0, 255, 102, 0); }
        }
        .brand-text h1 { font-size: 1.15rem; font-weight: 700; color: #fff; }
        .brand-text p { font-size: 0.75rem; color: var(--text-secondary); }
        .nav-controls { display: flex; align-items: center; gap: 1rem; }
        .lang-flags { display: flex; align-items: center; gap: 0.35rem; }
        .flag-btn {
            background: rgba(16, 23, 38, 0.9);
            border: 1px solid var(--bg-surface-border);
            color: var(--text-primary);
            padding: 0.35rem 0.65rem;
            border-radius: var(--radius-sm);
            font-size: 0.82rem;
            font-weight: 600;
            cursor: pointer;
            outline: none;
            transition: var(--transition);
        }
        .flag-btn:hover { border-color: var(--accent-cyan); }
        .flag-btn.active {
            background: rgba(0, 229, 255, 0.22);
            border-color: var(--accent-cyan);
            color: var(--accent-cyan);
            box-shadow: 0 0 8px rgba(0, 229, 255, 0.3);
        }

        /* Tab Navigation Bar */
        .tabs-nav {
            display: flex;
            gap: 0.5rem;
            padding: 0.75rem 2rem;
            background: rgba(13, 19, 32, 0.7);
            border-bottom: 1px solid rgba(255, 255, 255, 0.06);
            overflow-x: auto;
            position: relative;
            z-index: 10;
        }
        .tab-btn {
            background: transparent;
            border: 1px solid transparent;
            color: var(--text-secondary);
            padding: 0.55rem 1.1rem;
            border-radius: var(--radius-sm);
            font-size: 0.9rem;
            font-weight: 600;
            cursor: pointer;
            transition: var(--transition);
            display: flex;
            align-items: center;
            gap: 0.5rem;
            white-space: nowrap;
        }
        .tab-btn:hover {
            color: var(--text-primary);
            background: rgba(255, 255, 255, 0.04);
        }
        .tab-btn.active {
            color: var(--accent-cyan);
            background: rgba(0, 229, 255, 0.12);
            border-color: rgba(0, 229, 255, 0.3);
        }

        /* Container & Tabs */
        .container {
            max-width: 1280px;
            margin: 1.5rem auto 3rem auto;
            padding: 0 1.5rem;
            position: relative;
            z-index: 1;
        }
        .tab-content { display: none; }
        .tab-content.active { display: block; animation: fadeIn 0.25s ease-out; }
        @keyframes fadeIn { from { opacity: 0; transform: translateY(6px); } to { opacity: 1; transform: translateY(0); } }

        /* Stat Cards */
        .stats-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(210px, 1fr));
            gap: 1rem;
            margin-bottom: 1.5rem;
        }
        .stat-card {
            background: var(--bg-surface);
            border: 1px solid var(--bg-surface-border);
            border-radius: var(--radius-md);
            padding: 1.1rem 1.3rem;
            backdrop-filter: blur(16px);
            transition: var(--transition);
        }
        .stat-card:hover { border-color: rgba(0, 229, 255, 0.4); transform: translateY(-2px); }
        .stat-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.4rem; }
        .stat-title { font-size: 0.82rem; color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.05em; }
        .stat-badge { font-size: 0.72rem; padding: 0.15rem 0.5rem; border-radius: 20px; font-weight: 600; }
        .badge-cyan { background: rgba(0, 229, 255, 0.15); color: var(--accent-cyan); border: 1px solid rgba(0, 229, 255, 0.3); }
        .badge-green { background: rgba(0, 255, 102, 0.15); color: var(--accent-emerald); border: 1px solid rgba(0, 255, 102, 0.3); }
        .badge-purple { background: rgba(179, 136, 255, 0.15); color: var(--accent-purple); border: 1px solid rgba(179, 136, 255, 0.3); }
        .badge-red { background: rgba(255, 82, 82, 0.15); color: var(--accent-red); border: 1px solid rgba(255, 82, 82, 0.3); }
        .badge-amber { background: rgba(255, 179, 0, 0.15); color: var(--accent-amber); border: 1px solid rgba(255, 179, 0, 0.3); }
        .stat-value { font-size: 1.6rem; font-weight: 700; color: #fff; }
        .stat-footer { font-size: 0.76rem; color: var(--text-muted); margin-top: 0.3rem; }

        /* General Card Layout */
        .glass-card {
            background: var(--bg-surface);
            border: 1px solid var(--bg-surface-border);
            border-radius: var(--radius-md);
            padding: 1.5rem;
            backdrop-filter: blur(16px);
            margin-bottom: 1.5rem;
        }
        .card-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 1.2rem; }
        .card-title { display: flex; align-items: center; gap: 0.6rem; font-size: 1.15rem; font-weight: 700; color: #fff; }
        .card-badge { font-size: 0.75rem; padding: 0.2rem 0.6rem; border-radius: 20px; font-weight: 600; background: rgba(255, 255, 255, 0.08); color: var(--text-secondary); }

        /* Forms & Controls */
        .control-group { margin-bottom: 1.2rem; }
        .control-label { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.5rem; font-size: 0.88rem; color: var(--text-secondary); font-weight: 600; }
        .control-value { color: var(--accent-cyan); font-weight: 700; font-family: monospace; }
        .btn-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(130px, 1fr)); gap: 0.6rem; }
        .btn-toggle {
            background: rgba(255, 255, 255, 0.04);
            border: 1px solid rgba(255, 255, 255, 0.08);
            color: var(--text-secondary);
            padding: 0.65rem 0.8rem;
            border-radius: var(--radius-sm);
            font-size: 0.85rem;
            font-weight: 600;
            cursor: pointer;
            transition: var(--transition);
            text-align: center;
        }
        .btn-toggle:hover { background: rgba(255, 255, 255, 0.08); color: #fff; }
        .btn-toggle.active {
            background: rgba(0, 229, 255, 0.15);
            border-color: var(--accent-cyan);
            color: var(--accent-cyan);
            box-shadow: 0 0 10px rgba(0, 229, 255, 0.2);
        }
        .btn-chip {
            background: rgba(255, 255, 255, 0.05);
            border: 1px solid rgba(255, 255, 255, 0.12);
            color: var(--text-secondary);
            border-radius: 4px;
            padding: 0.12rem 0.4rem;
            font-size: 0.68rem;
            font-weight: 600;
            cursor: pointer;
            transition: var(--transition);
        }
        .btn-chip:hover { background: rgba(255, 255, 255, 0.12); color: #fff; }
        .btn-chip.active {
            background: rgba(0, 255, 102, 0.2);
            border-color: var(--accent-emerald);
            color: var(--accent-emerald);
            font-weight: 700;
        }
        .slider-wrap { padding: 0.4rem 0; }
        .range-slider {
            width: 100%;
            -webkit-appearance: none;
            height: 6px;
            border-radius: 3px;
            background: rgba(255, 255, 255, 0.12);
            outline: none;
        }
        .range-slider::-webkit-slider-thumb {
            -webkit-appearance: none;
            width: 18px; height: 18px;
            border-radius: 50%;
            background: var(--accent-cyan);
            cursor: pointer;
            box-shadow: 0 0 8px var(--accent-cyan);
        }
        .slider-labels { display: flex; justify-content: space-between; font-size: 0.72rem; color: var(--text-muted); margin-top: 0.4rem; }

        /* Interactive Cyberpunk Tooltips */
        .tip-wrap {
            position: relative;
            display: inline-flex;
            align-items: center;
            cursor: help;
        }
        .tip-icon {
            display: inline-flex;
            align-items: center;
            justify-content: center;
            width: 16px;
            height: 16px;
            border-radius: 50%;
            background: rgba(0, 229, 255, 0.15);
            border: 1px solid rgba(0, 229, 255, 0.45);
            color: var(--accent-cyan);
            font-size: 10px;
            font-weight: 700;
            margin-left: 6px;
            transition: var(--transition);
            user-select: none;
        }
        .tip-wrap:hover .tip-icon {
            background: var(--accent-cyan);
            color: var(--bg-primary);
            box-shadow: 0 0 8px rgba(0, 229, 255, 0.8);
            transform: scale(1.15);
        }
        .tip-box {
            visibility: hidden;
            opacity: 0;
            position: absolute;
            bottom: calc(100% + 8px);
            left: 50%;
            transform: translateX(-50%) translateY(4px);
            width: 280px;
            padding: 0.7rem 0.85rem;
            background: rgba(12, 18, 32, 0.98);
            backdrop-filter: blur(20px);
            border: 1px solid rgba(0, 229, 255, 0.4);
            border-radius: var(--radius-sm);
            color: var(--text-primary);
            font-size: 0.78rem;
            font-weight: 400;
            line-height: 1.45;
            box-shadow: 0 10px 30px rgba(0, 0, 0, 0.75), 0 0 15px rgba(0, 229, 255, 0.2);
            z-index: 1000;
            pointer-events: none;
            transition: opacity 0.2s cubic-bezier(0.16, 1, 0.3, 1), transform 0.2s cubic-bezier(0.16, 1, 0.3, 1), visibility 0.2s;
            text-align: left;
            white-space: normal;
        }
        .tip-box::after {
            content: '';
            position: absolute;
            top: 100%;
            left: 50%;
            margin-left: -6px;
            border-width: 6px;
            border-style: solid;
            border-color: rgba(12, 18, 32, 0.98) transparent transparent transparent;
        }
        .tip-wrap:hover .tip-box {
            visibility: visible;
            opacity: 1;
            transform: translateX(-50%) translateY(0);
        }

        /* iOS / Cyberpunk Toggle Switch */
        .switch {
            position: relative;
            display: inline-block;
            width: 44px;
            height: 24px;
            flex-shrink: 0;
        }
        .switch input {
            opacity: 0;
            width: 0;
            height: 0;
        }
        .toggle-slider {
            position: absolute;
            cursor: pointer;
            top: 0; left: 0; right: 0; bottom: 0;
            background-color: rgba(255, 255, 255, 0.15);
            transition: .25s ease;
            border-radius: 24px;
            border: 1px solid rgba(255, 255, 255, 0.2);
        }
        .toggle-slider:before {
            position: absolute;
            content: "";
            height: 16px;
            width: 16px;
            left: 3px;
            bottom: 3px;
            background-color: #cbd5e1;
            transition: .25s ease;
            border-radius: 50%;
        }
        input:checked + .toggle-slider {
            background-color: var(--accent-cyan);
            border-color: var(--accent-cyan);
            box-shadow: 0 0 10px rgba(0, 229, 255, 0.4);
        }
        input:checked + .toggle-slider:before {
            transform: translateX(20px);
            background-color: #0b0f17;
        }
        .mode-card {
            transition: all 0.3s ease;
        }
        .mode-card.disabled {
            opacity: 0.45;
            border-color: rgba(255, 255, 255, 0.08) !important;
            filter: grayscale(0.7);
        }

        /* Action Buttons */
        .action-row { display: flex; gap: 0.8rem; flex-wrap: wrap; margin-top: 1rem; }
        .btn-primary {
            background: linear-gradient(135deg, rgba(0, 229, 255, 0.25) 0%, rgba(0, 255, 102, 0.2) 100%);
            border: 1px solid var(--accent-cyan);
            color: #fff;
            padding: 0.7rem 1.4rem;
            border-radius: var(--radius-sm);
            font-size: 0.9rem;
            font-weight: 700;
            cursor: pointer;
            transition: var(--transition);
            display: inline-flex;
            align-items: center;
            gap: 0.5rem;
        }
        .btn-primary:hover { background: rgba(0, 229, 255, 0.35); box-shadow: 0 0 14px rgba(0, 229, 255, 0.3); transform: translateY(-1px); }
        .btn-secondary {
            background: rgba(255, 255, 255, 0.05);
            border: 1px solid rgba(255, 255, 255, 0.15);
            color: var(--text-primary);
            padding: 0.7rem 1.2rem;
            border-radius: var(--radius-sm);
            font-size: 0.9rem;
            font-weight: 600;
            cursor: pointer;
            transition: var(--transition);
            display: inline-flex;
            align-items: center;
            gap: 0.5rem;
        }
        .btn-secondary:hover { background: rgba(255, 255, 255, 0.1); border-color: rgba(255, 255, 255, 0.3); }
        .btn-danger {
            background: rgba(255, 82, 82, 0.15);
            border: 1px solid var(--accent-red);
            color: var(--accent-red);
            padding: 0.7rem 1.2rem;
            border-radius: var(--radius-sm);
            font-size: 0.9rem;
            font-weight: 600;
            cursor: pointer;
            transition: var(--transition);
            display: inline-flex;
            align-items: center;
            gap: 0.5rem;
        }
        .btn-danger:hover { background: rgba(255, 82, 82, 0.28); box-shadow: 0 0 12px rgba(255, 82, 82, 0.3); }

        /* Code & Command Blocks */
        .cmd-box {
            background: #06090f;
            border: 1px solid rgba(255, 255, 255, 0.1);
            border-radius: var(--radius-sm);
            padding: 0.9rem 1.2rem;
            font-family: 'SFMono-Regular', Consolas, 'Liberation Mono', Menlo, monospace;
            font-size: 0.88rem;
            color: #7ee787;
            display: flex;
            justify-content: space-between;
            align-items: center;
            gap: 1rem;
            margin: 0.6rem 0;
            overflow-x: auto;
        }
        .cmd-text { user-select: all; }
        .copy-btn {
            background: rgba(255, 255, 255, 0.08);
            border: 1px solid rgba(255, 255, 255, 0.15);
            color: var(--text-primary);
            padding: 0.35rem 0.75rem;
            border-radius: var(--radius-sm);
            font-size: 0.78rem;
            font-weight: 600;
            cursor: pointer;
            transition: var(--transition);
            white-space: nowrap;
        }
        .copy-btn:hover { background: var(--accent-cyan); color: #000; border-color: var(--accent-cyan); }
        .copy-btn.copied { background: var(--accent-emerald); color: #000; border-color: var(--accent-emerald); }

        /* Download Cards */
        .download-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
            gap: 1.2rem;
            margin-top: 1rem;
        }
        .dl-card {
            background: var(--bg-surface);
            border: 1px solid var(--bg-surface-border);
            border-radius: var(--radius-md);
            padding: 1.3rem;
            display: flex;
            flex-direction: column;
            justify-content: space-between;
            transition: var(--transition);
        }
        .dl-card:hover { border-color: var(--accent-cyan); transform: translateY(-2px); }
        .dl-icon { font-size: 2rem; margin-bottom: 0.5rem; }
        .dl-title { font-size: 1.05rem; font-weight: 700; color: #fff; margin-bottom: 0.2rem; }
        .dl-desc { font-size: 0.82rem; color: var(--text-secondary); margin-bottom: 1rem; line-height: 1.4; }
        .dl-link {
            text-decoration: none;
            background: rgba(0, 229, 255, 0.12);
            border: 1px solid var(--accent-cyan);
            color: var(--accent-cyan);
            padding: 0.6rem 1rem;
            border-radius: var(--radius-sm);
            font-size: 0.85rem;
            font-weight: 700;
            text-align: center;
            transition: var(--transition);
            display: block;
        }
        .dl-link:hover { background: var(--accent-cyan); color: #000; box-shadow: 0 0 12px rgba(0, 229, 255, 0.3); }

        /* Table */
        .proto-table { width: 100%; border-collapse: collapse; font-size: 0.86rem; margin-top: 1rem; }
        .proto-table th, .proto-table td { padding: 0.8rem 1rem; text-align: left; border-bottom: 1px solid rgba(255, 255, 255, 0.08); }
        .proto-table th { background: rgba(255, 255, 255, 0.03); color: var(--text-secondary); font-size: 0.78rem; text-transform: uppercase; }
        .proto-table tr:hover td { background: rgba(255, 255, 255, 0.02); }

        /* Modal */
        .modal-overlay {
            position: fixed;
            top: 0; left: 0; right: 0; bottom: 0;
            background: rgba(0, 0, 0, 0.75);
            backdrop-filter: blur(8px);
            display: none;
            align-items: center;
            justify-content: center;
            z-index: 1000;
        }
        .modal-overlay.active { display: flex; animation: fadeIn 0.15s ease-out; }
        .modal-box {
            background: #111827;
            border: 1px solid var(--accent-red);
            border-radius: var(--radius-md);
            padding: 1.8rem;
            max-width: 440px;
            width: 90%;
            box-shadow: 0 0 30px rgba(255, 82, 82, 0.25);
        }
        .modal-title { font-size: 1.25rem; font-weight: 700; color: #fff; margin-bottom: 0.6rem; display: flex; align-items: center; gap: 0.6rem; }
        .modal-desc { font-size: 0.9rem; color: var(--text-secondary); margin-bottom: 1.4rem; line-height: 1.5; }
        .modal-actions { display: flex; justify-content: flex-end; gap: 0.8rem; }

        /* Toast */
        .toast {
            position: fixed;
            bottom: 2rem;
            right: 2rem;
            background: #101726;
            border: 1px solid var(--accent-cyan);
            border-radius: var(--radius-sm);
            padding: 0.8rem 1.4rem;
            color: #fff;
            font-weight: 600;
            font-size: 0.9rem;
            box-shadow: 0 0 20px rgba(0, 229, 255, 0.3);
            display: none;
            z-index: 1000;
        }
        .toast.show { display: block; animation: slideUp 0.3s ease-out; }
        @keyframes slideUp { from { transform: translateY(20px); opacity: 0; } to { transform: translateY(0); opacity: 1; } }

        /* Display Visualizer */
        .monitor-frame {
            background: #000;
            border: 2px solid rgba(0, 229, 255, 0.4);
            border-radius: var(--radius-sm);
            aspect-ratio: 16 / 9;
            max-height: 220px;
            display: flex;
            align-items: center;
            justify-content: center;
            position: relative;
            overflow: hidden;
            box-shadow: inset 0 0 30px rgba(0, 229, 255, 0.15);
        }
        .monitor-scanline {
            position: absolute;
            top: 0; left: 0; right: 0; height: 2px;
            background: rgba(0, 229, 255, 0.6);
            box-shadow: 0 0 8px var(--accent-cyan);
            animation: scan 3s linear infinite;
        }
        @keyframes scan { 0% { top: 0; } 100% { top: 100%; } }
        .monitor-text { text-align: center; color: var(--accent-cyan); font-family: monospace; font-size: 0.95rem; }
    </style>
</head>
<body>
    <div class="glow-bg"></div>

    <!-- Navigation Bar -->
    <header class="navbar">
        <div class="brand">
            <div class="logo-icon"><div class="dot"></div></div>
            <div class="brand-text">
                <h1 data-i18n="title">Pi Zero Extended Monitor</h1>
                <p data-i18n="subtitle">Hardware GPU VideoCore IV Display Appliance</p>
            </div>
        </div>
        <div class="nav-controls">
            <div class="lang-flags">
                <button type="button" onclick="window.location.href='/?_v=' + Date.now()" class="flag-btn" style="border-color: rgba(0, 229, 255, 0.4); color: var(--accent-cyan); display: inline-flex; align-items: center; gap: 0.35rem; font-weight: 700;" title="Limpar cache e recarregar painel">
                    <span>🔄</span> <span>Recarregar</span>
                </button>
                <button type="button" onclick="setLanguage('en')" class="flag-btn active" id="btnLang_en" title="English">🇺🇸 EN</button>
                <button type="button" onclick="setLanguage('pt')" class="flag-btn" id="btnLang_pt" title="Português">🇧🇷 PT</button>
                <button type="button" onclick="setLanguage('it')" class="flag-btn" id="btnLang_it" title="Italiano">🇮🇹 IT</button>
                <button type="button" onclick="setLanguage('zh')" class="flag-btn" id="btnLang_zh" title="中文">🇨🇳 中文</button>
            </div>
        </div>
    </header>

    <!-- Tabs Navigation Bar -->
    <nav class="tabs-nav">
        <button class="tab-btn active" onclick="switchTab('monitor')" id="tabBtn_monitor">
            <span>📊</span> <span data-i18n="tabMonitor">Monitoring & Telemetry</span>
        </button>
        <button class="tab-btn" onclick="switchTab('config')" id="tabBtn_config">
            <span>⚙️</span> <span data-i18n="tabConfig">Tuning & Settings</span>
        </button>
        <button class="tab-btn" onclick="switchTab('downloads')" id="tabBtn_downloads">
            <span>📥</span> <span data-i18n="tabDownloads">Client Tools & Drivers</span>
        </button>
        <button class="tab-btn" onclick="switchTab('sdcard')" id="tabBtn_sdcard">
            <span>💾</span> <span data-i18n="tabSdCard">SD Card & RAM Upgrade</span>
        </button>
        <button class="tab-btn" onclick="switchTab('manual')" id="tabBtn_manual">
            <span>📖</span> <span data-i18n="tabManual">Operation Manual</span>
        </button>
        <a href="/cast" target="_blank" class="tab-btn" style="text-decoration:none; display:inline-flex; align-items:center; gap:0.5rem; border: 1px solid var(--accent-cyan); background: rgba(0, 229, 255, 0.12); color: var(--accent-cyan); font-weight: bold;" id="tabBtn_cast">
            <span>📺</span> <span>Web Cast (Aba / Tela)</span>
        </a>
        <a href="/swagger" target="_blank" class="tab-btn" style="text-decoration:none; display:inline-flex; align-items:center; gap:0.5rem;" id="tabBtn_swagger">
            <span>⚡</span> <span>Swagger API</span>
        </a>
    </nav>

    <main class="container">
        <!-- ================================================================= -->
        <!-- TAB 1: MONITORAMENTO & TELEMETRIA                                 -->
        <!-- ================================================================= -->
        <section id="tab-monitor" class="tab-content active">
            <!-- Live Streaming Mode & Active HDMI Output Banner -->
            <div class="glass-card" id="activeStreamBanner" style="border: 1px solid var(--accent-cyan); background: linear-gradient(135deg, rgba(0, 229, 255, 0.08) 0%, rgba(16, 23, 38, 0.95) 100%); margin-bottom: 1.5rem; padding: 1.25rem 1.6rem; box-shadow: 0 4px 24px rgba(0, 0, 0, 0.4); transition: var(--transition);">
                <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(320px, 1fr)); gap: 1.5rem; align-items: center;">
                    <!-- Left: Active Mode Feedback -->
                    <div style="display: flex; align-items: center; gap: 1.1rem;">
                        <div id="activeModeIcon" style="font-size: 2.2rem; width: 56px; height: 56px; min-width: 56px; display: flex; align-items: center; justify-content: center; background: rgba(0, 229, 255, 0.12); border-radius: var(--radius-md); border: 1px solid var(--accent-cyan); box-shadow: 0 0 16px rgba(0, 229, 255, 0.25);">🐧</div>
                        <div>
                            <div style="display: flex; align-items: center; gap: 0.6rem; margin-bottom: 0.2rem;">
                                <span style="font-size: 0.76rem; text-transform: uppercase; letter-spacing: 0.08em; color: var(--text-secondary); font-weight: 700;" data-i18n="lblActiveMode">Active Streaming Mode</span>
                                <span id="activeModeBadge" class="stat-badge badge-green" style="animation: pulse 2s infinite;" data-i18n="badgeStreaming">● STREAMING</span>
                            </div>
                            <div id="activeModeTitle" style="font-size: 1.25rem; font-weight: 800; color: #fff; line-height: 1.25;">Mode 1: Network UDP (Linux Wayland / X11)</div>
                            <div id="activeModeDesc" style="font-size: 0.84rem; color: var(--accent-cyan); font-family: monospace; margin-top: 0.25rem;">UDP Port 5000 • Latency &lt; 15ms • VA-API/M2M Pipeline</div>
                        </div>
                    </div>
                    <!-- Right: Active HDMI Output Feedback -->
                    <div style="display: flex; align-items: center; gap: 1.1rem; border-left: 1px solid rgba(255, 255, 255, 0.08); padding-left: 1rem;">
                        <div style="font-size: 2.2rem; width: 56px; height: 56px; min-width: 56px; display: flex; align-items: center; justify-content: center; background: rgba(179, 136, 255, 0.12); border-radius: var(--radius-md); border: 1px solid var(--accent-purple); box-shadow: 0 0 16px rgba(179, 136, 255, 0.25);">📺</div>
                        <div>
                            <div style="display: flex; align-items: center; gap: 0.6rem; margin-bottom: 0.2rem;">
                                <span style="font-size: 0.76rem; text-transform: uppercase; letter-spacing: 0.08em; color: var(--text-secondary); font-weight: 700;" data-i18n="lblActiveHdmi">Active Raspberry Pi HDMI Output</span>
                                <span id="activeHdmiPortBadge" class="stat-badge badge-purple">HDMI-A-1</span>
                            </div>
                            <div id="activeHdmiTitle" style="font-size: 1.2rem; font-weight: 800; color: #fff; line-height: 1.25;">Mini-HDMI Port (HDMI-A-1)</div>
                            <div id="activeHdmiDetails" style="font-size: 0.84rem; color: var(--text-secondary); margin-top: 0.25rem;">Detecting Monitor and Resolution...</div>
                        </div>
                    </div>
                </div>
            </div>

            <!-- Hardware Arbiter & 4-Level Service Hierarchy Status -->
            <div class="glass-card" style="margin-bottom: 1.5rem; padding: 0.9rem 1.6rem; border: 1px solid rgba(179, 136, 255, 0.25); background: rgba(16, 23, 38, 0.7); display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 1rem; box-shadow: 0 4px 20px rgba(0, 0, 0, 0.35);">
                <div style="display: flex; align-items: center; gap: 0.8rem;">
                    <span style="font-size: 1.4rem;">⚖️</span>
                    <div>
                        <div style="font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.08em; color: var(--text-secondary); font-weight: 700;">Árbitro de Display HDMI & Hierarquia de Serviços</div>
                        <div id="descHierarchy" style="font-size: 0.88rem; color: #fff; font-weight: 600;">Carregando estado do árbitro...</div>
                    </div>
                </div>
                <div style="display: flex; align-items: center; gap: 0.6rem;">
                    <span id="badgeHierarchy" class="stat-badge badge-green">NÍVEL 0: DESKTOP (EXCLUSIVO)</span>
                    <span id="badgeDisplayOwner" class="stat-badge badge-purple" style="font-family: monospace;">KMS Plane</span>
                </div>
            </div>

            <!-- Stat Cards Row -->
            <div class="stats-grid">
                <div class="stat-card">
                    <div class="stat-header">
                        <span class="stat-title tip-wrap">
                            <span data-i18n="statTemp">SoC Temperature</span>
                            <span class="tip-icon">?</span>
                            <span class="tip-box" data-i18n="tipTemp">Broadcom BCM2835 internal silicon temperature. Polled every 2s. Recommended: below 65°C.</span>
                        </span>
                        <span class="stat-badge badge-green" id="badgeTemp">Normal</span>
                    </div>
                    <div class="stat-value" id="valTemp">44.9°C</div>
                    <div class="stat-footer">Broadcom BCM2835 @ 1.0 GHz</div>
                </div>
                <div class="stat-card">
                    <div class="stat-header">
                        <span class="stat-title tip-wrap">
                            <span data-i18n="statCpu">CPU Load</span>
                            <span class="tip-icon">?</span>
                            <span class="tip-box" data-i18n="tipCpu">ARM11 CPU load. Stays below 2% because H.264 decoding is 100% offloaded to VideoCore IV VPU.</span>
                        </span>
                        <span class="stat-badge badge-cyan" data-i18n="badgeVpuOffload">VPU Offload</span>
                    </div>
                    <div class="stat-value" id="valCpu">0.67%</div>
                    <div class="stat-footer" data-i18n="statCpuDesc">Hardware GPU V4L2 M2M Active</div>
                </div>
                <div class="stat-card">
                    <div class="stat-header">
                        <span class="stat-title tip-wrap">
                            <span data-i18n="statRam">Free RAM</span>
                            <span class="tip-icon">?</span>
                            <span class="tip-box" data-i18n="tipRam">Free RAM out of 512 MB SDRAM. The entire system runs in RAM (initramfs) with zero SD card wear.</span>
                        </span>
                        <span class="stat-badge badge-purple" data-i18n="badgeRamApp">100% RAM</span>
                    </div>
                    <div class="stat-value" id="valRam">318 MB</div>
                    <div class="stat-footer">512 MB SDRAM (Zero SD wear)</div>
                </div>
                <div class="stat-card">
                    <div class="stat-header">
                        <span class="stat-title tip-wrap">
                            <span data-i18n="statStream">Stream State</span>
                            <span class="tip-icon">?</span>
                            <span class="tip-box" data-i18n="tipStream">Real-time state of the video decoding engine streaming to the HDMI TV screen.</span>
                        </span>
                        <span class="stat-badge badge-green" id="badgeStream">Active</span>
                    </div>
                    <div class="stat-value" id="valState">ONLINE</div>
                    <div class="stat-footer">1280x720 @ 60 FPS (HDMI)</div>
                </div>
            </div>



            <!-- Dynamic Displays Container (Renders a section per HDMI/DP video output) -->
            <div id="displaysSectionContainer" style="margin-bottom: 1.5rem;">
                <div class="glass-card" id="displayCard_default">
                    <div class="card-header">
                        <div class="card-title">
                            <span>📺</span>
                            <span data-i18n="displayHeader">HDMI Television & Display Telemetry</span>
                        </div>
                        <span class="card-badge badge-cyan" id="monitorBadge">HDMI-A-1 • Detecting...</span>
                    </div>
                    <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 1.5rem; align-items: center;">
                        <div class="monitor-frame">
                            <div class="monitor-scanline"></div>
                            <div class="monitor-text">
                                <p id="monitorName" style="font-size: 1.4rem; font-weight: 700;">🖥️ Detecting Monitor...</p>
                                <p id="monitorVpu" style="margin-top: 0.3rem;">VideoCore IV Hardware VPU</p>
                                <p id="monitorStatus" style="color: #7ee787; margin-top: 0.2rem;">● LIVE ZERO-COPY 60 FPS</p>
                            </div>
                        </div>
                        <div>
                            <p style="color: var(--text-secondary); font-size: 0.9rem; line-height: 1.5; margin-bottom: 1rem;" data-i18n="displayDesc">
                                The VideoCore IV hardware VPU decodes H.264 video streams directly to the HDMI scanout plane without touching the CPU.
                            </p>
                            <div class="action-row">
                                <button id="btnTriggerHud" class="btn-primary" onclick="triggerHud(true)" data-i18n="btnShowHud">✦ Show HUD on TV (60s)</button>
                                <button id="btnHideHud" class="btn-danger" onclick="triggerHud(false)" data-i18n="btnHideHud">✕ Turn Off HUD</button>
                            </div>
                        </div>
                    </div>
                </div>
            </div>

            <!-- Appliance Listener Daemons & Service Publishing -->
            <div class="glass-card">
                <div class="card-header">
                    <div class="card-title">
                        <span>📡</span>
                        <span data-i18n="servicesHeader">Appliance Listener Daemons & Services</span>
                    </div>
                    <span class="card-badge badge-purple" data-i18n="servicesBadge">Hardware Listeners</span>
                </div>
                <div class="btn-grid" style="grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));">
                    <div class="dl-card mode-card" id="cardMode1" style="border-color: var(--accent-cyan); cursor: pointer;" onclick="if(!event.target.closest('.switch') && !event.target.closest('button')) activateModeWithTopology('mode1_udp', currentTopology)">
                        <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.6rem;">
                            <div class="dl-title" style="margin: 0;">🐧 Mode 1: Linux Wayland</div>
                            <label class="switch" title="Toggle Mode 1">
                                <input type="checkbox" id="toggleMode1" checked onchange="toggleMode('mode1', this.checked)">
                                <span class="toggle-slider"></span>
                            </label>
                        </div>
                        <div class="dl-desc" data-i18n="m1Desc">Direct low-latency RTP H.264 stream on UDP port 5000 with AMD VA-API zero-copy offload (&lt; 15ms).</div>
                        <div style="display: flex; flex-direction: column; gap: 0.5rem; margin-top: 0.8rem;">
                            <div style="display: flex; justify-content: space-between; align-items: center;">
                                <span id="badgeMode1" class="stat-badge badge-cyan" data-i18n="badgeMode1On">Enabled (UDP 5000)</span>
                                <span style="font-size: 0.75rem; color: var(--text-muted); font-weight: 600;">Linux Wayland</span>
                            </div>
                            <button class="btn-toggle active" id="btnMode1Connect" style="padding: 0.5rem; font-size: 0.85rem; border-color: var(--accent-cyan); color: var(--accent-cyan); width: 100%;" onclick="activateModeWithTopology('mode1_udp', currentTopology); event.stopPropagation();">
                                ▶ Iniciar Modo 1 (Rede UDP)
                            </button>
                        </div>
                    </div>
                    <div class="dl-card mode-card" id="cardMode2" style="border-color: var(--accent-emerald); cursor: pointer;" onclick="if(!event.target.closest('.switch') && !event.target.closest('button')) activateModeWithTopology('mode2_miracast', 'miracast')">
                        <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.6rem;">
                            <div class="dl-title" style="margin: 0;">🪟 Mode 2: Windows Miracast</div>
                            <label class="switch" title="Toggle Mode 2">
                                <input type="checkbox" id="toggleMode2" checked onchange="toggleMode('mode2', this.checked)">
                                <span class="toggle-slider"></span>
                            </label>
                        </div>
                        <div class="dl-desc" data-i18n="m2Desc">Native Windows 10/11 wireless projection via Win + K on RTSP port 7236. Zero host drivers needed.</div>
                        <div style="display: flex; flex-direction: column; gap: 0.5rem; margin-top: 0.8rem;">
                            <div style="display: flex; justify-content: space-between; align-items: center;">
                                <span id="badgeMode2" class="stat-badge badge-green" data-i18n="badgeMode2On">Enabled (TCP 7236)</span>
                                <span style="font-size: 0.75rem; color: var(--text-muted); font-weight: 600;">Windows & Linux</span>
                            </div>
                            <button class="btn-toggle tip-wrap" id="btnMode2Connect" style="padding: 0.45rem 0.5rem; font-size: 0.82rem; border-radius: var(--radius-sm); border-color: var(--accent-emerald); color: var(--accent-emerald);" onclick="activateModeWithTopology('mode2_miracast', 'miracast'); event.stopPropagation();" title="Exec=env GST_PLUGIN_FEATURE_RANK=vaapih264enc:MAX,vah264enc:MAX gnome-network-displays" data-i18n="btnSelectMode2">
                                🪟 Conectar Miracast (Win+K / Linux GPU)
                            </button>
                            <div class="gpu-miracast-box" style="padding: 0.5rem 0.6rem; background: rgba(0, 255, 102, 0.05); border: 1px solid rgba(0, 255, 102, 0.25); border-radius: var(--radius-sm); font-size: 0.74rem;">
                                <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.35rem;">
                                    <span class="tip-wrap" style="cursor: help;">
                                        <span style="font-weight: 700; color: var(--accent-emerald); font-size: 0.76rem;">🚀 GPU Host (AMD / Intel / NVIDIA)</span>
                                        <span class="tip-icon" style="margin-left: 4px;">?</span>
                                        <span class="tip-box" style="width: 320px; font-family: monospace; font-size: 0.73rem; text-align: left; line-height: 1.4; left: 0;">
                                            <strong>🚀 Comandos de Aceleração por GPU:</strong><br>
                                            • <strong>AMD:</strong> <code>env GST_PLUGIN_FEATURE_RANK=vaapih264enc:MAX,vah264enc:MAX gnome-network-displays</code><br>
                                            • <strong>Intel:</strong> <code>env GST_PLUGIN_FEATURE_RANK=vaapih264enc:MAX,vah264enc:MAX,qsvh264enc:MAX gnome-network-displays</code><br>
                                            • <strong>NVIDIA:</strong> <code>env GST_PLUGIN_FEATURE_RANK=nvh264enc:MAX,vaapih264enc:MAX gnome-network-displays</code><br>
                                            • <strong>Auto:</strong> <code>env GST_PLUGIN_FEATURE_RANK=vaapih264enc:MAX,vah264enc:MAX,nvh264enc:MAX,qsvh264enc:MAX gnome-network-displays</code>
                                        </span>
                                    </span>
                                    <button class="btn-micro" style="padding: 0.15rem 0.4rem; font-size: 0.7rem; border-radius: 4px; border: 1px solid var(--accent-emerald); background: rgba(0,255,102,0.15); color: var(--accent-emerald); cursor: pointer;" onclick="copyGpuLaunchCommand(); event.stopPropagation();" title="Copiar comando">📋 Copiar</button>
                                </div>
                                <div style="font-family: monospace; font-size: 0.70rem; color: #a3e635; word-break: break-all; user-select: all; padding: 0.25rem 0.35rem; background: rgba(0,0,0,0.35); border-radius: 4px;" id="gpuLaunchCmdPreview">
                                    env GST_PLUGIN_FEATURE_RANK=vaapih264enc:MAX,vah264enc:MAX gnome-network-displays
                                </div>
                                <div style="display: flex; gap: 0.25rem; margin-top: 0.35rem;">
                                    <button class="btn-chip active" id="chipGpuAmd" onclick="selectGpuCmd('amd'); event.stopPropagation();">AMD</button>
                                    <button class="btn-chip" id="chipGpuIntel" onclick="selectGpuCmd('intel'); event.stopPropagation();">Intel</button>
                                    <button class="btn-chip" id="chipGpuNvidia" onclick="selectGpuCmd('nvidia'); event.stopPropagation();">NVIDIA</button>
                                    <button class="btn-chip" id="chipGpuAll" onclick="selectGpuCmd('all'); event.stopPropagation();">Auto</button>
                                </div>
                            </div>
                        </div>
                    </div>
                    <div class="dl-card mode-card" id="cardMode3" style="border-color: var(--accent-purple); cursor: pointer;" onclick="if(!event.target.closest('.switch') && !event.target.closest('button')) activateModeWithTopology('mode3_usb_bulk', currentTopology)">
                        <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.6rem;">
                            <div class="dl-title" style="margin: 0;">⚡ Mode 3: USB Bulk Direct</div>
                            <label class="switch" title="Toggle Mode 3">
                                <input type="checkbox" id="toggleMode3" checked onchange="toggleMode('mode3', this.checked)">
                                <span class="toggle-slider"></span>
                            </label>
                        </div>
                        <div class="dl-desc" data-i18n="m3Desc">Direct 480 Mbps raw hardware pipe via USB FunctionFS without network stack overhead (&lt; 1ms).</div>
                        <div style="display: flex; flex-direction: column; gap: 0.5rem; margin-top: 0.8rem;">
                            <div style="display: flex; justify-content: space-between; align-items: center;">
                                <span id="badgeMode3" class="stat-badge badge-purple" data-i18n="badgeMode3On">Enabled (USB Bulk)</span>
                                <span style="font-size: 0.75rem; color: var(--text-muted); font-weight: 600;">USB FunctionFS</span>
                            </div>
                            <button class="btn-toggle" id="btnMode3Connect" style="padding: 0.5rem; font-size: 0.85rem; border-color: var(--accent-purple); color: var(--accent-purple); width: 100%;" onclick="activateModeWithTopology('mode3_usb_bulk', currentTopology); event.stopPropagation();">
                                ⚡ Iniciar Modo 3 (USB Bulk Direct)
                            </button>
                        </div>
                    </div>
                </div>
            </div>

            <!-- Hi-Res Digital Audio & Hardware DAC Card -->
            <!-- Hi-Res Digital Audio & Hardware DAC Card with Integrated Streaming Controls -->
            <div class="glass-card" id="cardAudioDac" style="margin-top: 1.5rem; margin-bottom: 1.5rem; border: 1px solid rgba(179, 136, 255, 0.4); background: linear-gradient(135deg, rgba(16, 23, 38, 0.95) 0%, rgba(22, 17, 40, 0.98) 100%);">
                <div class="card-header" style="margin-bottom: 0.8rem;">
                    <div class="card-title">
                        <span>🔊</span>
                        <span data-i18n="audioHeader">Áudio Digital HDMI, DAC de Alta Fidelidade & Transmissão Integrada</span>
                    </div>
                    <div style="display: flex; gap: 0.5rem; align-items: center;">
                        <span class="card-badge badge-purple" id="badgeAudioStatus" data-i18n="audioBadge">ALSA Hardware PCM</span>
                        <span class="card-badge badge-cyan" id="badgeIntegratedState">Modo 3: USB Bulk</span>
                    </div>
                </div>
                <p style="color: var(--text-secondary); font-size: 0.88rem; margin-bottom: 1.2rem; line-height: 1.4;" data-i18n="audioCardDesc">
                    Controle centralizado e integrado: selecione o canal de transmissão ativo (Modo 1, 2 ou 3), a topologia de tela (Estender ou Clonar), ajuste o volume de áudio digital, gerencie o visualizador de espectro FFT e configure o clock mestre do hardware ALSA HDMI.
                </p>

                <!-- 1. Central Integrated Transmission Mode (Mode 1, 2, 3 & Standby) -->
                <div class="control-group" style="margin-bottom: 1.2rem; padding: 1rem; background: rgba(0, 0, 0, 0.25); border-radius: var(--radius-sm); border: 1px solid rgba(255, 255, 255, 0.06);">
                    <div class="control-label">
                        <span class="tip-wrap">
                            <span data-i18n="audioModeLabel">Canal de Transmissão Ativo (Vídeo + Áudio HDMI)</span>
                            <span class="tip-icon">?</span>
                            <span class="tip-box" data-i18n="tipAudioMode">Alterna o canal físico de streaming entre o computador e o Raspberry Pi. Troca a quente sem reiniciar o sistema.</span>
                        </span>
                        <span class="control-value" id="valIntegratedMode">⚡ Modo 3: USB Bulk Direct (&lt; 1ms)</span>
                    </div>
                    <div class="btn-grid" id="integratedModeGrid" style="grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));">
                        <button class="btn-toggle" id="btnTransport_mode1" onclick="setActiveTransport('mode1_udp')">
                            <div style="font-weight: 700; margin-bottom: 0.2rem;">🐧 Modo 1: Rede UDP</div>
                            <div style="font-size: 0.74rem; opacity: 0.8;">Porta 5000/5004 • &lt; 15ms</div>
                        </button>
                        <button class="btn-toggle" id="btnTransport_mode2" onclick="setActiveTransport('mode2_miracast')">
                            <div style="font-weight: 700; margin-bottom: 0.2rem;">🪟 Modo 2: Miracast</div>
                            <div style="font-size: 0.74rem; opacity: 0.8;">Win+K / Linux • TCP 7236</div>
                        </button>
                        <button class="btn-toggle active" id="btnTransport_mode3" onclick="setActiveTransport('mode3_usb_bulk')">
                            <div style="font-weight: 700; margin-bottom: 0.2rem;">⚡ Modo 3: USB Bulk Direct</div>
                            <div style="font-size: 0.74rem; opacity: 0.8;">FunctionFS 480Mbps • &lt; 1ms</div>
                        </button>
                        <button class="btn-toggle" id="btnActionStop" onclick="setExtensionAction('stop')" style="border-color: rgba(255, 82, 82, 0.6);">
                            <div style="font-weight: 700; margin-bottom: 0.2rem; color: #ff5252;">⏹️ Standby / Parar</div>
                            <div style="font-size: 0.74rem; opacity: 0.8;">Tela de Prontidão e Silêncio</div>
                        </button>
                    </div>
                </div>

                <!-- 2. Screen Topology Selector (Extend / Clone) -->
                <div class="control-group" style="margin-bottom: 1.2rem; padding: 1rem; background: rgba(0, 0, 0, 0.25); border-radius: var(--radius-sm); border: 1px solid rgba(255, 255, 255, 0.06);">
                    <div class="control-label">
                        <span class="tip-wrap">
                            <span data-i18n="audioTopologyLabel">Topologia de Tela no Laptop (Wayland / Mutter)</span>
                            <span class="tip-icon">?</span>
                            <span class="tip-box" data-i18n="tipAudioTopology">Estender a área de trabalho para a TV como segundo monitor virtual (HDMI-1) ou espelhar a tela principal do notebook (eDP-1).</span>
                        </span>
                        <span class="control-value" id="valIntegratedTopology">🖥️ Estendida (HDMI-1)</span>
                    </div>
                    <div class="btn-grid" id="integratedTopologyGrid" style="grid-template-columns: 1fr 1fr;">
                        <button class="btn-toggle active" id="btnActionExtend" onclick="setExtensionAction('extend')">
                            <div style="font-weight: 700; margin-bottom: 0.2rem;">🖥️ Estender Área de Trabalho</div>
                            <div style="font-size: 0.74rem; opacity: 0.8;">Segundo monitor virtual na TV (HDMI-1)</div>
                        </button>
                        <button class="btn-toggle" id="btnActionClone" onclick="setExtensionAction('clone')">
                            <div style="font-weight: 700; margin-bottom: 0.2rem;">💻 Clonar Tela do Notebook</div>
                            <div style="font-size: 0.74rem; opacity: 0.8;">Espelhar display principal (eDP-1)</div>
                        </button>
                    </div>
                </div>

                <!-- 3. Simultaneous Audio Streaming Toggle & HDMI Output Capability -->
                <div class="control-group" style="margin-bottom: 1.2rem; padding: 1rem; background: rgba(0, 0, 0, 0.25); border-radius: var(--radius-sm); border: 1px solid rgba(255, 255, 255, 0.06);">
                    <div style="display: flex; justify-content: space-between; align-items: center;">
                        <div style="display: flex; align-items: center; gap: 0.6rem;">
                            <span style="font-size: 1.25rem;">🔊</span>
                            <div>
                                <div style="font-weight: 700; font-size: 0.95rem;">Transmitir Áudio Simultaneamente para a TV</div>
                                <div style="font-size: 0.76rem; color: var(--text-secondary); margin-top: 0.15rem;">
                                    Desacoplado por padrão: extensão e clone operam com vídeo puro. Ative aqui para rotear o som do PC simultaneamente via HDMI.
                                </div>
                            </div>
                        </div>
                        <label class="switch" style="flex-shrink: 0; margin-left: 1rem;" title="Ativar Som Simultaneamente">
                            <input type="checkbox" id="toggleSimultaneousAudio" onchange="onSimultaneousAudioToggle(this.checked)">
                            <span class="toggle-slider"></span>
                        </label>
                    </div>
                    <div style="display: flex; justify-content: space-between; align-items: center; margin-top: 0.6rem; padding-top: 0.6rem; border-top: 1px solid rgba(255, 255, 255, 0.05);">
                        <span style="font-size: 0.78rem; color: var(--text-muted);">Capacidade de Saída de Áudio:</span>
                        <span id="badgeHdmiAudioCapability" class="stat-badge badge-green">✓ Saída HDMI com suporte a Áudio Digital</span>
                    </div>
                </div>

                <!-- 4. Audio Volume Slider & Action Row -->
                <div class="control-group">
                    <div class="control-label">
                        <span class="tip-wrap">
                            <span data-i18n="audioLabel">HDMI Digital Audio (Opus 48kHz / PCM)</span>
                            <span class="tip-icon">?</span>
                            <span class="tip-box" data-i18n="tipAudio">Digital audio volume sent to monitor/TV via HDMI cable. Sub-25ms latency with A/V sync.</span>
                        </span>
                        <span class="control-value" id="valAudioVolume">100%</span>
                    </div>
                    <div class="slider-wrap">
                        <input type="range" min="0" max="100" step="5" value="100" class="range-slider" id="audioVolumeSlider" oninput="updateAudioVolume(this.value)">
                        <div class="slider-labels">
                            <span>0% (Mute)</span>
                            <span>25%</span>
                            <span>50%</span>
                            <span>75%</span>
                            <span>100%</span>
                        </div>
                    </div>
                    <div class="action-row" style="margin-top: 0.75rem; display: flex; flex-wrap: wrap; gap: 0.6rem; align-items: center;">
                        <button id="btnAudioMute" class="btn-primary" onclick="toggleAudioMute()" data-i18n="btnAudioMute">🔊 Mute Audio</button>
                        <button class="btn-primary" onclick="testRealAudioSignal()" style="background: linear-gradient(135deg, #7c4dff, #00e5ff);" data-i18n="btnTestAudioChime">🔔 Test Hardware Audio Chime</button>
                    </div>
                </div>

                <!-- 4. Live Hardware Audio Spectrum Visualizer (30 FPS Canvas) -->
                <div class="control-group">
                    <div class="control-label">
                        <span class="tip-wrap">
                            <span data-i18n="mediaVisLabel">Live Hardware Audio Spectrum (HDMI Telemetry)</span>
                            <span class="tip-icon">?</span>
                            <span class="tip-box" data-i18n="tipMediaVis">Real-time 24-band frequency spectrum and VU meter generated from ALSA hardware PCM audio stream. Prevents HDMI screen sleeping.</span>
                        </span>
                        <span class="control-value" id="valVisualizerState" style="color: #7ee787;">Active (30 FPS)</span>
                    </div>
                    <canvas id="audioVisualizerCanvas" width="680" height="90" style="width: 100%; max-width: 680px; height: 90px; background: rgba(5,8,16,0.7); border-radius: 8px; border: 1px solid rgba(179,136,255,0.3); margin: 0.6rem 0; display: block;"></canvas>
                    <div class="btn-grid" style="grid-template-columns: 1fr 1fr; margin-top: 0.4rem;">
                        <button class="btn-toggle active" id="btnVisOn" onclick="toggleVisualizer(true)" style="border-color: #b388ff; color: #b388ff;" data-i18n="btnVisOn">🎨 Enable HDMI Visualizer</button>
                        <button class="btn-toggle" id="btnVisOff" onclick="toggleVisualizer(false)" data-i18n="btnVisOff">⏹ Disable Visualizer</button>
                    </div>
                </div>

                <!-- 5. HDMI Audio Hardware Profiles & Master Clock -->
                <div class="control-group" style="margin-bottom: 0;">
                    <div class="control-label">
                        <span class="tip-wrap">
                            <span data-i18n="audioProfileLabel">HDMI Master Audio Profile & Sample Rate</span>
                            <span class="tip-icon">?</span>
                            <span class="tip-box" data-i18n="tipAudioProfile">Select and force-load the hardware sample rate clock directly into the Pi Zero BCM2835 ALSA sound core. Supports true IEC958 subframe audio up to 192kHz 24-bit Hi-Res.</span>
                        </span>
                        <span class="control-value" id="valAudioRate">96 kHz (Hi-Res Studio - Default)</span>
                    </div>
                    <div class="btn-grid" id="audioRateGrid">
                        <button class="btn-toggle active" id="btnRate96k" onclick="setAudioRate(96000)" data-rate="96000">🎵 Hi-Res Studio (96 kHz / 24-bit)</button>
                        <button class="btn-toggle" id="btnRate192k" onclick="setAudioRate(192000)" data-rate="192000">🚀 Ultra Hi-Res (192 kHz / 24-bit)</button>
                        <button class="btn-toggle" id="btnRate48k" onclick="setAudioRate(48000)" data-rate="48000">🎬 Cinema Standard (48 kHz / 16-bit)</button>
                        <button class="btn-toggle" id="btnRate44k" onclick="setAudioRate(44100)" data-rate="44100">💿 CD Fidelity (44.1 kHz / 16-bit)</button>
                    </div>
                </div>
            </div>

            <!-- Web Sharing & Chromecast Card -->
            <div class="glass-card" id="cardChromecast" style="margin-bottom: 1.5rem; border: 1px solid rgba(0, 229, 255, 0.4); background: linear-gradient(135deg, rgba(16, 23, 38, 0.95) 0%, rgba(13, 26, 44, 0.98) 100%);">
                <div class="card-header" style="margin-bottom: 0.8rem;">
                    <div class="card-title">
                        <span>📺</span>
                        <span data-i18n="castHeader">Web Sharing & Chromecast-Style Casting (Google Cast)</span>
                    </div>
                    <div style="display: flex; gap: 0.5rem; align-items: center;">
                        <span class="card-badge badge-green" id="badgeCastStatus" data-i18n="googleCastReady">● Google Cast Active (Ports 8008/8009)</span>
                    </div>
                </div>

                <p style="color: var(--text-secondary); font-size: 0.88rem; margin-bottom: 1.2rem; line-height: 1.5;" data-i18n="castDesc">
                    Mirror your browser tabs, windows, video URLs or mobile screen directly to the TV just like a real Chromecast device.
                </p>

                <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 1rem; margin-bottom: 1.2rem;">
                    <!-- Option 1: 1-Click Web Cast (Tab / Window / Entire Screen) -->
                    <div style="background: rgba(0, 229, 255, 0.06); border: 1px solid rgba(0, 229, 255, 0.3); border-radius: var(--radius-md); padding: 1.1rem; display: flex; flex-direction: column; justify-content: space-between;">
                        <div>
                            <div style="display: flex; align-items: center; gap: 0.5rem; margin-bottom: 0.5rem;">
                                <span style="font-size: 1.3rem;">🌐</span>
                                <strong style="color: var(--accent-cyan); font-size: 1rem;" data-i18n="webCastTitle">Cast Browser Tab or Screen (Web Cast)</strong>
                            </div>
                            <p style="font-size: 0.82rem; color: var(--text-secondary); line-height: 1.4; margin-bottom: 0.8rem;" data-i18n="webCastDesc">
                                Stream any Chrome/Firefox tab, Meet/Teams call, or entire display with low-latency WebCodecs hardware encoding.
                            </p>
                        </div>
                        <a href="/cast" target="_blank" class="btn-primary" style="text-decoration: none; text-align: center; display: inline-flex; align-items: center; justify-content: center; gap: 0.5rem; background: linear-gradient(135deg, #00b0ff, #00e5ff); color: #000; font-weight: 700; padding: 0.65rem 1rem;" data-i18n="btnOpenWebCast">
                            🔴 Open Web Caster (/cast)
                        </a>
                    </div>

                    <!-- Option 2: Native Google Cast (Chrome, Android, YouTube, Pluto TV) -->
                    <div style="background: rgba(179, 136, 255, 0.06); border: 1px solid rgba(179, 136, 255, 0.3); border-radius: var(--radius-md); padding: 1.1rem; display: flex; flex-direction: column; justify-content: space-between;">
                        <div>
                            <div style="display: flex; align-items: center; gap: 0.5rem; margin-bottom: 0.5rem;">
                                <span style="font-size: 1.3rem;">📱</span>
                                <strong style="color: var(--accent-purple); font-size: 1rem;" data-i18n="googleCastTitle">Google Cast (Native Chromecast)</strong>
                            </div>
                            <p style="font-size: 0.82rem; color: var(--text-secondary); line-height: 1.4; margin-bottom: 0.8rem;" data-i18n="googleCastDesc">
                                In Chrome/Edge menu (Cast...) or phone apps (YouTube, Netflix, Pluto TV), select 'Ext-Monitor (Raspberry Pi)' to cast directly.
                            </p>
                        </div>
                        <button class="btn-toggle" style="width: 100%; border-color: var(--accent-purple); color: var(--accent-purple); font-size: 0.82rem; cursor: default;" data-i18n="googleCastReady">
                            ✓ mDNS & Cast V2 Active (Ports 8008 / 8009)
                        </button>
                    </div>
                </div>

                <!-- Direct Video URL Cast (Play on TV) -->
                <div class="control-group" style="margin-bottom: 0.75rem;">
                    <div class="control-label">
                        <span class="tip-wrap">
                            <span data-i18n="castUrlLabel">Direct Video URL Cast (Play on TV)</span>
                            <span class="tip-icon">?</span>
                            <span class="tip-box" data-i18n="tipCastUrl">Enter a video URL (MP4, WebM, HLS m3u8) to decode and display directly on the Raspberry Pi HDMI output.</span>
                        </span>
                    </div>
                    <div style="display: flex; gap: 0.6rem; margin-top: 0.4rem;">
                        <input type="text" id="castMediaUrlInput" placeholder="https://example.com/video.mp4 or stream..." style="flex: 1; padding: 0.6rem 0.8rem; background: rgba(0,0,0,0.3); border: 1px solid rgba(255,255,255,0.15); border-radius: var(--radius-sm); color: #fff; font-size: 0.85rem;">
                        <button class="btn-primary" onclick="castMediaUrl()" style="white-space: nowrap; background: linear-gradient(135deg, #00e5ff, #7c4dff);" data-i18n="btnCastUrl">
                            ▶ Cast to TV
                        </button>
                    </div>
                </div>

                <!-- Now Playing / Current Playback Status -->
                <div style="background: rgba(0,0,0,0.3); padding: 0.9rem; border-radius: 6px; border-left: 3px solid #00e5ff;">
                    <div style="display: flex; justify-content: space-between; align-items: center;">
                        <span style="font-size: 0.75rem; color: var(--text-secondary); text-transform: uppercase; font-weight: 700;">Cast / Media Player State:</span>
                        <span class="control-value" id="valMediaState" style="color: #00e5ff; font-size: 0.82rem;">Idle / Ready</span>
                    </div>
                    <div style="font-size: 1.05rem; font-weight: 700; color: #fff; margin-top: 0.25rem;" id="mediaTitle">Ext-Monitor Cast & Media Player</div>
                    <div style="font-size: 0.86rem; color: #00e5ff; margin-top: 0.2rem;" id="mediaArtist">Google Cast V2 (8009) • Web Cast (/cast) • DLNA / UPnP</div>
                    <div style="font-size: 0.8rem; color: var(--text-secondary); margin-top: 0.1rem;" id="mediaAlbum">Broadcom VideoCore IV HDMI Output (1280x720 60 FPS)</div>
                </div>
            </div>
        </section>

        <!-- ================================================================= -->
        <!-- TAB 2: CONFIGURAÇÕES & AJUSTES FINOS                              -->
        <!-- ================================================================= -->
        <section id="tab-config" class="tab-content">
            <!-- Host PC Remote Control Card -->
            <div class="glass-card" style="margin-bottom: 1.5rem; border: 1px solid rgba(0, 229, 255, 0.25);">
                <div class="card-header">
                    <div class="card-title">
                        <span>🚀</span>
                        <span data-i18n="hostHeader">Transmitter Remote Control (Host PC)</span>
                    </div>
                    <span class="card-badge badge-cyan" data-i18n="hostBadge">Bidirectional UDP 5001</span>
                </div>

                <!-- Live Transmission Actions -->
                <div class="control-group">
                    <div class="control-label">
                        <span data-i18n="hostStatusLabel">Transmission Status & Actions</span>
                        <span class="control-value" id="valHostStatus">Ready / Online</span>
                    </div>
                    <div class="btn-grid">
                        <button class="btn-toggle active" id="btnHostStart" onclick="sendHostControl({ action: 'start', mode: hostConfiguredMode })" style="border-color: #00e676; color: #00e676;" data-i18n="btnHostStart">▶ Start / Restart Stream</button>
                        <button class="btn-toggle" id="btnHostStop" onclick="sendHostControl({ action: 'stop' })" style="border-color: #ff5252; color: #ff5252;" data-i18n="btnHostStop">⏹ Stop Stream</button>
                    </div>
                </div>

                <!-- Active Screen Transport / Transmission Protocol -->
                <div class="control-group">
                    <div class="control-label">
                        <span class="tip-wrap">
                            <span data-i18n="hostTransportLabel">Active Transport / Transmission Mode</span>
                            <span class="tip-icon">?</span>
                            <span class="tip-box" data-i18n="tipHostTransport">Switch transmission protocol on the fly: USB Bulk Direct (&lt; 1ms raw pipe), Network UDP (port 5000), or Windows Miracast (Win+K RTSP).</span>
                        </span>
                        <span class="control-value" id="valHostTransport">Network UDP (Mode 1)</span>
                    </div>
                    <div class="btn-grid" id="hostTransportGrid">
                        <button class="btn-toggle active" id="btnHostTransport_mode1" onclick="setActiveTransport('mode1_udp')">
                            <div style="font-weight: 700;">🐧 Mode 1: Network UDP</div>
                            <div style="font-size: 0.76rem; opacity: 0.8;">UDP Port 5000 • &lt; 15ms</div>
                        </button>
                        <button class="btn-toggle" id="btnHostTransport_mode3" onclick="setActiveTransport('mode3_usb_bulk')">
                            <div style="font-weight: 700;">⚡ Mode 3: USB Bulk Direct</div>
                            <div style="font-size: 0.76rem; opacity: 0.8;">480 Mbps FunctionFS • &lt; 1ms</div>
                        </button>
                        <button class="btn-toggle tip-wrap" id="btnHostTransport_mode2" onclick="setActiveTransport('mode2_miracast')" title="Exec=env GST_PLUGIN_FEATURE_RANK=vaapih264enc:MAX,vah264enc:MAX gnome-network-displays">
                            <div style="font-weight: 700;">🪟 Mode 2: Miracast</div>
                            <div style="font-size: 0.76rem; opacity: 0.8;">Windows Win+K • Linux GPU</div>
                        </button>
                    </div>
                </div>

                <!-- Display Mode Selection: Extend vs Clone -->
                <div class="control-group">
                    <div class="control-label">
                        <span class="tip-wrap">
                            <span data-i18n="hostModeLabel">Display Mode / Chrome Cast Behavior</span>
                            <span class="tip-icon">?</span>
                            <span class="tip-box" data-i18n="tipHostMode">HDMI-1: Virtual extended second screen on TV. eDP-1: Clones notebook primary screen. Dialog prompt applies exclusively when casting from Google Chrome.</span>
                        </span>
                        <span class="control-value" id="valHostMode">🌐 Cast: Prompt on Screen</span>
                    </div>
                    <div class="btn-grid" id="hostModeGrid">
                        <button class="btn-toggle active" id="btnModeAsk" onclick="setHostMode('ask')" data-i18n="btnModeAsk">🌐 Cast: Prompt on Screen</button>
                        <button class="btn-toggle" id="btnModeExtend" onclick="setHostMode('extend')" data-i18n="btnModeExtend">🖥️ Extended (HDMI-1 TV)</button>
                        <button class="btn-toggle" id="btnModeClone" onclick="setHostMode('clone')" data-i18n="btnModeClone">💻 Cloned (eDP-1 Notebook)</button>
                    </div>
                </div>

                <!-- Network Audio & Bluetooth A2DP Sink -->
                <div class="control-group">
                    <div class="control-label">
                        <span class="tip-wrap">
                            <span data-i18n="hostAudioLabel">Hybrid Audio (IP Network Opus + Bluetooth A2DP)</span>
                            <span class="tip-icon">?</span>
                            <span class="tip-box" data-i18n="tipHostAudio">Enable network audio to stream PC sound via 48kHz Opus to TV HDMI. Use Bluetooth pairing to connect phones/tablets directly to TV.</span>
                        </span>
                        <span class="control-value" id="valHostAudio">Network Audio Active</span>
                    </div>
                    <div class="btn-grid">
                        <button class="btn-toggle active" id="btnHostAudioOn" onclick="setHostAudio(true)" data-i18n="btnHostAudioOn">🔊 Network Audio (Opus UDP)</button>
                        <button class="btn-toggle" id="btnHostAudioOff" onclick="setHostAudio(false)" data-i18n="btnHostAudioOff">🔇 Disable Network Audio</button>
                        <button class="btn-toggle" id="btnBtPair" onclick="triggerBtPairing()" style="border-color: #00e5ff; color: #00e5ff;" data-i18n="btnBtPair">📡 Pair Bluetooth A2DP (60s)</button>
                    </div>
                </div>

                <!-- HUD Overlay Control -->
                <div class="control-group">
                    <div class="control-label">
                        <span class="tip-wrap">
                            <span data-i18n="hostHudLabel">Telemetry HUD on TV Screen</span>
                            <span class="tip-icon">?</span>
                            <span class="tip-box" data-i18n="tipHostHud">Projects live FPS, bitrate, and latency stats in the bottom-right corner of the TV for 60 seconds.</span>
                        </span>
                        <span class="control-value" id="valHostHud">HUD Disabled</span>
                    </div>
                    <div class="btn-grid">
                        <button class="btn-toggle" onclick="sendHostControl({ action: 'trigger_hud' })" data-i18n="btnHostShowHud">📊 Show HUD on TV (60s)</button>
                        <button class="btn-toggle" onclick="sendHostControl({ action: 'hide_hud' })" data-i18n="btnHostHideHud">❌ Hide HUD</button>
                    </div>
                </div>
            </div>

            <!-- Network Interfaces & IP Configuration (Ethernet / Wi-Fi LAN) -->
            <div class="glass-card" style="margin-bottom: 1.5rem; border: 1px solid rgba(56, 189, 248, 0.35);">
                <div class="card-header">
                    <div class="card-title">
                        <span>🌐</span>
                        <span data-i18n="netHeader">Network Interfaces & IP Configuration (Ethernet / LAN / Wi-Fi)</span>
                    </div>
                    <span class="card-badge badge-blue" id="netActiveBadge">usb0 + eth0</span>
                </div>

                <!-- Interface status badges -->
                <div class="control-group" style="padding-bottom: 0.5rem;">
                    <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 0.75rem; margin-top: 0.5rem;">
                        <div style="background: rgba(22, 27, 34, 0.7); border: 1px solid rgba(56, 189, 248, 0.3); border-radius: 8px; padding: 0.75rem;">
                            <div style="font-size: 0.75rem; color: #8b949e; text-transform: uppercase; font-weight: 600;" data-i18n="netUsb0Label">USB OTG (usb0)</div>
                            <div style="font-size: 1.1rem; color: #58a6ff; font-weight: bold; margin: 0.25rem 0;" id="netUsb0Ip">192.168.7.2</div>
                            <div style="font-size: 0.75rem; color: #7ee787;" data-i18n="netUsb0Desc">● Permanent Host OTG</div>
                        </div>
                        <div style="background: rgba(22, 27, 34, 0.7); border: 1px solid rgba(63, 185, 80, 0.3); border-radius: 8px; padding: 0.75rem;">
                            <div style="font-size: 0.75rem; color: #8b949e; text-transform: uppercase; font-weight: 600;" data-i18n="netEth0Label">Physical Ethernet (eth0)</div>
                            <div style="font-size: 1.1rem; color: #7ee787; font-weight: bold; margin: 0.25rem 0;" id="netEth0Ip">192.168.1.50</div>
                            <div style="font-size: 0.75rem; color: #a5d6ff;" id="netEth0Status" data-i18n="netEth0Desc">● Static / DHCP</div>
                        </div>
                        <div style="background: rgba(22, 27, 34, 0.7); border: 1px solid rgba(139, 148, 158, 0.3); border-radius: 8px; padding: 0.75rem;">
                            <div style="font-size: 0.75rem; color: #8b949e; text-transform: uppercase; font-weight: 600;" data-i18n="netWlan0Label">Wi-Fi (wlan0)</div>
                            <div style="font-size: 1.1rem; color: #c9d1d9; font-weight: bold; margin: 0.25rem 0;" id="netWlan0Ip">Disconnected</div>
                            <div style="font-size: 0.75rem; color: #8b949e;" id="netWlan0Status" data-i18n="netWlan0Desc">● Optional</div>
                        </div>
                    </div>
                </div>

                <!-- Network Config Form -->
                <div class="control-group">
                    <div class="control-label">
                        <span class="tip-wrap">
                            <span data-i18n="netModeLabel">Secondary Interface Addressing Mode</span>
                            <span class="tip-icon">?</span>
                            <span class="tip-box" data-i18n="tipNetMode">Choose Static IP for peer-to-peer setups without a DHCP router (e.g. on another Raspberry or PC after boot), or DHCP for automatic local network assignment.</span>
                        </span>
                        <span class="control-value" id="valNetCurrentMode">Static (192.168.1.50)</span>
                    </div>

                    <div style="display: flex; gap: 1rem; align-items: center; margin: 0.75rem 0; flex-wrap: wrap;">
                        <label style="font-size: 0.85rem; color: #c9d1d9; font-weight: 600;" data-i18n="netIfaceLabel">Interface:</label>
                        <select id="netSelectIface" style="padding: 0.4rem 0.8rem; border-radius: 6px; background: #0d1117; color: #58a6ff; border: 1px solid #30363d;">
                            <option value="eth0">eth0 (Secondary Ethernet / HAT / USB LAN)</option>
                            <option value="wlan0">wlan0 (Integrated Wi-Fi)</option>
                        </select>

                        <div class="btn-grid" style="margin: 0; display: inline-flex; gap: 0.5rem;">
                            <button type="button" class="btn-toggle active" id="btnNetStatic" onclick="setNetModeUI('static')" data-i18n="btnNetStatic">📌 Static IP</button>
                            <button type="button" class="btn-toggle" id="btnNetDhcp" onclick="setNetModeUI('dhcp')" data-i18n="btnNetDhcp">🔄 Automatic DHCP</button>
                        </div>
                    </div>

                    <div id="netStaticFields" style="display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 0.75rem; margin-top: 0.75rem;">
                        <div>
                            <label style="font-size: 0.75rem; color: #8b949e; display: block; margin-bottom: 0.25rem;" data-i18n="netIpLabel">IP Address:</label>
                            <input type="text" id="netInputIp" value="192.168.1.50" style="width: 100%; box-sizing: border-box; padding: 0.45rem 0.6rem; background: #0d1117; border: 1px solid #30363d; border-radius: 6px; color: #f0f6fc; font-family: monospace;">
                        </div>
                        <div>
                            <label style="font-size: 0.75rem; color: #8b949e; display: block; margin-bottom: 0.25rem;" data-i18n="netMaskLabel">Subnet Mask:</label>
                            <input type="text" id="netInputMask" value="255.255.255.0" style="width: 100%; box-sizing: border-box; padding: 0.45rem 0.6rem; background: #0d1117; border: 1px solid #30363d; border-radius: 6px; color: #f0f6fc; font-family: monospace;">
                        </div>
                        <div>
                            <label style="font-size: 0.75rem; color: #8b949e; display: block; margin-bottom: 0.25rem;" data-i18n="netGwLabel">Default Gateway:</label>
                            <input type="text" id="netInputGw" value="192.168.1.1" style="width: 100%; box-sizing: border-box; padding: 0.45rem 0.6rem; background: #0d1117; border: 1px solid #30363d; border-radius: 6px; color: #f0f6fc; font-family: monospace;">
                        </div>
                        <div>
                            <label style="font-size: 0.75rem; color: #8b949e; display: block; margin-bottom: 0.25rem;" data-i18n="netDnsLabel">DNS Server:</label>
                            <input type="text" id="netInputDns" value="1.1.1.1, 8.8.8.8" style="width: 100%; box-sizing: border-box; padding: 0.45rem 0.6rem; background: #0d1117; border: 1px solid #30363d; border-radius: 6px; color: #f0f6fc; font-family: monospace;">
                        </div>
                    </div>

                    <div style="margin-top: 1rem; display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 0.5rem;">
                        <span id="netSaveFeedback" style="font-size: 0.85rem; color: #7ee787; font-weight: 500;"></span>
                        <button type="button" class="btn-primary" onclick="saveAndApplyNetworkConfig()" style="background: linear-gradient(135deg, #1f6feb, #238636); padding: 0.55rem 1.25rem;" data-i18n="btnApplyNet">💾 Save & Apply to Board</button>
                    </div>
                </div>
            </div>

            <div class="glass-card">
                <div class="card-header">
                    <div class="card-title">
                        <span>⚙️</span>
                        <span data-i18n="ctrlHeader">Display & Stream Optimization</span>
                    </div>
                    <span class="card-badge badge-green" data-i18n="badgeZeroCopy">VideoCore IV DMA</span>
                </div>

                <!-- Bitrate Control with 400kbps preset -->
                <div class="control-group">
                    <div class="control-label">
                        <span class="tip-wrap">
                            <span data-i18n="bitrateLabel">Streaming Bitrate (VBR)</span>
                            <span class="tip-icon">?</span>
                            <span class="tip-box" data-i18n="tipBitrate">Encoding bitrate. 400k saves 85% for text/code. 3000k to 6000k delivers smooth 1080p/720p 60 FPS video.</span>
                        </span>
                        <span class="control-value" id="valBitrate">400 kbps</span>
                    </div>
                    <div class="slider-wrap">
                        <input type="range" min="150" max="15000" step="50" value="400" class="range-slider" id="bitrateSlider" oninput="updateBitrateValue(this.value)">
                        <div class="slider-labels">
                            <span>150k</span>
                            <span>400k (Eco)</span>
                            <span>800k</span>
                            <span>1500k</span>
                            <span>3000k</span>
                            <span>6000k</span>
                            <span>15M</span>
                        </div>
                    </div>
                    <!-- Quick Bitrate Buttons -->
                    <div class="btn-grid" style="margin-top: 0.6rem;">
                        <button class="btn-toggle active" data-bitrate="400" onclick="setBitrate(400)">400k (Ultra-Eco)</button>
                        <button class="btn-toggle" data-bitrate="800" onclick="setBitrate(800)">800k (Recommended)</button>
                        <button class="btn-toggle" data-bitrate="1500" onclick="setBitrate(1500)">1500k (Balanced)</button>
                        <button class="btn-toggle" data-bitrate="3000" onclick="setBitrate(3000)">3000k (HD)</button>
                        <button class="btn-toggle" data-bitrate="6000" onclick="setBitrate(6000)">6000k (Fluidity)</button>
                    </div>
                </div>

                <!-- Framerate (FPS) -->
                <div class="control-group">
                    <div class="control-label">
                        <span class="tip-wrap">
                            <span data-i18n="fpsLabel">Framerate (FPS)</span>
                            <span class="tip-icon">?</span>
                            <span class="tip-box" data-i18n="tipFps">Target framerate. 15 FPS for ultra-economy, 30 FPS recommended balance, 60 FPS maximum fluidity.</span>
                        </span>
                        <span class="control-value" id="valFps">30 FPS</span>
                    </div>
                    <div class="btn-grid" id="fpsGrid">
                        <button class="btn-toggle" data-fps="15" onclick="setFps(15)">15 FPS (Ultra-Cool)</button>
                        <button class="btn-toggle active" data-fps="30" onclick="setFps(30)">30 FPS (Recommended)</button>
                        <button class="btn-toggle" data-fps="60" onclick="setFps(60)">60 FPS (Maximum Fluidity)</button>
                    </div>
                </div>

                <!-- Color Profile -->
                <div class="control-group">
                    <div class="control-label">
                        <span class="tip-wrap">
                            <span data-i18n="colorLabel">Color Profile</span>
                            <span class="tip-icon">?</span>
                            <span class="tip-box" data-i18n="tipColor">24-bit TrueColor for 1:1 RGB fidelity. 256 Colors quantizes for maximum bus bandwidth savings.</span>
                        </span>
                        <span class="control-value" id="valColor">24-bit TrueColor</span>
                    </div>
                    <div class="btn-grid" id="colorGrid">
                        <button class="btn-toggle active" data-color="full" onclick="setColor('full')" data-i18n="colorFull">24-bit TrueColor</button>
                        <button class="btn-toggle" data-color="256" onclick="setColor('256')" data-i18n="color256">256 Colors (QP 30-44)</button>
                        <button class="btn-toggle" data-color="gray" onclick="setColor('gray')" data-i18n="colorGray">Monochrome</button>
                    </div>
                </div>

                <!-- Transmission Mode: Continuous CFR vs Drop-Only Economy -->
                <div class="control-group">
                    <div class="control-label">
                        <span class="tip-wrap">
                            <span data-i18n="dropOnlyLabel">Transmission Mode (Continuous vs Economy)</span>
                            <span class="tip-icon">?</span>
                            <span class="tip-box" data-i18n="tipDropOnly">Continuous (Default): Steady 30/60 FPS stream for Network and USB Bulk, ensuring smooth videos without freezing even when mouse is still. Economy (drop-only): Drops duplicate frames, saving 95% bandwidth on static screens.</span>
                        </span>
                        <span class="control-value" id="valDropOnly">drop-only=false (Continuous - Default)</span>
                    </div>
                    <div class="btn-grid" id="dropOnlyGrid">
                        <button class="btn-toggle" id="btnDropOnlyTrue" onclick="setDropOnly(true)" data-i18n="dropOnlyTrue">Economy (Drop static frames)</button>
                        <button class="btn-toggle active" id="btnDropOnlyFalse" onclick="setDropOnly(false)" data-i18n="dropOnlyFalse">Continuous (Default: Videos / USB Bulk)</button>
                    </div>
                    <div style="font-size: 0.82rem; color: var(--text-secondary); margin-top: 0.4rem; line-height: 1.4;" data-i18n="dropOnlyDesc">
                        Default: Continuous (drop-only=false). The stream delivers a steady 30/60 FPS flow for Network and USB Bulk. For extreme savings on static text, select 'Economy' or use the --economy flag on the host.
                    </div>
                </div>

                <!-- Skip to First Frame -->
                <div class="control-group">
                    <div class="control-label">
                        <span class="tip-wrap">
                            <span data-i18n="skipFirstLabel">Instant Motion Delivery (skip-to-first)</span>
                            <span class="tip-icon">?</span>
                            <span class="tip-box" data-i18n="tipSkipFirst">Eliminates backlog delay by delivering the very first frame of motion immediately without queue latency.</span>
                        </span>
                        <span class="control-value" id="valSkipFirst">skip-to-first=true (Enabled)</span>
                    </div>
                    <div class="btn-grid" id="skipFirstGrid">
                        <button class="btn-toggle active" id="btnSkipFirstTrue" onclick="setSkipToFirst(true)" data-i18n="skipFirstTrue">Enabled (Zero Latency on Motion)</button>
                        <button class="btn-toggle" id="btnSkipFirstFalse" onclick="setSkipToFirst(false)" data-i18n="skipFirstFalse">Disabled (Strict Sync)</button>
                    </div>
                </div>

                <!-- IDR Keyframe Interval / Periodic Refresh -->
                <div class="control-group">
                    <div class="control-label">
                        <span class="tip-wrap">
                            <span data-i18n="keyIntLabel">Periodic Refresh / IDR Keyframe Interval (Clean Sweep)</span>
                            <span class="tip-icon">?</span>
                            <span class="tip-box" data-i18n="tipKeyInt">Frequency of full IDR I-Frames to sweep and recover from visual artifacts or packet drops.</span>
                        </span>
                        <span class="control-value" id="valKeyInt">30 frames (~1.0s)</span>
                    </div>
                    <div class="slider-wrap">
                        <input type="range" min="10" max="120" step="5" value="30" class="range-slider" id="keyIntSlider" oninput="updateKeyIntValue(this.value)">
                        <div class="slider-labels">
                            <span>10f (0.3s)</span>
                            <span>15f (0.5s)</span>
                            <span>30f (1.0s)</span>
                            <span>60f (2.0s)</span>
                            <span>90f (3.0s)</span>
                            <span>120f (4.0s)</span>
                        </div>
                    </div>
                    <div class="btn-grid" id="keyIntGrid" style="margin-top: 0.6rem;">
                        <button class="btn-toggle" data-keyint="15" onclick="setKeyInt(15)">15f (0.5s - Fast Clean)</button>
                        <button class="btn-toggle active" data-keyint="30" onclick="setKeyInt(30)">30f (1.0s - Recommended)</button>
                        <button class="btn-toggle" data-keyint="60" onclick="setKeyInt(60)">60f (2.0s - Low Bitrate)</button>
                        <button class="btn-toggle" data-keyint="120" onclick="setKeyInt(120)">120f (4.0s - Static Reading)</button>
                    </div>
                    <div style="font-size: 0.82rem; color: var(--text-secondary); margin-top: 0.4rem; line-height: 1.4;" data-i18n="keyIntDesc">
                        Injects a full IDR keyframe periodically to sweep and clear visual artifacts on HDMI/TV.
                    </div>
                </div>

                <!-- Capture Engine: KMS Direct vs GNOME Mutter -->
                <div class="control-group">
                    <div class="control-label">
                        <span class="tip-wrap">
                            <span data-i18n="captureLabel">Capture Engine (Dual-Engine)</span>
                            <span class="tip-icon">?</span>
                            <span class="tip-box" data-i18n="tipCapture">KMS Direct: Reads pixels directly from GPU hardware scanout via Linux Kernel DRM/KMS. Eliminates freezing even when mouse is stationary. Mutter: Captures via GNOME Mutter D-Bus screencast.</span>
                        </span>
                        <span class="control-value" id="valCapture">KMS Direct (Hardware Scanout)</span>
                    </div>
                    <div class="btn-grid" id="captureGrid">
                        <button class="btn-toggle active" data-capture="kms" onclick="setCapture('kms')" data-i18n="btnKmsDirect">⚡ KMS Direct (Anti-Freeze / GPU Scanout)</button>
                        <button class="btn-toggle" data-capture="mutter" onclick="setCapture('mutter')" data-i18n="btnGnomeMutter">🐧 GNOME Mutter (PipeWire Screencast)</button>
                    </div>
                </div>

                <!-- Active Display Monitor -->
                <div class="control-group">
                    <div class="control-label">
                        <span class="tip-wrap">
                            <span data-i18n="monitorTargetLabel">Recording / Capture Display</span>
                            <span class="tip-icon">?</span>
                            <span class="tip-box" data-i18n="tipMonitorTarget">Choose the video output to capture. HDMI-1 for extended second screen on TV/monitor, eDP-1 to clone notebook screen.</span>
                        </span>
                        <span class="control-value" id="valMonitor">HDMI-1 (Secondary Screen)</span>
                    </div>
                    <div class="btn-grid" id="monitorGrid">
                        <button class="btn-toggle active" data-monitor="HDMI-1" onclick="setMonitor('HDMI-1')">HDMI-1 (Extended Second Screen)</button>
                        <button class="btn-toggle" data-monitor="eDP-1" onclick="setMonitor('eDP-1')">eDP-1 (Notebook Display)</button>
                        <button class="btn-toggle" data-monitor="auto" onclick="setMonitor('auto')">Auto (First Active External)</button>
                    </div>
                </div>

                <!-- Scaling Mode & Resolution (Unified Dropdown + Silicon Scaler Toggle) -->
                <div class="control-group">
                    <div class="control-label">
                        <span class="tip-wrap">
                            <span data-i18n="scaleLabel">Resolution & Display Scaling</span>
                            <span class="tip-icon">?</span>
                            <span class="tip-box" data-i18n="tipScale">Select display resolution. 1:1 Native modes render crisp vector fonts without blur. Upscaling modes utilize VideoCore IV silicon HVS or GPU FSR to fill widescreen monitors.</span>
                        </span>
                        <span class="control-value" id="valScale">1280x720 (1:1 Native - Sharp)</span>
                    </div>

                    <!-- Silicon Scaler (HVS / FSR) Toggle Checkbox -->
                    <div style="margin-bottom: 0.6rem; display: flex; align-items: center; justify-content: space-between; background: rgba(0, 229, 255, 0.05); border: 1px solid rgba(0, 229, 255, 0.15); border-radius: var(--radius-sm); padding: 0.5rem 0.8rem;">
                        <label for="chkSiliconScaler" style="display: flex; align-items: center; gap: 0.6rem; cursor: pointer; font-size: 0.86rem; color: var(--text-primary); user-select: none;">
                            <input type="checkbox" id="chkSiliconScaler" checked onchange="toggleSiliconScaler(this.checked)" style="accent-color: var(--accent-cyan); width: 17px; height: 17px; cursor: pointer;">
                            <span data-i18n="lblSiliconScaler">Silicon Hardware Scaler (VideoCore IV HVS & FSR)</span>
                        </label>
                        <span id="badgeSiliconStatus" class="card-badge badge-green" style="font-size: 0.72rem;" data-i18n="badgeSiliconActive">HVS Active</span>
                    </div>

                    <!-- Unified Resolution Dropdown -->
                    <div style="position: relative;">
                        <select id="resSelect" onchange="onResolutionSelectChange(this.value)" style="width: 100%; background: #0e1626; color: #f0f6fc; border: 1px solid rgba(0, 229, 255, 0.35); border-radius: var(--radius-sm); padding: 0.65rem 0.9rem; font-size: 0.9rem; outline: none; cursor: pointer;">
                            <optgroup label="Native 1:1 Direct Modes (Crisp / No Blur)" id="optgroupNative" data-i18n-label="optgroupNative">
                                <option value="720p" selected>1280x720 @ 60Hz (1:1 Native - Recommended / Sharpest)</option>
                                <option value="1024x768">1024x768 @ 60Hz (1:1 Native 4:3)</option>
                                <option value="800x600">800x600 @ 60Hz (1:1 Native Eco)</option>
                                <option value="off">Off (1:1 Direct Hardware Passthrough)</option>
                            </optgroup>
                            <optgroup label="Super-Resolution Upscaling (Silicon HVS / FSR)" id="optgroupUpscale" data-i18n-label="optgroupUpscale">
                                <option value="1600x900">1600x900 @ 60Hz (Widescreen Stretch)</option>
                                <option value="1920x1080">1920x1080 @ 60Hz (Full HD Virtual Canvas)</option>
                            </optgroup>
                        </select>
                    </div>

                    <!-- Quick Preset Buttons -->
                    <div class="btn-grid" id="scaleGrid" style="margin-top: 0.6rem;">
                        <button class="btn-toggle active" data-scale="720p" onclick="setScale('720p')" data-i18n="btnScale720p">🎯 720p Native (1:1)</button>
                        <button class="btn-toggle" data-scale="1600x900" id="btnScale900" onclick="setScale('1600x900')" data-i18n="btnScale1600x900">📐 900p Upscale</button>
                        <button class="btn-toggle" data-scale="off" onclick="setScale('off')" data-i18n="btnScaleOff">⚡ Passthrough</button>
                    </div>
                </div>

                <!-- CAS Sharpening (Contrast Adaptive Sharpening) -->
                <div class="control-group">
                    <div class="control-label">
                        <span class="tip-wrap">
                            <span data-i18n="casLabel">Contrast Adaptive Sharpening (CAS)</span>
                            <span class="tip-icon">?</span>
                            <span class="tip-box" data-i18n="tipCas">Restores full PC color range (0-255) and deep contrast, eliminating washed-out video encoding artifacts.</span>
                        </span>
                        <span class="control-value" id="valCas">CAS Enabled (Full Contrast)</span>
                    </div>
                    <div class="btn-grid" id="casGrid">
                        <button class="btn-toggle active" data-cas="true" onclick="setCas(true)" data-i18n="btnCasTrue">✨ CAS Enabled (Crisp Text / Full Black)</button>
                        <button class="btn-toggle" data-cas="false" onclick="setCas(false)" data-i18n="btnCasFalse">Standard (TV Limited Range)</button>
                    </div>
                </div>

                <!-- Commit & Hardware Actions -->
                <div class="action-row" style="border-top: 1px solid rgba(255, 255, 255, 0.08); padding-top: 1.2rem;">
                    <button id="btnApply" class="btn-primary" onclick="applyConfiguration()" data-i18n="btnApply">💾 Apply Settings</button>
                    <button id="btnPause" class="btn-secondary" onclick="togglePauseStream()" data-i18n="btnPauseStream">⏸ Pause Display</button>
                    <button id="btnReboot" class="btn-danger" onclick="confirmReboot()" data-i18n="btnReboot">🔄 Reboot Appliance</button>
                </div>
            </div>
        </section>

        <!-- ================================================================= -->
        <!-- TAB 3: CLIENT DOWNLOADS & DRIVERS                                 -->
        <!-- ================================================================= -->
        <section id="tab-downloads" class="tab-content">
            <div class="glass-card">
                <div class="card-header">
                    <div class="card-title">
                        <span>⚡</span>
                        <span data-i18n="oneLinerTitle">One-Line Host Connector (1 Click)</span>
                    </div>
                    <span class="card-badge badge-green" data-i18n="badgeInstant">Instant</span>
                </div>
                <p style="color: var(--text-secondary); font-size: 0.9rem; line-height: 1.5; margin-bottom: 0.8rem;" data-i18n="oneLinerDesc">
                    On any Linux PC, paste this command in your terminal to start the extended monitor immediately:
                </p>
                <div class="cmd-box">
                    <span class="cmd-text" id="cmdOneLine">curl -sSL http://192.168.7.2:8080/connect.sh | bash</span>
                    <button class="copy-btn" onclick="copyCommand('cmdOneLine')" data-i18n="btnCopy">Copy</button>
                </div>
            </div>

            <!-- Direct File Downloads Grid -->
            <div class="download-grid">
                <div class="dl-card">
                    <div>
                        <div class="dl-icon">📦</div>
                        <div class="dl-title" data-i18n="dlPkgTitle">Full Client Package</div>
                        <div class="dl-desc" data-i18n="dlPkgDesc">Contains precompiled ext-sender binary, udev rules and documentation in tar.gz.</div>
                    </div>
                    <a href="/download/client.tar.gz" class="dl-link" download data-i18n="btnDlPkg">Download client.tar.gz</a>
                </div>
                <div class="dl-card">
                    <div>
                        <div class="dl-icon">⚙️</div>
                        <div class="dl-title" data-i18n="dlSenderTitle">ext-sender Binary</div>
                        <div class="dl-desc" data-i18n="dlSenderDesc">Standalone GPU offload transmitter binary compiled in Rust for Linux x86_64.</div>
                    </div>
                    <a href="/download/ext-sender" class="dl-link" download data-i18n="btnDlSender">Download ext-sender</a>
                </div>
                <div class="dl-card">
                    <div>
                        <div class="dl-icon">📜</div>
                        <div class="dl-title" data-i18n="dlScriptTitle">connect.sh Script</div>
                        <div class="dl-desc" data-i18n="dlScriptDesc">Portable script that detects USB, downloads needed tools and launches the stream.</div>
                    </div>
                    <a href="/connect.sh" class="dl-link" download data-i18n="btnDlScript">Download connect.sh</a>
                </div>
                <div class="dl-card">
                    <div>
                        <div class="dl-icon">🛡️</div>
                        <div class="dl-title" data-i18n="dlUdevTitle">udev Rules (Plug-and-Play)</div>
                        <div class="dl-desc" data-i18n="dlUdevDesc">Automatically configures host USB buffer (txqueuelen 100) to eliminate buffer bloat.</div>
                    </div>
                    <a href="/download/99-ext-monitor.rules" class="dl-link" download data-i18n="btnDlUdev">Download 99-ext-monitor.rules</a>
                </div>
            </div>
        </section>

        <!-- ================================================================= -->
        <!-- TAB 4: SD CARD & 100% RAM ARCHITECTURE                            -->
        <!-- ================================================================= -->
        <section id="tab-sdcard" class="tab-content">
            <div class="glass-card">
                <div class="card-header">
                    <div class="card-title">
                        <span>💾</span>
                        <span data-i18n="sdHeader">100% RAM Architecture & Firmware Upgrade</span>
                    </div>
                    <span class="card-badge badge-purple" data-i18n="badgeZeroSdWear">Zero SD Wear</span>
                </div>
                <div style="color: var(--text-secondary); font-size: 0.9rem; line-height: 1.6;">
                    <p style="margin-bottom: 0.8rem;" data-i18n="sdDesc1">
                        The Pi Zero boots entirely into RAM (initramfs). After a &lt; 2s boot, the physical micro-SD (/dev/mmcblk0) is decoupled and never written to during usage.
                    </p>
                    <p style="margin-bottom: 1rem;" data-i18n="sdDesc2">
                        This guarantees zero SD corruption risk and allows mounting the boot partition to upgrade firmware without removing the card!
                    </p>
                    
                    <div style="background: rgba(255, 255, 255, 0.03); border: 1px solid rgba(255, 255, 255, 0.08); border-radius: var(--radius-sm); padding: 1rem; margin-bottom: 1.2rem;">
                        <div style="font-weight: 700; color: #fff; margin-bottom: 0.5rem;" data-i18n="sdPartitionStatus">Physical Media Status:</div>
                        <p>Device: <span style="font-family: monospace; color: var(--accent-cyan);">/dev/mmcblk0</span> (Boot Partition: <span style="font-family: monospace; color: var(--accent-cyan);">/dev/mmcblk0p1 FAT16</span>)</p>
                        <p>Mount State: <span id="sdMountStatus" style="color: #7ee787; font-weight: 700;">Unmounted (Safe / Decoupled)</span></p>
                    </div>

                    <div class="action-row">
                        <button class="btn-secondary" onclick="mountSdCard(true)" data-i18n="btnMountSd">Mount Partition (/mnt/boot)</button>
                        <button class="btn-secondary" onclick="mountSdCard(false)" data-i18n="btnUnmountSd">Unmount Partition</button>
                    </div>

                    <div style="margin-top: 1.5rem;">
                        <h4 style="color: #fff; margin-bottom: 0.5rem;" data-i18n="sdUpgradeManualTitle">How to Upgrade Firmware without Removing Card:</h4>
                        <div class="cmd-box">
                            <span class="cmd-text" id="cmdUpgrade">mount -t vfat /dev/mmcblk0p1 /mnt && cp /tmp/ext-receiver /mnt/ && umount /mnt</span>
                            <button class="copy-btn" onclick="copyCommand('cmdUpgrade')" data-i18n="btnCopy">Copy</button>
                        </div>
                    </div>
                </div>
            </div>
        </section>

        <!-- ================================================================= -->
        <!-- TAB 5: OPERATION MANUAL & SPECIFICATIONS                          -->
        <!-- ================================================================= -->
        <section id="tab-manual" class="tab-content">
            <div class="glass-card">
                <div class="card-header">
                    <div class="card-title">
                        <span>📖</span>
                        <span data-i18n="manualHeader">Operation Manual & Technical Specifications</span>
                    </div>
                    <span class="card-badge badge-cyan" data-i18n="badgeFullDocs">Complete Documentation</span>
                </div>

                <div style="color: var(--text-secondary); font-size: 0.9rem; line-height: 1.6;">
                    <!-- Compendium Banner: The Ext-Monitor Book -->
                    <div style="background: rgba(126, 231, 135, 0.06); border: 1px solid rgba(126, 231, 135, 0.25); border-radius: 8px; padding: 1.2rem; margin-bottom: 1.5rem;">
                        <div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 0.6rem;">
                            <div style="display: flex; align-items: center; gap: 0.6rem;">
                                <span style="font-size: 1.4rem;">📚</span>
                                <strong style="color: #7ee787; font-size: 1.05rem;" data-i18n="bookTitle">The Ext-Monitor Book — Engineering & Architecture Compendium</strong>
                            </div>
                            <span class="card-badge badge-green" data-i18n="bookBadge">Compendium of 18 Blueprints</span>
                        </div>
                        <p style="font-size: 0.88rem; color: var(--text-secondary); line-height: 1.5; margin: 0 0 0.8rem 0;" data-i18n="bookDesc">
                            Hardware reverse engineering documentation, BCM2835 silicon decisions, network protocols (RFC 4571 / RFC 6184 / WFD), hybrid audio, and the Wayland/DRM-KMS pipeline are consolidated in the master compendium: <strong style="color: #fff; font-family: monospace;">docs/LIVRO-EXT-MONITOR.md</strong>.
                        </p>
                        <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: 0.6rem;">
                            <div style="background: rgba(0,0,0,0.25); padding: 0.6rem 0.8rem; border-radius: 6px; border-left: 3px solid #00e5ff;">
                                <strong style="color: #fff; font-size: 0.82rem;" data-i18n="bookPart1Title">Part I: BCM2835 Silicon & Boot</strong>
                                <div style="font-size: 0.76rem; color: var(--text-secondary); margin-top: 0.2rem;" data-i18n="bookPart1Desc">Boot 1.8s, FAT16 32MB, VideoCore IV V4L2 M2M, Zero-Copy DMA and DMA-BUF.</div>
                            </div>
                            <div style="background: rgba(0,0,0,0.25); padding: 0.6rem 0.8rem; border-radius: 6px; border-left: 3px solid #7ee787;">
                                <strong style="color: #fff; font-size: 0.82rem;" data-i18n="bookPart2Title">Part II: Protocols & Buses</strong>
                                <div style="font-size: 0.76rem; color: var(--text-secondary); margin-top: 0.2rem;" data-i18n="bookPart2Desc">USB Bulk RFC 4571 Marker Bit/ZLP, RTP H.264 FU-A, WFD Miracast and UAC2.</div>
                            </div>
                            <div style="background: rgba(0,0,0,0.25); padding: 0.6rem 0.8rem; border-radius: 6px; border-left: 3px solid #ffab40;">
                                <strong style="color: #fff; font-size: 0.82rem;" data-i18n="bookPart3Title">Part III: Linux Host & Wayland</strong>
                                <div style="font-size: 0.76rem; color: var(--text-secondary); margin-top: 0.2rem;" data-i18n="bookPart3Desc">GNOME Mutter Screencast D-Bus, PipeWire, AMD Radeon 610M DCN 3.1 and DRM/KMS.</div>
                            </div>
                            <div style="background: rgba(0,0,0,0.25); padding: 0.6rem 0.8rem; border-radius: 6px; border-left: 3px solid #b388ff;">
                                <strong style="color: #fff; font-size: 0.82rem;" data-i18n="bookPart4Title">Part IV: Hybrid Audio & Web Control</strong>
                                <div style="font-size: 0.76rem; color: var(--text-secondary); margin-top: 0.2rem;" data-i18n="bookPart4Desc">Opus 48kHz UDP, BlueZ A2DP Sink on Pi Zero W and Bidirectional Rust Web Panel.</div>
                            </div>
                        </div>
                    </div>

                    <!-- Section 1: Architecture -->
                    <h3 style="color: #fff; margin: 1rem 0 0.5rem 0;" data-i18n="docArchTitle">1. Hardware Architecture & VideoCore IV GPU</h3>
                    <p data-i18n="docArchDesc">
                        Raspberry Pi Zero utilizes the Broadcom BCM2835 SoC (ARM1176JZF-S @ 1.0 GHz) integrated with the VideoCore IV GPU @ 500 MHz. The ext-monitor runs a 100% userspace hardware pipeline: H.264 video streams are fed directly into the V4L2 M2M decoder (<span style="font-family: monospace; color: var(--accent-cyan);">/dev/video10</span>), which writes rendered frames via DMA directly to the HDMI scanout plane (<span style="font-family: monospace; color: #7ee787;">/dev/fb0</span>). This guarantees &lt; 15ms latency and &lt; 2% CPU usage.
                    </p>

                    <!-- Section 2: USB Protocol & RFC 4571 Framing -->
                    <h3 style="color: #fff; margin: 1.5rem 0 0.5rem 0;" data-i18n="docUsbProtoTitle">2. Direct USB Bulk Protocol, RFC 4571 & End-of-Frame Marker Bit (EOF / ZLP)</h3>
                    <p data-i18n="docUsbProtoDesc1">
                        On the USB 2.0 High-Speed bus (480 Mbps), packets travel in micro-frames up to 512 bytes (<span style="font-family: monospace; color: var(--accent-cyan);">wMaxPacketSize</span>). In raw H.264 Annex-B streams, decoders had to wait for the next frame's start code (<span style="font-family: monospace; color: var(--accent-cyan);">00 00 00 01</span>) to verify completion, causing stalls when mouse movement stopped.
                    </p>
                    <p data-i18n="docUsbProtoDesc2" style="margin-top: 0.5rem;">
                        ext-monitor resolves this by implementing standard <strong style="color: #fff;">RFC 4571</strong> 2-byte length-delimited framing with <strong style="color: var(--accent-cyan);">RTP Marker Bit (EOF)</strong> signaling. As soon as a frame finishes transmitting over USB, the receiver drains and renders it immediately on HDMI, eliminating mouse dependency and ensuring smooth YouTube playback. Furthermore, transfers that are exact multiples of 512 bytes trigger a <span style="font-family: monospace; color: #7ee787;">Zero-Length Packet (ZLP)</span>, preventing DWC2 hardware FIFO stalls.
                    </p>

                    <!-- Section 3: Audio Routing -->
                    <h3 style="color: #fff; margin: 1.5rem 0 0.5rem 0;" data-i18n="docAudioTitle">3. Multi-Mode Audio Routing (Normal vs. Hybrid)</h3>
                    <p data-i18n="docAudioDesc1">
                        ext-monitor supports stereo digital audio routing to HDMI TV speakers across 4 transport technologies:
                    </p>
                    <ul style="margin: 0.6rem 0 0.6rem 1.5rem; line-height: 1.5;">
                        <li data-i18n="docAudioMode1"><strong style="color: #fff;">Mode 1 (IP Network):</strong> Virtual PipeWire sink on PC transmitting RTP Opus/PCM on UDP port 5004 directly to the Pi Zero HDMI codec.</li>
                        <li data-i18n="docAudioMode2"><strong style="color: #fff;">Mode 2 (Windows Miracast):</strong> Native Wi-Fi Display (WFD) protocol routing stereo AAC/LPCM audio via RTSP TCP 7236.</li>
                        <li data-i18n="docAudioMode3"><strong style="color: #fff;">Mode 3 (USB Audio Gadget - UAC2):</strong> Plug-and-play USB sound card exposed by Composite Gadget on PC with direct PCM forwarding to TV HDMI.</li>
                        <li data-i18n="docAudioMode4"><strong style="color: #fff;">Mode 4 (Bluetooth A2DP Sink):</strong> Bluetooth audio receiver for pairing with smartphones, tablets, or secondary laptops.</li>
                    </ul>
                    <p data-i18n="docAudioDesc2">
                        <strong>Normal vs Hybrid Mode:</strong> In Normal mode, both PC video and audio play together on the TV. In Hybrid mode, you can keep audio playing on local PC speakers while video streams to TV, or pair a smartphone via Bluetooth to play music on the TV while the PC displays code.
                    </p>

                    <!-- Section 4: Serial Recovery -->
                    <h3 style="color: #fff; margin: 1.5rem 0 0.5rem 0;" data-i18n="docSerialTitle">4. Zero-IP Reconfiguration via USB Serial (/dev/ttyACM0)</h3>
                    <p data-i18n="docSerialDesc">
                        If network is disabled or misconfigured, the Pi Zero exposes an independent recovery serial console on /dev/ttyACM0 at 115200 baud.
                    </p>
                    <div class="cmd-box">
                        <span class="cmd-text" id="cmdSerial">picocom -b 115200 /dev/ttyACM0  # or: screen /dev/ttyACM0 115200</span>
                        <button class="copy-btn" onclick="copyCommand('cmdSerial')" data-i18n="btnCopy">Copy</button>
                    </div>

                    <!-- Section 5: Linux Wayland -->
                    <h3 style="color: #fff; margin: 1.5rem 0 0.5rem 0;" data-i18n="docLinuxTitle">5. Standard Linux Wayland (GNOME) Streaming</h3>
                    <p data-i18n="docLinuxDesc">
                        Plug into the center USB port. The PC gets IP 192.168.7.1 automatically. Then run ext-sender (zero configuration needed):
                    </p>
                    <div class="cmd-box">
                        <span class="cmd-text" id="cmdStart">ext-sender</span>
                        <button class="copy-btn" onclick="copyCommand('cmdStart')" data-i18n="btnCopy">Copy</button>
                    </div>

                    <!-- Section 6: Windows Miracast -->
                    <h3 style="color: #fff; margin: 1.5rem 0 0.5rem 0;" data-i18n="docWinTitle">6. Windows 10/11 Miracast (Zero Drivers)</h3>
                    <p data-i18n="docWinDesc">
                        Plug into USB, press <strong style="color: #fff;">Win + K</strong> on Windows, and select <em>'Pi Zero Wireless Display'</em>. Second screen is activated instantly without drivers.
                    </p>

                    <!-- Section 7: RAM Architecture -->
                    <h3 style="color: #fff; margin: 1.5rem 0 0.5rem 0;" data-i18n="docSdTitle">7. 100% RAM Architecture & Firmware Upgrade without SD Card Removal</h3>
                    <p data-i18n="docSdDesc">
                        The appliance runs 100% in an initramfs RAM disk. To upgrade receiver binaries, simply mount the boot FAT partition with <span style="font-family: monospace; color: var(--accent-cyan);">mount -t vfat /dev/mmcblk0p1 /mnt</span>, write the new image, and unmount without rebooting or touching the SD card.
                    </p>

                    <!-- Section 8: Browser Video Occlusion Tip -->
                    <div style="background: rgba(88, 166, 255, 0.08); border-left: 4px solid var(--accent-cyan); border-radius: 6px; padding: 1rem; margin: 1.5rem 0;">
                        <h3 style="color: var(--accent-cyan); margin: 0 0 0.5rem 0;" data-i18n="docBrowserVideoTitle">8. 💡 Tip: Continuous Browser Video Playback (Chrome / Firefox)</h3>
                        <p data-i18n="docBrowserVideoDesc" style="margin-bottom: 0.75rem;">
                            On Linux Wayland, browsers like Chrome and Firefox enable aggressive power-saving ('Window Occlusion Tracking') and pause video rendering when the mouse cursor leaves the window or it loses focus. To ensure smooth 60 FPS video without cursor focus:
                        </p>
                        <p style="margin: 0.35rem 0 0.2rem 1rem; color: #fff;">
                            <strong>Chrome / Chromium / Edge / Brave:</strong> <span data-i18n="docBrowserChrome">Visit chrome://flags/#calculate-native-win-occlusion, set to 'Disabled' and restart (or launch with --disable-backgrounding-occluded-windows).</span>
                        </p>
                        <p style="margin: 0.35rem 0 0.2rem 1rem; color: #fff;">
                            <strong>Mozilla Firefox:</strong> <span data-i18n="docBrowserFirefox">Visit about:config, search for media.suspend-bkgnd-video.enabled and toggle to 'false'.</span>
                        </p>
                        <p style="margin: 0.35rem 0 0.2rem 1rem; color: #fff;">
                            <strong>Native Players (VLC / MPV):</strong> <span data-i18n="docBrowserNative">Play at continuous 60 FPS by default without any cursor position restrictions.</span>
                        </p>
                    </div>

                    <!-- Section 9: ARMv6 Compilation -->
                    <h3 style="color: #fff; margin: 1.5rem 0 0.5rem 0;" data-i18n="docArmv6Title">9. Cross-Compiling for Raspberry Pi Zero (ARMv6)</h3>
                    <p data-i18n="docArmv6Desc">
                        The BCM2835 SoC on Pi Zero v1.2/v1.3/W requires the ARMv6l architecture. To compile with cross (Docker):
                    </p>
                    <pre style="background: rgba(0,0,0,0.5); padding: 0.75rem; border-radius: 6px; font-family: monospace; color: var(--accent-cyan); overflow-x: auto;"><code data-i18n="docArmv6Cmd">cargo install cross && cd receiver && cross build --target arm-unknown-linux-musleabihf --release</code></pre>

                    <!-- Section 10: Non-OTG Models -->
                    <h3 style="color: #fff; margin: 1.5rem 0 0.5rem 0;" data-i18n="docNonOtgTitle">10. Non-OTG Raspberry Pi Models (Pi 2, Pi 3, Pi 4, Pi 5) & Boot Config</h3>
                    <p data-i18n="docNonOtgDesc1">
                        These models do not support peripheral USB gadget mode on standard USB-A ports. Streaming is delivered via Ethernet or Wi-Fi.
                    </p>
                    <p data-i18n="docNonOtgDesc2" style="color: #8b949e;">
                        Required SD boot adjustments: In config.txt comment out 'dtoverlay=dwc2'. In cmdline.txt remove 'modules-load=dwc2'. Stream via: ext-sender --ip=&lt;PI_IP&gt;
                    </p>

                    <!-- Section 11: Conventional PC Receiver -->
                    <h3 style="color: #fff; margin: 1.5rem 0 0.5rem 0;" data-i18n="docPcReceiverTitle">11. Turning Any Linux PC / Laptop into a Secondary Screen Receiver</h3>
                    <p data-i18n="docPcReceiverDesc">
                        Any Linux computer can act as a receiver. Install gstreamer1.0-tools and run:
                    </p>
                    <pre style="background: rgba(0,0,0,0.5); padding: 0.75rem; border-radius: 6px; font-family: monospace; color: var(--accent-cyan); overflow-x: auto;"><code data-i18n="docPcReceiverCmd">gst-launch-1.0 -v udpsrc port=5000 buffer-size=524288 caps="application/x-rtp,media=video,clock-rate=90000,encoding-name=H264,payload=96" ! rtph264depay ! h264parse ! avdec_h264 ! autovideosink sync=false</code></pre>

                    <!-- Section 12: Multi-Monitor Targeting -->
                    <h3 style="color: #fff; margin: 1.5rem 0 0.5rem 0;" data-i18n="docMultiMonTitle">12. Directing Video to Screen 1 vs Screen 2 on Multi-Monitor PCs</h3>
                    <p data-i18n="docMultiMonDesc">
                        On receiver PCs with multiple connected displays, specify 'kmssink connector-id=&lt;ID&gt;' in direct DRM KMS mode or 'ffplay -left 1920 -top 0 -fs rtp://0.0.0.0:5000' in graphical sessions to target the desired monitor.
                    </p>

                    <!-- Section 13: Wayland Damage Pacer -->
                    <div style="background: rgba(46, 160, 67, 0.08); border-left: 4px solid #2ea043; border-radius: 6px; padding: 1rem; margin: 1.5rem 0;">
                        <h3 style="color: #2ea043; margin: 0 0 0.5rem 0;" data-i18n="docPacerTitle">13. 🚀 Wayland Damage Pacer: Continuous 60 FPS YouTube without Pausing</h3>
                        <p data-i18n="docPacerDesc" style="margin: 0; color: #fff;">
                            ext-monitor runs wayland-damage-pacer.py in the background on the Host. It emits 60 Hz micro-damage pulses to an invisible sub-surface with an empty Cairo click-through mask on the extended monitor, keeping the GNOME Mutter compositor active. YouTube videos, clocks, and terminals render at 60 FPS even when the mouse is motionless or on the primary screen.
                        </p>
                    </div>

                    <!-- Section 14: Non-GNOME Alternatives -->
                    <h3 style="color: #fff; margin: 1.5rem 0 0.5rem 0;" data-i18n="docAltPlayersTitle">14. Non-GNOME Receiver Alternatives (FFmpeg, MPV, VLC)</h3>
                    <p data-i18n="docAltPlayersDesc">
                        The RFC 4571 RTP H.264 video stream is fully cross-platform. Receive it on KDE, XFCE, i3, Windows, or macOS with zero-buffer low delay:
                    </p>
                    <pre style="background: rgba(0,0,0,0.5); padding: 0.75rem; border-radius: 6px; font-family: monospace; color: var(--accent-cyan); overflow-x: auto;"><code data-i18n="docAltPlayersCmd"># FFmpeg / ffplay (Low Latency):
ffplay -fflags nobuffer -flags low_delay -framedrop -an -sn -sync ext -protocol_whitelist file,udp,rtp rtp://0.0.0.0:5000

# MPV Player (Hardware Accelerated):
mpv --no-cache --untimed --no-correct-pts --fps=60 --profile=low-latency --hwdec=auto rtp://0.0.0.0:5000</code></pre>

                    <!-- Section 15: USB Bulk Default & Auto-Fallback -->
                    <div style="background: rgba(187, 134, 252, 0.08); border-left: 4px solid var(--accent-purple); border-radius: 6px; padding: 1rem; margin: 1.5rem 0;">
                        <h3 style="color: var(--accent-purple); margin: 0 0 0.5rem 0;" data-i18n="docBulkFallbackTitle">15. ⚡ Default USB Bulk Direct Mode & Automatic UDP Network Fallback</h3>
                        <p data-i18n="docBulkFallbackDesc" style="margin: 0; color: #fff;">
                            ext-sender prioritizes high-speed 480 Mbps USB Bulk Direct mode by default. If the USB gadget interface is not detected, it automatically falls back to UDP Network streaming (port 5000) without crashing.
                        </p>
                    </div>

                    <!-- Section 16: Hybrid Audio & Virtual HDMI Device -->
                    <div style="background: rgba(0, 229, 255, 0.08); border-left: 4px solid var(--accent-cyan); border-radius: 6px; padding: 1rem; margin: 1.5rem 0;">
                        <h3 style="color: var(--accent-cyan); margin: 0 0 0.5rem 0;" data-i18n="docAudioRoutingTitle">16. 🔊 Hybrid Audio Routing & Virtual HDMI Device</h3>
                        <p style="margin: 0 0 0.6rem 0; color: #fff;" data-i18n="docAudioRoutingDesc">
                            ext-monitor implements a 100% isolated and hybrid audio architecture using a virtual PipeWire/PulseAudio sink (<code>Raspberry_Pi_HDMI_Audio</code>). You can work on the extended TV display while keeping YouTube, meetings and music on your laptop speakers or USB headset:
                        </p>
                        <pre style="background: rgba(0,0,0,0.5); padding: 0.75rem; border-radius: 6px; font-family: monospace; color: var(--accent-cyan); overflow-x: auto;"><code># Hybrid Mode: Keeps PC audio on local headset/speakers
./scripts/audio-route.sh local

# TV Mode: Routes all system audio to HDMI TV via Opus 48kHz (UDP 5004)
./scripts/audio-route.sh pi

# Realtime status and active device
./scripts/audio-route.sh status</code></pre>
                    </div>

                    <!-- Section 17: Subhardware Clocks & Power -->
                    <div style="background: rgba(46, 160, 67, 0.08); border-left: 4px solid #2ea043; border-radius: 6px; padding: 1rem; margin: 1.5rem 0;">
                        <h3 style="color: #2ea043; margin: 0 0 0.5rem 0;" data-i18n="docClocksTitle">17. ⚙️ Subhardware Clocks (H.264/VPU/ARM) & Power Consumption</h3>
                        <p style="margin: 0; color: #fff;" data-i18n="docClocksDesc">
                            The Raspberry Pi Zero W exposes internal telemetry of every Broadcom BCM2835 silicon block via <code>debugfs</code>. ext-monitor monitors hardware clocks continuously in real-time:
                        </p>
                        <ul style="margin: 0.6rem 0 0 1.2rem; color: #ccc; font-size: 0.88rem; line-height: 1.6;">
                            <li><strong>H.264 V4L2 M2M Decoder:</strong> <code>200 MHz / 250 MHz</code> (dedicated hardware decompression engine)</li>
                            <li><strong>VideoCore IV VPU:</strong> <code>400 MHz</code> (video processor & KMS pipeline)</li>
                            <li><strong>ARM1176 CPU:</strong> <code>700 MHz - 1000 MHz</code> (energy-saving ondemand governor)</li>
                            <li><strong>V3D 3D Core:</strong> <code>250 MHz</code> | <strong>SDRAM:</strong> <code>166 MHz LPDDR</code></li>
                            <li><strong>Power & Current:</strong> <code>~0.85W</code> (idle) to <code>~1.15W</code> (continuous 60 FPS streaming @ 5V / 230mA)</li>
                        </ul>
                    </div>

                    <!-- Section 18: Full CLI Manual & Troubleshooting -->
                    <div style="background: rgba(255, 179, 0, 0.08); border-left: 4px solid var(--accent-amber); border-radius: 6px; padding: 1rem; margin: 1.5rem 0;">
                        <h3 style="color: var(--accent-amber); margin: 0 0 0.5rem 0;" data-i18n="docCliManualTitle">18. 📖 Full Operation Manual, CLI Flags & Troubleshooting</h3>
                        <p style="margin: 0 0 0.5rem 0; color: #fff;" data-i18n="docCliManualDesc">
                            Launcher syntax: <code>ext-sender [extend|clone] [fps] [bitrate] [options]</code>
                        </p>
                        <ul style="margin: 0.4rem 0 0 1.2rem; color: #ccc; font-size: 0.88rem; line-height: 1.6;">
                            <li><code>extend</code>: Creates or connects extended second screen on HDMI-1 [DEFAULT].</li>
                            <li><code>clone</code>: Mirrors 1:1 primary laptop screen (eDP-1) to external display.</li>
                            <li><code>--continuous</code> or <code>--no-drop-only</code>: Forces CFR transmission at steady 60 FPS even on static screens (eliminates pauses).</li>
                            <li><code>--network</code> or <code>--udp</code>: Transmits over UDP network (port 5000).</li>
                            <li><code>--transport=usb</code> or <code>--usb</code>: Transmits over direct USB Bulk channel (&lt; 1ms latency).</li>
                            <li><code>--no-audio</code>: Disables audio transmission stream.</li>
                            <li><code>--capture=kms</code>: Direct hardware DRM capture from GPU (/dev/dri/card*), immune to Wayland compositor idle states.</li>
                        </ul>
                        <div style="margin-top: 0.6rem; padding: 0.5rem; background: rgba(0,0,0,0.3); border-radius: 4px; font-size: 0.84rem; color: #ffab40;">
                            <strong>Quick Fix:</strong> If HDMI display goes dark when toggling modes, restart the pipeline with <code>ext-sender stop && ext-sender</code>. The receiver detects the stream and synchronizes IDR keyframes automatically in &lt; 1 second.
                        </div>
                    </div>

                    <!-- Comparison Table -->
                    <h3 style="color: #fff; margin: 1.5rem 0 0.5rem 0;" data-i18n="protoHeader">Protocol Comparison Table</h3>

                    <table class="proto-table">
                        <thead>
                            <tr>
                                <th data-i18n="thMethod">Method</th>
                                <th data-i18n="thProtocol">Protocol</th>
                                <th data-i18n="thLatency">Latency</th>
                                <th data-i18n="thBestFor">Best For</th>
                            </tr>
                        </thead>
                        <tbody>
                            <tr>
                                <td style="color: #fff; font-weight: 700;" data-i18n="tbM1Method">Direct Linux Wayland</td>
                                <td>RTP H.264 (UDP 5000)</td>
                                <td style="color: #7ee787; font-weight: 700;">&lt; 15 ms</td>
                                <td data-i18n="tbM1Best">Interactive desktop, smooth window dragging with mouse</td>
                            </tr>
                            <tr>
                                <td style="color: #fff; font-weight: 700;" data-i18n="tbM2Method">Windows 10/11 Miracast</td>
                                <td>WFD RTSP (TCP 7236)</td>
                                <td style="color: var(--accent-amber); font-weight: 700;">40–60 ms</td>
                                <td data-i18n="tbM2Best">Native driverless projection on Windows (Win + K)</td>
                            </tr>
                            <tr>
                                <td style="color: #fff; font-weight: 700;" data-i18n="tbM3Method">Direct USB Bulk</td>
                                <td>FunctionFS RFC 4571</td>
                                <td style="color: #7ee787; font-weight: 700;">&lt; 1 ms</td>
                                <td data-i18n="tbM3Best">Direct hardware communication without IP, RFC 4571 Marker Bit</td>
                            </tr>
                        </tbody>
                    </table>
                </div>
            </div>
        </section>
    </main>

    <!-- Modal de Confirmação de Reboot -->
    <div class="modal-overlay" id="rebootModal">
        <div class="modal-box">
            <div class="modal-title">
                <span>⚠️</span> <span data-i18n="modalRebootTitle">Reboot Appliance?</span>
            </div>
            <div class="modal-desc" data-i18n="modalRebootDesc">
                Are you sure you want to reboot the Raspberry Pi Zero? It will reboot in &lt; 2 seconds directly in RAM.
            </div>
            <div class="modal-actions">
                <button class="btn-secondary" onclick="closeRebootModal()" data-i18n="btnCancel">Cancel</button>
                <button class="btn-danger" onclick="executeReboot()" data-i18n="btnConfirmReboot">Yes, Reboot</button>
            </div>
        </div>
    </div>

    <!-- Toast Notification -->
    <div id="toast" class="toast"></div>

    <script>
        // State
        let currentFps = 30;
        let currentBitrate = 400;
        let currentColor = 'full';
        let currentDropOnly = false;
        let currentSkipToFirst = true;
        let currentKeyIntMax = 30;
        let currentCapture = 'kms';
        let currentMonitor = 'HDMI-1';
        let currentScale = '720p';
        let currentCas = true;
        let isPaused = false;

        // Internationalization Dictionary
        const I18N = {
            en: {
                title: "Pi Zero Extended Monitor",
                subtitle: "Hardware GPU VideoCore IV Display Appliance",
                tabMonitor: "Monitoring & Telemetry",
                tabConfig: "Tuning & Settings",
                tabDownloads: "Client Tools & Drivers",
                tabSdCard: "SD Card & RAM Upgrade",
                tabManual: "Operation Manual",
                statTemp: "SoC Temperature",
                statCpu: "CPU Load",
                statRam: "Free RAM",
                statStream: "Stream State",
                badgeVpuOffload: "VPU Offload",
                statCpuDesc: "Hardware GPU V4L2 M2M Active",
                badgeRamApp: "100% RAM",
                tipTemp: "Broadcom BCM2835 internal silicon temperature. Polled every 2s. Recommended: below 65°C.",
                tipCpu: "ARM11 CPU load. Stays below 2% because H.264 decoding is 100% offloaded to VideoCore IV VPU.",
                tipRam: "Free RAM out of 512 MB SDRAM. The entire system runs in RAM (initramfs) with zero SD card wear.",
                tipStream: "Real-time state of the video decoding engine streaming to the HDMI TV screen.",
                displayHeader: "HDMI Television & Display Telemetry",
                displayDesc: "The VideoCore IV hardware VPU decodes H.264 video streams directly to the HDMI scanout plane without touching the CPU.",
                btnShowHud: "✦ Show HUD on TV (60s)",
                btnHideHud: "✕ Turn Off HUD",
                modeHeader: "Active Streaming Modes",
                badgeMultiMode: "Concurrent Engine",
                m1Desc: "Direct low-latency RTP H.264 stream on UDP port 5000 with AMD VA-API zero-copy offload (< 15ms).",
                m2Desc: "Native Windows 10/11 wireless projection via Win + K on RTSP port 7236. Zero host drivers needed.",
                m3Desc: "Direct 480 Mbps raw hardware pipe via USB FunctionFS without network stack overhead (< 1ms).",
                netHeader: "Network Interfaces & IP Configuration (Ethernet / LAN / Wi-Fi)",
                netModeLabel: "Secondary Interface Addressing Mode",
                tipNetMode: "Choose Static IP for peer-to-peer setups without a DHCP router (e.g. on another Raspberry or PC after boot), or DHCP for automatic local network assignment.",
                ctrlHeader: "Display & Stream Optimization",
                badgeZeroCopy: "VideoCore IV DMA",
                bitrateLabel: "Streaming Bitrate (VBR)",
                tipBitrate: "Video encoding bitrate per second. 400k saves 85% bandwidth for text and code. 3000k to 6000k delivers smooth 60 FPS motion.",
                fpsLabel: "Framerate (FPS)",
                tipFps: "Target framerate. 15 FPS for ultra-cool SoC, 30 FPS recommended balance, 60 FPS maximum fluidity.",
                colorLabel: "Color Profile",
                tipColor: "24-bit TrueColor provides 1:1 RGB fidelity. 256 Colors uses adaptive QP (30-44) to minimize USB bus bandwidth.",
                colorFull: "24-bit TrueColor",
                color256: "256 Colors (QP 30-44)",
                colorGray: "Monochrome",
                dropOnlyLabel: "Transmission Mode (Continuous vs Economy)",
                tipDropOnly: "Continuous (Default): Steady 30/60 FPS stream for Network and USB Bulk, ensuring smooth YouTube playback without needing mouse movement. Economy: Drops duplicate frames, saving 95% bandwidth on static screens.",
                dropOnlyTrue: "Economy (Drop static frames)",
                dropOnlyFalse: "Continuous (Default: Videos / USB Bulk)",
                dropOnlyDesc: "Default: Continuous (drop-only=false). Steady stream for Network and USB Bulk. Use Economy or --economy flag for extreme battery/bandwidth savings on static text.",
                skipFirstLabel: "Instant Motion Delivery (skip-to-first)",
                tipSkipFirst: "Eliminates backlog delay by delivering the very first frame of motion immediately without queue latency.",
                skipFirstTrue: "Enabled (Zero latency on first motion)",
                skipFirstFalse: "Disabled (Strict timestamp alignment)",
                keyIntLabel: "Periodic Refresh / IDR Keyframe Interval (Clean Sweep)",
                tipKeyInt: "Frequency of full IDR I-Frames to sweep and recover from visual artifacts or packet drops.",
                keyIntDesc: "Injects a full IDR keyframe periodically to sweep and clear visual artifacts on HDMI/TV.",
                btnApply: "💾 Apply Settings",
                btnPauseStream: "⏸ Pause Display",
                btnResumeStream: "▶ Resume Display",
                btnReboot: "🔄 Reboot Appliance",
                oneLinerTitle: "One-Line Host Connector (1 Click)",
                badgeInstant: "Instant",
                oneLinerDesc: "On any Linux PC, paste this command in your terminal to start the extended monitor immediately:",
                btnCopy: "Copy",
                dlPkgTitle: "Full Client Package",
                dlPkgDesc: "Contains precompiled ext-sender binary, udev rules and documentation in tar.gz.",
                btnDlPkg: "Download client.tar.gz",
                dlSenderTitle: "ext-sender Binary",
                dlSenderDesc: "Standalone GPU offload transmitter binary compiled in Rust for Linux x86_64.",
                btnDlSender: "Download ext-sender",
                dlScriptTitle: "connect.sh Script",
                dlScriptDesc: "Portable script that detects USB, downloads needed tools and launches the stream.",
                btnDlScript: "Download connect.sh",
                dlUdevTitle: "udev Rules (Plug-and-Play)",
                dlUdevDesc: "Automatically configures host USB buffer (txqueuelen 100) to eliminate buffer bloat.",
                btnDlUdev: "Download 99-ext-monitor.rules",
                sdHeader: "100% RAM Architecture & Firmware Upgrade",
                badgeZeroSdWear: "Zero SD Wear",
                sdDesc1: "The Pi Zero boots entirely into RAM (initramfs). After a < 2s boot, the physical micro-SD (/dev/mmcblk0) is decoupled and never written to during usage.",
                sdDesc2: "This guarantees zero SD corruption risk and allows mounting the boot partition to upgrade firmware without removing the card!",
                sdPartitionStatus: "Physical Media Status:",
                btnMountSd: "Mount Partition (/mnt/boot)",
                btnUnmountSd: "Unmount Partition",
                sdUpgradeManualTitle: "How to Upgrade Firmware without Removing Card:",
                manualHeader: "Operation Manual & Technical Specifications",
                badgeFullDocs: "Complete Documentation",
                docArchTitle: "1. Hardware Architecture & VideoCore IV GPU",
                docArchDesc: "Raspberry Pi Zero utilizes the Broadcom BCM2835 SoC (ARM1176JZF-S @ 1.0 GHz) integrated with the VideoCore IV GPU @ 500 MHz. The ext-monitor runs a 100% userspace hardware pipeline: H.264 video streams are fed directly into the V4L2 M2M decoder (/dev/video10), which writes rendered frames via DMA directly to the HDMI scanout plane (/dev/fb0). This guarantees < 15ms latency and < 2% CPU usage.",
                docUsbProtoTitle: "2. Direct USB Bulk Protocol, RFC 4571 & End-of-Frame Marker Bit (EOF / ZLP)",
                docUsbProtoDesc1: "On the USB 2.0 High-Speed bus (480 Mbps), packets travel in micro-frames up to 512 bytes (wMaxPacketSize). In raw H.264 Annex-B streams, decoders had to wait for the next frame's start code (00 00 00 01) to verify completion, causing stalls when mouse movement stopped.",
                docUsbProtoDesc2: "ext-monitor resolves this by implementing standard RFC 4571 2-byte length-delimited framing with RTP Marker Bit (EOF) signaling. As soon as a frame finishes transmitting over USB, the receiver drains and renders it immediately on HDMI, eliminating mouse dependency and ensuring smooth YouTube playback. Furthermore, transfers that are exact multiples of 512 bytes trigger a Zero-Length Packet (ZLP), preventing DWC2 hardware FIFO stalls.",
                docAudioTitle: "3. Multi-Mode Audio Routing (Normal vs. Hybrid)",
                docAudioDesc1: "ext-monitor supports stereo digital audio routing to HDMI TV speakers across 4 transport technologies:",
                docAudioDesc2: "Normal vs Hybrid Mode: In Normal mode, both PC video and audio play together on the TV. In Hybrid mode, you can keep audio playing on local PC speakers while video streams to TV, or pair a smartphone via Bluetooth to play music on the TV while the PC displays code.",
                docSerialTitle: "4. Zero-IP Reconfiguration via USB Serial (/dev/ttyACM0)",
                docSerialDesc: "If network is disabled or misconfigured, the Pi Zero exposes a recovery serial console on /dev/ttyACM0 at 115200 baud.",
                docLinuxTitle: "5. Standard Linux Wayland (GNOME) Streaming",
                docLinuxDesc: "Plug into the center USB port. The PC gets IP 192.168.7.1 automatically. Then run ext-sender (zero configuration needed):",
                docWinTitle: "6. Windows 10/11 Miracast (Zero Drivers)",
                docWinDesc: "Plug into USB, press Win + K on Windows, select 'Pi Zero Wireless Display'.",
                docSdTitle: "7. 100% RAM Architecture & Firmware Upgrade without SD Card Removal",
                docSdDesc: "The appliance runs 100% in an initramfs RAM disk. To upgrade receiver binaries, simply mount the boot FAT partition with 'mount -t vfat /dev/mmcblk0p1 /mnt', write the new image, and unmount without rebooting or touching the SD card.",
                docBrowserVideoTitle: "8. 💡 Tip: Continuous Browser Video Playback (Chrome / Firefox)",
                docBrowserVideoDesc: "On Linux Wayland, browsers like Chrome and Firefox enable aggressive power-saving ('Window Occlusion Tracking') and pause video rendering when the mouse cursor leaves the window or it loses focus. To ensure smooth 60 FPS video without cursor focus:",
                docBrowserChrome: "Visit chrome://flags/#calculate-native-win-occlusion, set to 'Disabled' and restart (or launch with --disable-backgrounding-occluded-windows).",
                docBrowserFirefox: "Visit about:config, search for media.suspend-bkgnd-video.enabled and toggle to 'false'.",
                docBrowserNative: "Play at continuous 60 FPS by default without any cursor position restrictions.",
                docArmv6Title: "9. Cross-Compiling for Raspberry Pi Zero (ARMv6)",
                docArmv6Desc: "The BCM2835 SoC on Pi Zero v1.2/v1.3/W requires the ARMv6l architecture. To compile with cross (Docker):",
                docArmv6Cmd: "cargo install cross && cd receiver && cross build --target arm-unknown-linux-musleabihf --release",
                docNonOtgTitle: "10. Non-OTG Raspberry Pi Models (Pi 2, Pi 3, Pi 4, Pi 5) & Boot Config",
                docNonOtgDesc1: "These models do not support peripheral USB gadget mode on standard USB-A ports. Streaming is delivered via Ethernet or Wi-Fi.",
                docNonOtgDesc2: "Required SD boot adjustments: In config.txt comment out 'dtoverlay=dwc2'. In cmdline.txt remove 'modules-load=dwc2'. Stream via: ext-sender --ip=<PI_IP>",
                docPcReceiverTitle: "11. Turning Any Linux PC / Laptop into a Secondary Screen Receiver",
                docPcReceiverDesc: "Any Linux computer can act as a receiver. Install gstreamer1.0-tools and run:",
                docPcReceiverCmd: "gst-launch-1.0 -v udpsrc port=5000 buffer-size=524288 caps=\"application/x-rtp,media=video,clock-rate=90000,encoding-name=H264,payload=96\" ! rtph264depay ! h264parse ! avdec_h264 ! autovideosink sync=false",
                docMultiMonTitle: "12. Directing Video to Screen 1 vs Screen 2 on Multi-Monitor PCs",
                docMultiMonDesc: "On receiver PCs with multiple connected displays, specify 'kmssink connector-id=<ID>' in direct DRM KMS mode or 'ffplay -left 1920 -top 0 -fs rtp://0.0.0.0:5000' in graphical sessions to target the desired monitor.",
                docPacerTitle: "13. 🚀 Continuous 60 FPS Anti-Freeze Streaming",
                docPacerDesc: "ext-monitor provides continuous 60 FPS CFR anti-freeze streaming natively inside ext-sender. YouTube videos, clocks, and terminals render smoothly at 60 FPS even when the mouse is motionless or on the primary screen.",
                docAltPlayersTitle: "14. Non-GNOME Receiver Alternatives (FFmpeg, MPV, VLC)",
                docAltPlayersDesc: "The RFC 4571 RTP H.264 video stream is fully cross-platform. Receive it on KDE, XFCE, i3, Windows, or macOS with zero-buffer low delay:",
                docBulkFallbackTitle: "15. ⚡ Default USB Bulk Direct Mode & Automatic UDP Network Fallback",
                docBulkFallbackDesc: "ext-sender prioritizes high-speed 480 Mbps USB Bulk Direct mode by default. If the USB gadget interface is not detected, it automatically falls back to UDP Network streaming (port 5000) without crashing.",
                lblActiveMode: "Active Streaming Mode",
                lblActiveHdmi: "Active Raspberry Pi HDMI Output",
                protoHeader: "Protocol Comparison Table",
                thMethod: "Method",
                thProtocol: "Protocol",
                thLatency: "Latency",
                thBestFor: "Best For",
                tbM1Method: "Direct Linux Wayland",
                tbM1Best: "Interactive desktop, smooth window dragging with mouse",
                tbM2Method: "Windows 10/11 Miracast",
                tbM2Best: "Native driverless projection on Windows (Win + K)",
                tbM3Method: "Direct USB Bulk",
                tbM3Best: "Direct hardware communication without IP, RFC 4571 Marker Bit",
                bookTitle: "The Ext-Monitor Book — Engineering & Architecture Compendium",
                bookBadge: "Compendium of 18 Blueprints",
                bookDesc: "Hardware reverse engineering documentation, BCM2835 silicon decisions, network protocols (RFC 4571 / RFC 6184 / WFD), hybrid audio, and the Wayland/DRM-KMS pipeline are consolidated in the master compendium: <strong style=\"color: #fff; font-family: monospace;\">docs/LIVRO-EXT-MONITOR.md</strong>.",
                bookPart1Title: "Part I: BCM2835 Silicon & Boot",
                bookPart1Desc: "Boot 1.8s, FAT16 32MB, VideoCore IV V4L2 M2M, Zero-Copy DMA and DMA-BUF.",
                bookPart2Title: "Part II: Protocols & Buses",
                bookPart2Desc: "USB Bulk RFC 4571 Marker Bit/ZLP, RTP H.264 FU-A, WFD Miracast and UAC2.",
                bookPart3Title: "Part III: Linux Host & Wayland",
                bookPart3Desc: "GNOME Mutter Screencast D-Bus, PipeWire, AMD Radeon 610M DCN 3.1 and DRM/KMS.",
                bookPart4Title: "Part IV: Hybrid Audio & Web Control",
                bookPart4Desc: "Opus 48kHz UDP, BlueZ A2DP Sink on Pi Zero W and Bidirectional Rust Web Panel.",
                docAudioMode1: "<strong style=\"color: #fff;\">Mode 1 (IP Network):</strong> Virtual PipeWire sink on PC transmitting RTP Opus/PCM on UDP port 5004 directly to the Pi Zero HDMI codec.",
                docAudioMode2: "<strong style=\"color: #fff;\">Mode 2 (Windows Miracast):</strong> Native Wi-Fi Display (WFD) protocol routing stereo AAC/LPCM audio via RTSP TCP 7236.",
                docAudioMode3: "<strong style=\"color: #fff;\">Mode 3 (USB Audio Gadget - UAC2):</strong> Plug-and-play USB sound card exposed by Composite Gadget on PC with direct PCM forwarding to TV HDMI.",
                docAudioMode4: "<strong style=\"color: #fff;\">Mode 4 (Bluetooth A2DP Sink):</strong> Bluetooth audio receiver for pairing with smartphones, tablets, or secondary laptops.",
                docAudioRoutingTitle: "16. 🔊 Hybrid Audio Routing & Virtual HDMI Device",
                docAudioRoutingDesc: "ext-monitor implements a 100% isolated and hybrid audio architecture using a virtual PipeWire/PulseAudio sink (<code>Raspberry_Pi_HDMI_Audio</code>). You can work on the extended TV display while keeping YouTube, meetings and music on your laptop speakers or USB headset:",
                docClocksTitle: "17. ⚙️ Subhardware Clocks (H.264/VPU/ARM) & Power Consumption",
                docClocksDesc: "The Raspberry Pi Zero W exposes internal telemetry of every Broadcom BCM2835 silicon block via <code>debugfs</code>. ext-monitor monitors hardware clocks continuously in real-time:",
                docCliManualTitle: "18. 📖 Full Operation Manual, CLI Flags & Troubleshooting",
                docCliManualDesc: "Launcher syntax: <code>ext-sender [extend|clone] [fps] [bitrate] [options]</code>",
                badgeStreaming: "● STREAMING",
                badgeMode1On: "Enabled (UDP 5000)",
                badgeMode2On: "Enabled (TCP 7236)",
                badgeMode3On: "Enabled (USB Bulk)",
                hostHeader: "Transmitter Remote Control (Host PC)",
                hostBadge: "Bidirectional UDP 5001",
                hostStatusLabel: "Transmission Status & Actions",
                btnHostStart: "▶ Start / Restart Stream",
                btnHostStop: "⏹ Stop Stream",
                hostTransportLabel: "Active Transport / Transmission Mode",
                tipHostTransport: "Switch transmission protocol on the fly: USB Bulk Direct (< 1ms raw pipe), Network UDP (port 5000), or Windows Miracast (Win+K RTSP).",
                btnSelectMode1: "▶ Switch to Network UDP",
                btnSelectMode2: "▶ Switch to Miracast (Win+K)",
                btnSelectMode3: "▶ Switch to USB Bulk",
                hostModeLabel: "Display Mode / Chrome Cast Behavior",
                tipHostMode: "HDMI-1: Virtual extended second screen on TV. eDP-1: Clones notebook primary screen. Dialog prompt applies exclusively when casting from Google Chrome.",
                btnModeAsk: "🌐 Cast: Prompt on Screen",
                btnModeExtend: "🖥️ Extended (HDMI-1 TV)",
                btnModeClone: "💻 Cloned (eDP-1 Notebook)",
                toastModeAsk: "🌐 Google Cast: Will prompt to Extend or Clone when casting",
                hostAudioLabel: "Hybrid Audio (IP Network Opus + Bluetooth A2DP)",
                tipHostAudio: "Enable network audio to stream PC sound via 48kHz Opus to TV HDMI. Use Bluetooth pairing to connect phones/tablets directly to TV.",
                btnHostAudioOn: "🔊 Network Audio (Opus UDP)",
                btnHostAudioOff: "🔇 Disable Network Audio",
                btnBtPair: "📡 Pair Bluetooth A2DP (60s)",
                hostHudLabel: "Telemetry HUD on TV Screen",
                tipHostHud: "Projects live FPS, bitrate, and latency stats in the bottom-right corner of the TV for 60 seconds.",
                btnHostShowHud: "📊 Show HUD on TV (60s)",
                btnHostHideHud: "❌ Hide HUD",
                mediaHeader: "IoT Media Center & HDMI Visualizer (Chromecast / DLNA)",
                audioHeader: "Hi-Res Digital Audio, DAC & Integrated Streaming",
                audioBadge: "ALSA Hardware PCM",
                audioCardDesc: "Centralized control: select active transmission channel (Mode 1, 2, or 3), screen topology (Extend or Clone), adjust digital audio volume, manage FFT spectrum visualizer, and configure ALSA master clock.",
                audioModeLabel: "Active Transmission Channel (Video + HDMI Audio)",
                tipAudioMode: "Switches physical streaming pipeline between PC and Raspberry Pi. Instant on-the-fly switching without rebooting.",
                audioTopologyLabel: "Screen Topology on Laptop (Wayland / Mutter)",
                tipAudioTopology: "Extend desktop to TV as second virtual monitor (HDMI-1) or mirror notebook primary display (eDP-1).",
                castHeader: "Web Sharing & Chromecast-Style Casting (Google Cast)",
                castDesc: "Mirror your browser tabs, windows, video URLs or mobile screen directly to the TV just like a real Chromecast device.",
                webCastTitle: "Cast Browser Tab or Screen (Web Cast)",
                webCastDesc: "Stream any Chrome/Firefox tab, Meet/Teams call, or entire display with low-latency WebCodecs hardware encoding.",
                btnOpenWebCast: "🔴 Open Web Caster (/cast)",
                googleCastTitle: "Google Cast (Native Chromecast)",
                googleCastDesc: "In Chrome/Edge menu (Cast...) or phone apps (YouTube, Netflix, Pluto TV), select 'Ext-Monitor (Raspberry Pi)' to cast directly.",
                googleCastReady: "✓ mDNS & Cast V2 Active (Ports 8008 / 8009)",
                castUrlLabel: "Direct Video URL Cast (Play on TV)",
                tipCastUrl: "Enter a video URL (MP4, WebM, HLS m3u8) to decode and display directly on the Raspberry Pi HDMI output.",
                btnCastUrl: "▶ Cast to TV",
                toastEnterUrl: "Please enter a valid video URL",
                toastCasting: "Sending video to TV screen...",
                toastCastSuccess: "✓ Video stream casted to TV!",
                badgeHdmiMuxStandby: "Mode: Standby (Ready Splash)",
                badgeHdmiMuxPc: "Mode: PC Video (TV Screen + Audio)",
                badgeHdmiMuxIot: "Mode: IoT Audio (HDMI Visualizer 30 FPS)",
                mediaBadge: "Google Home • UPnP • Bluetooth",
                mediaPlaybackLabel: "Current Playback (IoT Audio on TV)",
                tipMediaPlayback: "Displays metadata of music currently playing via Bluetooth from smartphone or UPnP/Cast network stream.",
                mediaVisLabel: "Graphic Visualizer on TV Screen (Never Dark/Blank)",
                tipMediaVis: "When audio plays without PC desktop video, renders 24-band frequency spectrum and VU meter at 30 FPS on HDMI, preventing the TV from going dark or sleeping.",
                btnVisOn: "🎨 Enable HDMI Visualizer",
                btnVisOff: "⏹ Disable Visualizer",
                btnTestAudioChime: "🔊 Test Hardware Audio Signal",
                netUsb0Label: "USB OTG (usb0)",
                netUsb0Desc: "● Permanent Host OTG",
                netEth0Label: "Physical Ethernet (eth0)",
                netEth0Desc: "● Static / DHCP",
                netWlan0Label: "Wi-Fi (wlan0)",
                netWlan0Desc: "● Optional",
                netIfaceLabel: "Interface:",
                btnNetStatic: "📌 Static IP",
                btnNetDhcp: "🔄 Automatic DHCP",
                netIpLabel: "IP Address:",
                netMaskLabel: "Subnet Mask:",
                netGwLabel: "Default Gateway:",
                netDnsLabel: "DNS Server:",
                btnApplyNet: "💾 Save & Apply to Board",
                btnAudioMute: "🔊 Mute Audio",
                captureLabel: "Capture Engine (Dual-Engine)",
                tipCapture: "KMS Direct: Reads pixels directly from GPU hardware scanout via Linux Kernel DRM/KMS. Eliminates freezing even when mouse is stationary. Mutter: Captures via GNOME Mutter D-Bus screencast.",
                btnKmsDirect: "⚡ KMS Direct (Anti-Freeze / GPU Scanout)",
                btnGnomeMutter: "🐧 GNOME Mutter (PipeWire Screencast)",
                monitorTargetLabel: "Recording / Capture Display",
                tipMonitorTarget: "Choose the video output to capture. HDMI-1 for extended second screen on TV/monitor, eDP-1 to clone notebook screen.",
                scaleLabel: "Resolution & Display Scaling",
                tipScale: "Select display resolution. 1:1 Native modes render crisp vector fonts without blur. Upscaling modes utilize VideoCore IV silicon HVS or GPU FSR to fill widescreen monitors.",
                lblSiliconScaler: "Silicon Hardware Scaler (VideoCore IV HVS & FSR)",
                badgeSiliconActive: "HVS Active",
                badgeSiliconOff: "Native Only",
                optgroupNative: "Native 1:1 Direct Modes (Crisp / No Blur)",
                optgroupUpscale: "Super-Resolution Upscaling (Silicon HVS / FSR)",
                btnScale720p: "🎯 720p Native (1:1)",
                btnScale1600x900: "📐 900p Upscale",
                btnScaleOff: "⚡ Passthrough",
                casLabel: "Contrast Adaptive Sharpening (CAS)",
                tipCas: "Restores full PC color range (0-255) and deep contrast, eliminating washed-out video encoding artifacts.",
                btnCasTrue: "✨ CAS Enabled (Crisp Text / Full Black)",
                btnCasFalse: "Standard (TV Limited Range)",
                statStreamActive: "Active",
                statStreamPaused: "Paused",
                waitingStream: "Awaiting Stream (Splash Screen Ready)",
                mediaPlaying: "Playing Audio",
                mediaPaused: "Paused",
                mediaIdle: "Idle / Ready",
                visActive: "Active (30 FPS)",
                visDisabled: "Disabled",
                toastTestAudio: "🔊 Triggering hardware audio test signal on HDMI...",
                toastMuted: "HDMI Audio Muted",
                toastUnmuted: "HDMI Audio Active",
                mute: "Mute Audio",
                unmute: "Unmute Audio",
                netConnected: "Connected",
                netDisconnected: "Disconnected",
                cableDisconnected: "Cable Disconnected",
                modalRebootTitle: "Reboot Appliance?",
                modalRebootDesc: "Are you sure you want to reboot the Raspberry Pi Zero? It will reboot in < 2 seconds directly in RAM.",
                btnCancel: "Cancel",
                btnConfirmReboot: "Yes, Reboot",
                audioLabel: "HDMI Digital Audio (Opus 48kHz)",
                tipAudio: "Digital audio volume sent to monitor/TV via HDMI cable. Sub-25ms latency with A/V sync.",
                audioProfileLabel: "HDMI Master Audio Profile & Sample Rate",
                tipAudioProfile: "Select and force-load the hardware sample rate clock directly into the Pi Zero BCM2835 ALSA sound core. Supports true IEC958 subframe audio up to 192kHz 24-bit Hi-Res.",
                audioTransportLabel: "Audio Transport Architecture",
                tipAudioTransport: "Select the physical transport path for digital audio: Mode 1 UDP Network Stream (Port 5004), Mode 2 USB Audio Class (UAC2 Gadget), or Mode 3 USB Bulk Multiplexed.",
                audioTransportDesc: "Mode 1 UDP Network: Audio stream arrives via UDP port 5004 in 1024-byte unfragmented packets. True sub-5ms delay with automatic A/V synchronization.",
                audioTransUdpDesc: "Mode 1 UDP Network: Audio stream arrives via UDP port 5004 in 1024-byte unfragmented packets. True sub-5ms delay with automatic A/V synchronization.",
                audioTransUac2Desc: "Mode 2 UAC2 Gadget: Pi Zero acts as a native USB Sound Card on PC. Zero network dependency, plug-and-play in Windows and Linux.",
                audioTransBulkDesc: "Mode 3 USB Bulk: Audio is multiplexed into the /dev/usb-display-bulk pipe alongside H.264 video. Sub-1ms latency.",
                docAltPlayersCmd: "# FFmpeg / ffplay (Low Latency):\nffplay -probesize 32 -analyzeduration 0 -sync ext -fflags nobuffer -flags low_delay -i 'rtp://192.168.7.2:5000'",
                copied: "Copied!",
                copiedSuccess: "✓ Copied to clipboard!",
                m1Title: "Mode 1: UDP Network (Linux Wayland / X11)",
                m1Details: "UDP Port 5000 • Sub-15ms Latency • VA-API/M2M Pipeline",
                m2Title: "Mode 2: Windows Miracast (Wi-Fi Display)",
                m2Details: "RTSP Port 7236 • Windows Win+K • Hardware V4L2 M2M",
                m3Title: "Mode 3: USB Bulk Direct (480 Mbps)",
                m3Details: "USB 2.0 High-Speed • Zero-Network • Sub-1ms Latency",
                waitingStreamDesc: "Receiver in readiness displaying splash screen with IP & QR Code.",
                capKmsTitle: "KMS Direct (Anti-Freeze / GPU Scanout)",
                capMutterTitle: "GNOME Mutter (PipeWire Screencast)",
                connHeader: "Active Extension Connection (Active Transport)",
                connBadge: "Hot-Switchable",
                connDesc: "Select the active transmission pipeline between your PC and this screen. Switches immediately on the fly without rebooting.",
                extHeader: "Display Extension Action & Topology",
                extBadge: "Active Screen",
                extBadgeActive: "Extending (HDMI-1)",
                extBadgeClone: "Cloning (eDP-1)",
                extBadgeOff: "Standby (Paused)",
                extDesc: "Extend desktop area onto HDMI-1, clone primary notebook display (eDP-1), or turn off the extension to put the screen on standby.",
                btnActionExtend: "🖥️ Extended Display (HDMI-1)",
                btnActionExtendDesc: "Virtual second monitor on TV",
                btnActionClone: "💻 Clone Screen (eDP-1)",
                btnActionCloneDesc: "Mirror primary notebook screen",
                btnActionStop: "⏹ Disable Extension / Standby",
                btnActionStopDesc: "Stop transmission & put TV on standby",
                toastExtStopped: "⏹ Screen extension disabled. TV in standby.",
                toastExtCloned: "💻 Mirroring notebook display (eDP-1)...",
                toastExtExtended: "🖥️ Extending desktop to TV (HDMI-1)...",
                servicesHeader: "Appliance Listener Daemons & Services",
                servicesBadge: "Hardware Listeners",
                lblScreenMode: "Display Mode:",
                btnM1Extend: "🖥️ Extend (UDP)",
                btnM1Clone: "💻 Clone (UDP)",
                btnM3Extend: "🖥️ Extend (USB)",
                btnM3Clone: "💻 Clone (USB)"
            },
            pt: {
                title: "Pi Zero Monitor Estendido",
                subtitle: "Painel de Controle e Appliance GPU VideoCore IV",
                tabMonitor: "Monitoramento & Telemetria",
                tabConfig: "Ajustes & Configurações",
                tabDownloads: "Downloads & Driver",
                tabSdCard: "Cartão SD & Upgrade em RAM",
                tabManual: "Manual de Operação",
                statTemp: "Temperatura SoC",
                statCpu: "Carga da CPU",
                statRam: "RAM Disponível",
                statStream: "Estado do Stream",
                badgeVpuOffload: "GPU VPU Offload",
                statCpuDesc: "Hardware GPU V4L2 M2M Ativo",
                badgeRamApp: "100% em RAM",
                tipTemp: "Temperatura interna do processador BCM2835. Monitorada a cada 2s. Ideal: abaixo de 65°C.",
                tipCpu: "Carga da CPU ARM11. Permanece menor que 2% porque a decodificação H.264 ocorre na GPU VideoCore IV.",
                tipRam: "Memória RAM livre dos 512 MB SDRAM. O appliance opera 100% em RAM sem tocar no micro-SD.",
                tipStream: "Status de recebimento e decodificação do fluxo de vídeo transmitido para a porta HDMI.",
                displayHeader: "Telemetria do Monitor HDMI & TV",
                displayDesc: "A VPU de hardware VideoCore IV decodifica o stream H.264 direto na memória de scanout da TV sem tocar na CPU.",
                btnShowHud: "✦ Exibir HUD na TV (60s)",
                btnHideHud: "✕ Ocultar HUD",
                modeHeader: "Modos de Transmissão Ativos",
                badgeMultiMode: "Motor Concorrente",
                m1Desc: "Transmissão RTP H.264 de latência ultra-baixa na porta UDP 5000 com GPU AMD VA-API (< 15ms).",
                m2Desc: "Projeção nativa do Windows 10/11 via Win + K na porta RTSP 7236. Zero drivers no PC.",
                m3Desc: "Canal direto de 480 Mbps por hardware via USB FunctionFS sem pilha de rede (< 1ms).",
                netHeader: "Configuração de Rede & Interfaces (Ethernet / LAN / Wi-Fi)",
                netModeLabel: "Modo de Endereçamento da Interface Secundária",
                tipNetMode: "Escolha IP Estático para conexões ponto-a-ponto sem roteador DHCP (ex: em outro Raspberry ou PC após o boot), ou DHCP para obter IP automático da sua rede local.",
                ctrlHeader: "Otimização de Exibição e Stream",
                badgeZeroCopy: "VideoCore IV DMA",
                bitrateLabel: "Taxa de Bits (VBR)",
                tipBitrate: "Taxa de bits de codificação. 400k economiza 85% para texto/código. 3000k a 6000k entrega vídeo fluido em 1080p/720p.",
                fpsLabel: "Taxa de Quadros (FPS)",
                tipFps: "Taxa de quadros por segundo. 15 FPS para ultra-economia, 30 FPS padrão balanceado, 60 FPS fluidez total.",
                colorLabel: "Perfil de Cor",
                tipColor: "24-bit TrueColor para fidelidade RGB nativa. 256 Cores quantiza para economia máxima de barramento.",
                colorFull: "24-bit TrueColor",
                color256: "256 Cores (QP 30-44)",
                colorGray: "Monocromático",
                dropOnlyLabel: "Modo de Transmissão (Contínuo vs Econômico)",
                tipDropOnly: "Contínuo (Padrão): Transmissão ininterrupta a 30/60 FPS no modo de Rede e USB Bulk, garantindo YouTube e vídeos sem congelar mesmo sem mouse na tela. Econômico: Descarta quadros repetidos, economizando 95% de banda em telas estáticas.",
                dropOnlyTrue: "Econômico (Descarta estáticos)",
                dropOnlyFalse: "Contínuo (Padrão: Vídeos / USB Bulk)",
                dropOnlyDesc: "Padrão: Contínuo (drop-only=false). O stream entrega fluxo constante a 30/60 FPS no modo de Rede e USB Bulk. Para economizar banda/bateria em leitura de PDFs ou terminais estáticos, selecione 'Econômico' ou use a flag --economy no host.",
                skipFirstLabel: "Entrega Imediata no Primeiro Quadro (skip-to-first)",
                tipSkipFirst: "Elimina atrasos acumulados, entregando imediatamente o primeiro quadro assim que o mouse se move.",
                skipFirstTrue: "Ativo (Latência zero ao mover)",
                skipFirstFalse: "Desativado (Sincronismo rígido)",
                keyIntLabel: "Varredura Periódica / Intervalo IDR (Refresh Clean)",
                tipKeyInt: "Frequência de quadros-chave I-Frame para autolimpeza de ruídos visuais e recuperação de perdas.",
                keyIntDesc: "Injeta um quadro-chave IDR completo periodicamente para limpar qualquer resíduo visual na TV ou monitor HDMI.",
                btnApply: "💾 Aplicar Alterações",
                btnPauseStream: "⏸ Pausar Exibição",
                btnResumeStream: "▶ Retomar Exibição",
                btnReboot: "🔄 Reiniciar Appliance (Reboot)",
                oneLinerTitle: "Conector de 1 Linha para PC (1 Clique)",
                badgeInstant: "Instantâneo",
                oneLinerDesc: "Em qualquer computador Linux, abra o terminal e cole o comando abaixo para iniciar a segunda tela:",
                btnCopy: "Copiar",
                dlPkgTitle: "Pacote Completo do Cliente",
                dlPkgDesc: "Contém o binário ext-sender compilado, regras udev e documentação em tar.gz.",
                btnDlPkg: "Baixar client.tar.gz",
                dlSenderTitle: "Executável ext-sender",
                dlSenderDesc: "Binário standalone do transmissor GPU offload compilado em Rust para Linux x86_64.",
                btnDlSender: "Baixar ext-sender",
                dlScriptTitle: "Script connect.sh",
                dlScriptDesc: "Script portátil que detecta a USB, baixa os componentes necessários e inicia o streaming.",
                btnDlScript: "Baixar connect.sh",
                dlUdevTitle: "Regras udev (Plug-and-Play)",
                dlUdevDesc: "Configura automaticamente o buffer USB (txqueuelen 100) para eliminar buffer bloat ao plugar.",
                btnDlUdev: "Baixar 99-ext-monitor.rules",
                sdHeader: "Arquitetura 100% RAM & Atualização de Firmware",
                badgeZeroSdWear: "Zero Desgaste do SD",
                sdDesc1: "O Pi Zero roda 100% em RAM (initramfs). Após o boot de 2s, o cartão micro-SD (/dev/mmcblk0) fica desacoplado e nunca sofre escritas durante o uso diário.",
                sdDesc2: "Isso garante zero risco de corrupção e permite montar a partição FAT16 para atualizar o firmware sem retirar o cartão!",
                sdPartitionStatus: "Status da Mídia Física:",
                btnMountSd: "Montar Partição (/mnt/boot)",
                btnUnmountSd: "Desmontar Partição",
                sdUpgradeManualTitle: "Como Atualizar o Appliance sem Retirar o Cartão:",
                manualHeader: "Manual de Operação e Especificações Técnicas",
                badgeFullDocs: "Documentação Completa",
                docArchTitle: "1. Arquitetura de Hardware e GPU VideoCore IV",
                docArchDesc: "O Raspberry Pi Zero utiliza o SoC Broadcom BCM2835 (ARM1176JZF-S a 1.0 GHz) integrado à GPU VideoCore IV a 500 MHz. O ext-monitor opera com pipeline 100% de hardware em userspace: o stream H.264 é alimentado diretamente no decodificador V4L2 M2M (/dev/video10), que grava os quadros renderizados por DMA direto no framebuffer HDMI (/dev/fb0). Isso garante latência inferior a 15 ms e consumo de CPU inferior a 2%.",
                docUsbProtoTitle: "2. Protocolo USB Bulk, RFC 4571 e Marcador de Fim de Quadro (EOF / ZLP)",
                docUsbProtoDesc1: "No barramento USB 2.0 High-Speed (480 Mbps), pacotes de dados trafegam em micro-frames de até 512 bytes (wMaxPacketSize). Em fluxos H.264 Annex-B brutos, decodificadores precisavam aguardar o start code (00 00 00 01) do quadro seguinte para saber que o quadro atual terminou, causando congelamento quando o mouse parava ou vídeos estáticos entravam em pausa.",
                docUsbProtoDesc2: "O ext-monitor resolve isso implementando o padrão oficial RFC 4571 com enquadramento de comprimento de 2 bytes e preservação do Bit Marcador RTP (Marker Bit / EOF). No instante em que o último pedaço de um quadro chega pela USB, o receptor dispara a descarga imediata no display HDMI, eliminando qualquer dependência de movimento de mouse e permitindo reprodução estável de YouTube sem congelar. Além disso, pacotes múltiplos de 512 bytes disparam um ZLP (Zero-Length Packet), liberando a FIFO de hardware DWC2 sem travamentos.",
                docAudioTitle: "3. Roteamento de Áudio Multi-Modo (Normal vs. Híbrido)",
                docAudioDesc1: "O ext-monitor suporta roteamento de áudio digital estéreo para as caixas de som da TV HDMI através de 4 tecnologias de transporte:",
                docAudioDesc2: "Operação Normal vs Híbrida: No modo Normal, o vídeo e o som do seu PC tocam juntos na TV. No modo Híbrido, você pode escolher manter o áudio tocando nos alto-falantes locais do notebook enquanto apenas o vídeo vai para a TV, ou tocar músicas via Bluetooth do celular na TV enquanto o PC exibe seu editor de código.",
                docSerialTitle: "4. Reconfiguração sem IP via Serial USB (/dev/ttyACM0)",
                docSerialDesc: "Caso a rede seja desativada, o Pi Zero expõe um console serial independente no PC em /dev/ttyACM0 a 115200 baud.",
                docLinuxTitle: "5. Operação Normal no Linux Wayland (GNOME)",
                docLinuxDesc: "Conecte o cabo na porta USB central. O PC recebe IP 192.168.7.1 pelo DHCP nativo. Em seguida execute o ext-sender (sem parâmetros necessários):",
                docWinTitle: "6. Operação no Windows 10/11 (Miracast Sem Drivers)",
                docWinDesc: "Conecte na USB, pressione Win + K no Windows e selecione 'Pi Zero Wireless Display'.",
                docSdTitle: "7. Arquitetura 100% RAM & Atualização sem Retirar o Cartão",
                docSdDesc: "O appliance roda 100% em initramfs RAM disk. Para atualizar o binário do receptor, basta montar a partição FAT de boot com 'mount -t vfat /dev/mmcblk0p1 /mnt', gravar a nova imagem e desmontar, sem necessidade de desligar o dispositivo.",
                docBrowserVideoTitle: "8. 💡 Dica: Reprodução Contínua de Vídeos no Navegador (Chrome / Firefox)",
                docBrowserVideoDesc: "No Linux Wayland, navegadores como Chrome e Firefox ativam economia de energia ('Window Occlusion Tracking') e pausam a renderização de vídeos quando o cursor sai da janela ou ela perde o foco. Para manter 60 FPS contínuos mesmo sem o cursor sobre a janela:",
                docBrowserChrome: "Acesse chrome://flags/#calculate-native-win-occlusion, selecione 'Disabled' e reinicie o navegador (ou use a flag --disable-backgrounding-occluded-windows).",
                docBrowserFirefox: "Acesse about:config, busque por media.suspend-bkgnd-video.enabled e altere para 'false'.",
                docBrowserNative: "Reproduzem a 60 FPS contínuos por padrão, sem interrupção por foco ou posição de mouse.",
                docArmv6Title: "9. Compilação para Raspberry Pi Zero (ARMv6)",
                docArmv6Desc: "O SoC BCM2835 do Pi Zero v1.2/v1.3/W requer arquitetura ARMv6l. Para compilar com cross (Docker):",
                docArmv6Cmd: "cargo install cross && cd receiver && cross build --target arm-unknown-linux-musleabihf --release",
                docNonOtgTitle: "10. Modelos Raspberry Pi Não-OTG (Pi 2, Pi 3, Pi 4, Pi 5) & Ajuste de Boot",
                docNonOtgDesc1: "Esses modelos não possuem modo OTG periférico nas portas USB comuns. A conexão é feita via Ethernet ou Wi-Fi.",
                docNonOtgDesc2: "Ajustes obrigatórios no SD: No config.txt comente 'dtoverlay=dwc2'. No cmdline.txt remova 'modules-load=dwc2'. Conecte via: ext-sender --ip=<IP_DO_PI>",
                docPcReceiverTitle: "11. Transformar PC / Notebook Convencional em Segunda Tela",
                docPcReceiverDesc: "Qualquer computador Linux pode atuar como receptor. Instale gstreamer1.0-tools e execute:",
                docPcReceiverCmd: "gst-launch-1.0 -v udpsrc port=5000 buffer-size=524288 caps=\"application/x-rtp,media=video,clock-rate=90000,encoding-name=H264,payload=96\" ! rtph264depay ! h264parse ! avdec_h264 ! autovideosink sync=false",
                docMultiMonTitle: "12. Direcionamento para Tela 1 ou 2 em PCs com Múltiplos Monitores",
                docMultiMonDesc: "Em PCs receptores com mais de uma tela conectada, use 'kmssink connector-id=<ID>' no modo direto KMS DRM ou 'ffplay -left 1920 -top 0 -fs rtp://0.0.0.0:5000' em sessão gráfica para projetar exatamente no monitor desejado.",
                docPacerTitle: "13. 🚀 Transmissão Contínua 60 FPS Anti-Freeze",
                docPacerDesc: "O ext-monitor fornece transmissão contínua a 60 FPS CFR anti-freeze de forma nativa no ext-sender. Vídeos do YouTube, clocks e terminais atualizam a 60 FPS contínuos mesmo com o mouse parado ou na tela principal.",
                docAltPlayersTitle: "14. Alternativas ao GStreamer em Ambientes Não-GNOME (FFmpeg, MPV, VLC)",
                docAltPlayersDesc: "O stream RTP H.264 (RFC 4571 / PT 96) gerado pelo ext-sender é universal e funciona perfeitamente em KDE, XFCE, i3, Windows e macOS sem depender de GNOME:",
                docBulkFallbackTitle: "15. ⚡ Modo USB Bulk Direto Padrão & Auto-Fallback para Rede UDP",
                docBulkFallbackDesc: "O ext-sender e a imagem do Pi Zero vêm configurados por padrão para USB Bulk Direto (480 Mbps). Se o cabo estiver conectado a um dispositivo sem suporte USB gadget ou via rede (como Pi 4 ou PC secundário), o transmissor detecta a ausência da interface USB e comuta automaticamente e em tempo real para transmissão via Rede UDP (porta 5000) sem travar.",
                lblActiveMode: "Modo de Transmissão em Execução",
                lblActiveHdmi: "Tela HDMI do Raspberry Pi",
                protoHeader: "Tabela Comparativa de Métodos",
                thMethod: "Método",
                thProtocol: "Protocolo",
                thLatency: "Latência",
                thBestFor: "Caso de Uso Ideal",
                tbM1Method: "Linux Wayland Direto",
                tbM1Best: "Desktop interativo, arrasto suave de janelas com mouse",
                tbM2Method: "Windows 10/11 Miracast",
                tbM2Best: "Projeção nativa sem drivers no Windows (Win + K)",
                tbM3Method: "USB Bulk Direto",
                tbM3Best: "Comunicação direta por hardware sem IP, Marcador RTP (EOF)",
                bookTitle: "O Livro do Ext-Monitor — Compêndio de Engenharia & Arquitetura",
                bookBadge: "Compêndio de 18 Blueprints",
                bookDesc: "Documentação de engenharia reversa de hardware, decisões de silício BCM2835, protocolos de rede (RFC 4571 / RFC 6184 / WFD), áudio híbrido e o pipeline Wayland/DRM-KMS estão consolidados no compêndio mestre: <strong style=\"color: #fff; font-family: monospace;\">docs/LIVRO-EXT-MONITOR.md</strong>.",
                bookPart1Title: "Parte I: Silício BCM2835 & Boot",
                bookPart1Desc: "Boot 1.8s, FAT16 32MB, VideoCore IV V4L2 M2M, DMA Zero-Copy e DMA-BUF.",
                bookPart2Title: "Parte II: Protocolos & Barramentos",
                bookPart2Desc: "USB Bulk RFC 4571 Marker Bit/ZLP, RTP H.264 FU-A, WFD Miracast e UAC2.",
                bookPart3Title: "Parte III: Host Linux & Wayland",
                bookPart3Desc: "GNOME Mutter Screencast D-Bus, PipeWire, AMD Radeon 610M DCN 3.1 e DRM/KMS.",
                bookPart4Title: "Parte IV: Áudio Híbrido & Painel Web",
                bookPart4Desc: "Opus 48kHz UDP, BlueZ A2DP Sink no Pi Zero W e Painel Web Bidirecional em Rust.",
                docAudioMode1: "<strong style=\"color: #fff;\">Modo 1 (Rede IP):</strong> Sink virtual PipeWire no PC transmitindo RTP Opus/PCM na porta UDP 5004 direto ao codec HDMI do Pi Zero.",
                docAudioMode2: "<strong style=\"color: #fff;\">Modo 2 (Windows Miracast):</strong> Protocolo nativo Wi-Fi Display (WFD) roteando áudio estéreo AAC/LPCM via RTSP TCP 7236.",
                docAudioMode3: "<strong style=\"color: #fff;\">Modo 3 (USB Audio Gadget - UAC2):</strong> Placa de som USB Plug-and-Play exposta pelo Gadget Composto no PC com encaminhamento PCM direto à TV HDMI.",
                docAudioMode4: "<strong style=\"color: #fff;\">Modo 4 (Bluetooth A2DP Sink):</strong> Receptor de áudio Bluetooth para pareamento com smartphones, tablets ou notebooks secundários.",
                docAudioRoutingTitle: "16. 🔊 Roteamento de Áudio Híbrido & Dispositivo Virtual HDMI",
                docAudioRoutingDesc: "O ext-monitor implementa uma arquitetura de áudio 100% isolada e híbrida utilizando um sink virtual PipeWire/PulseAudio (<code>Raspberry_Pi_HDMI_Audio</code>). Você pode trabalhar na tela estendida da TV mantendo YouTube, reuniões e músicas tocando nos alto-falantes do notebook ou fone USB:",
                docClocksTitle: "17. ⚙️ Clocks do Subhardware (H.264/VPU/ARM) & Consumo de Energia",
                docClocksDesc: "O Raspberry Pi Zero W expõe a telemetria interna de cada bloco de silício Broadcom BCM2835 via <code>debugfs</code>. O ext-monitor monitora os clocks de hardware continuamente em tempo real:",
                docCliManualTitle: "18. 📖 Manual Completo de Operação, Flags CLI & Solução de Problemas",
                docCliManualDesc: "Sintaxe do transmissor: <code>ext-sender [extend|clone] [fps] [bitrate] [opções]</code>",
                badgeStreaming: "● TRANSMITINDO",
                badgeMode1On: "Ligado (UDP 5000)",
                badgeMode2On: "Ligado (TCP 7236)",
                badgeMode3On: "Ligado (USB Bulk)",
                hostHeader: "Controle Remoto do Transmissor (Host PC)",
                hostBadge: "UDP Bidirecional 5001",
                hostStatusLabel: "Status da Transmissão & Ações",
                btnHostStart: "▶ Iniciar / Reiniciar Stream",
                btnHostStop: "⏹ Parar Stream",
                hostTransportLabel: "Transporte Ativo / Modo de Transmissão",
                tipHostTransport: "Alterne o protocolo de transmissão em tempo real: USB Bulk Direto (latência < 1ms), Rede UDP (porta 5000) ou Windows Miracast (Win+K RTSP).",
                btnSelectMode1: "▶ Ativar Rede UDP",
                btnSelectMode2: "▶ Ativar Miracast (Win+K)",
                btnSelectMode3: "▶ Ativar USB Bulk",
                hostModeLabel: "Modo de Exibição / Transmissão Chrome Cast",
                tipHostMode: "Define a topologia de tela. 'Perguntar na Transmissão' aplica-se exclusivamente quando iniciado via Google Chrome Cast. Comandos pelo painel web e CLI aplicam imediatamente sem diálogo.",
                btnModeAsk: "🌐 Cast: Perguntar na Tela",
                btnModeExtend: "🖥️ Estendido (HDMI-1 TV)",
                btnModeClone: "💻 Clonado (eDP-1 Notebook)",
                toastModeAsk: "🌐 Google Cast: Perguntará se deseja Estender ou Clonar ao transmitir",
                hostAudioLabel: "Áudio Híbrido (Rede IP Opus + Bluetooth A2DP)",
                tipHostAudio: "Ative áudio de rede para transmitir som do PC via Opus 48kHz para o HDMI da TV. Use pareamento Bluetooth para conectar celulares/tablets direto à TV.",
                btnHostAudioOn: "🔊 Áudio Rede (Opus UDP)",
                btnHostAudioOff: "🔇 Desativar Áudio Rede",
                btnBtPair: "📡 Parear Bluetooth A2DP (60s)",
                hostHudLabel: "HUD de Telemetria na Tela da TV",
                tipHostHud: "Projeta FPS em tempo real, bitrate e latência no canto inferior direito da TV por 60 segundos.",
                btnHostShowHud: "📊 Exibir HUD na TV (60s)",
                btnHostHideHud: "❌ Ocultar HUD",
                mediaHeader: "Central de Mídia IoT & Visualizador HDMI (Chromecast / DLNA)",
                audioHeader: "Áudio Digital HDMI, DAC de Alta Fidelidade & Transmissão Integrada",
                audioBadge: "ALSA Hardware PCM",
                audioCardDesc: "Controle centralizado e integrado: selecione o canal de transmissão ativo (Modo 1, 2 ou 3), a topologia de tela (Estender ou Clonar), ajuste o volume de áudio digital, gerencie o visualizador de espectro FFT e configure o clock mestre do hardware ALSA HDMI.",
                audioModeLabel: "Canal de Transmissão Ativo (Vídeo + Áudio HDMI)",
                tipAudioMode: "Alterna o canal físico de streaming entre o computador e o Raspberry Pi. Troca a quente sem reiniciar o sistema.",
                audioTopologyLabel: "Topologia de Tela no Laptop (Wayland / Mutter)",
                tipAudioTopology: "Estender a área de trabalho para a TV como segundo monitor virtual (HDMI-1) ou espelhar a tela principal do notebook (eDP-1).",
                castHeader: "Transmissão Web & Compartilhamento Estilo Chromecast (Google Cast)",
                castDesc: "Espelhe suas abas do navegador, janelas, URLs de vídeo ou tela do celular diretamente na TV, exatamente como um Chromecast real.",
                webCastTitle: "Transmitir Esta Aba ou Tela (Web Cast)",
                webCastDesc: "Transmita qualquer aba do Chrome/Firefox, reuniões ou tela inteira com aceleração de hardware WebCodecs de baixa latência.",
                btnOpenWebCast: "🔴 Abrir Transmissor Web (/cast)",
                googleCastTitle: "Google Cast (Chromecast Nativo)",
                googleCastDesc: "No menu do Chrome/Edge (Transmitir...) ou em celulares (YouTube, Netflix, Pluto TV), selecione 'Ext-Monitor (Raspberry Pi)'.",
                googleCastReady: "✓ Descoberta mDNS & Cast V2 Ativos (Portas 8008 / 8009)",
                castUrlLabel: "Transmitir URL de Vídeo Direto na TV (Play URL)",
                tipCastUrl: "Insira uma URL direta de vídeo (MP4, WebM, HLS m3u8) para decodificar e reproduzir diretamente no HDMI do Raspberry Pi.",
                btnCastUrl: "▶ Transmitir na TV",
                toastEnterUrl: "Por favor, insira uma URL de vídeo válida.",
                toastCasting: "Enviando vídeo para o decodificador HDMI...",
                toastCastSuccess: "✓ Reprodução iniciada na TV!",
                badgeHdmiMuxStandby: "Modo: Standby (Splash Pronta)",
                badgeHdmiMuxPc: "Modo: Vídeo do PC (Tela da TV + Áudio)",
                badgeHdmiMuxIot: "Modo: Áudio IoT (Visualizador HDMI 30 FPS)",
                mediaBadge: "Google Home • UPnP • Bluetooth",
                mediaPlaybackLabel: "Reprodução Atual (Áudio IoT na TV)",
                tipMediaPlayback: "Exibe metadados da música em reprodução via Bluetooth pelo celular ou stream de rede UPnP/Cast.",
                mediaVisLabel: "Visualizador Gráfico na TV (Tela Nunca Apaga)",
                tipMediaVis: "Quando toca áudio sem transmissão de vídeo do PC, desenha o espectro de 24 bandas e VU meter a 30 FPS no HDMI, impedindo a TV de apagar ou suspender.",
                btnVisOn: "🎨 Ativar Visualizador HDMI",
                btnVisOff: "⏹ Desativar Visualizador",
                btnTestAudioChime: "🔊 Testar Sinal Real de Áudio",
                netUsb0Label: "USB OTG (usb0)",
                netUsb0Desc: "● Host OTG Permanente",
                netEth0Label: "Ethernet Física (eth0)",
                netEth0Desc: "● Estático / DHCP",
                netWlan0Label: "Wi-Fi (wlan0)",
                netWlan0Desc: "● Opcional",
                netIfaceLabel: "Interface:",
                btnNetStatic: "📌 IP Estático",
                btnNetDhcp: "🔄 DHCP Automático",
                netIpLabel: "Endereço IP:",
                netMaskLabel: "Máscara de Sub-rede:",
                netGwLabel: "Gateway Padrão:",
                netDnsLabel: "DNS Server:",
                btnApplyNet: "💾 Salvar & Aplicar na Placa",
                btnAudioMute: "🔊 Silenciar Áudio",
                captureLabel: "Motor de Captura (Dual-Engine)",
                tipCapture: "KMS Direct: Lê diretamente do scanout de hardware da GPU via Linux Kernel DRM/KMS, eliminando congelamento mesmo com mouse parado. Mutter: Captura via screencast D-Bus do GNOME Mutter.",
                btnKmsDirect: "⚡ KMS Direto (Anti-Congelamento / GPU Scanout)",
                btnGnomeMutter: "🐧 GNOME Mutter (PipeWire Screencast)",
                monitorTargetLabel: "Monitor de Gravação / Captura",
                tipMonitorTarget: "Escolha a saída de vídeo para capturar. HDMI-1 para segunda tela estendida na TV/monitor, eDP-1 para clonar a tela do notebook.",
                scaleLabel: "Resolução e Escala de Exibição",
                tipScale: "Selecione a resolução de exibição. Modos 1:1 Nativos entregam fontes vetoriais cristalinas sem desfoque. Modos de Super-Resolução utilizam o HVS em silício do VideoCore IV ou FSR para preencher telas panorâmicas.",
                lblSiliconScaler: "Scaler de Silício em Hardware (VideoCore IV HVS & FSR)",
                badgeSiliconActive: "HVS Ativo",
                badgeSiliconOff: "Apenas 1:1",
                optgroupNative: "Modos Nativos 1:1 Diretos (Nítidos / Sem Blur)",
                optgroupUpscale: "Super-Resolução / Upscaling (Silício HVS / FSR)",
                btnScale720p: "🎯 720p Nativo (1:1)",
                btnScale1600x900: "📐 900p Upscale",
                btnScaleOff: "⚡ Passthrough",
                casLabel: "Nitidez Adaptativa por Contraste (CAS)",
                tipCas: "Restaura o alcance total de cores do PC (0-255) e contraste profundo, eliminando o aspecto 'lavado' do encoder.",
                btnCasTrue: "✨ CAS Ativado (Texto Nítido / Preto Puro)",
                btnCasFalse: "Padrão (Alcance TV Limitado)",
                statStreamActive: "Ativo",
                statStreamPaused: "Pausado",
                waitingStream: "Aguardando Stream (Splash Ativa)",
                mediaPlaying: "Tocando Áudio",
                mediaPaused: "Pausado",
                mediaIdle: "Ocioso / Pronto",
                visActive: "Ativo (30 FPS)",
                visDisabled: "Desativado",
                toastTestAudio: "🔊 Disparando sinal de áudio real no HDMI...",
                toastMuted: "Áudio HDMI Silenciado",
                toastUnmuted: "Áudio HDMI Ativado",
                mute: "Silenciar Áudio",
                unmute: "Ativar Áudio",
                netConnected: "Conectado",
                netDisconnected: "Desconectado",
                cableDisconnected: "Cabo Desconectado",
                modalRebootTitle: "Reiniciar Appliance?",
                modalRebootDesc: "Tem certeza que deseja reiniciar o Raspberry Pi Zero? O sistema reiniciará em menos de 2 segundos diretamente na RAM.",
                btnCancel: "Cancelar",
                btnConfirmReboot: "Sim, Reiniciar",
                audioLabel: "Áudio Digital HDMI (Opus 48kHz)",
                tipAudio: "Volume do áudio digital enviado ao monitor/TV via cabo HDMI. Latência sub-25ms com sincronismo A/V.",
                audioProfileLabel: "Perfil de Áudio HDMI e Taxa de Amostragem",
                tipAudioProfile: "Selecione e force o carregamento do clock de hardware diretamente no núcleo de áudio ALSA BCM2835 do Pi Zero. Suporta áudio subframe IEC958 real de até 192kHz 24-bit Hi-Res.",
                audioTransportLabel: "Arquitetura de Transporte de Áudio",
                tipAudioTransport: "Selecione a via física de transporte do áudio digital: Modo 1 Rede UDP (Porta 5004), Modo 2 USB Audio Class (Gadget UAC2) ou Modo 3 Multiplexação USB Bulk.",
                audioTransportDesc: "Modo 1 Rede UDP: Fluxo de áudio via porta UDP 5004 em pacotes de 1024 bytes sem fragmentação. Latência sub-5ms com sincronismo A/V.",
                audioTransUdpDesc: "Modo 1 Rede UDP: Fluxo de áudio via porta UDP 5004 em pacotes de 1024 bytes sem fragmentação. Latência sub-5ms com sincronismo A/V.",
                audioTransUac2Desc: "Modo 2 Gadget UAC2: O Pi Zero opera como uma Placa de Som USB física no PC. Zero dependência de rede, Plug-and-Play no Windows e Linux.",
                audioTransBulkDesc: "Modo 3 USB Bulk: O áudio é multiplexado no mesmo tubo /dev/usb-display-bulk junto com o vídeo H.264. Latência sub-1ms.",
                docAltPlayersCmd: "# FFmpeg / ffplay (Baixa Latência):\nffplay -probesize 32 -analyzeduration 0 -sync ext -fflags nobuffer -flags low_delay -i 'rtp://192.168.7.2:5000'",
                copied: "Copiado!",
                copiedSuccess: "✓ Copiado para a área de transferência!",
                m1Title: "Modo 1: Rede UDP (Linux Wayland / X11)",
                m1Details: "Porta UDP 5000 • Latência < 15ms • Pipeline VA-API/M2M",
                m2Title: "Modo 2: Windows Miracast (Wi-Fi Display)",
                m2Details: "Porta RTSP 7236 • Windows Win+K • Decodificação V4L2 M2M",
                m3Title: "Modo 3: USB Bulk Direto (480 Mbps)",
                m3Details: "Barramento USB 2.0 High-Speed • Zero-Rede • Latência < 1ms",
                waitingStreamDesc: "Receptor em prontidão exibindo tela de splash com IP e QR Code.",
                capKmsTitle: "KMS Direct (Anti-Congelamento / GPU Scanout)",
                capMutterTitle: "GNOME Mutter (PipeWire Screencast)",
                connHeader: "Conexão Ativa da Extensão (Transporte Ativo)",
                connBadge: "Chaveamento a Quente",
                connDesc: "Selecione a via ativa de transmissão para a tela estendida. Chaveia na hora sem necessidade de reiniciar o sistema.",
                extHeader: "Ação da Extensão de Tela & Topologia",
                extBadge: "Tela Ativa",
                extBadgeActive: "Estendendo (HDMI-1)",
                extBadgeClone: "Clonando (eDP-1)",
                extBadgeOff: "Standby (Pausado)",
                extDesc: "Estenda a área de trabalho para o monitor HDMI-1, clone a tela primária do notebook (eDP-1), ou desative a extensão para repouso.",
                btnActionExtend: "🖥️ Estender Tela (HDMI-1)",
                btnActionExtendDesc: "Segundo monitor virtual na TV",
                btnActionClone: "💻 Espelhar / Clonar (eDP-1)",
                btnActionCloneDesc: "Espelha tela primária do notebook",
                btnActionStop: "⏹ Desativar Extensão / Standby",
                btnActionStopDesc: "Interrompe transmissão e repousa a TV",
                toastExtStopped: "⏹ Extensão de tela desativada. TV em repouso.",
                toastExtCloned: "💻 Espelhando tela primária do notebook (eDP-1)...",
                toastExtExtended: "🖥️ Estendendo área de trabalho para TV (HDMI-1)...",
                servicesHeader: "Daemons & Serviços de Escuta do Appliance",
                servicesBadge: "Listeners no Hardware",
                lblScreenMode: "Modo de Exibição:",
                btnM1Extend: "🖥️ Estender (UDP)",
                btnM1Clone: "💻 Clonar (UDP)",
                btnM3Extend: "🖥️ Estender (USB)",
                btnM3Clone: "💻 Clonar (USB)"
            },
            it: {
                title: "Pi Zero Monitor Esteso",
                subtitle: "Appliance Display Hardware GPU VideoCore IV",
                tabMonitor: "Monitoraggio & Telemetria",
                tabConfig: "Impostazioni & Streaming",
                tabDownloads: "Strumenti Client & Driver",
                tabSdCard: "Scheda SD & Aggiornamento RAM",
                tabManual: "Manuale Operativo",
                statTemp: "Temperatura SoC",
                statCpu: "Carico CPU",
                statRam: "RAM Libera",
                statStream: "Stato Streaming",
                badgeVpuOffload: "VPU Offload",
                statCpuDesc: "Hardware GPU V4L2 M2M Attivo",
                badgeRamApp: "100% RAM",
                tipTemp: "Temperatura interna del silicio Broadcom BCM2835. Monitorata ogni 2s. Ideale: sotto i 65°C.",
                tipCpu: "Carico della CPU ARM11. Rimane sotto il 2% poiché la decodifica H.264 è gestita dalla VPU VideoCore IV.",
                tipRam: "RAM disponibile dei 512 MB SDRAM. Il sistema funziona al 100% in RAM senza usura della scheda SD.",
                tipStream: "Stato in tempo reale della decodifica video verso l'uscita HDMI del televisore.",
                displayHeader: "Telemetria Display HDMI & TV",
                displayDesc: "La VPU hardware VideoCore IV decodifica il flusso H.264 direttamente nel piano HDMI senza usare la CPU.",
                btnShowHud: "✦ Mostra HUD su TV (60s)",
                btnHideHud: "✕ Nascondi HUD",
                modeHeader: "Modalità di Streaming Attive",
                badgeMultiMode: "Motore Concorrente",
                m1Desc: "Flusso RTP H.264 a bassissima latenza su porta UDP 5000 con GPU AMD VA-API (< 15ms).",
                m2Desc: "Proiezione nativa Windows 10/11 via Win + K su porta RTSP 7236. Zero driver sul PC.",
                m3Desc: "Canale hardware diretto a 480 Mbps via USB FunctionFS senza overhead di rete (< 1ms).",
                netHeader: "Configurazione Rete e Interfacce (Ethernet / LAN / Wi-Fi)",
                netModeLabel: "Modalità di Indirizzamento dell'Interfaccia Secondaria",
                tipNetMode: "Scegli IP Statico per connessioni punto-a-punto senza router DHCP (es. su un altro Raspberry o PC dopo il boot), oppure DHCP per IP automatico.",
                ctrlHeader: "Ottimizzazione Display e Streaming",
                badgeZeroCopy: "VideoCore IV DMA",
                bitrateLabel: "Bitrate di Streaming (VBR)",
                tipBitrate: "Bitrate di codifica VBR. 400k risparmia l'85% per testo/codice. 3000k-6000k per video 60 FPS fluido.",
                fpsLabel: "Frequenza Fotogrammi (FPS)",
                tipFps: "Frequenza fotogrammi. 15 FPS per basso calore, 30 FPS bilanciato consigliato, 60 FPS massima fluidità.",
                colorLabel: "Profilo Colore",
                tipColor: "24-bit TrueColor per fedeltà RGB 1:1. 256 Colori applica QP adattivo per ridurre la banda USB.",
                colorFull: "24-bit TrueColor",
                color256: "256 Colori (QP 30-44)",
                colorGray: "Monocromatico",
                dropOnlyLabel: "Salto Fotogrammi (Damage-Only Preserving)",
                tipDropOnly: "Quando Attivo, trasmette solo su variazioni dello schermo. Disattivato forza 30 FPS continui evitando blocchi su YouTube.",
                dropOnlyTrue: "Attivo (Risparmio 95% su schermo statico)",
                dropOnlyFalse: "Disattivato (Duplicazione continua)",
                dropOnlyDesc: "Quando attivo, il pipeline non duplica fotogrammi statici, risparmiando banda per il movimento del cursore.",
                skipFirstLabel: "Consegna Immediata Primo Fotogramma (skip-to-first)",
                tipSkipFirst: "Elimina i ritardi accumulati consegnando istantaneamente il primo fotogramma di movimento.",
                skipFirstTrue: "Attivo (Zero latenza al movimento)",
                skipFirstFalse: "Disattivato (Allineamento rigido)",
                keyIntLabel: "Scansione Periodica / Intervallo IDR (Refresh Clean)",
                tipKeyInt: "Frequenza dei fotogrammi IDR completi per eliminare artefatti visivi e perdite di pacchetti.",
                keyIntDesc: "Invia periodicamente un frame IDR completo per eliminare artefatti visivi sullo schermo HDMI/TV.",
                btnApply: "💾 Applica Modifiche",
                btnPauseStream: "⏸ Sospendi Display",
                btnResumeStream: "▶ Riprendi Display",
                btnReboot: "🔄 Riavvia Appliance (Reboot)",
                oneLinerTitle: "Connettore Host in 1 Riga (1 Clic)",
                badgeInstant: "Istantaneo",
                oneLinerDesc: "Su qualsiasi PC Linux, incolla questo comando nel terminale per avviare il monitor esteso:",
                btnCopy: "Copia",
                dlPkgTitle: "Pacchetto Completo Client",
                dlPkgDesc: "Contiene il binario ext-sender, le regole udev e la documentazione in tar.gz.",
                btnDlPkg: "Scarica client.tar.gz",
                dlSenderTitle: "Binario ext-sender",
                dlSenderDesc: "Binario autonomo del trasmettitore GPU compilato in Rust per Linux x86_64.",
                btnDlSender: "Scarica ext-sender",
                dlScriptTitle: "Script connect.sh",
                dlScriptDesc: "Script portatile che rileva USB, scarica i componenti e avvia lo streaming.",
                btnDlScript: "Scarica connect.sh",
                dlUdevTitle: "Regole udev (Plug-and-Play)",
                dlUdevDesc: "Configura automaticamente il buffer USB per eliminare i ritardi.",
                btnDlUdev: "Scarica 99-ext-monitor.rules",
                sdHeader: "Architettura 100% RAM & Aggiornamento Firmware",
                badgeZeroSdWear: "Zero Usura SD",
                sdDesc1: "Il Pi Zero si avvia interamente in RAM (initramfs). La scheda SD (/dev/mmcblk0) è disaccoppiata e non subisce scritture.",
                sdDesc2: "Questo garantisce zero rischi di corruzione e consente l'aggiornamento senza rimuovere la scheda!",
                sdPartitionStatus: "Stato della Memoria Fisica:",
                btnMountSd: "Monta Partizione (/mnt/boot)",
                btnUnmountSd: "Smonta Partizione",
                sdUpgradeManualTitle: "Come Aggiornare il Firmware senza Rimuovere la Scheda:",
                manualHeader: "Manuale Operativo e Specifiche Tecniche",
                badgeFullDocs: "Documentazione Completa",
                docArchTitle: "1. Architettura Hardware e GPU VideoCore IV",
                docArchDesc: "Raspberry Pi Zero utilizza il SoC Broadcom BCM2835 (ARM1176JZF-S a 1.0 GHz) con GPU VideoCore IV a 500 MHz. ext-monitor opera interamente via hardware: il flusso H.264 viene decodificato via V4L2 M2M (/dev/video10) e scritto via DMA direttamente nel framebuffer HDMI (/dev/fb0). Latenza < 15 ms e CPU < 2%.",
                docUsbProtoTitle: "2. Protocollo USB Bulk Diretto, RFC 4571 & Marker Bit (EOF / ZLP)",
                docUsbProtoDesc1: "Sul bus USB 2.0 High-Speed (480 Mbps), i pacchetti viaggiano in micro-frame fino a 512 byte (wMaxPacketSize). Nei flussi H.264 Annex-B grezzi, i decoder dovevano attendere lo start code successivo, bloccando l'immagine allo stop del mouse.",
                docUsbProtoDesc2: "ext-monitor implementa lo standard RFC 4571 con prefisso a 2 byte e bit marcatore RTP (EOF). Ogni fotogramma viene visualizzato istantaneamente su HDMI senza dipendere dal movimento del mouse, garantendo riproduzione YouTube stabile. I trasferimenti multipli di 512 byte inviano un pacchetto ZLP liberando la FIFO del DWC2.",
                docAudioTitle: "3. Routing Audio Multi-Modale (Normale vs Ibrido)",
                docAudioDesc1: "ext-monitor supporta l'audio digitale stereo verso la TV HDMI tramite 4 modalità di trasporto:",
                docAudioDesc2: "Modalità Normale vs Ibrida: In modalità Normale, audio e video del PC vengono riprodotti insieme sulla TV. In modalità Ibrida, puoi mantenere l'audio sugli altoparlanti del PC mentre il video va sulla TV, o riprodurre musica via Bluetooth dal telefono sulla TV.",
                docSerialTitle: "4. Riconfigurazione Senza IP via USB Seriale (/dev/ttyACM0)",
                docSerialDesc: "Se la rete è disabilitata, il Pi Zero offre una console seriale di ripristino su /dev/ttyACM0 a 115200 baud.",
                docLinuxTitle: "5. Funzionamento Standard su Linux Wayland (GNOME)",
                docLinuxDesc: "Collega il cavo alla porta USB centrale. Il PC ottiene l'IP 192.168.7.1 dal DHCP. Esegui ext-sender (senza parametri):",
                docWinTitle: "6. Proiezione Windows 10/11 (Miracast Senza Driver)",
                docWinDesc: "Collega via USB, premi Win + K su Windows e seleziona 'Pi Zero Wireless Display'.",
                docSdTitle: "7. Architettura 100% RAM & Aggiornamento senza rimuovere la scheda SD",
                docSdDesc: "L'appliance funziona al 100% in RAM disk initramfs. Per aggiornare, basta montare la partizione FAT di boot con 'mount -t vfat /dev/mmcblk0p1 /mnt' e copiare i nuovi binari senza spegnere il dispositivo.",
                docBrowserVideoTitle: "8. 💡 Suggerimento: Riproduzione Continua di Video nel Browser (Chrome / Firefox)",
                docBrowserVideoDesc: "Su Linux Wayland, browser come Chrome e Firefox attivano il risparmio energetico ('Window Occlusion Tracking') e sospendono i video quando il mouse esce dalla finestra o perde il focus. Per mantenere 60 FPS continui:",
                docBrowserChrome: "Apri chrome://flags/#calculate-native-win-occlusion, imposta su 'Disabled' e riavvia (o usa --disable-backgrounding-occluded-windows).",
                docBrowserFirefox: "Apri about:config, cerca media.suspend-bkgnd-video.enabled e imposta su 'false'.",
                docBrowserNative: "Riproducono a 60 FPS continui per impostazione predefinita, senza interruzioni per posizione del mouse.",
                docArmv6Title: "9. Compilazione per Raspberry Pi Zero (ARMv6)",
                docArmv6Desc: "Il SoC BCM2835 su Pi Zero v1.2/v1.3/W richiede l'architettura ARMv6l. Per compilare con cross (Docker):",
                docArmv6Cmd: "cargo install cross && cd receiver && cross build --target arm-unknown-linux-musleabihf --release",
                docNonOtgTitle: "10. Modelli Raspberry Pi Non-OTG (Pi 2, Pi 3, Pi 4, Pi 5) & Parametri di Boot",
                docNonOtgDesc1: "Questi modelli non supportano la modalità periferica USB sulle porte USB standard. La connessione avviene via Ethernet o Wi-Fi.",
                docNonOtgDesc2: "Modifiche obbligatorie sulla scheda SD: In config.txt commentare 'dtoverlay=dwc2'. In cmdline.txt rimuovere 'modules-load=dwc2'. Collegarsi con: ext-sender --ip=<IP_PI>",
                docPcReceiverTitle: "11. Trasformare qualsiasi PC / Laptop Linux in Schermo Secondario",
                docPcReceiverDesc: "Qualquer computer Linux pode funcionar como receptor. Installa gstreamer1.0-tools ed esegui:",
                docPcReceiverCmd: "gst-launch-1.0 -v udpsrc port=5000 buffer-size=524288 caps=\"application/x-rtp,media=video,clock-rate=90000,encoding-name=H264,payload=96\" ! rtph264depay ! h264parse ! avdec_h264 ! autovideosink sync=false",
                docMultiMonTitle: "12. Indirizzamento su Schermo 1 o 2 su PC con Più Monitor",
                docMultiMonDesc: "Su PC ricevitori con più monitor, specifica 'kmssink connector-id=<ID>' in KMS DRM diretto o 'ffplay -left 1920 -top 0 -fs rtp://0.0.0.0:5000' per proiettare sul monitor desiderato.",
                docPacerTitle: "13. 🚀 Streaming Continuo a 60 FPS Anti-Freeze",
                docPacerDesc: "ext-monitor fornisce streaming continuo a 60 FPS CFR anti-freeze in modo nativo in ext-sender. I video di YouTube e i terminali continuano a 60 FPS anche con mouse fermo.",
                docAltPlayersTitle: "14. Alternative al Ricevitore Non-GNOME (FFmpeg, MPV, VLC)",
                docAltPlayersDesc: "Il flusso video RFC 4571 RTP H.264 è universale. Ricevilo su KDE, XFCE, i3, Windows o macOS senza buffer:",
                docBulkFallbackTitle: "15. ⚡ Modalità USB Bulk Predefinita & Auto-Fallback su Rete UDP",
                docBulkFallbackDesc: "ext-sender tenta prioritariamente la modalità USB Bulk ad alta velocità (480 Mbps). Se l'interfaccia USB non è rilevata, commuta automaticamente su streaming UDP (porta 5000) senza interruzioni.",
                lblActiveMode: "Modalità di Streaming Attiva",
                lblActiveHdmi: "Uscita HDMI Raspberry Pi Attiva",
                protoHeader: "Tabella Comparativa Protocolli",
                thMethod: "Metodo",
                thProtocol: "Protocollo",
                thLatency: "Latenza",
                thBestFor: "Uso Ideale",
                tbM1Method: "Linux Wayland Diretto",
                tbM1Best: "Desktop interattivo, trascinamento finestre fluido con mouse",
                tbM2Method: "Windows 10/11 Miracast",
                tbM2Best: "Proiezione nativa senza driver su Windows (Win + K)",
                tbM3Method: "USB Bulk Diretto",
                tbM3Best: "Comunicazione hardware diretta senza IP, Bit Marcador RTP (EOF)",
                bookTitle: "Il Libro di Ext-Monitor — Compendio di Ingegneria e Architettura",
                bookBadge: "Compendio di 18 Blueprint",
                bookDesc: "La documentazione di reverse engineering hardware, le decisioni sul silicio BCM2835, i protocolli di rete (RFC 4571 / RFC 6184 / WFD), l'audio ibrido e la pipeline Wayland/DRM-KMS sono consolidati nel compendio principale: <strong style=\"color: #fff; font-family: monospace;\">docs/LIVRO-EXT-MONITOR.md</strong>.",
                bookPart1Title: "Parte I: Silicio BCM2835 & Boot",
                bookPart1Desc: "Boot 1.8s, FAT16 32MB, VideoCore IV V4L2 M2M, DMA Zero-Copy e DMA-BUF.",
                bookPart2Title: "Parte II: Protocolli e Bus",
                bookPart2Desc: "USB Bulk RFC 4571 Marker Bit/ZLP, RTP H.264 FU-A, WFD Miracast e UAC2.",
                bookPart3Title: "Parte III: Host Linux & Wayland",
                bookPart3Desc: "GNOME Mutter Screencast D-Bus, PipeWire, AMD Radeon 610M DCN 3.1 e DRM/KMS.",
                bookPart4Title: "Parte IV: Audio Ibrido & Pannello Web",
                bookPart4Desc: "Opus 48kHz UDP, BlueZ A2DP Sink su Pi Zero W e Pannello Web Bidirezionale in Rust.",
                docAudioMode1: "<strong style=\"color: #fff;\">Modalità 1 (Rete IP):</strong> Sink virtuale PipeWire su PC che trasmette RTP Opus/PCM su porta UDP 5004 direttamente al codec HDMI del Pi Zero.",
                docAudioMode2: "<strong style=\"color: #fff;\">Modalità 2 (Windows Miracast):</strong> Protocollo nativo Wi-Fi Display (WFD) che indirizza audio stereo AAC/LPCM via RTSP TCP 7236.",
                docAudioMode3: "<strong style=\"color: #fff;\">Modalità 3 (USB Audio Gadget - UAC2):</strong> Scheda audio USB Plug-and-Play esposta dal Composite Gadget sul PC con inoltro PCM diretto all'HDMI della TV.",
                docAudioMode4: "<strong style=\"color: #fff;\">Modalità 4 (Bluetooth A2DP Sink):</strong> Ricevitore audio Bluetooth per l'accoppiamento con smartphone, tablet o notebook secondari.",
                docAudioRoutingTitle: "16. 🔊 Routing Audio Ibrido & Dispositivo Virtuale HDMI",
                docAudioRoutingDesc: "ext-monitor implementa un'architettura audio ibrida e isolata al 100% utilizzando un sink virtuale PipeWire/PulseAudio (<code>Raspberry_Pi_HDMI_Audio</code>). Puoi lavorare sullo schermo TV esteso mantenendo YouTube, call e musica sugli altoparlanti del laptop o cuffie USB:",
                docClocksTitle: "17. ⚙️ Frequenze Subhardware (H.264/VPU/ARM) & Consumo Energetico",
                docClocksDesc: "Il Raspberry Pi Zero W espone la telemetria interna di ogni blocco Broadcom BCM2835 tramite <code>debugfs</code>. ext-monitor monitora costantemente le frequenze hardware in tempo reale:",
                docCliManualTitle: "18. 📖 Manuale Operativo Completo, Opzioni CLI & Risoluzione Problemi",
                docCliManualDesc: "Sintassi di avvio: <code>ext-sender [extend|clone] [fps] [bitrate] [opzioni]</code>",
                badgeStreaming: "● STREAMING ATTIVO",
                badgeMode1On: "Attivo (UDP 5000)",
                badgeMode2On: "Attivo (TCP 7236)",
                badgeMode3On: "Attivo (USB Bulk)",
                hostHeader: "Controllo Remoto del Trasmettitore (Host PC)",
                hostBadge: "UDP Bidirezionale 5001",
                hostStatusLabel: "Stato della Trasmissione & Azioni",
                btnHostStart: "▶ Avvia / Riavvia Stream",
                btnHostStop: "⏹ Ferma Stream",
                hostModeLabel: "Modalità di Visualizzazione del Monitor",
                tipHostMode: "HDMI-1: Secondo schermo virtuale esteso su TV. eDP-1: Clona lo schermo principale del notebook. Chiedi: Mostra finestra di selezione sul laptop.",
                btnModeAsk: "❓ Chiedi Sempre (Finestra)",
                btnModeExtend: "🖥️ Esteso (HDMI-1 TV)",
                btnModeClone: "💻 Clonato (eDP-1 Notebook)",
                toastModeAsk: "❓ Modalità configurata: Chiedi sempre sul laptop all'avvio",
                hostAudioLabel: "Audio Ibrido (Rete IP Opus + Bluetooth A2DP)",
                tipHostAudio: "Abilita l'audio di rete per trasmettere l'audio del PC via Opus 48kHz alla TV HDMI. Usa l'accoppiamento Bluetooth per connettere telefoni/tablet alla TV.",
                btnHostAudioOn: "🔊 Audio di Rete (Opus UDP)",
                btnHostAudioOff: "🔇 Disattiva Audio di Rete",
                btnBtPair: "📡 Accoppia Bluetooth A2DP (60s)",
                hostHudLabel: "HUD di Telemetria sullo Schermo TV",
                tipHostHud: "Proietta FPS in tempo reale, bitrate e latenza nell'angolo in basso a destra della TV per 60 secondi.",
                btnHostShowHud: "📊 Mostra HUD su TV (60s)",
                btnHostHideHud: "❌ Nascondi HUD",
                mediaHeader: "Centro Multimediale IoT & Visualizzatore HDMI (Chromecast / DLNA)",
                audioHeader: "Audio Digitale HDMI, DAC Hardware & Streaming Integrato",
                audioBadge: "ALSA Hardware PCM",
                audioCardDesc: "Controllo centralizzato: seleziona il canale di trasmissione attivo (Modo 1, 2 o 3), la topologia dello schermo (Estendi o Duplica), regola il volume e configura il clock ALSA HDMI.",
                audioModeLabel: "Canale di Trasmissione Attivo (Video + Audio HDMI)",
                tipAudioMode: "Cambia la pipeline di streaming fisica tra PC e Raspberry Pi istantaneamente.",
                audioTopologyLabel: "Topologia Schermo su Laptop (Wayland / Mutter)",
                tipAudioTopology: "Estendi il desktop sulla TV come secondo monitor virtuale (HDMI-1) o duplica lo schermo primario (eDP-1).",
                castHeader: "Condivisione Web & Trasmissione Stile Chromecast (Google Cast)",
                castDesc: "Trasmetti schede del browser, finestre, URL video o smartphone direttamente alla TV come un vero dispositivo Chromecast.",
                webCastTitle: "Trasmetti Scheda o Schermo (Web Cast)",
                webCastDesc: "Trasmetti qualsiasi scheda Chrome/Firefox, riunione o intero schermo con codifica WebCodecs a bassa latenza.",
                btnOpenWebCast: "🔴 Apri Trasmettitore Web (/cast)",
                googleCastTitle: "Google Cast (Chromecast Nativo)",
                googleCastDesc: "Nel menu di Chrome/Edge (Trasmetti...) o nelle app mobili (YouTube, Netflix, Pluto TV), seleziona 'Ext-Monitor (Raspberry Pi)'.",
                googleCastReady: "✓ Rilevamento mDNS & Cast V2 Attivi (Porte 8008 / 8009)",
                castUrlLabel: "Trasmetti URL Video Diretto sulla TV (Play URL)",
                tipCastUrl: "Inserisci l'URL di un video (MP4, WebM, HLS m3u8) per riprodurlo direttamente sull'uscita HDMI del Raspberry Pi.",
                btnCastUrl: "▶ Trasmetti sulla TV",
                toastEnterUrl: "Inserisci un URL video valido.",
                toastCasting: "Invio video alla TV in corso...",
                toastCastSuccess: "✓ Riproduzione video avviata sulla TV!",
                badgeHdmiMuxStandby: "Modalità: Standby (Splash Pronta)",
                badgeHdmiMuxPc: "Modalità: Video PC (Schermo TV + Audio)",
                badgeHdmiMuxIot: "Modalità: Audio IoT (Visualizzatore HDMI 30 FPS)",
                mediaBadge: "Google Home • UPnP • Bluetooth",
                mediaPlaybackLabel: "Riproduzione Corrente (Audio IoT su TV)",
                tipMediaPlayback: "Mostra i metadati della musica in riproduzione via Bluetooth dal telefono o stream di rete UPnP/Cast.",
                mediaVisLabel: "Visualizzatore Grafico su TV (Schermo Mai Nero)",
                tipMediaVis: "Quando l'audio è attivo senza video PC, genera lo spettro a 24 bande e VU meter a 30 FPS su HDMI, impedendo alla TV di spegnersi.",
                btnVisOn: "🎨 Abilita Visualizzatore HDMI",
                btnVisOff: "⏹ Disattiva Visualizzatore",
                btnTestAudioChime: "🔊 Testa Segnale Audio Hardware",
                netUsb0Label: "USB OTG (usb0)",
                netUsb0Desc: "● Host OTG Permanente",
                netEth0Label: "Ethernet Fisica (eth0)",
                netEth0Desc: "● Statico / DHCP",
                netWlan0Label: "Wi-Fi (wlan0)",
                netWlan0Desc: "● Opzionale",
                netIfaceLabel: "Interfaccia:",
                btnNetStatic: "📌 IP Statico",
                btnNetDhcp: "🔄 DHCP Automatico",
                netIpLabel: "Indirizzo IP:",
                netMaskLabel: "Maschera di Sottorete:",
                netGwLabel: "Gateway Predefinito:",
                netDnsLabel: "Server DNS:",
                btnApplyNet: "💾 Salva e Applica su Scheda",
                btnAudioMute: "🔊 Disattiva Audio",
                captureLabel: "Motore di Cattura (Dual-Engine)",
                tipCapture: "KMS Direct: Legge direttamente dallo scanout hardware della GPU via DRM/KMS del kernel Linux. Mutter: Cattura tramite screencast D-Bus di GNOME Mutter.",
                btnKmsDirect: "⚡ KMS Diretto (Anti-Blocco / GPU Scanout)",
                btnGnomeMutter: "🐧 GNOME Mutter (PipeWire Screencast)",
                monitorTargetLabel: "Monitor di Registrazione / Cattura",
                tipMonitorTarget: "Scegli l'uscita video da catturare. HDMI-1 per secondo schermo esteso su TV/monitor, eDP-1 per clonare lo schermo del notebook.",
                scaleLabel: "Risoluzione e Ridimensionamento Display",
                tipScale: "Seleziona la risoluzione di visualizzazione. I modi 1:1 Nativi offrono caratteri vettoriali nitidi senza sfocature. I modi Super-Risoluzione usano l'HVS hardware del VideoCore IV o FSR per schermi widescreen.",
                lblSiliconScaler: "Scaler Hardware in Silicio (VideoCore IV HVS & FSR)",
                badgeSiliconActive: "HVS Attivo",
                badgeSiliconOff: "Solo 1:1",
                optgroupNative: "Modi Nativi 1:1 Diretti (Nitidi / Senza Sfocatura)",
                optgroupUpscale: "Super-Risoluzione / Upscaling (Silicio HVS / FSR)",
                btnScale720p: "🎯 720p Nativo (1:1)",
                btnScale1600x900: "📐 900p Upscale",
                btnScaleOff: "⚡ Passthrough",
                casLabel: "Nitidezza Adattiva al Contrasto (CAS)",
                tipCas: "Ripristina la gamma dinamica completa del PC (0-255) e contrasto profondo.",
                btnCasTrue: "✨ CAS Attivo (Testo Nitido / Nero Profondo)",
                btnCasFalse: "Standard (Gamma TV Limitata)",
                statStreamActive: "Attivo",
                statStreamPaused: "In Pausa",
                waitingStream: "In Attesa di Stream (Splash Attiva)",
                mediaPlaying: "Riproduzione Audio",
                mediaPaused: "In Pausa",
                mediaIdle: "Inattivo / Pronto",
                visActive: "Attivo (30 FPS)",
                visDisabled: "Disattivato",
                toastTestAudio: "🔊 Invio segnale audio hardware su HDMI...",
                toastMuted: "Audio HDMI Disattivato",
                toastUnmuted: "Audio HDMI Attivo",
                mute: "Disattiva Audio",
                unmute: "Attiva Audio",
                netConnected: "Connesso",
                netDisconnected: "Disconnesso",
                cableDisconnected: "Cavo Scollegato",
                modalRebootTitle: "Riavviare Appliance?",
                modalRebootDesc: "Sei sicuro di voler riavviare il Raspberry Pi Zero? Si riavvierà in meno di 2 secondi direttamente in RAM.",
                btnCancel: "Annulla",
                btnConfirmReboot: "Sì, Riavvia",
                audioLabel: "Audio Digitale HDMI (Opus 48kHz)",
                tipAudio: "Volume dell'audio digitale inviato al monitor/TV tramite cavo HDMI. Latenza inferiore a 25ms con sincronizzazione A/V.",
                audioProfileLabel: "Profilo Audio HDMI e Frequenza di Campionamento",
                tipAudioProfile: "Seleziona e forza il clock hardware direttamente nel core audio ALSA BCM2835 del Pi Zero. Supporta audio subframe IEC958 reale fino a 192kHz 24-bit Hi-Res.",
                audioTransportLabel: "Architettura di Trasporto Audio",
                tipAudioTransport: "Seleziona la via fisica di trasporto dell'audio digitale: Modalità 1 Rete UDP (Porta 5004), Modalità 2 USB Audio Class (Gadget UAC2) o Modalità 3 Multiplex USB Bulk.",
                audioTransportDesc: "Modalità 1 Rete UDP: Flusso audio tramite porta UDP 5004 in pacchetti da 1024 byte senza frammentazione. Latenza inferiore a 5ms con sincronizzazione A/V.",
                audioTransUdpDesc: "Modalità 1 Rete UDP: Flusso audio tramite porta UDP 5004 in pacchetti da 1024 byte senza frammentazione. Latenza inferiore a 5ms con sincronizzazione A/V.",
                audioTransUac2Desc: "Modalità 2 Gadget UAC2: Il Pi Zero funge da scheda audio USB fisica sul PC. Zero dipendenza di rete, Plug-and-Play su Windows e Linux.",
                audioTransBulkDesc: "Modalità 3 USB Bulk: L'audio è multiplexato nel canale /dev/usb-display-bulk insieme al video H.264. Latenza sub-1ms.",
                docAltPlayersCmd: "# FFmpeg / ffplay (Bassa Latenza):\nffplay -probesize 32 -analyzeduration 0 -sync ext -fflags nobuffer -flags low_delay -i 'rtp://192.168.7.2:5000'",
                copied: "Copiato!",
                copiedSuccess: "✓ Copiato negli appunti!",
                m1Title: "Modalità 1: Rete UDP (Linux Wayland / X11)",
                m1Details: "Porta UDP 5000 • Latenza < 15ms • Pipeline VA-API/M2M",
                m2Title: "Modalità 2: Windows Miracast (Wi-Fi Display)",
                m2Details: "Porta RTSP 7236 • Windows Win+K • Decodifica Hardware V4L2 M2M",
                m3Title: "Modalità 3: USB Bulk Diretto (480 Mbps)",
                m3Details: "Bus USB 2.0 High-Speed • Zero-Network • Latenza < 1ms",
                waitingStreamDesc: "Ricevitore in attesa che mostra schermata splash con IP e QR Code.",
                capKmsTitle: "KMS Direct (Anti-Blocco / Scanout GPU)",
                capMutterTitle: "GNOME Mutter (PipeWire Screencast)",
                connHeader: "Connessione Schermo Attiva (Trasporto)",
                connBadge: "Scambio a Caldo",
                connDesc: "Seleziona il canale di trasmissione attivo tra il PC e lo schermo. Commuta istantaneamente senza riavviare.",
                extHeader: "Azione di Estensione dello Schermo & Topologia",
                extBadge: "Schermo Attivo",
                extBadgeActive: "Esteso (HDMI-1)",
                extBadgeClone: "Clonato (eDP-1)",
                extBadgeOff: "Standby (In Pausa)",
                extDesc: "Estendi l'area di lavoro sullo schermo HDMI-1, clona lo schermo principale del notebook (eDP-1), o disattiva l'estensione.",
                btnActionExtend: "🖥️ Estendi Schermo (HDMI-1)",
                btnActionExtendDesc: "Secondo monitor virtuale sulla TV",
                btnActionClone: "💻 Clona Schermo (eDP-1)",
                btnActionCloneDesc: "Duplica lo schermo del notebook",
                btnActionStop: "⏹ Disattiva Estensione / Standby",
                btnActionStopDesc: "Ferma la trasmissione e metti la TV in standby",
                toastExtStopped: "⏹ Estensione schermo disattivata. TV in standby.",
                toastExtCloned: "💻 Duplicazione schermo notebook (eDP-1)...",
                toastExtExtended: "🖥️ Estensione desktop su TV (HDMI-1)...",
                servicesHeader: "Servizi e Daemon di Ascolto dell'Appliance",
                servicesBadge: "Listener Hardware",
                lblScreenMode: "Modalità Display:",
                btnM1Extend: "🖥️ Estendi (UDP)",
                btnM1Clone: "💻 Clona (UDP)",
                btnM3Extend: "🖥️ Estendi (USB)",
                btnM3Clone: "💻 Clona (USB)"
            },
            zh: {
                title: "Pi Zero 扩展显示器",
                subtitle: "硬件 GPU VideoCore IV 显示设备",
                tabMonitor: "监控与实时遥测",
                tabConfig: "调节与系统设置",
                tabDownloads: "客户端工具与驱动",
                tabSdCard: "SD 卡与内存升级",
                tabManual: "操作与技术指南",
                statTemp: "SoC 核心温度",
                statCpu: "CPU 负载率",
                statRam: "可用内存 RAM",
                statStream: "推流状态",
                badgeVpuOffload: "硬件 VPU 卸载",
                statCpuDesc: "硬件 GPU V4L2 M2M 解码中",
                badgeRamApp: "100% 内存运行",
                tipTemp: "博通 BCM2835 核心硅片内部温度。每2秒轮询一次。建议保持在 65°C 以下。",
                tipCpu: "ARM11 CPU 负载。得益于 VideoCore IV VPU 硬件全卸载，CPU 占用率低于 2%。",
                tipRam: "512 MB SDRAM 中的可用内存。系统 100% 运行于 RAM 中，彻底杜绝 SD 卡读写磨损。",
                tipStream: "向 HDMI 电视输出 H.264 视频流的实时解码引擎状态。",
                displayHeader: "HDMI 电视与显示遥测",
                displayDesc: "VideoCore IV 硬件 VPU 直接将 H.264 解码输出至 HDMI 屏幕，完全不消耗 CPU 资源。",
                btnShowHud: "✦ 在电视上显示 HUD (60秒)",
                btnHideHud: "✕ 关闭 HUD",
                modeHeader: "多模式并行支持",
                badgeMultiMode: "并发引擎",
                m1Desc: "UDP 5000 端口超低延迟 RTP H.264 流，AMD VA-API 零拷贝 GPU 加速 (< 15ms)。",
                m2Desc: "Windows 10/11 原生 Win + K 无线投屏（RTSP 7236 端口），电脑无需安装任何驱动。",
                m3Desc: "USB FunctionFS 480 Mbps 裸硬件通道，无网络协议栈开销 (< 1ms)。",
                netHeader: "网络接口与 IP 配置 (以太网 / 局域网 / Wi-Fi)",
                netModeLabel: "辅助网络接口寻址模式",
                tipNetMode: "在没有 DHCP 路由器的点对点环境中（例如开机后在另一台树莓派或 PC 上）选择静态 IP，或者选择 DHCP 自动获取。",
                ctrlHeader: "显示与推流优化",
                badgeZeroCopy: "VideoCore IV DMA",
                bitrateLabel: "推流码率 (VBR)",
                tipBitrate: "实时 VBR 编码码率。400k 为代码与文本节省 85% 带宽；3000k 至 6000k 提供流畅 60 FPS 动态画面。",
                fpsLabel: "帧率 (FPS)",
                tipFps: "目标帧率。15 FPS 超低发热，30 FPS 推荐平衡模式，60 FPS 游戏与鼠标极致流畅。",
                colorLabel: "颜色配置",
                tipColor: "24位真彩色提供 1:1 RGB 原画质；256 色采用自适应量化 (QP 30-44) 最大限度节省 USB 带宽。",
                colorFull: "24位真彩色",
                color256: "256 色低功耗",
                colorGray: "单色灰度",
                dropOnlyLabel: "跳帧保护与损伤更新 (Damage-Only)",
                tipDropOnly: "启用时仅在检测到画面变动时传输。禁用时强制以 30 FPS 连续发送，确保 YouTube 视频在鼠标移开时永不卡顿。",
                dropOnlyTrue: "启用 (静态屏幕节省 95% 带宽)",
                dropOnlyFalse: "禁用 (强制连续重复帧)",
                dropOnlyDesc: "启用后，屏幕静止时不重复发送帧，仅在鼠标移动或打字时全力传输画面更新。",
                skipFirstLabel: "即时首帧传输 (skip-to-first)",
                tipSkipFirst: "消除累积缓冲延迟，在鼠标动作瞬间零延迟送达首帧画面。",
                skipFirstTrue: "启用 (动作发生时零延迟送达)",
                skipFirstFalse: "禁用 (严格时间戳对齐)",
                keyIntLabel: "定期全屏刷新 / IDR 关键帧间隔 (Refresh Clean)",
                tipKeyInt: "定期注入完整 IDR 关键帧的频率，用于彻底清除屏幕残影并从传输丢包中快速恢复。",
                keyIntDesc: "定期注入完整的 IDR 关键帧，彻底清除 HDMI/TV 显示器上的任何视觉残影。",
                btnApply: "💾 应用配置",
                btnPauseStream: "⏸ 暂停显示",
                btnResumeStream: "▶ 恢复显示",
                btnReboot: "🔄 重启设备 (Reboot)",
                oneLinerTitle: "主机一键连接脚本 (1 键执行)",
                badgeInstant: "即时生效",
                oneLinerDesc: "在任何 Linux 电脑上，只需在终端中运行以下命令即可立即扩展屏幕：",
                btnCopy: "复制",
                dlPkgTitle: "客户端完整安装包",
                dlPkgDesc: "包含预编译 ext-sender 二进制文件、udev 规则和说明文档的 tar.gz 压缩包。",
                btnDlPkg: "下载 client.tar.gz",
                dlSenderTitle: "ext-sender 二进制文件",
                dlSenderDesc: "为 Linux x86_64 编译的独立 Rust GPU 硬件推流程序。",
                btnDlSender: "下载 ext-sender",
                dlScriptTitle: "connect.sh 连接脚本",
                dlScriptDesc: "自动检测 USB 连接、下载必要工具并启动推流的便携式脚本。",
                btnDlScript: "下载 connect.sh",
                dlUdevTitle: "udev 即插即用规则",
                dlUdevDesc: "插入 USB 线缆时自动优化网络队列 (txqueuelen 100)，杜绝网络延迟积累。",
                btnDlUdev: "下载 99-ext-monitor.rules",
                sdHeader: "100% 内存运行架构与固件升级",
                badgeZeroSdWear: "零 SD 卡磨损",
                sdDesc1: "树莓派系统 100% 运行在内存中 (initramfs)。开机仅需不到2秒，物理 SD 卡 (/dev/mmcblk0) 处于解挂状态，避免读写老化。",
                sdDesc2: "彻底杜绝断电损坏 SD 卡的风险，并允许在线挂载 FAT16 引导分区，无需取出卡即可升级固件！",
                sdPartitionStatus: "物理存储状态：",
                btnMountSd: "挂载引导分区 (/mnt/boot)",
                btnUnmountSd: "卸载引导分区",
                sdUpgradeManualTitle: "无需取出 SD 卡在线更新固件方法：",
                manualHeader: "完整操作手册与技术规范指南",
                badgeFullDocs: "完整技术文档",
                docArchTitle: "1. 硬件架构与 VideoCore IV GPU 解码",
                docArchDesc: "树莓派 Pi Zero 搭载博通 BCM2835 SoC (ARM1176JZF-S @ 1.0 GHz) 与 500 MHz VideoCore IV GPU。ext-monitor 采用 100% 用户空间纯硬件管线：H.264 流直接喂入 V4L2 M2M 解码器 (/dev/video10)，解码后的帧通过 DMA 直接投射到 HDMI 扫描平面 (/dev/fb0)，延迟低于 15ms，CPU 占用低于 2%。",
                docUsbProtoTitle: "2. USB Bulk 裸通道、RFC 4571 协议与帧结束标记位 (EOF / ZLP)",
                docUsbProtoDesc1: "在 USB 2.0 高速总线 (480 Mbps) 上，数据包以最大 512 字节 (wMaxPacketSize) 传输。在传统的 H.264 Annex-B 裸流中，解码器必须等待下一帧的起始码 (00 00 00 01) 才能确认当前帧结束，导致鼠标静止或视频暂停时画面卡死。",
                docUsbProtoDesc2: "ext-monitor 引入官方标准 RFC 4571 双字节长度封包，并完整保留 RTP Marker Bit (EOF) 信号。USB 收到数据帧最后一个切片的瞬间立即触发 HDMI 显示，彻底摆脱对鼠标移动的依赖，YouTube 播放丝滑流畅。传输长度恰为 512 整数倍时自动发送 ZLP 零长度包，彻底释放 DWC2 硬件 FIFO 避免死锁。",
                docAudioTitle: "3. 多模式音频路由架构 (标准模式 vs 混合模式)",
                docAudioDesc1: "ext-monitor 支持通过 4 种传输通道将电脑立体声音频输出至 HDMI 电视音响：",
                docAudioDesc2: "标准模式 vs 混合模式：标准模式下电脑视频与音频同步输出至电视；混合模式下电脑画面投射至电视，声音由笔记本扬声器播放，或在电视显示电脑屏幕的同时，通过蓝牙播放手机音乐。",
                docSerialTitle: "4. 通过 USB 虚拟串口 (/dev/ttyACM0) 零 IP 维护",
                docSerialDesc: "若网络禁用或配置错误，树莓派会在电脑上提供 115200 波特率的 /dev/ttyACM0 救援控制台。",
                docLinuxTitle: "5. Linux Wayland (GNOME) 正常连接",
                docLinuxDesc: "将 USB 线插入中间的数据端口。电脑将通过内置 DHCP 自动获取 192.168.7.1，然后运行 ext-sender（无需传入参数）：",
                docWinTitle: "6. Windows 10/11 投屏 (Win + K 无需驱动)",
                docWinDesc: "插入 USB 后在 Windows 上按 Win + K，选择 'Pi Zero Wireless Display' 即可。",
                docSdTitle: "7. 100% 内存运行架构与免拔卡在线固件升级",
                docSdDesc: "系统 100% 运行于 initramfs 内存盘中。升级接收端时只需通过 'mount -t vfat /dev/mmcblk0p1 /mnt' 挂载 FAT 引导分区写入新镜像并卸载，无需断电或拔出 SD 卡。",
                docBrowserVideoTitle: "8. 💡 技巧：防止外部显示器上的浏览器视频在鼠标移出时暂停 (Chrome / Firefox)",
                docBrowserVideoDesc: "在 Linux Wayland 下，Chrome 和 Firefox 默认启用激进的节能策略 ('Window Occlusion Tracking')，当鼠标移出窗口或失去焦点时会自动挂起视频渲染。如需保持 60 FPS 持续平滑播放：",
                docBrowserChrome: "在地址栏打开 chrome://flags/#calculate-native-win-occlusion，设为 'Disabled' 并重启浏览器（或启动时添加参数 --disable-backgrounding-occluded-windows）。",
                docBrowserFirefox: "在地址栏打开 about:config，搜索 media.suspend-bkgnd-video.enabled 并修改为 'false'。",
                docBrowserNative: "原生播放器默认以 60 FPS 持续渲染，完全不受鼠标焦点或窗口层叠限制。",
                docArmv6Title: "9. 树莓派 Pi Zero (ARMv6) 交叉编译指南",
                docArmv6Desc: "Pi Zero v1.2/v1.3/W 搭载的 BCM2835 SoC 需使用 ARMv6l 架构。使用 cross (Docker) 编译：",
                docArmv6Cmd: "cargo install cross && cd receiver && cross build --target arm-unknown-linux-musleabihf --release",
                docNonOtgTitle: "10. 非 OTG 树莓派型号 (Pi 2, Pi 3, Pi 4, Pi 5) 与启动参数调整",
                docNonOtgDesc1: "上述型号的标准 USB-A 接口不支持 USB Gadget 外设模式。视频流通过以太网或 Wi-Fi 进行传输。",
                docNonOtgDesc2: "SD 引导必需调整：在 config.txt 中注释 'dtoverlay=dwc2'。在 cmdline.txt 中移除 'modules-load=dwc2'。连接命令：ext-sender --ip=<树莓派IP>",
                docPcReceiverTitle: "11. 将普通 Linux 电脑 / 笔记本改造为副屏接收器",
                docPcReceiverDesc: "任何 Linux 计算机均可充当接收端。安装 gstreamer1.0-tools 并运行：",
                docPcReceiverCmd: "gst-launch-1.0 -v udpsrc port=5000 buffer-size=524288 caps=\"application/x-rtp,media=video,clock-rate=90000,encoding-name=H264,payload=96\" ! rtph264depay ! h264parse ! avdec_h264 ! autovideosink sync=false",
                docMultiMonTitle: "12. 多显示器电脑接收端定向投影至屏幕 1 或屏幕 2",
                docMultiMonDesc: "在连接多个显示器的电脑接收端上，在 DRM KMS 裸机模式下指定 'kmssink connector-id=<ID>'，或在桌面图形会话中使用 'ffplay -left 1920 -top 0 -fs rtp://0.0.0.0:5000' 定位全屏播放。",
                docPacerTitle: "13. 🚀 持续 60 FPS 防冻结流畅推流",
                docPacerDesc: "ext-monitor 在 ext-sender 中原生提供 60 FPS CFR 持续防冻结推流。即使鼠标静止或停留在主屏，YouTube 视频与外部终端仍保持 60 FPS 极速刷新。",
                docAltPlayersTitle: "14. 非 GNOME 接收端通用替代方案 (FFmpeg, MPV, VLC)",
                docAltPlayersDesc: "RFC 4571 RTP H.264 视频流完全跨平台。在 KDE、XFCE、i3、Windows 或 macOS 上无需缓冲区极低延迟接收：",
                docBulkFallbackTitle: "15. ⚡ 默认 USB Bulk 裸通道模式与 UDP 网络自动平滑降级",
                docBulkFallbackDesc: "ext-sender 默认优先连接 480 Mbps 高速 USB Bulk 裸通道。若未检测到 USB Gadget 硬件外设，将自动透明降级为 UDP 网络推流 (5000 端口)，绝不崩溃。",
                lblActiveMode: "当前运行推流模式",
                lblActiveHdmi: "树莓派当前活跃 HDMI 输出",
                protoHeader: "传输协议特性对比表",
                thMethod: "传输方式",
                thProtocol: "通信协议",
                thLatency: "传输延迟",
                thBestFor: "适用场景",
                tbM1Method: "Linux Wayland 直连",
                tbM1Best: "交互式桌面，鼠标流畅拖拽窗口",
                tbM2Method: "Windows 10/11 Miracast",
                tbM2Best: "Windows 原生免驱无线投屏 (Win + K)",
                tbM3Method: "USB Bulk 裸硬件通道",
                tbM3Best: "无 IP 裸硬件通信，RFC 4571 帧结束标记位 (EOF)",
                bookTitle: "Ext-Monitor 权威技术手册 — 工程架构全景指南",
                bookBadge: "18 部核心技术架构图谱",
                bookDesc: "包含博通 BCM2835 芯片底层逆向、RFC 4571 / RFC 6184 / WFD 传输协议、混合音频架构以及 Wayland/DRM-KMS 渲染管线的全部技术细节已收录于：<strong style=\"color: #fff; font-family: monospace;\">docs/LIVRO-EXT-MONITOR.md</strong>。",
                bookPart1Title: "第一部分：BCM2835 硬件与启动",
                bookPart1Desc: "1.8秒极速启动、FAT16 32MB 引导盘、VideoCore IV V4L2 M2M 解码与零拷贝 DMA-BUF。",
                bookPart2Title: "第二部分：传输协议与总线技术",
                bookPart2Desc: "USB Bulk RFC 4571 Marker Bit/ZLP、RTP H.264 FU-A、WFD Miracast 无线投屏与 UAC2。",
                bookPart3Title: "第三部分：Linux 主机端与 Wayland",
                bookPart3Desc: "GNOME Mutter 屏幕录制 D-Bus、PipeWire、AMD Radeon 610M DCN 3.1 与 DRM/KMS。",
                bookPart4Title: "第四部分：混合音频架构与 Web 面板",
                bookPart4Desc: "48kHz Opus UDP、树莓派 BlueZ A2DP 蓝牙接收端与 Rust 双向 Web 控制面板。",
                docAudioMode1: "<strong style=\"color: #fff;\">模式 1 (IP 网络):</strong> 电脑端 PipeWire 虚拟音频设备通过 UDP 5004 端口将 RTP Opus/PCM 流直接发送至树莓派 HDMI 解码芯片。",
                docAudioMode2: "<strong style=\"color: #fff;\">模式 2 (Windows Miracast):</strong> 原生 Wi-Fi Display (WFD) 协议通过 RTSP TCP 7236 传输立体声 AAC/LPCM 音频。",
                docAudioMode3: "<strong style=\"color: #fff;\">模式 3 (USB 虚拟声卡 - UAC2):</strong> 通过复合外设在电脑上免驱枚举出即插即用 USB 声卡，将 PCM 音频直通电视 HDMI。",
                docAudioMode4: "<strong style=\"color: #fff;\">模式 4 (蓝牙 A2DP 接收端):</strong> 开启树莓派蓝牙音频接收端，支持手机、平板或副笔记本直连播放。",
                docAudioRoutingTitle: "16. 🔊 混合音频路由与 HDMI 虚拟音频设备",
                docAudioRoutingDesc: "ext-monitor 采用完全隔离的 PipeWire/PulseAudio 虚拟声卡 (<code>Raspberry_Pi_HDMI_Audio</code>)。您可以一边在电视副屏上办公编码，一边让会议、浏览器音频由笔记本内置扬声器或 USB 耳机独立播放：",
                docClocksTitle: "17. ⚙️ 硬件子模块时钟频率 (H.264/VPU/ARM) 与功耗",
                docClocksDesc: "树莓派 Pi Zero W 通过 <code>debugfs</code> 暴露了博通 BCM2835 芯片内部各子模块的实时运行频率。ext-monitor 全程高精度监测硬件时钟：",
                docCliManualTitle: "18. 📖 完整命令行手册、CLI 参数与故障排查",
                docCliManualDesc: "推流程序语法：<code>ext-sender [extend|clone] [帧率] [码率] [参数]</code>",
                badgeStreaming: "● 推流中",
                badgeMode1On: "已启用 (UDP 5000)",
                badgeMode2On: "已启用 (TCP 7236)",
                badgeMode3On: "已启用 (USB Bulk)",
                hostHeader: "主机端推流器远程控制 (Host PC)",
                hostBadge: "双向 UDP 5001 协议",
                hostStatusLabel: "推流状态与远程操作",
                btnHostStart: "▶ 启动 / 重启推流",
                btnHostStop: "⏹ 停止推流",
                hostModeLabel: "显示器投屏模式",
                tipHostMode: "HDMI-1: 在电视上扩展虚拟副屏；eDP-1: 镜像复制笔记本主屏；询问: 投屏前在笔记本弹出选择窗口。",
                btnModeAsk: "❓ 每次询问 (弹窗)",
                btnModeExtend: "🖥️ 扩展模式 (HDMI-1 电视)",
                btnModeClone: "💻 镜像模式 (eDP-1 笔记本)",
                toastModeAsk: "❓ 模式已配置：每次投屏前在笔记本弹出询问窗口",
                hostAudioLabel: "混合音频流 (IP 网络 Opus + 蓝牙 A2DP)",
                tipHostAudio: "启用网络音频将电脑声音以 48kHz Opus 传输至电视 HDMI；使用蓝牙配对可将手机/平板直接连至电视播放音频。",
                btnHostAudioOn: "🔊 网络音频 (Opus UDP)",
                btnHostAudioOff: "🔇 禁用网络音频",
                btnBtPair: "📡 开启蓝牙配对 A2DP (60秒)",
                hostHudLabel: "电视屏幕实时遥测 HUD",
                tipHostHud: "在电视屏幕右下角叠加投射 60 秒的实时 FPS、码率与延迟遥测面板。",
                btnHostShowHud: "📊 在电视上显示 HUD (60秒)",
                btnHostHideHud: "❌ 隐藏 HUD",
                mediaHeader: "IoT 媒体中心与 HDMI 音频频谱可视化 (Chromecast / DLNA)",
                audioHeader: "HDMI 数字高保真音频、硬件 DAC 与集成传输控制",
                audioBadge: "ALSA 硬件 PCM",
                audioCardDesc: "集中式集成控制：选择活动传输通道 (模式 1、2 或 3)、屏幕拓扑 (扩展或镜像)、调节数字音频音量并配置 ALSA 硬件主时钟。",
                audioModeLabel: "活动传输通道 (视频 + HDMI 音频)",
                tipAudioMode: "在电脑与树莓派之间快速热切换物理传输管线，无需重启。",
                audioTopologyLabel: "笔记本屏幕拓扑 (Wayland / Mutter)",
                tipAudioTopology: "将桌面扩展到电视作为第二虚拟显示器 (HDMI-1)，或镜像笔记本主屏幕 (eDP-1)。",
                castHeader: "网页共享与 Chromecast 风格投屏 (Google Cast)",
                castDesc: "像真正的 Chromecast 一样，将浏览器标签页、窗口、视频链接或手机屏幕直接镜像到电视。",
                webCastTitle: "投射此标签页或屏幕 (Web Cast)",
                webCastDesc: "通过低延迟 WebCodecs 硬件编码，投射任何 Chrome/Firefox 标签页、会议或整个屏幕。",
                btnOpenWebCast: "🔴 打开网页投屏器 (/cast)",
                googleCastTitle: "Google Cast (原生 Chromecast)",
                googleCastDesc: "在 Chrome/Edge 菜单（投射...）或手机应用（YouTube、Netflix、Pluto TV）中选择 'Ext-Monitor (Raspberry Pi)'。",
                googleCastReady: "✓ mDNS 与 Cast V2 活跃 (端口 8008 / 8009)",
                castUrlLabel: "直接在电视上播放视频链接 (Play URL)",
                tipCastUrl: "输入视频链接 (MP4, WebM, HLS m3u8)，直接在树莓派 HDMI 输出了硬件解码播放。",
                btnCastUrl: "▶ 在电视上播放",
                toastEnterUrl: "请输入有效的视频链接。",
                toastCasting: "正在将视频发送到电视屏幕...",
                toastCastSuccess: "✓ 视频已成功投射到电视！",
                badgeHdmiMuxStandby: "模式：待机 (就绪引导屏)",
                badgeHdmiMuxPc: "模式：PC 视频 (电视画面 + 音频)",
                badgeHdmiMuxIot: "模式：IoT 音频 (HDMI 频谱可视化 30 FPS)",
                mediaBadge: "Google Home • UPnP • 蓝牙",
                mediaPlaybackLabel: "当前播放信息 (电视 IoT 音频)",
                tipMediaPlayback: "显示正在通过手机蓝牙或 UPnP/Cast 网络播放的音乐元数据。",
                mediaVisLabel: "电视动态频谱图 (彻底杜绝黑屏待机)",
                tipMediaVis: "当仅有音频播放而无 PC 桌面推流时，在 HDMI 输出 30 FPS 的 24 频段动态频谱与 VU 表，防止电视息屏。",
                btnVisOn: "🎨 开启 HDMI 频谱可视化",
                btnVisOff: "⏹ 关闭可视化",
                btnTestAudioChime: "🔊 测试硬件音频信号",
                netUsb0Label: "USB OTG 虚拟网卡 (usb0)",
                netUsb0Desc: "● 主机 OTG 专属直连",
                netEth0Label: "物理以太网 (eth0)",
                netEth0Desc: "● 静态 IP / DHCP",
                netWlan0Label: "无线局域网 (wlan0)",
                netWlan0Desc: "● 可选接口",
                netIfaceLabel: "网络接口：",
                btnNetStatic: "📌 静态 IP",
                btnNetDhcp: "🔄 动态 DHCP",
                netIpLabel: "IP 地址：",
                netMaskLabel: "子网掩码：",
                netGwLabel: "默认网关：",
                netDnsLabel: "DNS 服务器：",
                btnApplyNet: "💾 保存并应用到设备",
                btnAudioMute: "🔊 静音音频",
                captureLabel: "捕获引擎 (双引擎支持)",
                tipCapture: "KMS Direct: 直接通过 Linux 内核 DRM/KMS 从 GPU 硬件扫描平面读取像素，即使鼠标静止也永不冻结；Mutter: 通过 GNOME Mutter D-Bus 屏幕录制接口捕获。",
                btnKmsDirect: "⚡ KMS 硬件直读 (抗冻结 / GPU 扫描面)",
                btnGnomeMutter: "🐧 GNOME Mutter (PipeWire 投屏)",
                monitorTargetLabel: "捕获目标显示器",
                tipMonitorTarget: "选择要捕获的视频输出。HDMI-1 用于在电视上扩展副屏，eDP-1 用于复制笔记本主屏。",
                scaleLabel: "显示分辨率与画面缩放",
                tipScale: "选择显示分辨率。原生 1:1 模式提供最清晰的矢量文字渲染且无模糊。超分辨率模式利用 VideoCore IV 硅芯片硬件 HVS 或 GPU FSR 拉伸填满宽屏显示器。",
                lblSiliconScaler: "硬件硅芯片缩放器 (VideoCore IV HVS & FSR)",
                badgeSiliconActive: "HVS 已启用",
                badgeSiliconOff: "仅原生 1:1",
                optgroupNative: "原生 1:1 直读模式 (超锐利 / 无模糊)",
                optgroupUpscale: "超分辨率缩放 (硅芯片 HVS / FSR)",
                btnScale720p: "🎯 原生 720p (1:1)",
                btnScale1600x900: "📐 900p 超分拉伸",
                btnScaleOff: "⚡ 直通模式",
                casLabel: "对比度自适应锐化 (CAS)",
                tipCas: "恢复 PC 全范围动态色彩 (0-255) 与纯正黑阶，消除偏白洗白伪影。",
                btnCasTrue: "✨ 启用 CAS (清晰文字 / 纯黑阶)",
                btnCasFalse: "标准 (TV 压缩色彩范围)",
                statStreamActive: "推流中",
                statStreamPaused: "已暂停",
                waitingStream: "等待推流 (引导屏已就绪)",
                mediaPlaying: "音频播放中",
                mediaPaused: "已暂停",
                mediaIdle: "待机 / 就绪",
                visActive: "活跃 (30 FPS)",
                visDisabled: "已禁用",
                toastTestAudio: "🔊 正在触发 HDMI 硬件音频测试信号...",
                toastMuted: "HDMI 音频已静音",
                toastUnmuted: "HDMI 音频已启用",
                mute: "静音音频",
                unmute: "取消静音",
                netConnected: "已连接",
                netDisconnected: "未连接",
                cableDisconnected: "网线未插入",
                modalRebootTitle: "确定要重启设备？",
                modalRebootDesc: "确定要重启树莓派 Pi Zero 吗？系统将在不到2秒内直接在内存中快速重启。",
                btnCancel: "取消",
                btnConfirmReboot: "确认重启",
                audioLabel: "HDMI 数字音频 (Opus 48kHz)",
                tipAudio: "通过 HDMI 发送到监视器/电视的数字音频音量。低于 25ms 延迟并保证音画同步。",
                audioProfileLabel: "HDMI 主音频配置与采样率",
                tipAudioProfile: "选择并强制将硬件采样率时钟直接载入树莓派 Pi Zero BCM2835 ALSA 核心。支持高达 192kHz 24-bit Hi-Res 真实 IEC958 子帧音频。",
                audioTransportLabel: "音频传输架构与物理通道",
                tipAudioTransport: "选择数字音频的物理传输通道：模式 1 UDP 网络流 (端口 5004)、模式 2 USB 声卡设备 (UAC2 Gadget) 或模式 3 USB Bulk 复用传输。",
                audioTransportDesc: "模式 1 UDP 网络：音频通过 UDP 端口 5004 传输，1024 字节防分片封包。低于 5ms 延迟并保证音画同步。",
                audioTransUdpDesc: "模式 1 UDP 网络：音频通过 UDP 端口 5004 传输，1024 字节防分片封包。低于 5ms 延迟并保证音画同步。",
                audioTransUac2Desc: "模式 2 UAC2 设备：树莓派 Pi Zero 在 PC 上识别为即插即用物理 USB 声卡。零网络协议依赖，免驱支持 Win/Linux。",
                audioTransBulkDesc: "模式 3 USB Bulk 复用：音频与 H.264 视频直接复用在 /dev/usb-display-bulk 管道中。低于 1ms 极限延迟。",
                docAltPlayersCmd: "# FFmpeg / ffplay (超低延迟播放):\nffplay -probesize 32 -analyzeduration 0 -sync ext -fflags nobuffer -flags low_delay -i 'rtp://192.168.7.2:5000'",
                copied: "已复制!",
                copiedSuccess: "✓ 已复制到剪贴板!",
                m1Title: "模式 1: UDP 网络 (Linux Wayland / X11)",
                m1Details: "UDP 端口 5000 • 低于 15ms 延迟 • VA-API/M2M 流水线",
                m2Title: "模式 2: Windows Miracast (无线投屏)",
                m2Details: "RTSP 端口 7236 • Windows Win+K • 硬件 V4L2 M2M 解码",
                m3Title: "模式 3: USB Bulk 直连 (480 Mbps)",
                m3Details: "USB 2.0 高速总线 • 无需网络协议栈 • 低于 1ms 延迟",
                waitingStreamDesc: "接收端待机就绪，正在显示包含 IP 和二维码的启动屏。",
                capKmsTitle: "KMS 直连 (防冻结 / GPU 扫描帧)",
                capMutterTitle: "GNOME Mutter (PipeWire 屏幕录制)",
                connHeader: "活动屏幕连接 (活动传输通道)",
                connBadge: "热切换",
                connDesc: "选择 PC 与此屏幕之间的活动传输通道。无需重启即可即时切换。",
                extHeader: "屏幕扩展与拓扑操作",
                extBadge: "活动屏幕",
                extBadgeActive: "扩展模式 (HDMI-1)",
                extBadgeClone: "克隆模式 (eDP-1)",
                extBadgeOff: "待机 (暂停)",
                extDesc: "将桌面扩展到HDMI-1电视，克隆主笔记本屏幕（eDP-1），或停用屏幕扩展让电视进入待机状态。",
                btnActionExtend: "🖥️ 扩展屏幕 (HDMI-1)",
                btnActionExtendDesc: "电视作为第二虚拟显示器",
                btnActionClone: "💻 克隆屏幕 (eDP-1)",
                btnActionCloneDesc: "镜像笔记本主屏幕",
                btnActionStop: "⏹ 停用扩展 / 待机",
                btnActionStopDesc: "停止传输并让电视待机",
                toastExtStopped: "⏹ 屏幕扩展已停用。电视进入待机。",
                toastExtCloned: "💻 正在镜像笔记本屏幕 (eDP-1)...",
                toastExtExtended: "🖥️ 正在将桌面扩展到电视 (HDMI-1)...",
                servicesHeader: "设备监听守护进程与服务",
                servicesBadge: "硬件监听服务",
                lblScreenMode: "显示模式:",
                btnM1Extend: "🖥️ 扩展 (UDP)",
                btnM1Clone: "💻 克隆 (UDP)",
                btnM3Extend: "🖥️ 扩展 (USB)",
                btnM3Clone: "💻 克隆 (USB)"
            }
        };

        // Tab Switching
        function switchTab(tabId) {
            document.querySelectorAll('.tab-btn').forEach(b => b.classList.remove('active'));
            document.querySelectorAll('.tab-content').forEach(c => c.classList.remove('active'));
            
            const btn = document.getElementById('tabBtn_' + tabId);
            const content = document.getElementById('tab-' + tabId);
            if (btn && content) {
                btn.classList.add('active');
                content.classList.add('active');
            }
        }

        // Translation Lookup Helper
        function t(key) {
            const lang = localStorage.getItem('ext_monitor_lang') || 'en';
            const dict = I18N[lang] || I18N.en;
            return (dict && dict[key]) || (I18N.en && I18N.en[key]) || key;
        }

        // Language Switcher (Supports HTML formatting)
        function setLanguage(lang) {
            if (!I18N[lang]) lang = 'en';
            localStorage.setItem('ext_monitor_lang', lang);

            document.querySelectorAll('.flag-btn').forEach(btn => btn.classList.remove('active'));
            const activeFlag = document.getElementById('btnLang_' + lang);
            if (activeFlag) activeFlag.classList.add('active');

            const dict = I18N[lang];
            document.querySelectorAll('[data-i18n]').forEach(el => {
                const key = el.getAttribute('data-i18n');
                if (dict[key]) {
                    if (el.tagName === 'INPUT' && el.type === 'button') {
                        el.value = dict[key];
                    } else if (typeof dict[key] === 'string' && (dict[key].includes('<') || dict[key].includes('&'))) {
                        el.innerHTML = dict[key];
                    } else {
                        el.textContent = dict[key];
                    }
                }
            });

            // Refresh dynamic labels to reflect new language
            if (typeof currentColor !== 'undefined') {
                const lbl = document.getElementById('valColor');
                const labels = { full: t('colorFull'), '256': t('color256'), gray: t('colorGray') };
                if (lbl) lbl.textContent = labels[currentColor] || currentColor;
            }
            if (typeof currentCapture !== 'undefined') {
                const lbl = document.getElementById('valCapture');
                if (lbl) lbl.textContent = currentCapture === 'kms' ? t('capKmsTitle') : t('capMutterTitle');
            }
            if (typeof currentDropOnly !== 'undefined') setDropOnly(currentDropOnly);
            if (typeof currentSkipToFirst !== 'undefined') setSkipToFirst(currentSkipToFirst);
            if (typeof currentNetMode !== 'undefined') setNetModeUI(currentNetMode);
            pollTelemetry();
        }

        // Bitrate & FPS Controls
        function updateBitrateValue(val) {
            currentBitrate = parseInt(val, 10);
            localStorage.setItem('ext_bitrate', currentBitrate);
            document.getElementById('valBitrate').textContent = currentBitrate + ' kbps';
            document.querySelectorAll('[data-bitrate]').forEach(b => {
                b.classList.toggle('active', parseInt(b.getAttribute('data-bitrate'), 10) === currentBitrate);
            });
        }

        function setBitrate(kbps) {
            currentBitrate = parseInt(kbps, 10);
            localStorage.setItem('ext_bitrate', currentBitrate);
            const slider = document.getElementById('bitrateSlider');
            if (slider) slider.value = kbps;
            updateBitrateValue(kbps);
        }

        function setFps(fps) {
            currentFps = fps;
            localStorage.setItem('ext_fps', fps);
            document.getElementById('valFps').textContent = fps + ' FPS';
            document.querySelectorAll('#fpsGrid .btn-toggle').forEach(b => {
                b.classList.toggle('active', parseInt(b.getAttribute('data-fps'), 10) === fps);
            });
        }

        function setColor(profile) {
            currentColor = profile;
            localStorage.setItem('ext_color', profile);
            const labels = { full: t('colorFull'), '256': t('color256'), gray: t('colorGray') };
            const el = document.getElementById('valColor');
            if (el) el.textContent = labels[profile] || profile;
            document.querySelectorAll('#colorGrid .btn-toggle').forEach(b => {
                b.classList.toggle('active', b.getAttribute('data-color') === profile);
            });
        }

        function setDropOnly(val) {
            currentDropOnly = val;
            localStorage.setItem('ext_drop_only', val);
            const el = document.getElementById('valDropOnly');
            if (el) el.textContent = val ? 'drop-only=true (' + t('dropOnlyTrue') + ')' : 'drop-only=false (' + t('dropOnlyFalse') + ')';
            const btnT = document.getElementById('btnDropOnlyTrue');
            const btnF = document.getElementById('btnDropOnlyFalse');
            if (btnT) btnT.classList.toggle('active', val);
            if (btnF) btnF.classList.toggle('active', !val);
        }

        function setSkipToFirst(val) {
            currentSkipToFirst = val;
            localStorage.setItem('ext_skip_to_first', val);
            const el = document.getElementById('valSkipFirst');
            if (el) el.textContent = val ? 'skip-to-first=true (' + t('skipFirstTrue') + ')' : 'skip-to-first=false (' + t('skipFirstFalse') + ')';
            const btnT = document.getElementById('btnSkipFirstTrue');
            const btnF = document.getElementById('btnSkipFirstFalse');
            if (btnT) btnT.classList.toggle('active', val);
            if (btnF) btnF.classList.toggle('active', !val);
        }

        function updateKeyIntValue(val) {
            currentKeyIntMax = parseInt(val, 10);
            localStorage.setItem('ext_key_int_max', currentKeyIntMax);
            const sec = (currentKeyIntMax / (currentFps || 30)).toFixed(1);
            const el = document.getElementById('valKeyInt');
            if (el) el.textContent = `${currentKeyIntMax} frames (~${sec}s)`;
            document.querySelectorAll('#keyIntGrid [data-keyint]').forEach(b => {
                b.classList.toggle('active', parseInt(b.getAttribute('data-keyint'), 10) === currentKeyIntMax);
            });
        }

        function setKeyInt(val) {
            const slider = document.getElementById('keyIntSlider');
            if (slider) slider.value = val;
            updateKeyIntValue(val);
        }

        // Active Extension Connection Transport and Topology
        let currentTransport = 'mode1_udp';
        let currentTopology = 'extend';

        function isSimultaneousAudioEnabled() {
            const toggle = document.getElementById('toggleSimultaneousAudio');
            return toggle ? toggle.checked : false;
        }

        function onSimultaneousAudioToggle(checked) {
            localStorage.setItem('ext_simultaneous_audio', checked ? 'true' : 'false');
            showToast(checked ? '🔊 Áudio simultâneo para TV ATIVADO' : '🔇 Áudio simultâneo DESATIVADO (Vídeo puro)');
            sendHostControl({ audio: checked });
        }

        function activateModeWithTopology(transport, topology) {
            currentTransport = transport;
            if (topology && topology !== 'miracast') {
                currentTopology = topology;
            }

            if (transport === 'mode2_miracast') {
                setActiveTransport('mode2_miracast');
                return;
            }

            const targetMode = (currentTopology === 'clone') ? 'clone' : 'extend';
            currentTopology = targetMode;
            const modeLabel = targetMode === 'clone' ? 'Clonar (eDP-1)' : 'Estender (HDMI-1)';
            const transLabel = transport.includes('usb') ? 'USB Bulk Direct' : 'Rede UDP';
            const activeTrans = transport.includes('usb') ? 'usb_bulk' : 'network';
            const withAudio = isSimultaneousAudioEnabled();
            showToast(`🚀 Ativando ${transLabel} no modo ${modeLabel}${withAudio ? ' com Áudio' : ''}...`);

            // 1. Tell receiver to switch transport
            fetch('/api/transport/active', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ active_transport: transport, transport: transport, action: 'start' })
            }).catch(() => {});

            // 2. Tell host sender to switch transport, mode, and simultaneous audio flag
            sendHostControl({ action: 'start', mode: targetMode, transport: activeTrans, audio: withAudio });
            fetch('/api/stream/start', { method: 'POST' }).then(() => {
                setTimeout(pollTelemetry, 250);
                setTimeout(pollTelemetry, 800);
            });

            updateModeAndTopologyButtons();
        }

        const gpuCommands = {
            amd: 'env GST_PLUGIN_FEATURE_RANK=vaapih264enc:MAX,vah264enc:MAX gnome-network-displays',
            intel: 'env GST_PLUGIN_FEATURE_RANK=vaapih264enc:MAX,vah264enc:MAX,qsvh264enc:MAX gnome-network-displays',
            nvidia: 'env GST_PLUGIN_FEATURE_RANK=nvh264enc:MAX,vaapih264enc:MAX gnome-network-displays',
            all: 'env GST_PLUGIN_FEATURE_RANK=vaapih264enc:MAX,vah264enc:MAX,nvh264enc:MAX,qsvh264enc:MAX gnome-network-displays'
        };

        function selectGpuCmd(vendor) {
            const preview = document.getElementById('gpuLaunchCmdPreview');
            if (preview && gpuCommands[vendor]) {
                preview.textContent = gpuCommands[vendor];
            }
            ['chipGpuAmd', 'chipGpuIntel', 'chipGpuNvidia', 'chipGpuAll'].forEach(id => {
                const btn = document.getElementById(id);
                if (btn) btn.classList.remove('active');
            });
            const activeBtn = document.getElementById('chipGpu' + vendor.charAt(0).toUpperCase() + vendor.slice(1));
            if (activeBtn) activeBtn.classList.add('active');
        }

        function copyGpuLaunchCommand() {
            const preview = document.getElementById('gpuLaunchCmdPreview');
            if (preview) {
                const text = preview.textContent.trim();
                if (navigator.clipboard && navigator.clipboard.writeText) {
                    navigator.clipboard.writeText(text).then(() => {
                        showToast('✓ Comando GPU copiado para a Área de Transferência!');
                    }).catch(() => {
                        showToast('Comando: ' + text);
                    });
                } else {
                    showToast('Comando: ' + text);
                }
            }
        }

        function setActiveTransport(transport) {
            currentTransport = transport;
            const shortKey = transport.replace('_udp', '').replace('_miracast', '').replace('_usb_bulk', '');

            if (transport === 'mode2_miracast') {
                showToast('🪟 ' + (t('m2Title') || 'Miracast') + ' • Abrindo GNOME Displays com GPU no Laptop...');
                sendHostControl({ action: 'launch_miracast', transport: 'miracast' });
                fetch('/api/mode', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ mode: 'mode2_miracast' })
                }).catch(() => {});
            } else {
                const targetMode = (currentTopology === 'clone') ? 'clone' : 'extend';
                currentTopology = targetMode;
                showToast('Chaveando transporte para ' + shortKey.toUpperCase() + ' (' + (targetMode === 'clone' ? 'Clonar' : 'Estender') + ')...');
                const activeTrans = transport.includes('usb') ? 'usb_bulk' : 'network';
                const withAudio = isSimultaneousAudioEnabled();
                sendHostControl({ action: 'start', mode: targetMode, transport: activeTrans, audio: withAudio });
                fetch('/api/stream/start', { method: 'POST' }).catch(() => {});
            }

            fetch('/api/transport/active', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ active_transport: transport, transport: transport, action: 'start' })
            }).then(() => {
                setTimeout(pollTelemetry, 250);
                setTimeout(pollTelemetry, 800);
            }).catch(() => {});

            updateModeAndTopologyButtons();
        }

        // Display Extension Actions: Extend (HDMI-1), Clone (eDP-1), or Stop / Standby
        function setExtensionAction(action) {
            currentTopology = action;
            const activeTrans = currentTransport.includes('usb') ? 'usb_bulk' : 'network';
            const withAudio = isSimultaneousAudioEnabled();

            if (action === 'stop') {
                showToast(t('toastExtStopped') || '⏹ Extension disabled (Standby)');
                sendHostControl({ action: 'stop' });
                fetch('/api/stream/stop', { method: 'POST' }).then(() => setTimeout(pollTelemetry, 300));
            } else if (action === 'clone') {
                showToast(t('toastExtCloned') || '💻 Mirroring notebook display (eDP-1)...');
                sendHostControl({ action: 'start', mode: 'clone', transport: activeTrans, audio: withAudio });
                fetch('/api/stream/start', { method: 'POST' }).then(() => setTimeout(pollTelemetry, 300));
            } else {
                showToast(t('toastExtExtended') || '🖥️ Extending desktop to TV (HDMI-1)...');
                sendHostControl({ action: 'start', mode: 'extend', transport: activeTrans, audio: withAudio });
                fetch('/api/stream/start', { method: 'POST' }).then(() => setTimeout(pollTelemetry, 300));
            }

            updateModeAndTopologyButtons();
        }

        let hostConfiguredMode = 'ask';

        function setHostMode(mode) {
            hostConfiguredMode = mode;
            if (mode === 'ask') {
                showToast(t('toastModeAsk') || '❓ Modo configurado: Perguntar Sempre no Laptop');
                sendHostControl({ mode: 'ask' });
                updateModeAndTopologyButtons();
                return;
            }
            sendHostControl({ mode: mode });
            setExtensionAction(mode);
        }

        function updateModeAndTopologyButtons() {
            const shortKey = currentTransport.replace('_udp', '').replace('_miracast', '').replace('_usb_bulk', '');

            // 1. Update Transport buttons across Tab 1 & Tab 2 (including integrated card)
            document.querySelectorAll('#activeTransportGrid .btn-toggle, #hostTransportGrid .btn-toggle, #integratedModeGrid .btn-toggle').forEach(b => {
                const isCurrent = b.id === 'btnTransport_' + shortKey || b.id === 'btnHostTransport_' + shortKey;
                b.classList.toggle('active', isCurrent && currentTopology !== 'stop');
            });
            const valHostTransport = document.getElementById('valHostTransport');
            if (valHostTransport) {
                valHostTransport.textContent = currentTransport.includes('mode3') ? 'USB Bulk Direct (Mode 3)' :
                    (currentTransport.includes('mode2') ? 'Windows Miracast (Mode 2)' : 'Network UDP (Mode 1)');
            }

            // 1.1 Update Integrated Audio & Streaming Card Feedback
            const valIntMode = document.getElementById('valIntegratedMode');
            const badgeInt = document.getElementById('badgeIntegratedState');
            if (valIntMode) {
                if (currentTopology === 'stop') {
                    valIntMode.textContent = '⏹️ Standby / Tela de Prontidão (Ocioso)';
                } else if (currentTransport.includes('mode3')) {
                    valIntMode.textContent = '⚡ Modo 3: USB Bulk Direct (< 1ms)';
                } else if (currentTransport.includes('mode2')) {
                    valIntMode.textContent = '🪟 Modo 2: Windows Miracast (TCP 7236)';
                } else {
                    valIntMode.textContent = '🐧 Modo 1: Rede UDP (< 15ms)';
                }
            }
            if (badgeInt) {
                if (currentTopology === 'stop') {
                    badgeInt.textContent = 'Standby';
                    badgeInt.className = 'card-badge badge-amber';
                } else if (currentTransport.includes('mode3')) {
                    badgeInt.textContent = 'Modo 3: USB Bulk';
                    badgeInt.className = 'card-badge badge-purple';
                } else if (currentTransport.includes('mode2')) {
                    badgeInt.textContent = 'Modo 2: Miracast';
                    badgeInt.className = 'card-badge badge-green';
                } else {
                    badgeInt.textContent = 'Modo 1: Rede UDP';
                    badgeInt.className = 'card-badge badge-cyan';
                }
            }

            const valIntTopo = document.getElementById('valIntegratedTopology');
            if (valIntTopo) {
                if (currentTopology === 'stop') {
                    valIntTopo.textContent = '⏹️ Desativada (Standby)';
                } else if (currentTopology === 'clone') {
                    valIntTopo.textContent = '💻 Clonada (eDP-1)';
                } else {
                    valIntTopo.textContent = '🖥️ Estendida (HDMI-1)';
                }
            }

            // 2. Update Topology buttons across Tab 1 & Tab 2
            const btnExt = document.getElementById('btnActionExtend');
            const btnCln = document.getElementById('btnActionClone');
            const btnStop = document.getElementById('btnActionStop');
            const badge = document.getElementById('badgeExtState');
            const hostAsk = document.getElementById('btnModeAsk');
            const hostExt = document.getElementById('btnModeExtend');
            const hostCln = document.getElementById('btnModeClone');
            const valHostMode = document.getElementById('valHostMode');

            if (currentTopology === 'stop') {
                if (btnExt) btnExt.classList.remove('active');
                if (btnCln) btnCln.classList.remove('active');
                if (btnStop) btnStop.classList.add('active');
                if (badge) {
                    badge.textContent = t('extBadgeOff') || 'Standby (Desativado)';
                    badge.className = 'card-badge badge-amber';
                }
            } else if (currentTransport === 'mode2_miracast') {
                if (btnExt) btnExt.classList.remove('active');
                if (btnCln) btnCln.classList.remove('active');
                if (btnStop) btnStop.classList.remove('active');
                if (badge) {
                    badge.textContent = 'Miracast Ativo';
                    badge.className = 'card-badge badge-green';
                }
            } else {
                if (btnExt) btnExt.classList.toggle('active', currentTopology === 'extend');
                if (btnCln) btnCln.classList.toggle('active', currentTopology === 'clone');
                if (btnStop) btnStop.classList.remove('active');
                if (badge) {
                    if (currentTopology === 'clone') {
                        badge.textContent = t('extBadgeClone');
                        badge.className = 'card-badge badge-cyan';
                    } else {
                        badge.textContent = t('extBadgeActive');
                        badge.className = 'card-badge badge-green';
                    }
                }
            }

            if (hostAsk) hostAsk.classList.toggle('active', hostConfiguredMode === 'ask');
            if (hostExt) hostExt.classList.toggle('active', hostConfiguredMode === 'extend');
            if (hostCln) hostCln.classList.toggle('active', hostConfiguredMode === 'clone');
            if (valHostMode) {
                if (hostConfiguredMode === 'ask') {
                    valHostMode.textContent = t('btnModeAsk') || '❓ Perguntar Sempre (Diálogo)';
                } else if (hostConfiguredMode === 'clone') {
                    valHostMode.textContent = t('btnModeClone') || '💻 Cloned (eDP-1)';
                } else {
                    valHostMode.textContent = t('btnModeExtend') || '🖥️ Extended (HDMI-1)';
                }
            }

            // 3. Update Listener Daemon Card action buttons
            const btnM1 = document.getElementById('btnMode1Connect');
            const btnM2 = document.getElementById('btnMode2Connect');
            const btnM3 = document.getElementById('btnMode3Connect');

            if (btnM1) btnM1.classList.toggle('active', currentTransport === 'mode1_udp' && currentTopology !== 'stop');
            if (btnM2) btnM2.classList.toggle('active', currentTransport === 'mode2_miracast' && currentTopology !== 'stop');
            if (btnM3) btnM3.classList.toggle('active', currentTransport === 'mode3_usb_bulk' && currentTopology !== 'stop');
        }

        // Operating Modes State & Toggle
        const activeModes = {
            mode1: true,
            mode2: true,
            mode3: true
        };

        function setModeToggleUI(modeKey, enabled) {
            activeModes[modeKey] = enabled;
            const toggle = document.getElementById('toggle' + modeKey.charAt(0).toUpperCase() + modeKey.slice(1));
            const card = document.getElementById('card' + modeKey.charAt(0).toUpperCase() + modeKey.slice(1));
            const badge = document.getElementById('badge' + modeKey.charAt(0).toUpperCase() + modeKey.slice(1));
            const transBtn = document.getElementById('btnTransport_' + modeKey);

            if (toggle) toggle.checked = enabled;
            if (card) {
                if (enabled) {
                    card.classList.remove('disabled');
                } else {
                    card.classList.add('disabled');
                }
            }
            if (transBtn) {
                transBtn.style.opacity = enabled ? '1.0' : '0.45';
                transBtn.style.pointerEvents = enabled ? 'auto' : 'none';
            }
            if (badge) {
                if (enabled) {
                    badge.className = modeKey === 'mode1' ? 'stat-badge badge-cyan' : modeKey === 'mode2' ? 'stat-badge badge-green' : 'stat-badge badge-purple';
                    badge.textContent = modeKey === 'mode1' ? t('badgeMode1On') : modeKey === 'mode2' ? t('badgeMode2On') : t('badgeMode3On');
                } else {
                    badge.className = 'stat-badge badge-red';
                    badge.textContent = t('visDisabled');
                }
            }
        }

        function toggleMode(modeKey, enabled) {
            setModeToggleUI(modeKey, enabled);
            localStorage.setItem('ext_' + modeKey, enabled);
            
            showToast(enabled ? `✓ ${modeKey.toUpperCase()} daemon enabled!` : `✕ ${modeKey.toUpperCase()} daemon disabled.`);
            
            if (enabled) {
                const targetTrans = modeKey === 'mode3' ? 'mode3_usb_bulk' : (modeKey === 'mode2' ? 'mode2_miracast' : 'mode1_udp');
                setActiveTransport(targetTrans);
            } else {
                const anyActive = activeModes.mode1 || activeModes.mode2 || activeModes.mode3;
                const currentActiveKey = currentTransport.replace('_udp', '').replace('_miracast', '').replace('_usb_bulk', '');

                if (!anyActive) {
                    setExtensionAction('stop');
                } else if (currentActiveKey === modeKey) {
                    const fallbackTrans = activeModes.mode1 ? 'mode1_udp' : (activeModes.mode3 ? 'mode3_usb_bulk' : 'mode2_miracast');
                    setActiveTransport(fallbackTrans);
                }

                fetch('/api/modes', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        [modeKey]: enabled,
                        mode1: activeModes.mode1,
                        mode2: activeModes.mode2,
                        mode3: activeModes.mode3
                    })
                }).then(() => {
                    setTimeout(pollTelemetry, 300);
                }).catch(() => {});
            }
        }

        function setCapture(cap) {
            currentCapture = cap;
            document.querySelectorAll('#captureGrid .btn-toggle').forEach(b => {
                b.classList.toggle('active', b.dataset.capture === cap);
            });
            document.getElementById('valCapture').textContent = cap === 'kms' ? t('btnKmsDirect') : t('btnGnomeMutter');
            localStorage.setItem('ext_capture', cap);
        }

        function setMonitor(mon) {
            currentMonitor = mon;
            document.querySelectorAll('#monitorGrid .btn-toggle').forEach(b => {
                b.classList.toggle('active', b.dataset.monitor === mon);
            });
            document.getElementById('valMonitor').textContent = mon;
            localStorage.setItem('ext_monitor', mon);
        }

        function toggleSiliconScaler(enabled) {
            const grp = document.getElementById('optgroupUpscale');
            const badge = document.getElementById('badgeSiliconStatus');
            const btn900 = document.getElementById('btnScale900');
            if (grp) grp.style.display = enabled ? '' : 'none';
            if (btn900) btn900.style.display = enabled ? '' : 'none';
            if (badge) {
                badge.textContent = enabled ? t('badgeSiliconActive') : t('badgeSiliconOff');
                badge.className = 'card-badge ' + (enabled ? 'badge-green' : 'badge-amber');
            }
            localStorage.setItem('ext_silicon_scaler', enabled);
            if (!enabled && (currentScale === '1600x900' || currentScale === '1920x1080')) {
                setScale('720p');
            }
            showToast(enabled ? '✓ Silicon Hardware Scaler (HVS) Enabled' : '✕ Silicon Scaler Disabled (Native 1:1 Only)');
        }

        function onResolutionSelectChange(val) {
            setScale(val);
        }

        function setScale(scale) {
            currentScale = scale;
            const sel = document.getElementById('resSelect');
            if (sel && sel.value !== scale) sel.value = scale;
            document.querySelectorAll('#scaleGrid .btn-toggle').forEach(b => {
                b.classList.toggle('active', b.dataset.scale === scale);
            });
            const labels = {
                '720p': '1280x720 (1:1 Native - Sharp)',
                '1024x768': '1024x768 (1:1 Native)',
                '800x600': '800x600 (1:1 Eco)',
                '1600x900': '1600x900 (HVS Upscale)',
                '1920x1080': '1920x1080 (FSR/HVS 1080p)',
                'off': 'Off (1:1 Passthrough)'
            };
            document.getElementById('valScale').textContent = labels[scale] || scale;
            localStorage.setItem('ext_scale', scale);
            sendHostControl({ scale: scale });
            showToast('⚡ Resolution / Scale: ' + (labels[scale] || scale));
        }

        function setCas(enabled) {
            currentCas = enabled;
            document.querySelectorAll('#casGrid .btn-toggle').forEach(b => {
                b.classList.toggle('active', (b.dataset.cas === 'true') === enabled);
            });
            document.getElementById('valCas').textContent = enabled ? 'CAS Enabled (Full Contrast)' : 'Standard (TV Limited Range)';
            localStorage.setItem('ext_cas', enabled);
            sendHostControl({ cas: enabled });
            showToast('✨ CAS Sharpening: ' + (enabled ? 'ON' : 'OFF'));
        }

        // Hot-Apply Configuration
        function applyConfiguration() {
            localStorage.setItem('ext_color', currentColor);
            localStorage.setItem('ext_fps', currentFps);
            localStorage.setItem('ext_bitrate', currentBitrate);
            localStorage.setItem('ext_drop_only', currentDropOnly);
            localStorage.setItem('ext_skip_to_first', currentSkipToFirst);
            localStorage.setItem('ext_key_int_max', currentKeyIntMax);
            localStorage.setItem('ext_capture', currentCapture);
            localStorage.setItem('ext_monitor', currentMonitor);
            localStorage.setItem('ext_scale', currentScale);
            localStorage.setItem('ext_cas', currentCas);
            showToast('Applying configuration via UDP 5001...');
            fetch('/api/config', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    fps: currentFps,
                    bitrate: currentBitrate,
                    color: currentColor,
                    drop_only: currentDropOnly,
                    skip_to_first: currentSkipToFirst,
                    key_int_max: currentKeyIntMax,
                    capture: currentCapture,
                    monitor: currentMonitor,
                    scale: currentScale,
                    cas: currentCas
                })
            })
            .then(res => res.json())
            .then(() => showToast('✓ Configuration applied successfully!'))
            .catch(() => showToast('✓ Sent to host streamer'));
        }

        // HUD Trigger
        function triggerHud(show) {
            const action = show ? 'trigger_hud' : 'hide_hud';
            fetch('/api/config', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ action: action })
            })
            .then(() => showToast(show ? '✓ HUD enabled on TV screen for 60s' : '✓ HUD hidden from TV screen'))
            .catch(() => showToast('HUD command sent'));
        }

        // Host PC Remote Control
        function sendHostControl(payload) {
            fetch('/api/host/control', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(payload)
            })
            .then(res => res.json())
            .then(() => showToast('✓ Command sent to Host: ' + JSON.stringify(payload)))
            .catch(() => showToast('✓ Command transmitted'));
        }

        function setHostAudio(enabled) {
            const btnOn = document.getElementById('btnHostAudioOn');
            const btnOff = document.getElementById('btnHostAudioOff');
            if (btnOn) btnOn.classList.toggle('active', enabled);
            if (btnOff) btnOff.classList.toggle('active', !enabled);
            const lbl = document.getElementById('valHostAudio');
            if (lbl) lbl.textContent = enabled ? t('btnHostAudioOn') : t('btnHostAudioOff');
            sendHostControl({ audio: enabled });
        }

        function triggerBtPairing() {
            showToast('📡 Enabling Bluetooth A2DP pairing for 60s...');
            fetch('/api/bluetooth/discoverable', { method: 'POST' })
                .then(r => r.json())
                .then(() => showToast('✓ Raspberry Pi discoverable via Bluetooth! Search for "ext-monitor" on phone/PC.'))
                .catch(() => showToast('Bluetooth command sent'));
        }

        // IoT Media & HDMI Visualizer Control
        function toggleVisualizer(enabled) {
            fetch('/api/media/visualizer', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ enabled: enabled })
            })
            .then(res => res.json())
            .then(() => {
                const btnOn = document.getElementById('btnVisOn');
                const btnOff = document.getElementById('btnVisOff');
                const valSt = document.getElementById('valVisualizerState');
                if (btnOn) btnOn.classList.toggle('active', enabled);
                if (btnOff) btnOff.classList.toggle('active', !enabled);
                if (valSt) valSt.textContent = enabled ? t('visActive') : t('visDisabled');
                showToast(enabled ? '✓ HDMI Visualizer enabled on TV' : '✓ HDMI Visualizer disabled');
            });
        }

        // Trigger real hardware audio chime / test pulse
        function testRealAudioSignal() {
            showToast(t('toastTestAudio'));
            fetch('/api/media/test_sound', { method: 'POST' })
                .then(r => r.json())
                .then(() => {
                    showToast('✓ Audio signal pulse sent to HDMI TV pipeline');
                    pollMediaStatus();
                })
                .catch(() => showToast('Audio pulse transmitted'));
        }

        // Direct Video URL Cast (Play on TV)
        function castMediaUrl() {
            const input = document.getElementById('castMediaUrlInput');
            const url = input ? input.value.trim() : '';
            if (!url) {
                showToast(t('toastEnterUrl') || 'Please enter a valid video URL');
                return;
            }
            showToast(t('toastCasting') || 'Sending video to TV screen...');
            fetch('/api/media/control', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ action: 'play', url: url })
            })
            .then(r => r.json())
            .then(() => {
                showToast(t('toastCastSuccess') || '✓ Video stream casted to TV!');
                pollMediaStatus();
            })
            .catch(() => showToast('Cast URL command transmitted'));
        }

        // Stream Pause / Resume
        function togglePauseStream() {
            isPaused = !isPaused;
            const endpoint = isPaused ? '/api/stream/stop' : '/api/stream/start';
            fetch(endpoint, { method: 'POST' })
                .then(r => r.json())
                .then(() => {
                    const btn = document.getElementById('btnPause');
                    btn.textContent = isPaused ? t('btnResumeStream') : t('btnPauseStream');
                    document.getElementById('valState').textContent = isPaused ? t('statStreamPaused').toUpperCase() : 'ONLINE';
                    document.getElementById('badgeStream').textContent = isPaused ? t('statStreamPaused') : t('statStreamActive');
                    document.getElementById('badgeStream').className = isPaused ? 'stat-badge badge-red' : 'stat-badge badge-green';
                    showToast(isPaused ? 'Display paused' : 'Display resumed');
                });
        }

        // Reboot Modal
        function confirmReboot() {
            document.getElementById('rebootModal').classList.add('active');
        }

        function closeRebootModal() {
            document.getElementById('rebootModal').classList.remove('active');
        }

        function executeReboot() {
            closeRebootModal();
            showToast('Rebooting Raspberry Pi Zero...');
            fetch('/api/system/reboot', { method: 'POST' })
                .then(() => {
                    document.getElementById('valState').textContent = 'REBOOTING';
                    document.getElementById('badgeStream').className = 'stat-badge badge-red';
                    setTimeout(() => location.reload(), 4000);
                });
        }

        // SD Card Mount / Unmount
        function mountSdCard(mount) {
            const endpoint = mount ? '/api/sdcard/mount' : '/api/sdcard/unmount';
            fetch(endpoint, { method: 'POST' })
                .then(r => r.json())
                .then(res => {
                    const statusEl = document.getElementById('sdMountStatus');
                    if (mount && res.status === 'mounted') {
                        statusEl.textContent = 'Mounted at /mnt/boot (Ready for Firmware Upgrade)';
                        statusEl.style.color = '#00e5ff';
                        showToast('✓ Micro-SD mounted at /mnt/boot');
                    } else {
                        statusEl.textContent = 'Unmounted (Safe / Decoupled 100% RAM)';
                        statusEl.style.color = '#7ee787';
                        showToast('✓ Micro-SD safely unmounted');
                    }
                });
        }

        // Copy Helper (HTTP-Safe Clipboard with textarea execCommand fallback)
        function copyCommand(id) {
            const el = document.getElementById(id);
            if (!el) return;
            const text = el.textContent.trim();
            const btn = el.parentElement ? el.parentElement.querySelector('.copy-btn') : null;

            const markCopied = () => {
                if (btn) {
                    const oldText = btn.textContent;
                    btn.textContent = '✓ ' + (t('copied') || 'Copied!');
                    btn.classList.add('copied');
                    setTimeout(() => {
                        btn.textContent = oldText;
                        btn.classList.remove('copied');
                    }, 2000);
                }
                showToast(t('copiedSuccess') || '✓ Copied to clipboard!');
            };

            const fallbackCopy = (str) => {
                try {
                    const ta = document.createElement('textarea');
                    ta.value = str;
                    ta.setAttribute('readonly', '');
                    ta.style.position = 'fixed';
                    ta.style.top = '-9999px';
                    ta.style.left = '-9999px';
                    document.body.appendChild(ta);
                    ta.focus();
                    ta.select();
                    const success = document.execCommand('copy');
                    document.body.removeChild(ta);
                    if (success) {
                        markCopied();
                    } else {
                        showToast('Failed to copy');
                    }
                } catch (err) {
                    console.error('execCommand copy failed:', err);
                    showToast('Failed to copy');
                }
            };

            if (navigator.clipboard && window.isSecureContext) {
                navigator.clipboard.writeText(text).then(markCopied).catch(() => fallbackCopy(text));
            } else {
                fallbackCopy(text);
            }
        }

        // Toast Helper
        function showToast(msg) {
            const t = document.getElementById('toast');
            t.textContent = msg;
            t.classList.add('show');
            setTimeout(() => t.classList.remove('show'), 2500);
        }

        let isAudioMuted = false;
        function updateAudioVolume(val) {
            document.getElementById('valAudioVolume').textContent = val + '%';
            fetch('/api/audio/volume', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ volume: parseInt(val, 10) })
            }).then(r => r.json()).then(st => {
                showToast('HDMI Volume: ' + st.volume + '%');
            }).catch(() => {});
        }

        function toggleAudioMute() {
            isAudioMuted = !isAudioMuted;
            fetch('/api/audio/mute', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ muted: isAudioMuted })
            }).then(r => r.json()).then(st => {
                const btn = document.getElementById('btnAudioMute');
                if (btn) {
                    btn.textContent = st.muted ? '🔇 ' + t('unmute') : '🔊 ' + t('mute');
                    btn.className = st.muted ? 'btn-danger' : 'btn-primary';
                }
                showToast(st.muted ? t('toastMuted') : t('toastUnmuted'));
            }).catch(() => {});
        }

        let currentAudioRate = 96000;

        function setAudioRate(rate) {
            currentAudioRate = rate;
            localStorage.setItem('ext_audio_rate', rate);
            updateAudioRateUI(rate);

            // 1. Reconfigure Receiver ALSA Hardware directly
            fetch('/api/audio/rate', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ rate: rate })
            })
            .then(r => r.json())
            .then(() => {
                showToast(`✓ Receiver ALSA clock reconfigured: ${rate} Hz`);
            })
            .catch(() => showToast(`Receiver rate set to ${rate} Hz`));

            // 2. Transmit rate change to Host Sender via UDP control channel
            sendHostControl({ audio_rate: rate });
        }

        function updateAudioRateUI(rate) {
            const lbl = document.getElementById('valAudioRate');
            let desc = `${rate} Hz`;
            if (rate === 96000) desc = '96 kHz (Hi-Res Studio - Default)';
            else if (rate === 192000) desc = '192 kHz (Ultra Hi-Res)';
            else if (rate === 48000) desc = '48 kHz (Cinema Standard)';
            else if (rate === 44100) desc = '44.1 kHz (CD Fidelity)';
            if (lbl) lbl.textContent = desc;

            document.querySelectorAll('#audioRateGrid .btn-toggle').forEach(b => {
                b.classList.toggle('active', parseInt(b.getAttribute('data-rate'), 10) === rate);
            });
        }

        let currentAudioTransport = 'network_udp';

        function setAudioTransport(transport) {
            currentAudioTransport = transport;
            localStorage.setItem('ext_audio_transport', transport);
            updateAudioTransportUI(transport);

            fetch('/api/audio/transport', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ transport: transport })
            })
            .then(r => r.json())
            .then(st => {
                showToast(`✓ Audio transport switched to: ${formatTransportName(transport)}`);
                if (st.transport) updateAudioTransportUI(st.transport);
            })
            .catch(() => showToast(`Audio transport set to ${transport}`));
        }

        function formatTransportName(transport) {
            if (transport === 'uac2_gadget') return 'Mode 2: USB Audio Class (UAC2)';
            if (transport === 'usb_bulk_mux') return 'Mode 3: USB Bulk Mux (Offline)';
            return 'Mode 1: UDP Network (Port 5004)';
        }

        function updateAudioTransportUI(transport) {
            const btnUdp = document.getElementById('btnAudioTransUdp');
            const btnUac2 = document.getElementById('btnAudioTransUac2');
            const btnBulk = document.getElementById('btnAudioTransBulk');
            const valLabel = document.getElementById('valAudioTransport');
            const desc = document.getElementById('descAudioTransport');

            if (btnUdp) btnUdp.classList.toggle('active', transport === 'network_udp');
            if (btnUac2) btnUac2.classList.toggle('active', transport === 'uac2_gadget');
            if (btnBulk) btnBulk.classList.toggle('active', transport === 'usb_bulk_mux');

            if (valLabel) {
                if (transport === 'uac2_gadget') {
                    valLabel.textContent = 'Mode 2: USB Audio Class (UAC2 Gadget)';
                } else if (transport === 'usb_bulk_mux') {
                    valLabel.textContent = 'Mode 3: USB Bulk Mux (Offline)';
                } else {
                    valLabel.textContent = 'Mode 1: UDP Stream (Port 5004 - Active)';
                }
            }

            if (desc) {
                if (transport === 'uac2_gadget') {
                    desc.textContent = t('audioTransUac2Desc') || 'Mode 2 UAC2 Gadget: Pi Zero acts as a native USB Sound Card on PC. Zero network dependency, plug-and-play in Windows and Linux.';
                } else if (transport === 'usb_bulk_mux') {
                    desc.textContent = t('audioTransBulkDesc') || 'Mode 3 USB Bulk: Audio is multiplexed into the /dev/usb-display-bulk pipe alongside H.264 video. Sub-1ms latency.';
                } else {
                    desc.textContent = t('audioTransUdpDesc') || 'Mode 1 UDP Network: Audio stream arrives via UDP port 5004 in 1024-byte unfragmented packets. True sub-5ms delay with automatic A/V synchronization.';
                }
            }
        }

        // Highlight Active Streaming Card
        function highlightActiveCard(activeId) {
            ['cardMode1', 'cardMode2', 'cardMode3'].forEach(id => {
                const card = document.getElementById(id);
                if (!card) return;
                const badge = card.querySelector('.stat-badge');
                if (id === activeId) {
                    card.style.transform = 'translateY(-2px)';
                    card.style.boxShadow = '0 0 20px rgba(0, 229, 255, 0.4)';
                    if (badge) {
                        badge.textContent = t('badgeStreaming');
                        badge.className = 'stat-badge badge-green';
                    }
                } else {
                    card.style.transform = 'none';
                    card.style.boxShadow = 'none';
                    if (badge) {
                        if (id === 'cardMode1') { badge.textContent = t('badgeMode1On'); badge.className = 'stat-badge badge-cyan'; }
                        if (id === 'cardMode2') { badge.textContent = t('badgeMode2On'); badge.className = 'stat-badge badge-green'; }
                        if (id === 'cardMode3') { badge.textContent = t('badgeMode3On'); badge.className = 'stat-badge badge-purple'; }
                    }
                }
            });
        }

        // Live Real-Time Hardware Audio Visualizer Engine (30 FPS Canvas)
        let liveAudioBars = new Array(24).fill(0.0);
        let liveAudioPeaks = new Array(24).fill(0.0);
        let liveRmsDb = -60.0;
        let liveAudioActive = false;
        let liveVideoActive = false;
        let smoothBars = new Array(24).fill(0.0);
        let smoothRms = -60.0;

        function initAudioVisualizerCanvas() {
            const canvas = document.getElementById('audioVisualizerCanvas');
            if (!canvas) return;
            const ctx = canvas.getContext('2d');
            if (!ctx) return;

            let lastDraw = 0;
            function renderLoop(now) {
                requestAnimationFrame(renderLoop);
                if (now - lastDraw < 33) return; // ~30 FPS
                lastDraw = now;

                const w = canvas.width;
                const h = canvas.height;
                ctx.clearRect(0, 0, w, h);

                // Background gradient
                const bgGrad = ctx.createLinearGradient(0, 0, 0, h);
                bgGrad.addColorStop(0, 'rgba(10, 16, 28, 0.95)');
                bgGrad.addColorStop(1, 'rgba(5, 8, 16, 0.98)');
                ctx.fillStyle = bgGrad;
                ctx.fillRect(0, 0, w, h);

                const numBars = 24;
                const vuWidth = 64;
                const specWidth = w - vuWidth - 30;
                const barSpacing = 4;
                const barWidth = Math.max(3, Math.floor((specWidth - (numBars - 1) * barSpacing) / numBars));
                const startX = 16;
                const baselineY = h - 22;

                // Draw 24 Realtime Spectrum Bars with smooth 60 FPS physics
                for (let i = 0; i < numBars; i++) {
                    const x = startX + i * (barWidth + barSpacing);
                    let targetVal = liveAudioActive ? (liveAudioBars[i] || 0.0) : 0.0;
                    smoothBars[i] += (targetVal - smoothBars[i]) * 0.45;
                    let val = Math.max(0.0, Math.min(1.0, smoothBars[i]));
                    if (val < 0.008) val = 0.0;
                    const barH = Math.floor(val * (baselineY - 14));
                    const y = baselineY - barH;

                    if (barH > 0) {
                        // Neon gradient: Purple -> Cyan -> Emerald
                        const barGrad = ctx.createLinearGradient(0, baselineY, 0, 10);
                        barGrad.addColorStop(0, '#7c4dff');
                        barGrad.addColorStop(0.5, '#00e5ff');
                        barGrad.addColorStop(1.0, '#7ee787');

                        ctx.fillStyle = barGrad;
                        if (ctx.roundRect) {
                            ctx.beginPath();
                            ctx.roundRect(x, y, barWidth, barH, [3, 3, 0, 0]);
                            ctx.fill();
                        } else {
                            ctx.fillRect(x, y, barWidth, barH);
                        }

                        // Peak drop marker
                        if (liveAudioActive) {
                            let targetPeak = liveAudioPeaks[i] || targetVal;
                            const peakY = Math.max(8, baselineY - Math.floor(targetPeak * (baselineY - 14)));
                            ctx.fillStyle = '#ffffff';
                            ctx.fillRect(x, peakY - 2, barWidth, 2);
                        }
                    } else {
                        // Minimal baseline dot during silence
                        ctx.fillStyle = 'rgba(255, 255, 255, 0.05)';
                        ctx.fillRect(x, baselineY - 1, barWidth, 1);
                    }
                }

                // Frequency Axis Labels
                ctx.fillStyle = '#64748b';
                ctx.font = '10px monospace';
                ctx.fillText('20Hz', startX, h - 7);
                ctx.fillText('500Hz', startX + specWidth * 0.35, h - 7);
                ctx.fillText('2.5kHz', startX + specWidth * 0.65, h - 7);
                ctx.fillText('20kHz', startX + specWidth - 32, h - 7);

                // Stereo VU Meter (Right Side)
                const vuX = w - vuWidth + 8;
                const vuHeight = baselineY - 10;
                const vuY = 10;
                const chWidth = 18;

                // Channel backgrounds
                ctx.fillStyle = 'rgba(255, 255, 255, 0.06)';
                ctx.fillRect(vuX, vuY, chWidth, vuHeight);
                ctx.fillRect(vuX + chWidth + 6, vuY, chWidth, vuHeight);

                // Fill level normalized from RMS dB (-60 to 0) with smooth damping
                if (liveAudioActive) {
                    smoothRms += (liveRmsDb - smoothRms) * 0.35;
                    const normDb = Math.max(0.0, Math.min(1.0, (smoothRms + 60.0) / 60.0));
                    const fillH = Math.floor(normDb * vuHeight);

                    if (fillH > 0) {
                        const vuGrad = ctx.createLinearGradient(0, vuY + vuHeight, 0, vuY);
                        vuGrad.addColorStop(0, '#7ee787');
                        vuGrad.addColorStop(0.7, '#ffeb3b');
                        vuGrad.addColorStop(1.0, '#f85149');

                        ctx.fillStyle = vuGrad;
                        ctx.fillRect(vuX + 1, vuY + vuHeight - fillH, chWidth - 2, fillH);
                        ctx.fillRect(vuX + chWidth + 7, vuY + vuHeight - Math.floor(fillH * 0.97), chWidth - 2, Math.floor(fillH * 0.97));
                    }
                } else {
                    smoothRms = -60.0;
                }

                // Channel labels
                ctx.fillStyle = '#94a3b8';
                ctx.font = '9px monospace';
                ctx.fillText('L', vuX + 5, h - 7);
                ctx.fillText('R', vuX + chWidth + 11, h - 7);

                // Telemetry DSP Status Badge
                ctx.fillStyle = liveAudioActive ? '#7ee787' : '#64748b';
                ctx.font = '10px monospace';
                const statusTxt = liveAudioActive ? `${liveRmsDb.toFixed(1)} dB RMS • ALSA Hardware PCM` : 'Standby / Silent (Zero Telemetry)';
                ctx.fillText(statusTxt, startX + 2, 16);
            }
            requestAnimationFrame(renderLoop);
        }

        // Telemetry Poller
        function pollTelemetry() {
            fetch('/api/status')
                .then(r => r.json())
                .then(data => {
                    if (data.temp) document.getElementById('valTemp').textContent = data.temp + '°C';
                    if (data.cpu) document.getElementById('valCpu').textContent = data.cpu;
                    if (data.ram) document.getElementById('valRam').textContent = data.ram + ' MB';
                    if (data.audio) {
                        const a = data.audio;
                        const elVol = document.getElementById('valAudioVolume');
                        const slider = document.getElementById('audioVolumeSlider');
                        if (elVol && slider && !slider.matches(':active')) {
                            elVol.textContent = a.volume + '%';
                            slider.value = a.volume;
                        }
                        const btn = document.getElementById('btnAudioMute');
                        if (btn) {
                            btn.textContent = a.muted ? '🔇 ' + t('unmute') : '🔊 ' + t('mute');
                            btn.className = a.muted ? 'btn-danger' : 'btn-primary';
                        }
                        if (a.rate && typeof currentAudioRate !== 'undefined' && a.rate !== currentAudioRate) {
                            currentAudioRate = a.rate;
                            updateAudioRateUI(a.rate);
                        }
                        if (a.transport && typeof currentAudioTransport !== 'undefined' && a.transport !== currentAudioTransport) {
                            currentAudioTransport = a.transport;
                            updateAudioTransportUI(a.transport);
                        }
                    }

                    // 0. Hardware Arbiter & 4-Level Service Hierarchy Feedback
                    if (data.hierarchy) {
                        const h = data.hierarchy;
                        const badgeH = document.getElementById('badgeHierarchy');
                        const descH = document.getElementById('descHierarchy');
                        const badgeOwner = document.getElementById('badgeDisplayOwner');
                        if (badgeH) {
                            if (h.level === 0) {
                                badgeH.textContent = '● NÍVEL 0: DESKTOP (EXCLUSIVO)';
                                badgeH.className = 'stat-badge badge-green';
                            } else if (h.level === 1) {
                                badgeH.textContent = '● NÍVEL 1: STREAMING MÍDIA';
                                badgeH.className = 'stat-badge badge-cyan';
                            } else if (h.level === 2) {
                                badgeH.textContent = '● NÍVEL 2: ÁUDIO STANDALONE';
                                badgeH.className = 'stat-badge badge-purple';
                            } else {
                                badgeH.textContent = '● NÍVEL 3: MODO STANDBY';
                                badgeH.className = 'stat-badge badge-amber';
                            }
                        }
                        if (descH) {
                            descH.textContent = h.level_name;
                        }
                        if (badgeOwner) {
                            badgeOwner.textContent = h.display_owner;
                        }
                    }

                    // 1. Active Mode Visual Feedback
                    if (data.active_mode) {
                        const am = data.active_mode;
                        const elTitle = document.getElementById('activeModeTitle');
                        const elDesc = document.getElementById('activeModeDesc');
                        const elBadge = document.getElementById('activeModeBadge');
                        const elIcon = document.getElementById('activeModeIcon');
                        const banner = document.getElementById('activeStreamBanner');

                        if (elIcon) elIcon.textContent = am.icon || '📺';

                        if (am.id === 'mode1_udp') {
                            if (elTitle) elTitle.textContent = t('m1Title') || am.name;
                            if (elDesc) elDesc.textContent = t('m1Details') || am.details;
                            if (elBadge) { elBadge.textContent = '● ' + t('tbM1Method') + ' (60 FPS)'; elBadge.className = 'stat-badge badge-cyan'; }
                            if (banner) { banner.style.borderColor = 'var(--accent-cyan)'; banner.style.boxShadow = '0 0 25px rgba(0, 229, 255, 0.25)'; }
                            highlightActiveCard('cardMode1');
                        } else if (am.id === 'mode2_miracast') {
                            if (elTitle) elTitle.textContent = t('m2Title') || am.name;
                            if (elDesc) elDesc.textContent = t('m2Details') || am.details;
                            if (elBadge) { elBadge.textContent = '● ' + t('tbM2Method') + ' (60 FPS)'; elBadge.className = 'stat-badge badge-green'; }
                            if (banner) { banner.style.borderColor = 'var(--accent-emerald)'; banner.style.boxShadow = '0 0 25px rgba(0, 255, 102, 0.25)'; }
                            highlightActiveCard('cardMode2');
                        } else if (am.id === 'mode3_usb_bulk') {
                            if (elTitle) elTitle.textContent = t('m3Title') || am.name;
                            if (elDesc) elDesc.textContent = t('m3Details') || am.details;
                            if (elBadge) { elBadge.textContent = '● ' + t('tbM3Method') + ' (480 Mbps)'; elBadge.className = 'stat-badge badge-purple'; }
                            if (banner) { banner.style.borderColor = 'var(--accent-purple)'; banner.style.boxShadow = '0 0 25px rgba(179, 136, 255, 0.25)'; }
                            highlightActiveCard('cardMode3');
                        } else {
                            if (elTitle) elTitle.textContent = t('waitingStream');
                            if (elDesc) elDesc.textContent = t('waitingStreamDesc');
                            if (elBadge) { elBadge.textContent = '⏳ ' + t('waitingStream'); elBadge.className = 'stat-badge badge-amber'; }
                            if (banner) { banner.style.borderColor = 'rgba(255, 179, 0, 0.4)'; banner.style.boxShadow = 'none'; }
                            highlightActiveCard(null);
                        }

                        // Update Active Transport Connection buttons across both tabs
                        const activeTrans = data.active_transport || am.id;
                        if (data.active_transport) {
                            currentTransport = data.active_transport;
                        } else if (am.id && am.id !== 'idle' && am.id !== 'standby') {
                            currentTransport = am.id;
                        }

                        const isPaused = data.stream_state === 'paused' || am.id === 'standby';
                        if (isPaused) {
                            currentTopology = 'stop';
                        }
                        updateModeAndTopologyButtons();

                        // Also update statStream card
                        const valState = document.getElementById('valState');
                        const badgeStream = document.getElementById('badgeStream');
                        if (valState) {
                            valState.textContent = am.id === 'mode1_udp' ? 'UDP' : (am.id === 'mode2_miracast' ? 'MIRACAST' : (am.id === 'mode3_usb_bulk' ? 'USB BULK' : 'STANDBY'));
                        }
                        if (badgeStream) {
                            badgeStream.textContent = am.id !== 'idle' && !isPaused ? t('statStreamActive') : t('statStreamPaused');
                            badgeStream.className = am.id !== 'idle' && !isPaused ? 'stat-badge badge-green' : 'stat-badge badge-amber';
                        }
                    }

                    // 2. Dynamic HDMI Displays Visual Feedback
                    const displaysList = data.displays || (data.hdmi ? [data.hdmi] : []);
                    renderDisplays(displaysList);

                    const h = displaysList.find(d => d.connected) || displaysList[0];
                    if (h) {
                        const elPort = document.getElementById('activeHdmiPortBadge');
                        const elHdmiTitle = document.getElementById('activeHdmiTitle');
                        const elHdmiDetails = document.getElementById('activeHdmiDetails');

                        if (elPort) elPort.textContent = h.connector || 'HDMI-A-1';
                        if (elHdmiTitle) elHdmiTitle.textContent = h.connector_friendly || ('HDMI (' + (h.connector || 'HDMI-A-1') + ')');
                        if (elHdmiDetails) {
                            const hw = h.hardware_model || 'Raspberry Pi';
                            const mon = h.name || 'HDMI Monitor';
                            const mode = h.active_mode || '1280x720 @ 60 Hz';
                            elHdmiDetails.textContent = mon + ' • ' + mode + ' • ' + hw;
                        }

                        const badgeAudioCap = document.getElementById('badgeHdmiAudioCapability');
                        if (badgeAudioCap) {
                            const hasAudio = h.has_audio !== false;
                            if (hasAudio) {
                                badgeAudioCap.textContent = '✓ Saída ' + (h.connector || 'HDMI') + ' com suporte a Áudio Digital';
                                badgeAudioCap.className = 'stat-badge badge-green';
                            } else {
                                badgeAudioCap.textContent = '⚠️ Conector ' + (h.connector || 'Vídeo') + ' sem Áudio Integrado (Vídeo Puro)';
                                badgeAudioCap.className = 'stat-badge badge-amber';
                            }
                        }
                    }
                })
                .catch(() => {});

            pollMediaStatus();
        }

        function renderDisplays(displays) {
            const container = document.getElementById('displaysSectionContainer');
            if (!container || !displays || displays.length === 0) return;

            // Only redraw if number of displays changed or data changed to avoid flicker
            const cacheKey = JSON.stringify(displays.map(d => ({ c: d.connector, st: d.connected, m: d.active_mode, n: d.name })));
            if (container.dataset.cacheKey === cacheKey) return;
            container.dataset.cacheKey = cacheKey;

            let html = '';
            displays.forEach((h, idx) => {
                const conn = h.connector || ('HDMI-' + (idx + 1));
                const isConn = !!h.connected;
                const title = h.connector_friendly || ('Digital Output (' + conn + ')');
                const monName = isConn ? (h.name || 'HDMI Television / Monitor') : ('Standby / ' + t('netDisconnected') + ' (' + conn + ')');
                const statusText = isConn ? ('● LIVE ZERO-COPY 60 FPS (' + conn + ')') : ('● ' + t('netDisconnected').toUpperCase() + ' (HEADLESS GUARD)');
                const statusColor = isConn ? '#7ee787' : '#f85149';
                const badgeText = conn + ' • ' + (h.active_mode || h.preferred_mode || '1280x720');
                const badgeClass = isConn ? 'stat-badge badge-cyan' : 'stat-badge badge-amber';
                const vpu = h.vpu || 'Hardware Accelerated VPU';

                html += `
                <div class="glass-card" id="displayCard_${conn}" style="margin-bottom: 1.25rem;">
                    <div class="card-header">
                        <div class="card-title">
                            <span>📺</span>
                            <span>${title}</span>
                        </div>
                        <span class="${badgeClass}">${badgeText}</span>
                    </div>
                    <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 1.5rem; align-items: center;">
                        <div class="monitor-frame">
                            <div class="monitor-scanline"></div>
                            <div class="monitor-text">
                                <p style="font-size: 1.3rem; font-weight: 700;">🖥️ ${monName}</p>
                                <p style="margin-top: 0.3rem; font-size: 0.85rem; opacity: 0.85;">${vpu}</p>
                                <p style="color: ${statusColor}; margin-top: 0.2rem; font-weight: 600; font-size: 0.85rem;">${statusText}</p>
                            </div>
                        </div>
                        <div>
                            <p style="color: var(--text-secondary); font-size: 0.88rem; line-height: 1.5; margin-bottom: 1rem;">
                                ${isConn ? (h.hardware_model + ' • ' + (h.active_mode || '1280x720 @ 60 Hz') + ' • VideoCore IV scanout') : 'DRM connector active in headless guard mode with zero CPU overhead.'}
                            </p>
                            <div class="action-row">
                                <button class="btn-primary" onclick="triggerHud(true)">✦ Show HUD on TV (60s)</button>
                                <button class="btn-danger" onclick="triggerHud(false)">✕ Turn Off HUD</button>
                            </div>
                        </div>
                    </div>
                </div>`;
            });
            container.innerHTML = html;
        }

        function pollMediaStatus() {
            fetch('/api/media/status')
                .then(r => r.json())
                .then(m => {
                    const elTitle = document.getElementById('mediaTitle');
                    const elArtist = document.getElementById('mediaArtist');
                    const elAlbum = document.getElementById('mediaAlbum');
                    const elState = document.getElementById('valMediaState');
                    const elVis = document.getElementById('valVisualizerState');
                    const btnOn = document.getElementById('btnVisOn');
                    const btnOff = document.getElementById('btnVisOff');

                    if (elTitle && m.title) elTitle.textContent = m.title;
                    if (elArtist && m.artist) elArtist.textContent = m.artist;
                    if (elAlbum && m.album) elAlbum.textContent = m.album;
                    if (elState) {
                        elState.textContent = m.state === 'playing' ? '▶ ' + t('mediaPlaying') : (m.state === 'paused' ? '⏸ ' + t('mediaPaused') : t('mediaIdle'));
                        elState.style.color = m.state === 'playing' ? '#7ee787' : '#b388ff';
                    }
                    if (elVis) {
                        elVis.textContent = m.visualizer_enabled ? t('visActive') : t('visDisabled');
                    }
                    if (btnOn && btnOff) {
                        btnOn.classList.toggle('active', m.visualizer_enabled);
                        btnOff.classList.toggle('active', !m.visualizer_enabled);
                    }

                    // Update HDMI multiplexer status badge
                    const badgeHdmiMux = document.getElementById('badgeHdmiMuxMode');
                    if (badgeHdmiMux) {
                        if (m.video_active) {
                            badgeHdmiMux.textContent = t('badgeHdmiMuxPc');
                            badgeHdmiMux.className = 'card-badge badge-blue';
                        } else if (m.audio_active) {
                            badgeHdmiMux.textContent = t('badgeHdmiMuxIot');
                            badgeHdmiMux.className = 'card-badge badge-purple';
                        } else {
                            badgeHdmiMux.textContent = t('badgeHdmiMuxStandby');
                            badgeHdmiMux.className = 'card-badge badge-green';
                        }
                    }

                    // Update live audio telemetry for 30 FPS canvas visualizer
                    if (m.bars && m.bars.length === 24) {
                        liveAudioActive = !!m.audio_active;
                        liveVideoActive = !!m.video_active;
                        if (liveAudioActive) {
                            liveAudioBars = m.bars;
                            liveAudioPeaks = m.peaks || m.bars;
                            liveRmsDb = (typeof m.rms_db === 'number') ? m.rms_db : -60.0;
                        } else {
                            liveAudioBars = new Array(24).fill(0.0);
                            liveAudioPeaks = new Array(24).fill(0.0);
                            liveRmsDb = -60.0;
                        }
                    }
                })
                .catch(() => {});
        }

        let currentNetMode = 'static';

        function setNetModeUI(mode) {
            currentNetMode = mode;
            const btnStatic = document.getElementById('btnNetStatic');
            const btnDhcp = document.getElementById('btnNetDhcp');
            const staticFields = document.getElementById('netStaticFields');
            const valMode = document.getElementById('valNetCurrentMode');

            if (btnStatic) btnStatic.classList.toggle('active', mode === 'static');
            if (btnDhcp) btnDhcp.classList.toggle('active', mode === 'dhcp');
            if (staticFields) staticFields.style.display = mode === 'static' ? 'grid' : 'none';
            if (valMode) {
                valMode.textContent = mode === 'static' ? (t('btnNetStatic') + ' (' + (document.getElementById('netInputIp')?.value || '192.168.1.50') + ')') : t('btnNetDhcp');
            }
        }

        function saveAndApplyNetworkConfig() {
            const iface = document.getElementById('netSelectIface')?.value || 'eth0';
            const ip = document.getElementById('netInputIp')?.value.trim() || '192.168.1.50';
            const netmask = document.getElementById('netInputMask')?.value.trim() || '255.255.255.0';
            const gateway = document.getElementById('netInputGw')?.value.trim() || '192.168.1.1';
            const dns = document.getElementById('netInputDns')?.value.trim() || '1.1.1.1, 8.8.8.8';
            const feedback = document.getElementById('netSaveFeedback');

            if (feedback) {
                feedback.textContent = '⏳ Applying network configuration...';
                feedback.style.color = '#e3b341';
            }

            fetch('/api/network', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    interface: iface,
                    mode: currentNetMode,
                    ip: ip,
                    netmask: netmask,
                    gateway: gateway,
                    dns: dns
                })
            })
            .then(r => r.json())
            .then(res => {
                if (feedback) {
                    feedback.textContent = '✓ Configuration saved to /boot/network.conf and applied!';
                    feedback.style.color = '#7ee787';
                    setTimeout(() => { if (feedback) feedback.textContent = ''; }, 6000);
                }
                pollNetworkStatus();
            })
            .catch(err => {
                if (feedback) {
                    feedback.textContent = '❌ Error applying configuration: ' + err;
                    feedback.style.color = '#f85149';
                }
            });
        }

        let netConfigLoaded = false;
        function pollNetworkStatus() {
            fetch('/api/network')
                .then(r => r.json())
                .then(data => {
                    const elUsb0 = document.getElementById('netUsb0Ip');
                    const elEth0 = document.getElementById('netEth0Ip');
                    const elWlan0 = document.getElementById('netWlan0Ip');
                    const elEth0St = document.getElementById('netEth0Status');

                    if (elUsb0 && data.usb0) {
                        elUsb0.textContent = data.usb0.ipv4 || '192.168.7.2';
                    }
                    if (elEth0 && data.eth0) {
                        elEth0.textContent = (data.eth0.ipv4 && data.eth0.ipv4 !== 'disconnected') ? data.eth0.ipv4 : (data.config?.ip || '192.168.1.50');
                        if (elEth0St) {
                            elEth0St.textContent = data.eth0.detected ? ('● ' + t('netConnected') + ' (' + (data.config?.mode === 'dhcp' ? 'DHCP' : 'Static') + ')') : ('● ' + t('cableDisconnected'));
                            elEth0St.style.color = data.eth0.detected ? '#7ee787' : '#8b949e';
                        }
                    }
                    if (elWlan0 && data.wlan0) {
                        elWlan0.textContent = (data.wlan0.ipv4 && data.wlan0.ipv4 !== 'disconnected') ? data.wlan0.ipv4 : t('netDisconnected');
                    }

                    if (data.config && !netConfigLoaded) {
                        netConfigLoaded = true;
                        const elIp = document.getElementById('netInputIp');
                        const elMask = document.getElementById('netInputMask');
                        const elGw = document.getElementById('netInputGw');
                        const elDns = document.getElementById('netInputDns');
                        const selIface = document.getElementById('netSelectIface');

                        if (elIp && data.config.ip) elIp.value = data.config.ip;
                        if (elMask && data.config.netmask) elMask.value = data.config.netmask;
                        if (elGw && data.config.gateway) elGw.value = data.config.gateway;
                        if (elDns && data.config.dns) elDns.value = data.config.dns;
                        if (selIface && data.config.interface) selIface.value = data.config.interface;

                        if (data.config.mode) {
                            setNetModeUI(data.config.mode);
                        }
                    }
                })
                .catch(() => {});
        }

        // Init: Restore from localStorage first (for instant snappy UI on F5), then sync with server
        const savedLang = localStorage.getItem('ext_monitor_lang') || 'en';
        setLanguage(savedLang);

        const savedColor = localStorage.getItem('ext_color');
        if (savedColor) setColor(savedColor);

        const savedFps = localStorage.getItem('ext_fps');
        if (savedFps) setFps(parseInt(savedFps, 10));

        const savedBitrate = localStorage.getItem('ext_bitrate');
        if (savedBitrate) setBitrate(parseInt(savedBitrate, 10));

        const savedRate = localStorage.getItem('ext_audio_rate');
        if (savedRate) updateAudioRateUI(parseInt(savedRate, 10));

        const savedTransport = localStorage.getItem('ext_audio_transport');
        if (savedTransport) updateAudioTransportUI(savedTransport);

        const savedDropOnly = localStorage.getItem('ext_drop_only');
        if (savedDropOnly !== null) setDropOnly(savedDropOnly === 'true');

        const savedSkipToFirst = localStorage.getItem('ext_skip_to_first');
        if (savedSkipToFirst !== null) setSkipToFirst(savedSkipToFirst === 'true');

        const savedKeyInt = localStorage.getItem('ext_key_int_max');
        if (savedKeyInt) setKeyInt(parseInt(savedKeyInt, 10));

        const savedM1 = localStorage.getItem('ext_mode1');
        if (savedM1 !== null) setModeToggleUI('mode1', savedM1 === 'true');

        const savedM2 = localStorage.getItem('ext_mode2');
        if (savedM2 !== null) setModeToggleUI('mode2', savedM2 === 'true');

        const savedM3 = localStorage.getItem('ext_mode3');
        if (savedM3 !== null) setModeToggleUI('mode3', savedM3 === 'true');

        const savedScale = localStorage.getItem('ext_scale');
        if (savedScale) setScale(savedScale);

        const savedCas = localStorage.getItem('ext_cas');
        if (savedCas !== null) setCas(savedCas === 'true');

        const savedSilicon = localStorage.getItem('ext_silicon_scaler');
        if (savedSilicon !== null) {
            const isEnabled = savedSilicon === 'true';
            const chk = document.getElementById('chkSiliconScaler');
            if (chk) chk.checked = isEnabled;
            toggleSiliconScaler(isEnabled);
        }

        // Initialize 30 FPS Hardware Audio Spectrum Canvas
        initAudioVisualizerCanvas();

        // Fetch server state to sync if not set locally
        fetch('/api/config')
            .then(r => r.json())
            .then(cfg => {
                if (cfg.color && !savedColor) setColor(cfg.color);
                if (cfg.fps && !savedFps) setFps(cfg.fps);
                if (cfg.bitrate && !savedBitrate) setBitrate(cfg.bitrate);
                if (cfg.drop_only !== undefined && savedDropOnly === null) setDropOnly(cfg.drop_only);
                if (cfg.skip_to_first !== undefined && savedSkipToFirst === null) setSkipToFirst(cfg.skip_to_first);
                if (cfg.key_int_max !== undefined && !savedKeyInt) setKeyInt(cfg.key_int_max);
                if (cfg.mode1 !== undefined && savedM1 === null) setModeToggleUI('mode1', cfg.mode1);
                if (cfg.mode2 !== undefined && savedM2 === null) setModeToggleUI('mode2', cfg.mode2);
                if (cfg.mode3 !== undefined && savedM3 === null) setModeToggleUI('mode3', cfg.mode3);
            })
            .catch(() => {});

        const savedSimAudio = localStorage.getItem('ext_simultaneous_audio') === 'true';
        const toggleSim = document.getElementById('toggleSimultaneousAudio');
        if (toggleSim) toggleSim.checked = savedSimAudio;

        pollNetworkStatus();
        updateModeAndTopologyButtons();
        pollTelemetry();
        setInterval(pollTelemetry, 2000);
        setInterval(pollNetworkStatus, 3500);
        setInterval(pollMediaStatus, 250);
    </script>
</body>
</html>
"##;
