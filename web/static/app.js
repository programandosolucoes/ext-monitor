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

    // Suggested bitrates per framerate
    const suggestedBitrates = {
        12: 2500,
        15: 3500,
        24: 5000,
        30: 6000,
        60: 8000,
    };

    // Update Command Box
    function updateCommandPreview() {
        let cmd = `./scripts/start.sh extend auto ${state.fps} hud`;
        if (state.color === '256') {
            cmd += ' 256';
        } else if (state.color === 'gray') {
            cmd += ' gray';
        }
        cmdPreview.textContent = cmd;
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
                bitrateVal.textContent = `${state.bitrate} kbps (Auto)`;
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
        bitrateVal.textContent = `${state.bitrate} kbps`;
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
                    btnTriggerHud.textContent = '👁️ Reativar HUD na Tela (60 segundos)';
                }, 3000);
            }
        } catch (err) {
            btnTriggerHud.disabled = false;
            btnTriggerHud.textContent = '👁️ Reativar HUD na Tela (60 segundos)';
        }
    });

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

    // Copy Command to Clipboard
    btnCopyCmd.addEventListener('click', () => {
        const text = cmdPreview.textContent;
        navigator.clipboard.writeText(text).then(() => {
            const original = btnCopyCmd.textContent;
            btnCopyCmd.textContent = '✓ Copiado!';
            btnCopyCmd.style.background = 'var(--accent-emerald)';
            btnCopyCmd.style.color = '#000';
            setTimeout(() => {
                btnCopyCmd.textContent = original;
                btnCopyCmd.style.background = '';
                btnCopyCmd.style.color = '';
            }, 2500);
        });
    });

    // Initial setups
    updateCommandPreview();
    fetchTelemetry();
    setInterval(fetchTelemetry, 1000);
});
