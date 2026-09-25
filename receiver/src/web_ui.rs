//! Embedded Web Dashboard Frontend UI in 100% Pure Rust
//!
//! Stores the complete, self-contained HTML, CSS, JavaScript, and internationalization
//! dictionary directly in the binary's read-only data segment.
//!
//! Eliminates external disk files and runtime web assets entirely.
//!
//! Features:
//! - Multilingual UI support: English (en), Portuguese (pt), Italian (it), Chinese (zh)
//! - Real-time hardware telemetry display (SoC temperature, CPU load, RAM usage, decoder status)
//! - Stream tuning: framerate (FPS), adaptive VBR bitrate slider, color profiles, on-screen HUD
//! - Control actions: Pause Display Stream, Resume Stream, and Clean Shutdown of Panel & Service
//! - Step-by-step casting instructions for Windows 10/11 (Win + K) and Linux Wayland (ext-sender)
//! - Comprehensive technical comparison between all 4 streaming modes (Linux Direct, GNOME Miracast, Windows, USB Bulk)
//! - Diagnostic scripts and copyable commands for host and Pi Zero
//! - System prerequisites and troubleshooting guide
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
            --bg-surface: rgba(16, 23, 38, 0.78);
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
            padding: 1.1rem 2rem;
            background: rgba(10, 14, 23, 0.85);
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
            background: var(--accent-cyan);
            border-radius: 50%;
            box-shadow: 0 0 12px var(--accent-cyan);
        }
        .brand-text h1 { font-size: 1.2rem; font-weight: 700; color: #fff; }
        .brand-text p { font-size: 0.78rem; color: var(--text-secondary); }
        .nav-controls { display: flex; align-items: center; gap: 1rem; }
        .lang-select {
            background: rgba(16, 23, 38, 0.9);
            border: 1px solid var(--bg-surface-border);
            color: var(--text-primary);
            padding: 0.4rem 0.8rem;
            border-radius: var(--radius-sm);
            font-size: 0.85rem;
            cursor: pointer;
            outline: none;
            transition: var(--transition);
        }
        .lang-select:focus { border-color: var(--accent-cyan); }
        .status-badge {
            display: flex; align-items: center; gap: 0.5rem;
            padding: 0.35rem 0.85rem;
            background: rgba(0, 255, 102, 0.1);
            border: 1px solid rgba(0, 255, 102, 0.3);
            border-radius: 9999px;
            font-size: 0.8rem; font-weight: 600; color: var(--accent-emerald);
        }
        .status-badge.paused {
            background: rgba(255, 179, 0, 0.1);
            border-color: rgba(255, 179, 0, 0.3);
            color: var(--accent-amber);
        }
        .status-badge.stopped {
            background: rgba(255, 82, 82, 0.1);
            border-color: rgba(255, 82, 82, 0.3);
            color: var(--accent-red);
        }
        .status-badge .pulse {
            width: 8px; height: 8px;
            background: currentColor;
            border-radius: 50%;
            box-shadow: 0 0 8px currentColor;
            animation: pulse-glow 2s infinite;
        }
        @keyframes pulse-glow { 0%, 100% { opacity: 1; } 50% { opacity: 0.35; } }
        .container {
            max-width: 1300px; margin: 0 auto;
            padding: 2rem 1.5rem; position: relative; z-index: 1;
        }
        .dashboard-grid {
            display: grid;
            grid-template-columns: minmax(0, 1.25fr) minmax(0, 1fr);
            gap: 1.75rem;
        }
        @media (max-width: 980px) {
            .dashboard-grid { grid-template-columns: 1fr; }
        }
        .glass-card {
            background: var(--bg-surface);
            border: 1px solid var(--bg-surface-border);
            border-radius: var(--radius-lg);
            padding: 1.75rem;
            backdrop-filter: blur(16px);
            min-width: 0;
            box-shadow: 0 8px 32px 0 rgba(0, 0, 0, 0.36);
            margin-bottom: 1.75rem;
        }
        .card-header {
            display: flex; justify-content: space-between; align-items: center;
            margin-bottom: 1.5rem; padding-bottom: 0.8rem;
            border-bottom: 1px solid rgba(255, 255, 255, 0.06);
        }
        .card-title {
            font-size: 1.1rem; font-weight: 700; color: #fff;
            display: flex; align-items: center; gap: 0.6rem;
        }
        .card-badge {
            font-size: 0.72rem; padding: 0.2rem 0.6rem;
            background: rgba(0, 229, 255, 0.12);
            border: 1px solid rgba(0, 229, 255, 0.25);
            border-radius: var(--radius-sm);
            color: var(--accent-cyan);
        }
        .control-group { margin-bottom: 1.5rem; }
        .control-label {
            display: flex; justify-content: space-between; align-items: center;
            font-size: 0.9rem; font-weight: 600; margin-bottom: 0.75rem;
            color: var(--text-primary);
        }
        .control-value { font-family: monospace; color: var(--accent-cyan); font-weight: 700; }
        .btn-grid { display: flex; flex-wrap: wrap; gap: 0.6rem; }
        .btn-toggle {
            flex: 1; min-width: 60px;
            background: rgba(255, 255, 255, 0.04);
            border: 1px solid rgba(255, 255, 255, 0.08);
            color: var(--text-secondary);
            padding: 0.7rem 0.5rem;
            border-radius: var(--radius-md);
            font-size: 0.85rem; font-weight: 600;
            cursor: pointer; transition: var(--transition);
            text-align: center;
        }
        .btn-toggle:hover {
            background: rgba(0, 229, 255, 0.08);
            border-color: rgba(0, 229, 255, 0.3);
            color: #fff;
        }
        .btn-toggle.active {
            background: rgba(0, 229, 255, 0.18);
            border-color: var(--accent-cyan);
            color: #fff;
            box-shadow: 0 0 16px rgba(0, 229, 255, 0.25);
        }
        .slider-wrap { position: relative; margin: 1rem 0; }
        .range-slider {
            -webkit-appearance: none; width: 100%; height: 6px;
            border-radius: 9999px;
            background: rgba(255, 255, 255, 0.12);
            outline: none;
        }
        .range-slider::-webkit-slider-thumb {
            -webkit-appearance: none; width: 20px; height: 20px;
            border-radius: 50%;
            background: var(--accent-cyan);
            cursor: pointer;
            box-shadow: 0 0 12px var(--accent-cyan);
        }
        .slider-labels {
            display: flex; justify-content: space-between;
            font-size: 0.72rem; color: var(--text-muted);
            margin-top: 0.4rem;
        }
        .action-row { display: flex; gap: 0.8rem; margin-top: 1rem; }
        .btn-primary {
            flex: 1; padding: 0.85rem 1rem;
            background: linear-gradient(135deg, rgba(0, 229, 255, 0.2), rgba(179, 136, 255, 0.2));
            border: 1px solid var(--accent-cyan);
            border-radius: var(--radius-md);
            color: #fff; font-weight: 700; font-size: 0.9rem;
            cursor: pointer; transition: var(--transition);
            display: flex; align-items: center; justify-content: center; gap: 0.5rem;
        }
        .btn-primary:hover {
            box-shadow: 0 0 20px rgba(0, 229, 255, 0.4);
            transform: translateY(-1px);
        }
        .btn-warning {
            flex: 1; padding: 0.85rem 1rem;
            background: rgba(255, 179, 0, 0.12);
            border: 1px solid var(--accent-amber);
            border-radius: var(--radius-md);
            color: #ffe082; font-weight: 700; font-size: 0.9rem;
            cursor: pointer; transition: var(--transition);
            display: flex; align-items: center; justify-content: center; gap: 0.5rem;
        }
        .btn-warning:hover {
            background: rgba(255, 179, 0, 0.24);
            color: #fff; box-shadow: 0 0 16px rgba(255, 179, 0, 0.35);
        }
        .btn-danger {
            flex: 1; padding: 0.85rem 1rem;
            background: rgba(255, 82, 82, 0.12);
            border: 1px solid var(--accent-red);
            border-radius: var(--radius-md);
            color: #ff8a80; font-weight: 700; font-size: 0.9rem;
            cursor: pointer; transition: var(--transition);
            display: flex; align-items: center; justify-content: center; gap: 0.5rem;
        }
        .btn-danger:hover {
            background: rgba(255, 82, 82, 0.24);
            color: #fff; box-shadow: 0 0 16px rgba(255, 82, 82, 0.35);
        }
        .stats-grid {
            display: grid; grid-template-columns: 1fr 1fr; gap: 1rem;
        }
        .stat-card {
            background: rgba(255, 255, 255, 0.02);
            border: 1px solid rgba(255, 255, 255, 0.05);
            border-radius: var(--radius-md);
            padding: 1rem;
        }
        .stat-name { font-size: 0.78rem; color: var(--text-secondary); margin-bottom: 0.3rem; }
        .stat-val { font-size: 1.15rem; font-weight: 700; color: #fff; font-family: monospace; }
        .stat-val.cyan { color: var(--accent-cyan); }
        .stat-val.emerald { color: var(--accent-emerald); }
        .stat-val.purple { color: var(--accent-purple); }
        .code-box {
            background: #06090f;
            border: 1px solid rgba(255, 255, 255, 0.08);
            border-radius: var(--radius-md);
            padding: 1rem;
            font-family: 'SF Mono', Consolas, Monaco, monospace;
            font-size: 0.8rem;
            color: #7dd3fc;
            overflow-x: auto;
            white-space: pre-wrap;
            word-break: break-all;
            margin-top: 0.8rem;
            position: relative;
        }
        .copy-btn {
            position: absolute; top: 0.6rem; right: 0.6rem;
            background: rgba(255, 255, 255, 0.08);
            border: 1px solid rgba(255, 255, 255, 0.15);
            color: #fff; border-radius: var(--radius-sm);
            padding: 0.25rem 0.5rem; font-size: 0.7rem;
            cursor: pointer;
        }
        .mode-banner {
            display: flex; align-items: center; justify-content: space-between;
            padding: 1rem 1.25rem;
            background: rgba(0, 229, 255, 0.06);
            border: 1px solid var(--accent-cyan);
            border-radius: var(--radius-md);
            margin-bottom: 1.5rem;
        }
        .mode-banner-info h3 { font-size: 0.95rem; font-weight: 700; color: #fff; }
        .mode-banner-info p { font-size: 0.8rem; color: var(--text-secondary); }
        .req-list { list-style: none; margin-top: 0.6rem; font-size: 0.85rem; color: var(--text-secondary); }
        .req-list li { margin-bottom: 0.5rem; display: flex; align-items: flex-start; gap: 0.5rem; }
        .req-list li strong { color: var(--text-primary); }
        .bullet { color: var(--accent-cyan); }
        /* Protocols Comparison Table */
        .proto-table-wrap { overflow-x: auto; margin-top: 1rem; }
        .proto-table {
            width: 100%; border-collapse: collapse; font-size: 0.85rem; text-align: left;
        }
        .proto-table th, .proto-table td {
            padding: 0.85rem 0.75rem;
            border-bottom: 1px solid rgba(255, 255, 255, 0.06);
        }
        .proto-table th {
            color: var(--text-muted); font-size: 0.75rem; font-weight: 700; text-transform: uppercase;
            letter-spacing: 0.5px;
        }
        .proto-table tr:hover td { background: rgba(255, 255, 255, 0.02); }
        .proto-badge {
            display: inline-block; padding: 0.2rem 0.55rem; border-radius: var(--radius-sm);
            font-size: 0.72rem; font-weight: 700; font-family: monospace;
        }
        .proto-badge.fast {
            background: rgba(0, 255, 102, 0.12); border: 1px solid rgba(0, 255, 102, 0.3); color: var(--accent-emerald);
        }
        .proto-badge.std {
            background: rgba(0, 229, 255, 0.12); border: 1px solid rgba(0, 229, 255, 0.3); color: var(--accent-cyan);
        }
        .proto-badge.usb {
            background: rgba(179, 136, 255, 0.12); border: 1px solid rgba(179, 136, 255, 0.3); color: var(--accent-purple);
        }
        /* Network Management Grid & Inputs */
        .net-grid {
            display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 1.25rem; margin-top: 1rem;
        }
        .net-card {
            background: rgba(255, 255, 255, 0.02); border: 1px solid rgba(255, 255, 255, 0.08); border-radius: var(--radius-md); padding: 1.1rem;
        }
        .net-card.active { border-color: rgba(0, 229, 255, 0.35); }
        .net-field { margin-bottom: 0.8rem; }
        .net-label { font-size: 0.76rem; color: var(--text-secondary); margin-bottom: 0.3rem; display: block; }
        .net-input {
            width: 100%; background: rgba(10, 14, 23, 0.85); border: 1px solid rgba(255, 255, 255, 0.12);
            border-radius: var(--radius-sm); padding: 0.45rem 0.75rem; color: #fff; font-size: 0.85rem;
            outline: none; transition: var(--transition);
        }
        .net-input:focus { border-color: var(--accent-cyan); box-shadow: 0 0 10px rgba(0, 229, 255, 0.2); }
    </style>
</head>
<body>
    <div class="glow-bg"></div>

    <nav class="navbar">
        <div class="brand">
            <div class="logo-icon"><div class="dot"></div></div>
            <div class="brand-text">
                <h1 data-i18n="title">Pi Zero Extended Display</h1>
                <p data-i18n="subtitle">Hardware GPU Video Receiver Dashboard</p>
            </div>
        </div>
        <div class="nav-controls">
            <select id="langSelect" class="lang-select">
                <option value="en">🇺🇸 English</option>
                <option value="pt" selected>🇧🇷 Português</option>
                <option value="it">🇮🇹 Italiano</option>
                <option value="zh">🇨🇳 中文</option>
            </select>
            <div class="status-badge" id="mainStatusBadge">
                <div class="pulse"></div>
                <span id="systemStatus" data-i18n="statusLive">V4L2 M2M Online</span>
            </div>
        </div>
    </nav>

    <div class="container">
        <!-- Dual Mode Banner -->
        <div class="mode-banner">
            <div class="mode-banner-info">
                <h3 data-i18n="modeTitle">Dual-Mode Display Engine: Mode 1 (Network + Miracast)</h3>
                <p data-i18n="modeDesc">Supporting Linux Wayland ScreenCast & Windows 10/11 native wireless casting (Win + K)</p>
            </div>
            <button id="modeSwitchBtn" class="btn-primary" style="flex: 0 0 auto; padding: 0.5rem 1rem;" data-i18n="modeSwitch">
                Switch to USB Bulk
            </button>
        </div>

        <div class="dashboard-grid">
            <!-- Left Column: Controls & Power -->
            <div class="controls-col">
                <!-- Power & Stream State Controls -->
                <div class="glass-card">
                    <div class="card-header">
                        <div class="card-title">
                            <span>⚡</span>
                            <span data-i18n="powerHeader">Panel & Stream Control</span>
                        </div>
                        <span class="card-badge" data-i18n="badgeState">Session</span>
                    </div>
                    <p style="font-size: 0.85rem; color: var(--text-secondary); margin-bottom: 1rem;" data-i18n="powerDesc">
                        Manage display streaming or stop the service and web panel cleanly:
                    </p>
                    <div class="action-row">
                        <button id="btnPauseStream" class="btn-warning" data-i18n="btnPauseStream">⏸ Pause Stream</button>
                        <button id="btnResumeStream" class="btn-primary" data-i18n="btnResumeStream">▶ Resume Stream</button>
                        <button id="btnStopService" class="btn-danger" data-i18n="btnStopService">⏹ Stop Panel & Service</button>
                    </div>
                    <div id="serviceStatusMessage" style="margin-top: 0.8rem; font-size: 0.8rem; color: var(--accent-amber); display: none;"></div>
                </div>

                <!-- Display & Stream Optimization -->
                <div class="glass-card">
                    <div class="card-header">
                        <div class="card-title">
                            <span>⚙️</span>
                            <span data-i18n="ctrlHeader">Display & Stream Optimization</span>
                        </div>
                        <span class="card-badge" data-i18n="badgeZeroCopy">VideoCore IV DMA</span>
                    </div>

                    <!-- Framerate -->
                    <div class="control-group">
                        <div class="control-label">
                            <span data-i18n="fpsLabel">Framerate (FPS)</span>
                            <span class="control-value" id="valFps">30 FPS</span>
                        </div>
                        <div class="btn-grid" id="fpsGrid">
                            <button class="btn-toggle" data-fps="10">10</button>
                            <button class="btn-toggle" data-fps="15">15</button>
                            <button class="btn-toggle" data-fps="24">24</button>
                            <button class="btn-toggle active" data-fps="30">30</button>
                            <button class="btn-toggle" data-fps="60">60</button>
                        </div>
                    </div>

                    <!-- Bitrate -->
                    <div class="control-group">
                        <div class="control-label">
                            <span data-i18n="bitrateLabel">Streaming Bitrate (VBR)</span>
                            <span class="control-value" id="valBitrate">3000 kbps</span>
                        </div>
                        <div class="slider-wrap">
                            <input type="range" min="150" max="15000" step="50" value="3000" class="range-slider" id="bitrateSlider">
                            <div class="slider-labels">
                                <span data-i18n="rateMin">150 kbps (Min)</span>
                                <span>3000 kbps</span>
                                <span>6000 kbps</span>
                                <span>10000 kbps</span>
                                <span data-i18n="rateMax">15 Mbps (Max)</span>
                            </div>
                        </div>
                    </div>

                    <!-- Color Profile -->
                    <div class="control-group">
                        <div class="control-label">
                            <span data-i18n="colorLabel">Color Profile</span>
                            <span class="control-value" id="valColor">24-bit TrueColor</span>
                        </div>
                        <div class="btn-grid" id="colorGrid">
                            <button class="btn-toggle active" data-color="full" data-i18n="colorFull">24-bit TrueColor</button>
                            <button class="btn-toggle" data-color="256" data-i18n="color256">256-Color (QP 30-44)</button>
                            <button class="btn-toggle" data-color="gray" data-i18n="colorGray">Monochrome</button>
                        </div>
                    </div>

                    <!-- Diagnostic HUD Trigger -->
                    <div class="control-group">
                        <div class="control-label">
                            <span data-i18n="hudLabel">Diagnostic Telemetry HUD</span>
                        </div>
                        <div class="action-row">
                            <button id="btnTriggerHud" class="btn-primary" data-i18n="btnShowHud">✦ Show HUD (60s)</button>
                            <button id="btnHideHud" class="btn-danger" data-i18n="btnHideHud">✕ Turn Off HUD</button>
                        </div>
                    </div>
                </div>
            </div>

            <!-- Right Column: Telemetry & Host Connection -->
            <div class="telemetry-col">
                <!-- System Requirements & What is Needed -->
                <div class="glass-card">
                    <div class="card-header">
                        <div class="card-title">
                            <span>📋</span>
                            <span data-i18n="reqHeader">Prerequisites & What is Needed</span>
                        </div>
                        <span class="card-badge" data-i18n="badgeGuide">Guide</span>
                    </div>
                    <ul class="req-list">
                        <li><span class="bullet">▸</span><span><strong data-i18n="reqPi">Pi Zero Hardware:</strong> <span data-i18n="reqPiDesc">Pi Zero W or Pi Zero 2 connected via Micro-USB data cable (OTG port).</span></span></li>
                        <li><span class="bullet">▸</span><span><strong data-i18n="reqGpu">GPU VideoCore IV:</strong> <span data-i18n="reqGpuDesc">Broadcom V4L2 M2M hardware decoder (/dev/video10) & KMS DRM output.</span></span></li>
                        <li><span class="bullet">▸</span><span><strong data-i18n="reqHost">Host System:</strong> <span data-i18n="reqHostDesc">Linux Wayland (GNOME Mutter) with VA-API GPU encoding, or Windows 10/11 with Miracast.</span></span></li>
                        <li><span class="bullet">▸</span><span><strong data-i18n="reqStopCli">How to Stop/Restart:</strong> <span data-i18n="reqStopCliDesc">Run 'sudo systemctl stop ext-receiver' to stop daemon and panel. Run 'sudo systemctl start ext-receiver' to start.</span></span></li>
                    </ul>
                </div>

                <!-- Hardware Telemetry -->
                <div class="glass-card">
                    <div class="card-header">
                        <div class="card-title">
                            <span>📊</span>
                            <span data-i18n="telemetryHeader">Hardware Telemetry</span>
                        </div>
                        <span class="card-badge" data-i18n="badgeRealtime">Real-Time</span>
                    </div>
                    <div class="stats-grid">
                        <div class="stat-card">
                            <div class="stat-name" data-i18n="statDecoder">Decoder Engine</div>
                            <div class="stat-val cyan">V4L2 M2M (IV)</div>
                        </div>
                        <div class="stat-card">
                            <div class="stat-name" data-i18n="statSink">Display Sink</div>
                            <div class="stat-val emerald">KMS DRM HDMI</div>
                        </div>
                        <div class="stat-card">
                            <div class="stat-name" data-i18n="statTemp">SoC Temperature</div>
                            <div class="stat-val" id="statTemp">45.2 °C</div>
                        </div>
                        <div class="stat-card">
                            <div class="stat-name" data-i18n="statLatency">Link Latency (RTT)</div>
                            <div class="stat-val cyan">&lt; 0.35 ms</div>
                        </div>
                        <div class="stat-card">
                            <div class="stat-name" data-i18n="statCpu">CPU Usage</div>
                            <div class="stat-val emerald" id="statCpu">~0.4%</div>
                        </div>
                        <div class="stat-card">
                            <div class="stat-name" data-i18n="statRam">Free RAM</div>
                            <div class="stat-val purple" id="statRam">312 MB</div>
                        </div>
                    </div>
                </div>
            </div>
        </div>

        <!-- Full-Width Card: Network & Interface Management (OTG Zero-Gateway & LAN/Wi-Fi) -->
        <div class="glass-card">
            <div class="card-header">
                <div class="card-title">
                    <span>🖧</span>
                    <span data-i18n="netHeader">Network & Interface Management (OTG Zero-Gateway & LAN/Wi-Fi)</span>
                </div>
                <span class="card-badge" data-i18n="netBadge">Zero-Gateway DHCP + Static</span>
            </div>
            <p style="font-size: 0.85rem; color: var(--text-secondary);" data-i18n="netDesc">
                Pi Zero runs a dedicated Zero-Gateway DHCP server on USB OTG (assigning 192.168.7.1 to your PC without breaking internet). For physical Ethernet/Wi-Fi, select DHCP or define a Static IP:
            </p>

            <div class="net-grid">
                <!-- OTG Virtual Network -->
                <div class="net-card active">
                    <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:0.75rem;">
                        <strong style="color:var(--accent-cyan); font-size:0.95rem;">🔌 USB OTG (usb0)</strong>
                        <span class="proto-badge fast" data-i18n="netOtgStatus">Zero-Gateway Active</span>
                    </div>
                    <div class="net-field">
                        <span class="net-label" data-i18n="netPiIp">Raspberry Pi IP:</span>
                        <div style="font-family:monospace; font-weight:700; color:#fff;" id="netUsb0Ip">192.168.7.2 / 24</div>
                    </div>
                    <div class="net-field">
                        <span class="net-label" data-i18n="netHostIp">Host PC IP (DHCP Lease):</span>
                        <div style="font-family:monospace; font-weight:700; color:var(--accent-emerald);">192.168.7.1</div>
                    </div>
                    <div class="net-field">
                        <span class="net-label" data-i18n="netGateway">Default Gateway:</span>
                        <div style="font-family:monospace; font-size:0.8rem; color:var(--text-muted);" data-i18n="netNoneGw">None (Direct Link - Safe for PC Internet)</div>
                    </div>
                    <div class="net-field">
                        <span class="net-label" data-i18n="netIpv6">IPv6 Address:</span>
                        <div style="font-family:monospace; font-size:0.75rem; color:#7dd3fc;" id="netUsb0Ipv6">fe80:: (Link-Local)</div>
                    </div>
                </div>

                <!-- Physical / Wi-Fi Adapter Configuration -->
                <div class="net-card">
                    <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:0.75rem;">
                        <strong style="color:#fff; font-size:0.95rem;">📶 LAN / Wi-Fi / Miracast</strong>
                        <span class="proto-badge std" id="netPhysStatus" data-i18n="netPhysBadge">Adapter Config</span>
                    </div>

                    <div class="net-field">
                        <span class="net-label" data-i18n="netSelectIface">Interface:</span>
                        <select id="netSelectIface" class="net-input">
                            <option value="eth0">eth0 (Physical Ethernet)</option>
                            <option value="wlan0">wlan0 (Wi-Fi / Miracast P2P)</option>
                        </select>
                    </div>

                    <div class="net-field">
                        <span class="net-label" data-i18n="netModeLabel">Mode:</span>
                        <select id="netSelectMode" class="net-input">
                            <option value="dhcp" data-i18n="netModeDhcp">DHCP Client (Automatic from Router)</option>
                            <option value="static" data-i18n="netModeStatic">Static IP (Manual)</option>
                        </select>
                    </div>

                    <div id="staticNetFields" style="display:none;">
                        <div class="net-field">
                            <span class="net-label" data-i18n="netStaticIp">IPv4 Address:</span>
                            <input type="text" id="netInputIp" class="net-input" value="192.168.1.150" placeholder="e.g. 192.168.1.150">
                        </div>
                        <div class="net-field">
                            <span class="net-label" data-i18n="netNetmask">Subnet Mask:</span>
                            <input type="text" id="netInputMask" class="net-input" value="255.255.255.0" placeholder="255.255.255.0">
                        </div>
                        <div class="net-field">
                            <span class="net-label" data-i18n="netGatewayInput">Gateway:</span>
                            <input type="text" id="netInputGw" class="net-input" value="192.168.1.1" placeholder="e.g. 192.168.1.1">
                        </div>
                        <div class="net-field">
                            <span class="net-label" data-i18n="netDns">DNS Servers:</span>
                            <input type="text" id="netInputDns" class="net-input" value="1.1.1.1, 8.8.8.8" placeholder="1.1.1.1, 8.8.8.8">
                        </div>
                    </div>

                    <div class="net-field">
                        <span class="net-label" data-i18n="netIpv6Mode">IPv6 Mode:</span>
                        <select id="netSelectIpv6" class="net-input">
                            <option value="auto" data-i18n="netIpv6Auto">Auto SLAAC / Link-Local</option>
                            <option value="disable" data-i18n="netIpv6Disable">Disabled</option>
                        </select>
                    </div>

                    <button id="btnApplyNetwork" class="btn-primary" style="width:100%; margin-top:0.5rem;" data-i18n="netBtnApply">
                        💾 Apply Network Settings
                    </button>
                    <div id="netStatusMessage" style="margin-top:0.6rem; font-size:0.8rem; display:none;"></div>
                </div>
            </div>
        </div>

        <!-- Full-Width Card: Connection Protocols & Comparison -->
        <div class="glass-card">
            <div class="card-header">
                <div class="card-title">
                    <span>🌐</span>
                    <span data-i18n="protoHeader">Connection Methods & Protocol Comparison</span>
                </div>
                <span class="card-badge" data-i18n="protoBadge">Multi-Protocol</span>
            </div>
            <p style="font-size: 0.85rem; color: var(--text-secondary);" data-i18n="protoDesc">
                Pi Zero supports multiple concurrent streaming protocols. Choose the optimal method for your OS:
            </p>
            <div class="proto-table-wrap">
                <table class="proto-table">
                    <thead>
                        <tr>
                            <th data-i18n="thMethod">Method / Client</th>
                            <th data-i18n="thProtocol">Protocol & Port</th>
                            <th data-i18n="thLatency">Latency</th>
                            <th data-i18n="thBestFor">Ideal Use-Case</th>
                            <th data-i18n="thCommand">How to Connect</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr>
                            <td><strong style="color:#fff;" data-i18n="m1Name">Linux Wayland Direct (ext-sender)</strong></td>
                            <td><span class="proto-badge fast" data-i18n="m1Proto">Raw RTP H.264 (UDP 5000)</span></td>
                            <td style="color:var(--accent-emerald); font-weight:700;" data-i18n="m1Lat">&lt; 15 ms</td>
                            <td data-i18n="m1Use">Interactive desktop, fluid mouse, zero container overhead</td>
                            <td><code style="color:var(--accent-cyan);">ext-sender 192.168.7.2 5000 3000 extend auto 30</code></td>
                        </tr>
                        <tr>
                            <td><strong style="color:#fff;" data-i18n="m2Name">Linux Wayland Miracast (GNOME Displays)</strong></td>
                            <td><span class="proto-badge std" data-i18n="m2Proto">WFD RTSP 7236 + MPEG-TS (UDP 5002)</span></td>
                            <td style="color:var(--accent-cyan); font-weight:700;" data-i18n="m2Lat">40–70 ms</td>
                            <td data-i18n="m2Use">GNOME official UI tool, same protocol as Windows</td>
                            <td><code style="color:var(--accent-cyan);">gnome-network-displays</code></td>
                        </tr>
                        <tr>
                            <td><strong style="color:#fff;" data-i18n="m3Name">Windows 10/11 Wireless Cast (Win + K)</strong></td>
                            <td><span class="proto-badge std" data-i18n="m3Proto">MS-MICE RTSP 7236 + MPEG-TS (UDP 5002)</span></td>
                            <td style="color:var(--accent-cyan); font-weight:700;" data-i18n="m3Lat">40–70 ms</td>
                            <td data-i18n="m3Use">Native Windows projection without any extra drivers</td>
                            <td>Press <strong style="color:var(--accent-cyan);">Win + K</strong> & select Pi Zero</td>
                        </tr>
                        <tr>
                            <td><strong style="color:#fff;" data-i18n="m4Name">Mode 2: USB Bulk Direct (FunctionFS)</strong></td>
                            <td><span class="proto-badge usb" data-i18n="m4Proto">USB 2.0 Bulk 480 Mbps (Zero Network)</span></td>
                            <td style="color:var(--accent-purple); font-weight:700;" data-i18n="m4Lat">&lt; 1 ms</td>
                            <td data-i18n="m4Use">No network stack, direct high-speed hardware pipe</td>
                            <td><code style="color:var(--accent-purple);">ext-sender --transport=usb</code></td>
                        </tr>
                    </tbody>
                </table>
            </div>
        </div>

        <!-- Full-Width Card: Scripts & Diagnostic Commands -->
        <div class="glass-card">
            <div class="card-header">
                <div class="card-title">
                    <span>💻</span>
                    <span data-i18n="scriptsHeader">Diagnostic Scripts & Helper Commands</span>
                </div>
                <span class="card-badge">CLI Tools</span>
            </div>
            
            <p style="font-size: 0.85rem; color: var(--text-secondary); margin-top: 0.5rem;" data-i18n="scriptGnomeDesc">
                Launch GNOME Network Displays (Linux Miracast UI):
            </p>
            <div class="code-box">
                <button class="copy-btn" onclick="copyCode('cmdGnome')">Copy</button>
                <code id="cmdGnome">gnome-network-displays</code>
            </div>

            <p style="font-size: 0.85rem; color: var(--text-secondary); margin-top: 1rem;" data-i18n="scriptWfdDesc">
                Test WFD / Miracast RTSP handshake & streaming:
            </p>
            <div class="code-box">
                <button class="copy-btn" onclick="copyCode('cmdTestWfd')">Copy</button>
                <code id="cmdTestWfd">python3 /home/carlos/ide/ext-monitor/scripts/test-wfd-client.py 192.168.7.2</code>
            </div>

            <p style="font-size: 0.85rem; color: var(--text-secondary); margin-top: 1rem;" data-i18n="scriptModeDesc">
                Check Pi Zero subsystem status (Serial, Network, Video):
            </p>
            <div class="code-box">
                <button class="copy-btn" onclick="copyCode('cmdMode')">Copy</button>
                <code id="cmdMode">ext-mode status</code>
            </div>

            <p style="font-size: 0.85rem; color: var(--text-secondary); margin-top: 1rem;" data-i18n="scriptSenderDesc">
                Start Linux Wayland virtual monitor streaming:
            </p>
            <div class="code-box">
                <button class="copy-btn" onclick="copyCode('cmdSender')">Copy</button>
                <code id="cmdSender">ext-sender 192.168.7.2 5000 3000 extend auto 30</code>
            </div>

            <p style="font-size: 0.85rem; color: var(--text-secondary); margin-top: 1rem;" data-i18n="scriptStopDesc">
                Stop Linux sender process:
            </p>
            <div class="code-box">
                <button class="copy-btn" onclick="copyCode('cmdStop')">Copy</button>
                <code id="cmdStop">pkill -f ext-sender</code>
            </div>
        </div>
    </div>

    <script>
        // Internationalization dictionary (4 languages)
        const i18n = {
            en: {
                title: "Pi Zero Extended Display",
                subtitle: "Hardware GPU Video Receiver Dashboard",
                statusLive: "V4L2 M2M Online",
                statusPaused: "Display Paused",
                statusStopped: "Service Stopped",
                modeTitle: "Dual-Mode Display Engine: Mode 1 (Network + Miracast)",
                modeDesc: "Supporting Linux Wayland ScreenCast & Windows 10/11 native wireless casting (Win + K)",
                modeSwitch: "Switch to USB Bulk",
                powerHeader: "Panel & Stream Control",
                badgeState: "Session",
                powerDesc: "Manage display streaming or stop the service and web panel cleanly:",
                btnPauseStream: "⏸ Pause Stream",
                btnResumeStream: "▶ Resume Stream",
                btnStopService: "⏹ Stop Panel & Service",
                ctrlHeader: "Display & Stream Optimization",
                badgeZeroCopy: "VideoCore IV DMA",
                fpsLabel: "Framerate (FPS)",
                bitrateLabel: "Streaming Bitrate (VBR)",
                rateMin: "150 kbps (Min)",
                rateMax: "15 Mbps (Max)",
                colorLabel: "Color Profile",
                colorFull: "24-bit TrueColor",
                color256: "256-Color (QP 30-44)",
                colorGray: "Monochrome",
                hudLabel: "Diagnostic Telemetry HUD",
                btnShowHud: "✦ Show HUD (60s)",
                btnHideHud: "✕ Turn Off HUD",
                winCastHeader: "Windows 10/11 Wireless Cast (Win + K)",
                badgeNative: "Zero Drivers",
                winCastDesc: "Pi Zero acts as a native Miracast & MS-MICE receiver on TCP port 7236. To cast directly from Windows without installing any software:",
                winStep1: "Connect PC to the Pi Zero via USB cable (Ethernet gadget) or Wi-Fi.",
                winStep2: "Press Win + K to open the Cast flyout.",
                winStep3: "Select 'Pi Zero Wireless Display' to mirror or extend.",
                reqHeader: "Prerequisites & What is Needed",
                badgeGuide: "Guide",
                reqPi: "Pi Zero Hardware:",
                reqPiDesc: "Pi Zero W or Pi Zero 2 connected via Micro-USB data cable (OTG port).",
                reqGpu: "GPU VideoCore IV:",
                reqGpuDesc: "Broadcom V4L2 M2M hardware decoder (/dev/video10) & KMS DRM output.",
                reqHost: "Host System:",
                reqHostDesc: "Linux Wayland (GNOME Mutter) with VA-API GPU encoding, or Windows 10/11 with Miracast.",
                reqStopCli: "How to Stop/Restart:",
                reqStopCliDesc: "Run 'sudo systemctl stop ext-receiver' to stop daemon and panel. Run 'sudo systemctl start ext-receiver' to start.",
                telemetryHeader: "Hardware Telemetry",
                badgeRealtime: "Real-Time",
                statDecoder: "Decoder Engine",
                statSink: "Display Sink",
                statTemp: "SoC Temperature",
                statLatency: "Link Latency (RTT)",
                statCpu: "CPU Usage",
                statRam: "Free RAM",
                protoHeader: "Connection Methods & Protocol Comparison",
                protoBadge: "Multi-Protocol",
                protoDesc: "Pi Zero supports multiple concurrent streaming protocols. Choose the optimal method for your OS:",
                thMethod: "Method / Client",
                thProtocol: "Protocol & Port",
                thLatency: "Latency",
                thBestFor: "Ideal Use-Case",
                thCommand: "How to Connect",
                m1Name: "Linux Wayland Direct (ext-sender)",
                m1Proto: "Raw RTP H.264 (UDP 5000)",
                m1Lat: "< 15 ms",
                m1Use: "Interactive desktop, fluid mouse, zero container overhead",
                m2Name: "Linux Wayland Miracast (GNOME Displays)",
                m2Proto: "WFD RTSP 7236 + MPEG-TS (UDP 5002)",
                m2Lat: "40–70 ms",
                m2Use: "GNOME official UI tool, same protocol as Windows",
                m3Name: "Windows 10/11 Wireless Cast (Win + K)",
                m3Proto: "MS-MICE RTSP 7236 + MPEG-TS (UDP 5002)",
                m3Lat: "40–70 ms",
                m3Use: "Native Windows projection without any extra drivers",
                m4Name: "Mode 2: USB Bulk Direct (FunctionFS)",
                m4Proto: "USB 2.0 Bulk 480 Mbps (Zero Network)",
                m4Lat: "< 1 ms",
                m4Use: "No network stack, direct high-speed hardware pipe",
                scriptsHeader: "Diagnostic Scripts & Helper Commands",
                scriptGnomeDesc: "Launch GNOME Network Displays (Linux Miracast UI):",
                scriptWfdDesc: "Test WFD / Miracast RTSP handshake & streaming:",
                scriptModeDesc: "Check Pi Zero subsystem status (Serial, Network, Video):",
                scriptSenderDesc: "Start Linux Wayland virtual monitor streaming:",
                scriptStopDesc: "Stop Linux sender process:",
                netHeader: "Network & Interface Management (OTG Zero-Gateway & LAN/Wi-Fi)",
                netBadge: "Zero-Gateway DHCP + Static",
                netDesc: "Pi Zero runs a dedicated Zero-Gateway DHCP server on USB OTG (assigning 192.168.7.1 to your PC without breaking internet). For physical Ethernet/Wi-Fi, select DHCP or define a Static IP:",
                netOtgStatus: "Zero-Gateway Active",
                netPiIp: "Raspberry Pi IP:",
                netHostIp: "Host PC IP (DHCP Lease):",
                netGateway: "Default Gateway:",
                netNoneGw: "None (Direct Link - Safe for PC Internet)",
                netIpv6: "IPv6 Address:",
                netPhysBadge: "Adapter Config",
                netSelectIface: "Interface:",
                netModeLabel: "Mode:",
                netModeDhcp: "DHCP Client (Automatic from Router)",
                netModeStatic: "Static IP (Manual)",
                netStaticIp: "IPv4 Address:",
                netNetmask: "Subnet Mask:",
                netGatewayInput: "Gateway:",
                netDns: "DNS Servers:",
                netIpv6Mode: "IPv6 Mode:",
                netIpv6Auto: "Auto SLAAC / Link-Local",
                netIpv6Disable: "Disabled",
                netBtnApply: "💾 Apply Network Settings"
            },
            pt: {
                title: "Pi Zero Monitor Estendido",
                subtitle: "Painel de Controle e Recepção GPU por Hardware",
                statusLive: "V4L2 M2M Online",
                statusPaused: "Exibição Pausada",
                statusStopped: "Serviço Parado",
                modeTitle: "Motor Dual-Mode: Modo 1 (Rede USB + Miracast)",
                modeDesc: "Suporte simultâneo ao Linux Wayland e transmissão nativa Windows 10/11 (Win + K)",
                modeSwitch: "Alternar para USB Bulk",
                powerHeader: "Controle do Painel e Transmissão",
                badgeState: "Sessão",
                powerDesc: "Gerencie a exibição de tela ou pare o painel e serviço com segurança:",
                btnPauseStream: "⏸ Pausar Exibição",
                btnResumeStream: "▶ Retomar Exibição",
                btnStopService: "⏹ Parar Painel e Serviço",
                ctrlHeader: "Otimização de Exibição e Stream",
                badgeZeroCopy: "VideoCore IV DMA",
                fpsLabel: "Taxa de Quadros (FPS)",
                bitrateLabel: "Taxa de Bits (VBR Adaptativo)",
                rateMin: "150 kbps (Mín)",
                rateMax: "15 Mbps (Máx)",
                colorLabel: "Perfil de Cor",
                colorFull: "24-bit TrueColor",
                color256: "256 Cores (QP 30-44)",
                colorGray: "Monocromático",
                hudLabel: "Painel de Telemetria na Tela (HUD)",
                btnShowHud: "✦ Exibir HUD (60s)",
                btnHideHud: "✕ Desligar HUD",
                winCastHeader: "Transmissão Nativa Windows 10/11 (Win + K)",
                badgeNative: "Zero Drivers",
                winCastDesc: "O Pi Zero atua como receptor nativo Miracast & MS-MICE na porta TCP 7236. Para projetar do Windows sem instalar nenhum driver:",
                winStep1: "Conecte o PC ao Pi Zero via cabo USB ou Wi-Fi.",
                winStep2: "Pressione Win + K para abrir o menu de Transmissão.",
                winStep3: "Selecione 'Pi Zero Wireless Display' para estender ou duplicar.",
                reqHeader: "Pré-requisitos e O que é Necessário",
                badgeGuide: "Guia",
                reqPi: "Hardware Pi Zero:",
                reqPiDesc: "Pi Zero W ou Pi Zero 2 conectado via cabo Micro-USB de dados (porta OTG).",
                reqGpu: "GPU VideoCore IV:",
                reqGpuDesc: "Decodificador Broadcom V4L2 M2M (/dev/video10) e saída KMS DRM ativa.",
                reqHost: "Sistema Host:",
                reqHostDesc: "Linux Wayland (GNOME) com VA-API, ou Windows 10/11 com Miracast.",
                reqStopCli: "Como Parar/Reiniciar:",
                reqStopCliDesc: "Execute 'sudo systemctl stop ext-receiver' para parar o painel e daemon. 'sudo systemctl start ext-receiver' para iniciar.",
                telemetryHeader: "Telemetria de Hardware",
                badgeRealtime: "Tempo Real",
                statDecoder: "Motor de Decodificação",
                statSink: "Saída de Vídeo",
                statTemp: "Temperatura SoC",
                statLatency: "Latência de Link (RTT)",
                statCpu: "Carga da CPU",
                statRam: "RAM Disponível",
                protoHeader: "Comparativo de Protocolos e Métodos de Conexão",
                protoBadge: "Multi-Protocolo",
                protoDesc: "O Pi Zero suporta múltiplos protocolos de streaming em paralelo. Escolha o método ideal para o seu caso:",
                thMethod: "Método / Cliente",
                thProtocol: "Protocolo / Porta",
                thLatency: "Latência",
                thBestFor: "Caso de Uso Ideal",
                thCommand: "Como Executar",
                m1Name: "Linux Wayland Direto (ext-sender)",
                m1Proto: "Raw RTP H.264 (UDP 5000)",
                m1Lat: "< 15 ms",
                m1Use: "Desktop interativo, mouse fluido e menor latência",
                m2Name: "Linux Wayland Miracast (GNOME Displays)",
                m2Proto: "WFD RTSP 7236 + MPEG-TS (UDP 5002)",
                m2Lat: "40–70 ms",
                m2Use: "App oficial do GNOME, mesmo protocolo do Windows",
                m3Name: "Transmissão Windows 10/11 (Win + K)",
                m3Proto: "MS-MICE RTSP 7236 + MPEG-TS (UDP 5002)",
                m3Lat: "40–70 ms",
                m3Use: "Projeção nativa do Windows sem instalar drivers",
                m4Name: "Modo 2: USB Bulk Direto (FunctionFS)",
                m4Proto: "USB 2.0 Bulk 480 Mbps (Zero Rede)",
                m4Lat: "< 1 ms",
                m4Use: "Sem overhead de rede, canal de hardware direto",
                scriptsHeader: "Scripts e Comandos de Diagnóstico",
                scriptGnomeDesc: "Abrir o GNOME Network Displays (Miracast no Linux):",
                scriptWfdDesc: "Testar o handshake WFD / Miracast RTSP:",
                scriptModeDesc: "Verificar status dos subsistemas (Serial, Rede, Vídeo):",
                scriptSenderDesc: "Iniciar streaming do monitor virtual no Linux:",
                scriptStopDesc: "Parar processo do sender no Linux:",
                netHeader: "Gerenciador de Redes e Adaptadores (OTG Zero-Gateway & LAN/Wi-Fi)",
                netBadge: "DHCP Zero-Gateway + IP Estático",
                netDesc: "O Pi Zero executa um servidor DHCP Zero-Gateway no USB OTG (atribuindo 192.168.7.1 ao PC sem derrubar sua internet). Para Ethernet/Wi-Fi físico, escolha DHCP ou IP estático:",
                netOtgStatus: "Zero-Gateway Ativo",
                netPiIp: "IP do Raspberry Pi:",
                netHostIp: "IP do PC Host (DHCP Automático):",
                netGateway: "Gateway Padrão:",
                netNoneGw: "Nenhum (Ponto a Ponto - Seguro para a Internet do PC)",
                netIpv6: "Endereço IPv6:",
                netPhysBadge: "Config. de Adaptador",
                netSelectIface: "Interface de Rede:",
                netModeLabel: "Modo de IP:",
                netModeDhcp: "Cliente DHCP (Automático do Roteador)",
                netModeStatic: "IP Estático (Manual)",
                netStaticIp: "Endereço IPv4:",
                netNetmask: "Máscara de Sub-rede:",
                netGatewayInput: "Gateway:",
                netDns: "Servidores DNS:",
                netIpv6Mode: "Modo IPv6:",
                netIpv6Auto: "Auto SLAAC / Link-Local",
                netIpv6Disable: "Desativado",
                netBtnApply: "💾 Aplicar Configurações de Rede"
            },
            it: {
                title: "Pi Zero Monitor Esteso",
                subtitle: "Pannello di Controllo Ricevitore GPU Hardware",
                statusLive: "V4L2 M2M Online",
                statusPaused: "Display Sospeso",
                statusStopped: "Servizio Arrestato",
                modeTitle: "Motore Dual-Mode: Modalità 1 (Rete USB + Miracast)",
                modeDesc: "Supporto simultaneo per Linux Wayland e proiezione nativa Windows 10/11 (Win + K)",
                modeSwitch: "Passa a USB Bulk",
                powerHeader: "Controllo Pannello e Trasmissione",
                badgeState: "Sessione",
                powerDesc: "Gestisci il flusso a schermo o arresta il pannello e il servizio in modo pulito:",
                btnPauseStream: "⏸ Sospendi Display",
                btnResumeStream: "▶ Riprendi Display",
                btnStopService: "⏹ Arresta Pannello e Servizio",
                ctrlHeader: "Ottimizzazione Display e Streaming",
                badgeZeroCopy: "VideoCore IV DMA",
                fpsLabel: "Frequenza Fotogrammi (FPS)",
                bitrateLabel: "Bitrate di Streaming (VBR)",
                rateMin: "150 kbps (Min)",
                rateMax: "15 Mbps (Max)",
                colorLabel: "Profilo Colore",
                colorFull: "TrueColor 24-bit",
                color256: "256 Colori (QP 30-44)",
                colorGray: "Monocromatico",
                hudLabel: "Telemetria a Schermo (HUD)",
                btnShowHud: "✦ Mostra HUD (60s)",
                btnHideHud: "✕ Spegni HUD",
                winCastHeader: "Proiezione Wireless Windows 10/11 (Win + K)",
                badgeNative: "Zero Driver",
                winCastDesc: "Il Pi Zero funziona come ricevitore nativo Miracast & MS-MICE su porta 7236. Per trasmettere da Windows senza software:",
                winStep1: "Collega il PC al Pi Zero tramite cavo USB o Wi-Fi.",
                winStep2: "Premi Win + K per aprire il menu Trasmetti.",
                winStep3: "Seleziona 'Pi Zero Wireless Display' per connetterti.",
                reqHeader: "Prerequisiti e Informazioni Necessarie",
                badgeGuide: "Guida",
                reqPi: "Hardware Pi Zero:",
                reqPiDesc: "Pi Zero W o Pi Zero 2 collegato via cavo Micro-USB (porta OTG).",
                reqGpu: "GPU VideoCore IV:",
                reqGpuDesc: "Decoder hardware Broadcom V4L2 M2M (/dev/video10) e output KMS DRM.",
                reqHost: "Sistema Host:",
                reqHostDesc: "Linux Wayland (GNOME) con accelerazione VA-API, o Windows 10/11 con Miracast.",
                reqStopCli: "Come Arrestare/Riavviare:",
                reqStopCliDesc: "Esegui 'sudo systemctl stop ext-receiver' per arrestare pannello e demone. 'sudo systemctl start ext-receiver' per avviare.",
                telemetryHeader: "Telemetria Hardware",
                badgeRealtime: "Tempo Reale",
                statDecoder: "Motore Decoder",
                statSink: "Uscita Display",
                statTemp: "Temperatura SoC",
                statLatency: "Latenza Link (RTT)",
                statCpu: "Carico CPU",
                statRam: "RAM Libera",
                protoHeader: "Confronto Metodi di Connessione e Protocolli",
                protoBadge: "Multi-Protocollo",
                protoDesc: "Il Pi Zero supporta molteplici protocolli di streaming in parallelo. Scegli il metodo ideale:",
                thMethod: "Metodo / Client",
                thProtocol: "Protocollo / Porta",
                thLatency: "Latenza",
                thBestFor: "Uso Consigliato",
                thCommand: "Come Avviare",
                m1Name: "Linux Wayland Diretto (ext-sender)",
                m1Proto: "Raw RTP H.264 (UDP 5000)",
                m1Lat: "< 15 ms",
                m1Use: "Desktop interattivo, mouse fluido, bassissima latenza",
                m2Name: "Linux Wayland Miracast (GNOME Displays)",
                m2Proto: "WFD RTSP 7236 + MPEG-TS (UDP 5002)",
                m2Lat: "40–70 ms",
                m2Use: "App ufficiale GNOME, stesso protocollo di Windows",
                m3Name: "Proiezione Windows 10/11 (Win + K)",
                m3Proto: "MS-MICE RTSP 7236 + MPEG-TS (UDP 5002)",
                m3Lat: "40–70 ms",
                m3Use: "Proiezione nativa Windows senza alcun driver",
                m4Name: "Modalità 2: USB Bulk Diretto (FunctionFS)",
                m4Proto: "USB 2.0 Bulk 480 Mbps (Zero Rete)",
                m4Lat: "< 1 ms",
                m4Use: "Zero overhead di rete, canale hardware diretto",
                scriptsHeader: "Script e Comandi di Diagnostica",
                scriptGnomeDesc: "Apri GNOME Network Displays (Miracast su Linux):",
                scriptWfdDesc: "Test handshake WFD / Miracast RTSP:",
                scriptModeDesc: "Verifica stato sottosistemi (Seriale, Rete, Video):",
                scriptSenderDesc: "Avvia streaming monitor virtuale Linux:",
                scriptStopDesc: "Arresta processo sender su Linux:",
                netHeader: "Gestione Rete e Adattatori (OTG Zero-Gateway & LAN/Wi-Fi)",
                netBadge: "DHCP Zero-Gateway + IP Statico",
                netDesc: "Il Pi Zero esegue un server DHCP Zero-Gateway su USB OTG (assegna 192.168.7.1 al PC senza interrompere la connessione internet). Per Ethernet/Wi-Fi, scegli DHCP o IP statico:",
                netOtgStatus: "Zero-Gateway Attivo",
                netPiIp: "IP del Raspberry Pi:",
                netHostIp: "IP del PC Host (Assegnato DHCP):",
                netGateway: "Gateway Predefinito:",
                netNoneGw: "Nessuno (Punto-Punto - Sicuro per Internet del PC)",
                netIpv6: "Indirizzo IPv6:",
                netPhysBadge: "Config. Adattatore",
                netSelectIface: "Interfaccia:",
                netModeLabel: "Modalità IP:",
                netModeDhcp: "Client DHCP (Automatico dal Router)",
                netModeStatic: "IP Statico (Manuale)",
                netStaticIp: "Indirizzo IPv4:",
                netNetmask: "Maschera di Sottorete:",
                netGatewayInput: "Gateway:",
                netDns: "Server DNS:",
                netIpv6Mode: "Modalità IPv6:",
                netIpv6Auto: "Auto SLAAC / Link-Local",
                netIpv6Disable: "Disabilitato",
                netBtnApply: "💾 Applica Impostazioni di Rete"
            },
            zh: {
                title: "树莓派 Zero 扩展显示屏",
                subtitle: "硬件 GPU 视频接收器控制仪表盘",
                statusLive: "V4L2 M2M 在线",
                statusPaused: "显示画面已暂停",
                statusStopped: "服务已停止",
                modeTitle: "双模式引擎: 模式 1 (网络 + Miracast 投屏)",
                modeDesc: "完美支持 Linux Wayland 扩展屏幕与 Windows 10/11 原生无线投屏 (Win + K)",
                modeSwitch: "切换至 USB Bulk 直通",
                powerHeader: "控制面板与画面推流管理",
                badgeState: "运行状态",
                powerDesc: "管理当前显示推流状态，或彻底停止后台服务及 Web 控制面板:",
                btnPauseStream: "⏸ 暂停画面推流",
                btnResumeStream: "▶ 恢复画面推流",
                btnStopService: "⏹ 停止控制面板及服务",
                ctrlHeader: "显示与传输参数优化",
                badgeZeroCopy: "VideoCore IV DMA 零拷贝",
                fpsLabel: "帧率 (FPS)",
                bitrateLabel: "流码率 (自适应 VBR)",
                rateMin: "150 kbps (极速)",
                rateMax: "15 Mbps (高清)",
                colorLabel: "色彩配置文件",
                colorFull: "24位 全彩 (TrueColor)",
                color256: "256色 粗量化 (省带宽)",
                colorGray: "黑白单色 (极低延迟)",
                hudLabel: "屏幕诊断遥测浮层 (HUD)",
                btnShowHud: "✦ 开启 HUD 浮层 (60秒)",
                btnHideHud: "✕ 立即关闭 HUD",
                winCastHeader: "Windows 10/11 原生无线投屏 (Win + K)",
                badgeNative: "免驱即用",
                winCastDesc: "树莓派 Zero 在 TCP 端口 7236 运行原生 Miracast / MS-MICE 服务。无需在电脑上安装任何驱动即可直连:",
                winStep1: "使用 USB 数据线或 Wi-Fi 将电脑与树莓派连接至同一局域网。",
                winStep2: "按下快捷键 Win + K 打开 Windows 投屏浮窗。",
                winStep3: "在设备列表中点击 'Pi Zero Wireless Display' 即可一键扩展或复制屏幕。",
                reqHeader: "运行前提条件与环境说明",
                badgeGuide: "运行指南",
                reqPi: "树莓派硬件:",
                reqPiDesc: "树莓派 Zero W 或 Zero 2，通过 Micro-USB 数据线连接至主机 OTG 端口。",
                reqGpu: "GPU VideoCore IV 核心:",
                reqGpuDesc: "博通 V4L2 M2M 硬件解码器 (/dev/video10) 及 KMS DRM 显示输出已激活。",
                reqHost: "主机操作系统:",
                reqHostDesc: "支持 VA-API GPU 加速的 Linux Wayland (GNOME)，或支持 Miracast 的 Windows 10/11。",
                reqStopCli: "如何停止/重启服务:",
                reqStopCliDesc: "运行 'sudo systemctl stop ext-receiver' 停止面板和服务；运行 'sudo systemctl start ext-receiver' 启动。",
                telemetryHeader: "硬件运行状态遥测",
                badgeRealtime: "实时监控",
                statDecoder: "硬件解码核心",
                statSink: "输出显示接口",
                statTemp: "SoC 核心温度",
                statLatency: "传输延迟 (RTT)",
                statCpu: "CPU 占用率",
                statRam: "空闲内存",
                protoHeader: "连接协议与模式深度对比",
                protoBadge: "多协议支持",
                protoDesc: "树莓派 Zero 支持并行运行多种流媒体协议，请根据操作系统选择最佳连接方式:",
                thMethod: "连接方式 / 客户端",
                thProtocol: "网络协议 / 端口",
                thLatency: "传输延迟",
                thBestFor: "最佳应用场景",
                thCommand: "启动方式",
                m1Name: "Linux Wayland 直推 (ext-sender)",
                m1Proto: "原生 RTP H.264 (UDP 5000)",
                m1Lat: "< 15 ms",
                m1Use: "交互式桌面办公、流畅光标、超低延迟",
                m2Name: "Linux Wayland Miracast (GNOME 网络显示)",
                m2Proto: "WFD RTSP 7236 + MPEG-TS (UDP 5002)",
                m2Lat: "40–70 ms",
                m2Use: "GNOME 官方图形投屏工具，与 Windows 协议完全一致",
                m3Name: "Windows 10/11 原生投屏 (Win + K)",
                m3Proto: "MS-MICE RTSP 7236 + MPEG-TS (UDP 5002)",
                m3Lat: "40–70 ms",
                m3Use: "Windows 系统免驱原生投屏，一键连接",
                m4Name: "模式 2: USB Bulk 直通 (FunctionFS)",
                m4Proto: "USB 2.0 Bulk 480 Mbps (完全绕过网络栈)",
                m4Lat: "< 1 ms",
                m4Use: "零网络开销，纯硬件通道高速推流",
                scriptsHeader: "诊断脚本与实用命令大全",
                scriptGnomeDesc: "启动 GNOME Network Displays (Linux 原生 Miracast 图形界面):",
                scriptWfdDesc: "测试 WFD / Miracast RTSP 握手与推流状态:",
                scriptModeDesc: "检查树莓派子系统状态 (串口、网络、视频):",
                scriptSenderDesc: "在 Linux 上启动虚拟扩展屏推流:",
                scriptStopDesc: "在 Linux 终端中停止推流进程:",
                netHeader: "网络与网络适配器管理 (OTG 免网关与局域网/Wi-Fi)",
                netBadge: "免网关 DHCP + 静态 IP",
                netDesc: "Pi Zero 在 USB OTG 上运行专用免网关 DHCP 服务（自动为电脑分配 192.168.7.1 且不影响电脑原有外网）。对于实体网卡或 Wi-Fi，可自由配置 DHCP 或静态 IP：",
                netOtgStatus: "免网关服务运行中",
                netPiIp: "树莓派 IP 地址:",
                netHostIp: "电脑主机 IP (DHCP 自动分配):",
                netGateway: "默认网关:",
                netNoneGw: "无网关 (点对点直连 - 不影响电脑外网)",
                netIpv6: "IPv6 地址:",
                netPhysBadge: "适配器配置",
                netSelectIface: "网卡接口:",
                netModeLabel: "IP 模式:",
                netModeDhcp: "DHCP 客户端 (从路由器自动获取)",
                netModeStatic: "静态 IP (手动设定)",
                netStaticIp: "IPv4 地址:",
                netNetmask: "子网掩码:",
                netGatewayInput: "网关:",
                netDns: "DNS 服务器:",
                netIpv6Mode: "IPv6 模式:",
                netIpv6Auto: "自动 SLAAC / 链路本地",
                netIpv6Disable: "禁用",
                netBtnApply: "💾 应用网络配置"
            }
        };

        let currentLang = 'pt';
        function setLanguage(lang) {
            currentLang = lang;
            const dict = i18n[lang] || i18n.en;
            document.querySelectorAll('[data-i18n]').forEach(el => {
                const key = el.getAttribute('data-i18n');
                if (dict[key]) {
                    el.textContent = dict[key];
                }
            });
            document.getElementById('langSelect').value = lang;
            localStorage.setItem('ext_lang', lang);
        }

        document.getElementById('langSelect').addEventListener('change', (e) => {
            setLanguage(e.target.value);
        });

        const savedLang = localStorage.getItem('ext_lang') || 'pt';
        setLanguage(savedLang);

        // Control state
        let currentFps = 30;
        let currentBitrate = 3000;
        let currentColor = 'full';

        function sendConfig(payload) {
            fetch('/api/config', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(payload)
            }).catch(e => console.error("Error sending config:", e));
        }

        // FPS selection
        document.querySelectorAll('#fpsGrid .btn-toggle').forEach(btn => {
            btn.addEventListener('click', () => {
                document.querySelectorAll('#fpsGrid .btn-toggle').forEach(b => b.classList.remove('active'));
                btn.classList.add('active');
                currentFps = parseInt(btn.getAttribute('data-fps'));
                document.getElementById('valFps').textContent = currentFps + ' FPS';
                sendConfig({ fps: currentFps });
            });
        });

        // Bitrate slider
        const slider = document.getElementById('bitrateSlider');
        slider.addEventListener('input', () => {
            currentBitrate = parseInt(slider.value);
            document.getElementById('valBitrate').textContent = currentBitrate + ' kbps';
        });
        slider.addEventListener('change', () => {
            sendConfig({ bitrate: currentBitrate });
        });

        // Color profiles
        document.querySelectorAll('#colorGrid .btn-toggle').forEach(btn => {
            btn.addEventListener('click', () => {
                document.querySelectorAll('#colorGrid .btn-toggle').forEach(b => b.classList.remove('active'));
                btn.classList.add('active');
                currentColor = btn.getAttribute('data-color');
                const labels = { full: '24-bit TrueColor', '256': '256-Color (QP 30-44)', gray: 'Monochrome' };
                document.getElementById('valColor').textContent = labels[currentColor];
                sendConfig({ color: currentColor });
            });
        });

        // HUD buttons
        document.getElementById('btnTriggerHud').addEventListener('click', () => {
            sendConfig({ action: 'trigger_hud' });
        });
        document.getElementById('btnHideHud').addEventListener('click', () => {
            sendConfig({ action: 'hide_hud' });
        });

        // Stream Control Buttons
        document.getElementById('btnPauseStream').addEventListener('click', () => {
            fetch('/api/stream/stop', { method: 'POST' }).then(() => {
                const badge = document.getElementById('mainStatusBadge');
                badge.className = 'status-badge paused';
                document.getElementById('systemStatus').textContent = (i18n[currentLang] || i18n.en).statusPaused;
                showServiceMsg('Stream paused.');
            }).catch(e => console.error(e));
        });

        document.getElementById('btnResumeStream').addEventListener('click', () => {
            fetch('/api/stream/start', { method: 'POST' }).then(() => {
                const badge = document.getElementById('mainStatusBadge');
                badge.className = 'status-badge';
                document.getElementById('systemStatus').textContent = (i18n[currentLang] || i18n.en).statusLive;
                showServiceMsg('Stream resumed.');
            }).catch(e => console.error(e));
        });

        document.getElementById('btnStopService').addEventListener('click', () => {
            const confirmMsg = currentLang === 'pt' ? 'Tem certeza que deseja parar o painel e o serviço receptor?' :
                             currentLang === 'it' ? 'Sei sicuro di voler arrestare il pannello e il servizio?' :
                             currentLang === 'zh' ? '确定要停止 Web 控制面板及接收器后台服务吗？' :
                             'Are you sure you want to stop the web panel and receiver service?';
            if (confirm(confirmMsg)) {
                fetch('/api/service/stop', { method: 'POST' }).then(() => {
                    const badge = document.getElementById('mainStatusBadge');
                    badge.className = 'status-badge stopped';
                    document.getElementById('systemStatus').textContent = (i18n[currentLang] || i18n.en).statusStopped;
                    const restartNote = currentLang === 'pt' ? 'Serviço parado. Para reiniciar via terminal: sudo systemctl start ext-receiver' :
                                       currentLang === 'it' ? 'Servizio arrestato. Per riavviare: sudo systemctl start ext-receiver' :
                                       currentLang === 'zh' ? '服务已停止。如需在终端重启，请运行: sudo systemctl start ext-receiver' :
                                       'Service stopped. To restart via terminal: sudo systemctl start ext-receiver';
                    showServiceMsg(restartNote, true);
                }).catch(e => console.error(e));
            }
        });

        function showServiceMsg(msg, persistent = false) {
            const el = document.getElementById('serviceStatusMessage');
            el.textContent = msg;
            el.style.display = 'block';
            if (!persistent) {
                setTimeout(() => { el.style.display = 'none'; }, 4000);
            }
        }

        // Mode switch
        document.getElementById('modeSwitchBtn').addEventListener('click', () => {
            fetch('/api/mode', { method: 'POST' }).then(() => {
                location.reload();
            }).catch(e => console.error("Error switching mode:", e));
        });

        function copyCode(id) {
            const text = document.getElementById(id).innerText;
            navigator.clipboard.writeText(text);
        }

        // Poll telemetry
        setInterval(() => {
            fetch('/api/status').then(r => r.json()).then(data => {
                if (data.temp) document.getElementById('statTemp').textContent = data.temp + ' °C';
                if (data.cpu) document.getElementById('statCpu').textContent = data.cpu;
                if (data.ram) document.getElementById('statRam').textContent = data.ram + ' MB';
            }).catch(() => {});
        }, 2000);

        // Network configuration UI handling
        const netSelectMode = document.getElementById('netSelectMode');
        const staticNetFields = document.getElementById('staticNetFields');
        if (netSelectMode && staticNetFields) {
            netSelectMode.addEventListener('change', (e) => {
                staticNetFields.style.display = e.target.value === 'static' ? 'block' : 'none';
            });
        }

        const btnApplyNetwork = document.getElementById('btnApplyNetwork');
        if (btnApplyNetwork) {
            btnApplyNetwork.addEventListener('click', () => {
                const payload = {
                    iface: document.getElementById('netSelectIface').value,
                    mode: document.getElementById('netSelectMode').value,
                    ip: document.getElementById('netInputIp').value,
                    netmask: document.getElementById('netInputMask').value,
                    gateway: document.getElementById('netInputGw').value,
                    dns: document.getElementById('netInputDns').value,
                    ipv6: document.getElementById('netSelectIpv6').value
                };
                const msgEl = document.getElementById('netStatusMessage');
                msgEl.style.display = 'block';
                msgEl.style.color = 'var(--accent-amber)';
                msgEl.textContent = 'Applying network configuration...';

                fetch('/api/network', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(payload)
                }).then(r => r.json()).then(() => {
                    msgEl.style.color = 'var(--accent-emerald)';
                    msgEl.textContent = '✓ Network settings applied successfully!';
                    setTimeout(() => { msgEl.style.display = 'none'; }, 4000);
                    pollNetworkStatus();
                }).catch(err => {
                    msgEl.style.color = 'var(--accent-red)';
                    msgEl.textContent = '✗ Error applying settings: ' + err;
                });
            });
        }

        function pollNetworkStatus() {
            fetch('/api/network').then(r => r.json()).then(data => {
                if (data.usb0) {
                    if (data.usb0.ipv4) document.getElementById('netUsb0Ip').textContent = data.usb0.ipv4 + ' / 24';
                    if (data.usb0.ipv6) document.getElementById('netUsb0Ipv6').textContent = data.usb0.ipv6;
                }
                const curIface = document.getElementById('netSelectIface').value;
                const ifaceData = data[curIface];
                const badge = document.getElementById('netPhysStatus');
                if (ifaceData && ifaceData.detected) {
                    badge.className = 'proto-badge fast';
                    badge.textContent = ifaceData.ipv4 && ifaceData.ipv4 !== 'disconnected' ? ifaceData.ipv4 : 'Connected';
                } else {
                    badge.className = 'proto-badge std';
                    badge.textContent = 'Disconnected';
                }
            }).catch(() => {});
        }

        pollNetworkStatus();
        setInterval(pollNetworkStatus, 4000);
    </script>
</body>
</html>
"#;
