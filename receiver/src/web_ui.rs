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

                <!-- Windows Casting Instructions -->
                <div class="glass-card">
                    <div class="card-header">
                        <div class="card-title">
                            <span>🪟</span>
                            <span data-i18n="winCastHeader">Windows 10/11 Wireless Cast (Win + K)</span>
                        </div>
                        <span class="card-badge" data-i18n="badgeNative">Zero Drivers</span>
                    </div>
                    <p style="font-size: 0.85rem; color: var(--text-secondary); line-height: 1.6;" data-i18n="winCastDesc">
                        Pi Zero acts as a native Miracast & MS-MICE receiver on TCP port 7236. To cast directly from Windows without installing any software:
                    </p>
                    <ol style="margin-left: 1.2rem; margin-top: 0.6rem; font-size: 0.85rem; color: var(--text-primary); line-height: 1.7;" id="winCastSteps">
                        <li data-i18n="winStep1">Connect PC to the Pi Zero via USB cable (Ethernet gadget) or Wi-Fi.</li>
                        <li data-i18n="winStep2">Press <strong style="color: var(--accent-cyan);">Win + K</strong> to open the Cast flyout.</li>
                        <li data-i18n="winStep3">Select <strong>"Pi Zero Wireless Display"</strong> to mirror or extend.</li>
                    </ol>
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

                <!-- Linux Sender Connection Command -->
                <div class="glass-card">
                    <div class="card-header">
                        <div class="card-title">
                            <span>🐧</span>
                            <span data-i18n="linuxHeader">Linux Wayland Host Command</span>
                        </div>
                        <span class="card-badge">Wayland</span>
                    </div>
                    <p style="font-size: 0.85rem; color: var(--text-secondary);" data-i18n="linuxDesc">
                        Run ext-sender on your Linux host to start streaming the virtual extended monitor:
                    </p>
                    <div class="code-box">
                        <button class="copy-btn" onclick="copyCode('linuxCmd')">Copy</button>
                        <code id="linuxCmd">ext-sender 192.168.7.2 5000 3000 extend auto 30</code>
                    </div>

                    <p style="font-size: 0.85rem; color: var(--text-secondary); margin-top: 1rem;" data-i18n="linuxStopDesc">
                        To stop streaming from Linux host terminal:
                    </p>
                    <div class="code-box">
                        <button class="copy-btn" onclick="copyCode('linuxStopCmd')">Copy</button>
                        <code id="linuxStopCmd">pkill -f ext-sender</code>
                    </div>
                </div>
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
                linuxHeader: "Linux Wayland Host Command",
                linuxDesc: "Run ext-sender on your Linux host to start streaming the virtual extended monitor:",
                linuxStopDesc: "To stop streaming from Linux host terminal:"
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
                linuxHeader: "Comando para Host Linux Wayland",
                linuxDesc: "Execute o ext-sender no Linux para iniciar o streaming do monitor virtual:",
                linuxStopDesc: "Para parar a transmissão pelo terminal do Linux:"
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
                linuxHeader: "Comando per Host Linux Wayland",
                linuxDesc: "Esegui ext-sender su Linux per avviare il monitor virtuale:",
                linuxStopDesc: "Per arrestare lo streaming dal terminale di Linux:"
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
                linuxHeader: "Linux Wayland 主机端启动命令",
                linuxDesc: "在 Linux 主机上运行 ext-sender 即可开启虚拟显示器推流:",
                linuxStopDesc: "在 Linux 终端中停止推流的命令:"
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
    </script>
</body>
</html>
"#;
