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
//! Author: Carlos Alberto <psncarlosalberto4ti@gmail.com>

pub const DASHBOARD_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
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
                <button type="button" onclick="setLanguage('en')" class="flag-btn" id="btnLang_en" title="English">🇺🇸 EN</button>
                <button type="button" onclick="setLanguage('pt')" class="flag-btn active" id="btnLang_pt" title="Português">🇧🇷 PT</button>
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
    </nav>

    <main class="container">
        <!-- ================================================================= -->
        <!-- TAB 1: MONITORAMENTO & TELEMETRIA                                 -->
        <!-- ================================================================= -->
        <section id="tab-monitor" class="tab-content active">
            <!-- Stat Cards Row -->
            <div class="stats-grid">
                <div class="stat-card">
                    <div class="stat-header">
                        <span class="stat-title" data-i18n="statTemp">SoC Temperature</span>
                        <span class="stat-badge badge-green" id="badgeTemp">Normal</span>
                    </div>
                    <div class="stat-value" id="valTemp">44.9°C</div>
                    <div class="stat-footer">Broadcom BCM2835 @ 1.0 GHz</div>
                </div>
                <div class="stat-card">
                    <div class="stat-header">
                        <span class="stat-title" data-i18n="statCpu">CPU Load</span>
                        <span class="stat-badge badge-cyan" data-i18n="badgeVpuOffload">VPU Offload</span>
                    </div>
                    <div class="stat-value" id="valCpu">0.67%</div>
                    <div class="stat-footer" data-i18n="statCpuDesc">Hardware GPU V4L2 M2M Active</div>
                </div>
                <div class="stat-card">
                    <div class="stat-header">
                        <span class="stat-title" data-i18n="statRam">Free RAM</span>
                        <span class="stat-badge badge-purple" data-i18n="badgeRamApp">100% RAM</span>
                    </div>
                    <div class="stat-value" id="valRam">318 MB</div>
                    <div class="stat-footer">512 MB SDRAM (Zero SD wear)</div>
                </div>
                <div class="stat-card">
                    <div class="stat-header">
                        <span class="stat-title" data-i18n="statStream">Stream State</span>
                        <span class="stat-badge badge-green" id="badgeStream">Active</span>
                    </div>
                    <div class="stat-value" id="valState">ONLINE</div>
                    <div class="stat-footer">1280x720 @ 60 FPS (HDMI)</div>
                </div>
            </div>

            <!-- Display Monitor & Quick Telemetry -->
            <div class="glass-card">
                <div class="card-header">
                    <div class="card-title">
                        <span>📺</span>
                        <span data-i18n="displayHeader">HDMI Television & Display Telemetry</span>
                    </div>
                    <span class="card-badge badge-cyan">HDMI-A-1 • 1280x720</span>
                </div>
                <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 1.5rem; align-items: center;">
                    <div class="monitor-frame">
                        <div class="monitor-scanline"></div>
                        <div class="monitor-text">
                            <p style="font-size: 1.4rem; font-weight: 700;">🖥️ SAMSUNG TV</p>
                            <p style="margin-top: 0.3rem;">VideoCore IV Hardware VPU</p>
                            <p style="color: #7ee787; margin-top: 0.2rem;">● LIVE ZERO-COPY 60 FPS</p>
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

            <!-- Streaming Mode Selector -->
            <div class="glass-card">
                <div class="card-header">
                    <div class="card-title">
                        <span>🔀</span>
                        <span data-i18n="modeHeader">Active Streaming Modes</span>
                    </div>
                    <span class="card-badge badge-purple" data-i18n="badgeMultiMode">Concurrent Engine</span>
                </div>
                <div class="btn-grid" style="grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));">
                    <div class="dl-card mode-card" id="cardMode1" style="border-color: var(--accent-cyan);">
                        <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.6rem;">
                            <div class="dl-title" style="margin: 0;">🐧 Mode 1: Linux Wayland</div>
                            <label class="switch" title="Ligar / Desligar Modo 1">
                                <input type="checkbox" id="toggleMode1" checked onchange="toggleMode('mode1', this.checked)">
                                <span class="toggle-slider"></span>
                            </label>
                        </div>
                        <div class="dl-desc" data-i18n="m1Desc">Direct low-latency RTP H.264 stream on UDP port 5000 with AMD VA-API zero-copy offload (&lt; 15ms).</div>
                        <span id="badgeMode1" class="stat-badge badge-cyan" style="align-self: flex-start;">Ligado (UDP 5000)</span>
                    </div>
                    <div class="dl-card mode-card" id="cardMode2" style="border-color: var(--accent-emerald);">
                        <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.6rem;">
                            <div class="dl-title" style="margin: 0;">🪟 Mode 2: Windows Miracast</div>
                            <label class="switch" title="Ligar / Desligar Modo 2">
                                <input type="checkbox" id="toggleMode2" checked onchange="toggleMode('mode2', this.checked)">
                                <span class="toggle-slider"></span>
                            </label>
                        </div>
                        <div class="dl-desc" data-i18n="m2Desc">Native Windows 10/11 wireless projection via Win + K on RTSP port 7236. Zero host drivers needed.</div>
                        <span id="badgeMode2" class="stat-badge badge-green" style="align-self: flex-start;">Ligado (TCP 7236)</span>
                    </div>
                    <div class="dl-card mode-card" id="cardMode3" style="border-color: var(--accent-purple);">
                        <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.6rem;">
                            <div class="dl-title" style="margin: 0;">⚡ Mode 3: USB Bulk Direct</div>
                            <label class="switch" title="Ligar / Desligar Modo 3">
                                <input type="checkbox" id="toggleMode3" checked onchange="toggleMode('mode3', this.checked)">
                                <span class="toggle-slider"></span>
                            </label>
                        </div>
                        <div class="dl-desc" data-i18n="m3Desc">Direct 480 Mbps raw hardware pipe via USB FunctionFS without network stack overhead (&lt; 1ms).</div>
                        <span id="badgeMode3" class="stat-badge badge-purple" style="align-self: flex-start;">Ligado (USB Bulk)</span>
                    </div>
                </div>
            </div>
        </section>

        <!-- ================================================================= -->
        <!-- TAB 2: CONFIGURAÇÕES & AJUSTES FINOS                              -->
        <!-- ================================================================= -->
        <section id="tab-config" class="tab-content">
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
                        <span data-i18n="bitrateLabel">Streaming Bitrate (VBR)</span>
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
                        <button class="btn-toggle" data-bitrate="800" onclick="setBitrate(800)">800k (Recomendado)</button>
                        <button class="btn-toggle" data-bitrate="1500" onclick="setBitrate(1500)">1500k (Balanceado)</button>
                        <button class="btn-toggle" data-bitrate="3000" onclick="setBitrate(3000)">3000k (HD)</button>
                        <button class="btn-toggle" data-bitrate="6000" onclick="setBitrate(6000)">6000k (Fluidez)</button>
                    </div>
                </div>

                <!-- Framerate (FPS) -->
                <div class="control-group">
                    <div class="control-label">
                        <span data-i18n="fpsLabel">Framerate (FPS)</span>
                        <span class="control-value" id="valFps">30 FPS</span>
                    </div>
                    <div class="btn-grid" id="fpsGrid">
                        <button class="btn-toggle" data-fps="15" onclick="setFps(15)">15 FPS (Ultra-Leve)</button>
                        <button class="btn-toggle active" data-fps="30" onclick="setFps(30)">30 FPS (Recomendado)</button>
                        <button class="btn-toggle" data-fps="60" onclick="setFps(60)">60 FPS (Máxima Fluidez)</button>
                    </div>
                </div>

                <!-- Color Profile -->
                <div class="control-group">
                    <div class="control-label">
                        <span data-i18n="colorLabel">Color Profile</span>
                        <span class="control-value" id="valColor">256-Color (Economy)</span>
                    </div>
                    <div class="btn-grid" id="colorGrid">
                        <button class="btn-toggle" data-color="full" onclick="setColor('full')" data-i18n="colorFull">24-bit TrueColor</button>
                        <button class="btn-toggle active" data-color="256" onclick="setColor('256')" data-i18n="color256">256 Cores (QP 30-44)</button>
                        <button class="btn-toggle" data-color="gray" onclick="setColor('gray')" data-i18n="colorGray">Monocromático</button>
                    </div>
                </div>

                <!-- Commit & Hardware Actions -->
                <div class="action-row" style="border-top: 1px solid rgba(255, 255, 255, 0.08); padding-top: 1.2rem;">
                    <button id="btnApply" class="btn-primary" onclick="applyConfiguration()" data-i18n="btnApply">💾 Aplicar Alterações</button>
                    <button id="btnPause" class="btn-secondary" onclick="togglePauseStream()" data-i18n="btnPauseStream">⏸ Pausar Exibição</button>
                    <button id="btnReboot" class="btn-danger" onclick="confirmReboot()" data-i18n="btnReboot">🔄 Reiniciar Appliance (Reboot)</button>
                </div>
            </div>
        </section>

        <!-- ================================================================= -->
        <!-- TAB 3: DOWNLOADS DE FERRAMENTAS & DRIVER                          -->
        <!-- ================================================================= -->
        <section id="tab-downloads" class="tab-content">
            <div class="glass-card">
                <div class="card-header">
                    <div class="card-title">
                        <span>⚡</span>
                        <span data-i18n="oneLinerTitle">One-Line Host Connector (1 Clique)</span>
                    </div>
                    <span class="card-badge badge-green" data-i18n="badgeInstant">Instantâneo</span>
                </div>
                <p style="color: var(--text-secondary); font-size: 0.9rem; line-height: 1.5; margin-bottom: 0.8rem;" data-i18n="oneLinerDesc">
                    Em qualquer novo PC com Linux, basta abrir o terminal e colar o comando abaixo para iniciar a segunda tela imediatamente:
                </p>
                <div class="cmd-box">
                    <span class="cmd-text" id="cmdOneLine">curl -sSL http://192.168.7.2:8080/connect.sh | bash</span>
                    <button class="copy-btn" onclick="copyCommand('cmdOneLine')" data-i18n="btnCopy">Copiar</button>
                </div>
            </div>

            <!-- Direct File Downloads Grid -->
            <div class="download-grid">
                <div class="dl-card">
                    <div>
                        <div class="dl-icon">📦</div>
                        <div class="dl-title" data-i18n="dlPkgTitle">Pacote Completo do Cliente</div>
                        <div class="dl-desc" data-i18n="dlPkgDesc">Contém o executável ext-sender compilado, script start.sh, regras udev e instalador em tar.gz.</div>
                    </div>
                    <a href="/download/client.tar.gz" class="dl-link" download data-i18n="btnDlPkg">Baixar client.tar.gz</a>
                </div>
                <div class="dl-card">
                    <div>
                        <div class="dl-icon">⚙️</div>
                        <div class="dl-title" data-i18n="dlSenderTitle">Executável ext-sender</div>
                        <div class="dl-desc" data-i18n="dlSenderDesc">Binário standalone do transmissor GPU offload compilado em Rust para Linux x86_64.</div>
                    </div>
                    <a href="/download/ext-sender" class="dl-link" download data-i18n="btnDlSender">Baixar ext-sender</a>
                </div>
                <div class="dl-card">
                    <div>
                        <div class="dl-icon">📜</div>
                        <div class="dl-title" data-i18n="dlScriptTitle">Script connect.sh</div>
                        <div class="dl-desc" data-i18n="dlScriptDesc">Script portátil que detecta a conexão USB, baixa os componentes necessários e inicia o streaming.</div>
                    </div>
                    <a href="/connect.sh" class="dl-link" download data-i18n="btnDlScript">Baixar connect.sh</a>
                </div>
                <div class="dl-card">
                    <div>
                        <div class="dl-icon">🛡️</div>
                        <div class="dl-title" data-i18n="dlUdevTitle">Regras udev (Plug-and-Play)</div>
                        <div class="dl-desc" data-i18n="dlUdevDesc">Configura automaticamente o buffer USB (txqueuelen 100) para eliminar buffer bloat ao plugar o cabo.</div>
                    </div>
                    <a href="/download/99-ext-monitor.rules" class="dl-link" download data-i18n="btnDlUdev">Baixar 99-ext-monitor.rules</a>
                </div>
            </div>
        </section>

        <!-- ================================================================= -->
        <!-- TAB 4: CARTÃO SD & UPGRADE EM MEMÓRIA RAM                         -->
        <!-- ================================================================= -->
        <section id="tab-sdcard" class="tab-content">
            <div class="glass-card">
                <div class="card-header">
                    <div class="card-title">
                        <span>💾</span>
                        <span data-i18n="sdHeader">Arquitetura 100% RAM & Atualização de Firmware</span>
                    </div>
                    <span class="card-badge badge-purple" data-i18n="badgeZeroSdWear">Zero SD Wear</span>
                </div>
                <div style="color: var(--text-secondary); font-size: 0.9rem; line-height: 1.6;">
                    <p style="margin-bottom: 0.8rem;" data-i18n="sdDesc1">
                        O Raspberry Pi Zero carrega o sistema operacional inteiramente na memória RAM (<span style="color: var(--accent-cyan);">initramfs</span>). Após o boot em menos de 2 segundos, o cartão micro-SD físico (<span style="color: #7ee787;">/dev/mmcblk0</span>) é completamente desacoplado e nunca sofre escritas durante o uso diário.
                    </p>
                    <p style="margin-bottom: 1rem;" data-i18n="sdDesc2">
                        Isso traz duas grandes vantagens: <strong>zero risco de corrupção do cartão</strong> (mesmo arrancando da tomada) e a capacidade de atualizar o kernel, DTBs e receptor direto pelo sistema sem precisar retirar o cartão!
                    </p>
                    
                    <div style="background: rgba(255, 255, 255, 0.03); border: 1px solid rgba(255, 255, 255, 0.08); border-radius: var(--radius-sm); padding: 1rem; margin-bottom: 1.2rem;">
                        <div style="font-weight: 700; color: #fff; margin-bottom: 0.5rem;" data-i18n="sdPartitionStatus">Status da Mídia Física:</div>
                        <p>Dispositivo: <span style="font-family: monospace; color: var(--accent-cyan);">/dev/mmcblk0</span> (Partição de Boot: <span style="font-family: monospace; color: var(--accent-cyan);">/dev/mmcblk0p1 FAT16</span>)</p>
                        <p>Estado de Montagem: <span id="sdMountStatus" style="color: #7ee787; font-weight: 700;">Desmontado (Seguro / Desacoplado)</span></p>
                    </div>

                    <div class="action-row">
                        <button class="btn-secondary" onclick="mountSdCard(true)" data-i18n="btnMountSd">Montar Partição (/mnt/boot)</button>
                        <button class="btn-secondary" onclick="mountSdCard(false)" data-i18n="btnUnmountSd">Desmontar Partição</button>
                    </div>

                    <div style="margin-top: 1.5rem;">
                        <h4 style="color: #fff; margin-bottom: 0.5rem;" data-i18n="sdUpgradeManualTitle">Como Atualizar o Appliance sem Retirar o Cartão:</h4>
                        <div class="cmd-box">
                            <span class="cmd-text" id="cmdUpgrade">mount -t vfat /dev/mmcblk0p1 /mnt && cp /tmp/ext-receiver /mnt/ && umount /mnt</span>
                            <button class="copy-btn" onclick="copyCommand('cmdUpgrade')" data-i18n="btnCopy">Copiar</button>
                        </div>
                    </div>
                </div>
            </div>
        </section>

        <!-- ================================================================= -->
        <!-- TAB 5: MANUAL COMPLETO & GUIAS DE OPERAÇÃO                        -->
        <!-- ================================================================= -->
        <section id="tab-manual" class="tab-content">
            <div class="glass-card">
                <div class="card-header">
                    <div class="card-title">
                        <span>📖</span>
                        <span data-i18n="manualHeader">Manual de Operação e Conexão sem IP</span>
                    </div>
                    <span class="card-badge badge-cyan" data-i18n="badgeFullDocs">Documentação</span>
                </div>

                <div style="color: var(--text-secondary); font-size: 0.9rem; line-height: 1.6;">
                    <h3 style="color: #fff; margin: 1rem 0 0.5rem 0;" data-i18n="docSerialTitle">1. Reconfiguração sem IP via Serial USB (/dev/ttyACM0)</h3>
                    <p data-i18n="docSerialDesc">
                        Caso o modo de rede seja desativado ou você esteja em um computador sem suporte a CDC-ECM, o Pi Zero expõe um console serial de recuperação independente no PC através do arquivo de dispositivo <span style="font-family: monospace; color: var(--accent-cyan);">/dev/ttyACM0</span> a 115200 baud.
                    </p>
                    <div class="cmd-box">
                        <span class="cmd-text" id="cmdSerial">picocom -b 115200 /dev/ttyACM0  # ou: screen /dev/ttyACM0 115200</span>
                        <button class="copy-btn" onclick="copyCommand('cmdSerial')" data-i18n="btnCopy">Copiar</button>
                    </div>

                    <h3 style="color: #fff; margin: 1.5rem 0 0.5rem 0;" data-i18n="docLinuxTitle">2. Operação Normal no Linux Wayland (GNOME)</h3>
                    <p data-i18n="docLinuxDesc">
                        Basta conectar o cabo na porta USB de dados (a porta central). O PC receberá automaticamente o IP 192.168.7.1 pelo DHCP nativo em Rust. Em seguida execute o conector:
                    </p>
                    <div class="cmd-box">
                        <span class="cmd-text" id="cmdStart">./scripts/start.sh extend auto 30 false economy --bitrate=400</span>
                        <button class="copy-btn" onclick="copyCommand('cmdStart')" data-i18n="btnCopy">Copiar</button>
                    </div>

                    <h3 style="color: #fff; margin: 1.5rem 0 0.5rem 0;" data-i18n="docWinTitle">3. Operação no Windows 10/11 (Miracast Sem Drivers)</h3>
                    <p data-i18n="docWinDesc">
                        Conecte o Pi Zero na porta USB. Pressione as teclas <strong style="color: #fff;">Win + K</strong> no teclado do Windows e selecione <em>'Pi Zero Wireless Display'</em>. A segunda tela será ativada instantaneamente sem instalar drivers adicionais.
                    </p>

                    <!-- Comparison Table -->
                    <h3 style="color: #fff; margin: 1.5rem 0 0.5rem 0;" data-i18n="protoHeader">Tabela Comparativa de Métodos</h3>
                    <table class="proto-table">
                        <thead>
                            <tr>
                                <th data-i18n="thMethod">Método</th>
                                <th data-i18n="thProtocol">Protocolo</th>
                                <th data-i18n="thLatency">Latência</th>
                                <th data-i18n="thBestFor">Caso de Uso</th>
                            </tr>
                        </thead>
                        <tbody>
                            <tr>
                                <td style="color: #fff; font-weight: 700;">Linux Wayland Direto</td>
                                <td>RTP H.264 (UDP 5000)</td>
                                <td style="color: #7ee787; font-weight: 700;">&lt; 15 ms</td>
                                <td>Desktop interativo, arrastar janelas com mouse suave</td>
                            </tr>
                            <tr>
                                <td style="color: #fff; font-weight: 700;">Windows 10/11 Miracast</td>
                                <td>WFD RTSP (TCP 7236)</td>
                                <td style="color: var(--accent-amber); font-weight: 700;">40–60 ms</td>
                                <td>Projeção nativa sem drivers no Windows (Win + K)</td>
                            </tr>
                            <tr>
                                <td style="color: #fff; font-weight: 700;">USB Bulk Direto</td>
                                <td>FunctionFS (Raw Pipe)</td>
                                <td style="color: #7ee787; font-weight: 700;">&lt; 1 ms</td>
                                <td>Comunicação direta por hardware sem IP</td>
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
                <span>⚠️</span> <span data-i18n="modalRebootTitle">Reiniciar Appliance?</span>
            </div>
            <div class="modal-desc" data-i18n="modalRebootDesc">
                Tem certeza que deseja reiniciar o Raspberry Pi Zero? O sistema reiniciará em menos de 2 segundos diretamente na memória RAM.
            </div>
            <div class="modal-actions">
                <button class="btn-secondary" onclick="closeRebootModal()" data-i18n="btnCancel">Cancelar</button>
                <button class="btn-danger" onclick="executeReboot()" data-i18n="btnConfirmReboot">Sim, Reiniciar</button>
            </div>
        </div>
    </div>

    <!-- Toast Notification -->
    <div id="toast" class="toast"></div>

    <script>
        // State
        let currentFps = 30;
        let currentBitrate = 400;
        let currentColor = '256';
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
                displayHeader: "HDMI Television & Display Telemetry",
                displayDesc: "The VideoCore IV hardware VPU decodes H.264 video streams directly to the HDMI scanout plane without touching the CPU.",
                btnShowHud: "✦ Show HUD on TV (60s)",
                btnHideHud: "✕ Turn Off HUD",
                modeHeader: "Active Streaming Modes",
                badgeMultiMode: "Concurrent Engine",
                m1Desc: "Direct low-latency RTP H.264 stream on UDP port 5000 with AMD VA-API zero-copy offload (< 15ms).",
                m2Desc: "Native Windows 10/11 wireless projection via Win + K on RTSP port 7236. Zero host drivers needed.",
                m3Desc: "Direct 480 Mbps raw hardware pipe via USB FunctionFS without network stack overhead (< 1ms).",
                ctrlHeader: "Display & Stream Optimization",
                badgeZeroCopy: "VideoCore IV DMA",
                bitrateLabel: "Streaming Bitrate (VBR)",
                fpsLabel: "Framerate (FPS)",
                colorLabel: "Color Profile",
                colorFull: "24-bit TrueColor",
                color256: "256 Cores (QP 30-44)",
                colorGray: "Monochrome",
                btnApply: "💾 Apply Settings",
                btnPauseStream: "⏸ Pause Display",
                btnResumeStream: "▶ Resume Display",
                btnReboot: "🔄 Reboot Appliance",
                oneLinerTitle: "One-Line Host Connector (1 Click)",
                badgeInstant: "Instant",
                oneLinerDesc: "On any Linux PC, paste this command in your terminal to start the extended monitor immediately:",
                btnCopy: "Copy",
                dlPkgTitle: "Full Client Package",
                dlPkgDesc: "Contains precompiled ext-sender binary, start.sh, udev rules and installer in tar.gz.",
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
                manualHeader: "Operation Manual & Zero-IP Diagnostics",
                badgeFullDocs: "Documentation",
                docSerialTitle: "1. Zero-IP Reconfiguration via USB Serial (/dev/ttyACM0)",
                docSerialDesc: "If network is disabled or misconfigured, the Pi Zero exposes a recovery serial console on /dev/ttyACM0 at 115200 baud.",
                docLinuxTitle: "2. Standard Linux Wayland (GNOME) Streaming",
                docLinuxDesc: "Plug into the center USB port. The PC gets IP 192.168.7.1 automatically. Then run start.sh:",
                docWinTitle: "3. Windows 10/11 Miracast (Zero Drivers)",
                docWinDesc: "Plug into USB, press Win + K on Windows, select 'Pi Zero Wireless Display'.",
                protoHeader: "Protocol Comparison Table",
                thMethod: "Method",
                thProtocol: "Protocol",
                thLatency: "Latency",
                thBestFor: "Best For",
                modalRebootTitle: "Reboot Appliance?",
                modalRebootDesc: "Are you sure you want to reboot the Raspberry Pi Zero? It will reboot in < 2 seconds directly in RAM.",
                btnCancel: "Cancel",
                btnConfirmReboot: "Yes, Reboot"
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
                displayHeader: "Telemetria do Monitor HDMI & TV",
                displayDesc: "A VPU de hardware VideoCore IV decodifica o stream H.264 direto na memória de scanout da TV sem tocar na CPU.",
                btnShowHud: "✦ Exibir HUD na TV (60s)",
                btnHideHud: "✕ Ocultar HUD",
                modeHeader: "Modos de Transmissão Ativos",
                badgeMultiMode: "Motor Concorrente",
                m1Desc: "Transmissão RTP H.264 de latência ultra-baixa na porta UDP 5000 com GPU AMD VA-API (< 15ms).",
                m2Desc: "Projeção nativa do Windows 10/11 via Win + K na porta RTSP 7236. Zero drivers no PC.",
                m3Desc: "Canal direto de 480 Mbps por hardware via USB FunctionFS sem pilha de rede (< 1ms).",
                ctrlHeader: "Otimização de Exibição e Stream",
                badgeZeroCopy: "VideoCore IV DMA",
                bitrateLabel: "Taxa de Bits (VBR)",
                fpsLabel: "Taxa de Quadros (FPS)",
                colorLabel: "Perfil de Cor",
                colorFull: "24-bit TrueColor",
                color256: "256 Cores (QP 30-44)",
                colorGray: "Monocromático",
                btnApply: "💾 Aplicar Alterações",
                btnPauseStream: "⏸ Pausar Exibição",
                btnResumeStream: "▶ Retomar Exibição",
                btnReboot: "🔄 Reiniciar Appliance (Reboot)",
                oneLinerTitle: "Conector de 1 Linha para PC (1 Clique)",
                badgeInstant: "Instantâneo",
                oneLinerDesc: "Em qualquer computador Linux, abra o terminal e cole o comando abaixo para iniciar a segunda tela:",
                btnCopy: "Copiar",
                dlPkgTitle: "Pacote Completo do Cliente",
                dlPkgDesc: "Contém o binário ext-sender compilado, script start.sh, regras udev e instalador em tar.gz.",
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
                manualHeader: "Manual de Operação e Conexão sem IP",
                badgeFullDocs: "Documentação",
                docSerialTitle: "1. Reconfiguração sem IP via Serial USB (/dev/ttyACM0)",
                docSerialDesc: "Caso a rede seja desativada, o Pi Zero expõe um console serial independente no PC em /dev/ttyACM0 a 115200 baud.",
                docLinuxTitle: "2. Operação Normal no Linux Wayland (GNOME)",
                docLinuxDesc: "Conecte o cabo na porta USB central. O PC recebe IP 192.168.7.1 pelo DHCP nativo. Em seguida execute o conector:",
                docWinTitle: "3. Operação no Windows 10/11 (Miracast Sem Drivers)",
                docWinDesc: "Conecte na USB, pressione Win + K no Windows e selecione 'Pi Zero Wireless Display'.",
                protoHeader: "Tabela Comparativa de Métodos",
                thMethod: "Método",
                thProtocol: "Protocolo",
                thLatency: "Latência",
                thBestFor: "Caso de Uso Ideal",
                modalRebootTitle: "Reiniciar Appliance?",
                modalRebootDesc: "Tem certeza que deseja reiniciar o Raspberry Pi Zero? O sistema reiniciará em menos de 2 segundos diretamente na RAM.",
                btnCancel: "Cancelar",
                btnConfirmReboot: "Sim, Reiniciar"
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
                displayHeader: "Telemetria Display HDMI & TV",
                displayDesc: "La VPU hardware VideoCore IV decodifica il flusso H.264 direttamente nel piano HDMI senza usare la CPU.",
                btnShowHud: "✦ Mostra HUD su TV (60s)",
                btnHideHud: "✕ Nascondi HUD",
                modeHeader: "Modalità di Streaming Attive",
                badgeMultiMode: "Motore Concorrente",
                m1Desc: "Flusso RTP H.264 a bassissima latenza su porta UDP 5000 con GPU AMD VA-API (< 15ms).",
                m2Desc: "Proiezione nativa Windows 10/11 via Win + K su porta RTSP 7236. Zero driver sul PC.",
                m3Desc: "Canale hardware diretto a 480 Mbps via USB FunctionFS senza overhead di rete (< 1ms).",
                ctrlHeader: "Ottimizzazione Display e Streaming",
                badgeZeroCopy: "VideoCore IV DMA",
                bitrateLabel: "Bitrate di Streaming (VBR)",
                fpsLabel: "Frequenza Fotogrammi (FPS)",
                colorLabel: "Profilo Colore",
                colorFull: "24-bit TrueColor",
                color256: "256 Colori (QP 30-44)",
                colorGray: "Monocromatico",
                btnApply: "💾 Applica Modifiche",
                btnPauseStream: "⏸ Sospendi Display",
                btnResumeStream: "▶ Riprendi Display",
                btnReboot: "🔄 Riavvia Appliance (Reboot)",
                oneLinerTitle: "Connettore Host in 1 Riga (1 Clic)",
                badgeInstant: "Istantaneo",
                oneLinerDesc: "Su qualsiasi PC Linux, incolla questo comando nel terminale per avviare il monitor esteso:",
                btnCopy: "Copia",
                dlPkgTitle: "Pacchetto Completo Client",
                dlPkgDesc: "Contiene il binario ext-sender, lo script start.sh, le regole udev e l'installer in tar.gz.",
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
                manualHeader: "Manuale Operativo e Connessione Senza IP",
                badgeFullDocs: "Documentazione",
                docSerialTitle: "1. Riconfigurazione Senza IP via USB Seriale (/dev/ttyACM0)",
                docSerialDesc: "Se la rete è disabilitata, il Pi Zero offre una console seriale di ripristino su /dev/ttyACM0 a 115200 baud.",
                docLinuxTitle: "2. Funzionamento Standard su Linux Wayland (GNOME)",
                docLinuxDesc: "Collega il cavo alla porta USB centrale. Il PC ottiene l'IP 192.168.7.1 dal DHCP. Esegui il connettore:",
                docWinTitle: "3. Proiezione Windows 10/11 (Miracast Senza Driver)",
                docWinDesc: "Collega via USB, premi Win + K su Windows e seleziona 'Pi Zero Wireless Display'.",
                protoHeader: "Tabella Comparativa Protocolli",
                thMethod: "Metodo",
                thProtocol: "Protocollo",
                thLatency: "Latenza",
                thBestFor: "Uso Ideale",
                modalRebootTitle: "Riavviare Appliance?",
                modalRebootDesc: "Sei sicuro di voler riavviare il Raspberry Pi Zero? Si riavvierà in meno di 2 secondi direttamente in RAM.",
                btnCancel: "Annulla",
                btnConfirmReboot: "Sì, Riavvia"
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
                displayHeader: "HDMI 电视与显示遥测",
                displayDesc: "VideoCore IV 硬件 VPU 直接将 H.264 解码输出至 HDMI 屏幕，完全不消耗 CPU 资源。",
                btnShowHud: "✦ 在电视上显示 HUD (60秒)",
                btnHideHud: "✕ 关闭 HUD",
                modeHeader: "多模式并行支持",
                badgeMultiMode: "并发引擎",
                m1Desc: "UDP 5000 端口超低延迟 RTP H.264 流，AMD VA-API 零拷贝 GPU 加速 (< 15ms)。",
                m2Desc: "Windows 10/11 原生 Win + K 无线投屏（RTSP 7236 端口），电脑无需安装任何驱动。",
                m3Desc: "USB FunctionFS 480 Mbps 裸硬件通道，无网络协议栈开销 (< 1ms)。",
                ctrlHeader: "显示与推流优化",
                badgeZeroCopy: "VideoCore IV DMA",
                bitrateLabel: "推流码率 (VBR)",
                fpsLabel: "帧率 (FPS)",
                colorLabel: "颜色配置",
                colorFull: "24位真彩色",
                color256: "256 色低功耗",
                colorGray: "单色灰度",
                btnApply: "💾 应用配置",
                btnPauseStream: "⏸ 暂停显示",
                btnResumeStream: "▶ 恢复显示",
                btnReboot: "🔄 重启设备 (Reboot)",
                oneLinerTitle: "主机一键连接脚本 (1 键执行)",
                badgeInstant: "即时生效",
                oneLinerDesc: "在任何 Linux 电脑上，只需在终端中运行以下命令即可立即扩展屏幕：",
                btnCopy: "复制",
                dlPkgTitle: "客户端完整安装包",
                dlPkgDesc: "包含预编译 ext-sender 二进制文件、start.sh 脚本、udev 规则和安装器的 tar.gz 压缩包。",
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
                manualHeader: "完整操作手册与零 IP 串口配置",
                badgeFullDocs: "技术文档",
                docSerialTitle: "1. 通过 USB 虚拟串口 (/dev/ttyACM0) 零 IP 维护",
                docSerialDesc: "若网络禁用或配置错误，树莓派会在电脑上提供 115200 波特率的 /dev/ttyACM0 救援控制台。",
                docLinuxTitle: "2. Linux Wayland (GNOME) 正常连接",
                docLinuxDesc: "将 USB 线插入中间的数据端口。电脑将通过内置 DHCP 自动获取 192.168.7.1，然后运行启动脚本：",
                docWinTitle: "3. Windows 10/11 投屏 (Win + K 无需驱动)",
                docWinDesc: "插入 USB 后在 Windows 上按 Win + K，选择 'Pi Zero Wireless Display' 即可。",
                protoHeader: "传输协议特性对比表",
                thMethod: "传输方式",
                thProtocol: "通信协议",
                thLatency: "传输延迟",
                thBestFor: "适用场景",
                modalRebootTitle: "确定要重启设备？",
                modalRebootDesc: "确定要重启树莓派 Pi Zero 吗？系统将在不到2秒内直接在内存中快速重启。",
                btnCancel: "取消",
                btnConfirmReboot: "确认重启"
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

        // Language Switcher
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
                    } else {
                        el.textContent = dict[key];
                    }
                }
            });
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
            const labels = { full: '24-bit TrueColor', '256': '256-Color (QP 30-44)', gray: 'Monochrome' };
            const el = document.getElementById('valColor');
            if (el) el.textContent = labels[profile] || profile;
            document.querySelectorAll('#colorGrid .btn-toggle').forEach(b => {
                b.classList.toggle('active', b.getAttribute('data-color') === profile);
            });
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

            if (toggle) toggle.checked = enabled;
            if (card) {
                if (enabled) {
                    card.classList.remove('disabled');
                } else {
                    card.classList.add('disabled');
                }
            }
            if (badge) {
                if (enabled) {
                    badge.className = modeKey === 'mode1' ? 'stat-badge badge-cyan' : modeKey === 'mode2' ? 'stat-badge badge-green' : 'stat-badge badge-purple';
                    badge.textContent = modeKey === 'mode1' ? 'Ligado (UDP 5000)' : modeKey === 'mode2' ? 'Ligado (TCP 7236)' : 'Ligado (USB Bulk)';
                } else {
                    badge.className = 'stat-badge badge-red';
                    badge.textContent = 'Desligado (Inativo)';
                }
            }
        }

        function toggleMode(modeKey, enabled) {
            setModeToggleUI(modeKey, enabled);
            localStorage.setItem('ext_' + modeKey, enabled);
            
            showToast(enabled ? `✓ ${modeKey.toUpperCase()} ligado!` : `✕ ${modeKey.toUpperCase()} desligado.`);
            
            fetch('/api/modes', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    [modeKey]: enabled,
                    mode1: activeModes.mode1,
                    mode2: activeModes.mode2,
                    mode3: activeModes.mode3
                })
            }).catch(() => {});
        }

        // Hot-Apply Configuration
        function applyConfiguration() {
            localStorage.setItem('ext_color', currentColor);
            localStorage.setItem('ext_fps', currentFps);
            localStorage.setItem('ext_bitrate', currentBitrate);
            showToast('Applying configuration via UDP 5001...');
            fetch('/api/config', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    fps: currentFps,
                    bitrate: currentBitrate,
                    color: currentColor
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

        // Stream Pause / Resume
        function togglePauseStream() {
            isPaused = !isPaused;
            const endpoint = isPaused ? '/api/stream/stop' : '/api/stream/start';
            fetch(endpoint, { method: 'POST' })
                .then(r => r.json())
                .then(() => {
                    const btn = document.getElementById('btnPause');
                    btn.textContent = isPaused ? '▶ Retomar Exibição' : '⏸ Pausar Exibição';
                    document.getElementById('valState').textContent = isPaused ? 'PAUSADO' : 'ONLINE';
                    document.getElementById('badgeStream').textContent = isPaused ? 'Pausado' : 'Ativo';
                    document.getElementById('badgeStream').className = isPaused ? 'stat-badge badge-red' : 'stat-badge badge-green';
                    showToast(isPaused ? 'Exibição pausada' : 'Exibição retomada');
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
                        statusEl.textContent = 'Montado em /mnt/boot (Pronto para Upgrade)';
                        statusEl.style.color = '#00e5ff';
                        showToast('✓ Cartão montado em /mnt/boot');
                    } else {
                        statusEl.textContent = 'Desmontado (Seguro / Desacoplado)';
                        statusEl.style.color = '#7ee787';
                        showToast('✓ Cartão desmontado com segurança');
                    }
                });
        }

        // Copy Helper
        function copyCommand(id) {
            const el = document.getElementById(id);
            if (el) {
                navigator.clipboard.writeText(el.textContent.trim()).then(() => {
                    const btn = el.parentElement.querySelector('.copy-btn');
                    if (btn) {
                        const oldText = btn.textContent;
                        btn.textContent = '✓ Copiado!';
                        btn.classList.add('copied');
                        setTimeout(() => {
                            btn.textContent = oldText;
                            btn.classList.remove('copied');
                        }, 2000);
                    }
                });
            }
        }

        // Toast Helper
        function showToast(msg) {
            const t = document.getElementById('toast');
            t.textContent = msg;
            t.classList.add('show');
            setTimeout(() => t.classList.remove('show'), 2500);
        }

        // Telemetry Poller
        function pollTelemetry() {
            fetch('/api/status')
                .then(r => r.json())
                .then(data => {
                    if (data.temp) document.getElementById('valTemp').textContent = data.temp + '°C';
                    if (data.cpu) document.getElementById('valCpu').textContent = data.cpu;
                    if (data.ram) document.getElementById('valRam').textContent = data.ram + ' MB';
                })
                .catch(() => {});
        }

        // Init: Restore from localStorage first (for instant snappy UI on F5), then sync with server
        const savedLang = localStorage.getItem('ext_monitor_lang') || navigator.language.slice(0, 2);
        setLanguage(savedLang);

        const savedColor = localStorage.getItem('ext_color');
        if (savedColor) setColor(savedColor);

        const savedFps = localStorage.getItem('ext_fps');
        if (savedFps) setFps(parseInt(savedFps, 10));

        const savedBitrate = localStorage.getItem('ext_bitrate');
        if (savedBitrate) setBitrate(parseInt(savedBitrate, 10));

        const savedM1 = localStorage.getItem('ext_mode1');
        if (savedM1 !== null) setModeToggleUI('mode1', savedM1 === 'true');

        const savedM2 = localStorage.getItem('ext_mode2');
        if (savedM2 !== null) setModeToggleUI('mode2', savedM2 === 'true');

        const savedM3 = localStorage.getItem('ext_mode3');
        if (savedM3 !== null) setModeToggleUI('mode3', savedM3 === 'true');

        // Fetch server state to sync if not set locally
        fetch('/api/config')
            .then(r => r.json())
            .then(cfg => {
                if (cfg.color && !savedColor) setColor(cfg.color);
                if (cfg.fps && !savedFps) setFps(cfg.fps);
                if (cfg.bitrate && !savedBitrate) setBitrate(cfg.bitrate);
                if (cfg.mode1 !== undefined && savedM1 === null) setModeToggleUI('mode1', cfg.mode1);
                if (cfg.mode2 !== undefined && savedM2 === null) setModeToggleUI('mode2', cfg.mode2);
                if (cfg.mode3 !== undefined && savedM3 === null) setModeToggleUI('mode3', cfg.mode3);
            })
            .catch(() => {});

        setInterval(pollTelemetry, 2000);
    </script>
</body>
</html>
"#;
