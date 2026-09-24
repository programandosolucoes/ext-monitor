// Pi Zero GPU Display Engine • Control Dashboard Frontend Logic

document.addEventListener('DOMContentLoaded', () => {
    // State
    const state = {
        fps: 12,
        color: '256',
        bitrate: 2500,
        autoBitrate: true,
        autoHideHud: true,
        cpuHistory: new Array(25).fill(20),
    };

    // DOM Elements
    const elVpu = document.getElementById('val-vpu');
    const elThrottled = document.getElementById('val-throttled');
    const elCpu = document.getElementById('val-cpu');
    const elChipCpu = document.getElementById('chip-cpu');
    const elTemp = document.getElementById('val-temp');
    const elTempFill = document.getElementById('temp-fill');
    const elRamUsed = document.getElementById('val-ram-used');
    const elRamFree = document.getElementById('val-ram-free');
    
    const fpsButtons = document.querySelectorAll('#fps-group .btn-select');
    const colorButtons = document.querySelectorAll('#color-group .btn-select');
    const bitrateSlider = document.getElementById('bitrate-slider');
    const bitrateVal = document.getElementById('bitrate-val');
    const checkAutoHide = document.getElementById('check-auto-hide');
    const btnTriggerHud = document.getElementById('btn-trigger-hud');
    const btnApply = document.getElementById('btn-apply-config');
    const applyStatus = document.getElementById('apply-status');
    const cmdPreview = document.getElementById('cmd-preview');
    const btnCopyCmd = document.getElementById('btn-copy-cmd');
    const canvasCpu = document.getElementById('chart-cpu');
    const ctxCpu = canvasCpu ? canvasCpu.getContext('2d') : null;

    // Suggested bitrates per framerate (Ultra-Low Latency / Realtime tuned)
    const suggestedBitrates = {
        12: 1200,
        15: 1500,
        24: 2500,
        30: 3000,
        60: 5000,
    };


    const cmdCurl = document.getElementById('cmd-curl');

    // Update Command Boxes
    function updateCommandPreview() {
        let localCmd = `./scripts/start.sh extend auto ${state.fps} hud`;
        if (state.color === '256') {
            localCmd += ' 256';
        } else if (state.color === 'gray') {
            localCmd += ' gray';
        }
        if (cmdPreview) cmdPreview.textContent = localCmd;

        let curlCmd = `curl -sSL http://192.168.7.2:8080/connect.sh | bash`;
        if (state.fps !== 12 || state.color !== '256') {
            curlCmd = `curl -sSL http://192.168.7.2:8080/connect.sh | bash -s -- extend ${state.fps} ${state.color} hud`;
        }
        if (cmdCurl) cmdCurl.textContent = curlCmd;
    }

    // Canvas Sparkline Chart
    function drawCpuChart() {
        if (!ctxCpu) return;
        const w = canvasCpu.width;
        const h = canvasCpu.height;
        ctxCpu.clearRect(0, 0, w, h);

        const data = state.cpuHistory;
        const step = w / (data.length - 1);

        // Gradient line
        const grad = ctxCpu.createLinearGradient(0, 0, 0, h);
        grad.addColorStop(0, 'rgba(0, 255, 102, 0.4)');
        grad.addColorStop(1, 'rgba(0, 255, 102, 0.0)');

        ctxCpu.beginPath();
        ctxCpu.moveTo(0, h - (data[0] / 100) * h);
        for (let i = 1; i < data.length; i++) {
            const x = i * step;
            const y = h - (data[i] / 100) * h;
            ctxCpu.lineTo(x, y);
        }
        ctxCpu.strokeStyle = '#00ff66';
        ctxCpu.lineWidth = 2;
        ctxCpu.stroke();

        // Fill area
        ctxCpu.lineTo(w, h);
        ctxCpu.lineTo(0, h);
        ctxCpu.fillStyle = grad;
        ctxCpu.fill();
    }

    // Telemetry Poller
    async function fetchTelemetry() {
        try {
            const res = await fetch('/api/status');
            if (!res.ok) return;
            const data = await res.json();

            // Update UI
            if (elVpu) elVpu.textContent = data.vpu_freq_mhz || 500;
            if (elThrottled) elThrottled.textContent = data.throttled || '0x0 (OK)';
            
            const cpuVal = data.cpu_percent !== undefined ? data.cpu_percent : 22.2;
            if (elCpu) elCpu.textContent = cpuVal.toFixed(1);
            if (elChipCpu) elChipCpu.textContent = `${cpuVal.toFixed(0)}% Carga`;

            state.cpuHistory.push(cpuVal);
            state.cpuHistory.shift();
            drawCpuChart();

            const tempVal = data.temp_c || 49.2;
            if (elTemp) elTemp.textContent = tempVal.toFixed(1);
            if (elTempFill) {
                const pct = Math.min(100, Math.max(10, (tempVal / 85) * 100));
                elTempFill.style.width = `${pct}%`;
            }

            if (elRamUsed) elRamUsed.textContent = Math.round(data.ram_used_mb || 141);
            if (elRamFree) elRamFree.textContent = `${Math.round(data.ram_free_mb || 222)} MiB`;
        } catch (e) {
            // Server offline or network issue
        }
    }

    // Helper to format bitrate label cleanly without causing line breaks
    function updateBitrateLabel(val) {
        let tag = '';
        if (val <= 400) {
            tag = ' (Ultra Leve • Sub-10ms)';
        } else if (val <= 1000) {
            tag = ' (Realtime • 15ms)';
        } else if (val <= 2000) {
            tag = ' (Equilibrado)';
        } else {
            tag = ' (Alta Fidelidade)';
        }
        if (bitrateVal) bitrateVal.textContent = `${val} kbps${tag}`;
    }

    // Button Selection Listeners (FPS)
    fpsButtons.forEach(btn => {
        btn.addEventListener('click', () => {
            fpsButtons.forEach(b => b.classList.remove('active'));
            btn.classList.add('active');
            state.fps = parseInt(btn.dataset.fps, 10);

            // Auto-update bitrate slider
            if (suggestedBitrates[state.fps]) {
                state.bitrate = suggestedBitrates[state.fps];
                bitrateSlider.value = state.bitrate;
                updateBitrateLabel(state.bitrate);
            }

            updateCommandPreview();
        });
    });

    // Button Selection Listeners (Color)
    colorButtons.forEach(btn => {
        btn.addEventListener('click', () => {
            colorButtons.forEach(b => b.classList.remove('active'));
            btn.classList.add('active');
            state.color = btn.dataset.color;
            updateCommandPreview();
        });
    });

    // Bitrate Slider
    bitrateSlider.addEventListener('input', (e) => {
        state.bitrate = parseInt(e.target.value, 10);
        updateBitrateLabel(state.bitrate);
        updateCommandPreview();
    });


    // Trigger HUD on display
    btnTriggerHud.addEventListener('click', async () => {
        btnTriggerHud.disabled = true;
        btnTriggerHud.textContent = '⏳ Ativando HUD...';
        try {
            const res = await fetch('/api/hud/trigger', { method: 'POST' });
            if (res.ok) {
                btnTriggerHud.textContent = '✓ HUD Ativo por 60s!';
                setTimeout(() => {
                    btnTriggerHud.disabled = false;
                    btnTriggerHud.textContent = '👁️ Exibir HUD (60s)';
                }, 3000);
            }
        } catch (err) {
            btnTriggerHud.disabled = false;
            btnTriggerHud.textContent = '👁️ Exibir HUD (60s)';
        }
    });

    // Hide HUD immediately on display
    const btnHideHud = document.getElementById('btn-hide-hud');
    if (btnHideHud) {
        btnHideHud.addEventListener('click', async () => {
            btnHideHud.disabled = true;
            btnHideHud.textContent = '⏳ Desligando...';
            try {
                const res = await fetch('/api/hud/hide', { method: 'POST' });
                if (res.ok) {
                    btnHideHud.textContent = '✓ HUD Desligado!';
                    setTimeout(() => {
                        btnHideHud.disabled = false;
                        btnHideHud.textContent = '✕ Desligar HUD';
                    }, 2500);
                }
            } catch (err) {
                btnHideHud.disabled = false;
                btnHideHud.textContent = '✕ Desligar HUD';
            }
        });
    }


    // Apply Config Button (Hot-Apply)
    btnApply.addEventListener('click', async () => {
        btnApply.disabled = true;
        applyStatus.textContent = 'Aplicando a quente...';
        applyStatus.style.color = 'var(--accent-cyan)';

        try {
            const payload = {
                fps: state.fps,
                color: state.color,
                bitrate: state.bitrate,
                hud: true,
                auto_hide: checkAutoHide.checked
            };

            const res = await fetch('/api/config', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(payload)
            });

            if (res.ok) {
                applyStatus.textContent = '✓ Aplicado com sucesso!';
                applyStatus.style.color = 'var(--accent-emerald)';
                setTimeout(() => { applyStatus.textContent = ''; }, 4000);
            } else {
                applyStatus.textContent = 'Erro ao salvar.';
                applyStatus.style.color = 'var(--accent-red)';
            }
        } catch (e) {
            applyStatus.textContent = 'Erro de comunicação.';
            applyStatus.style.color = 'var(--accent-red)';
        } finally {
            btnApply.disabled = false;
        }
    });

    // Copy Commands to Clipboard
    document.querySelectorAll('.btn-copy').forEach(btn => {
        btn.addEventListener('click', () => {
            const targetId = btn.getAttribute('data-target') || 'cmd-preview';
            const targetEl = document.getElementById(targetId);
            if (!targetEl) return;
            const text = targetEl.textContent.trim();
            navigator.clipboard.writeText(text).then(() => {
                const original = btn.textContent;
                btn.textContent = '✓ Copiado!';
                btn.style.background = 'var(--accent-emerald)';
                btn.style.color = '#000';
                setTimeout(() => {
                    btn.textContent = original;
                    btn.style.background = '';
                    btn.style.color = '';
                }, 2500);
            });
        });
    });


    // Initial setups
    updateCommandPreview();
    fetchTelemetry();
    setInterval(fetchTelemetry, 1000);
});
