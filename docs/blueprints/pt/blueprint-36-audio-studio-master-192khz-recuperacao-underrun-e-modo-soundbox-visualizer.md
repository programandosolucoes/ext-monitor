# Blueprint 36: Áudio Studio Master 192 kHz, Recuperação Anti-Underrun ALSA e Modo Invertido Soundbox + Visualizador

## 1. Visão Geral e Objetivos

Este documento formaliza a implementação, estabilização e validação empírica do subsistema de áudio digital HDMI em **192.000 Hz (192 kHz / 16-bit Stereo PCM encapsulado em IEC 60958-3 Subframes)**, bem como o **Modo Invertido (Soundbox / Level 2)** onde a tela virtual do PC é desligada e a TV opera como receptor de som de altíssima fidelidade com visualizador gráfico de espectro FFT em tempo real.

---

## 2. Diagnóstico: O Desafio do Buffer Underrun (XRUN) em Altas Frequências

Ao operar o subsistema de áudio a 192.000 amostras por segundo:
1. **Consumo Acelerado do Buffer:** Em 192 kHz, cada frame (2 canais × 32 bits = 8 bytes) consome o buffer ALSA 4 vezes mais rápido do que a 48 kHz.
2. **Bloqueio do Driver VC4 HDMI (`MAI PCM i2s-hifi-0`):** Quando ocorre um underrun de rede (XRUN / `EPIPE`), o hardware ALSA entra em estado suspenso. Apenas invocar `SNDRV_PCM_IOCTL_PREPARE` pode falhar silenciosamente se houver dados residuais no FIFO do MAI.
3. **Solução Anti-Freeze Implementada:**
   - **Sequência Atômica de Recuperação:** Antes de `PREPARE`, o subsistema agora dispara incondicionalmente `SNDRV_PCM_IOCTL_DROP`, esvaziando o hardware e restabelecendo o streaming em < 2 ms sem travamentos.
   - **Dimensionamento Ótimo do Buffer:** Buffer ALSA ampliado para **16.384 frames** (8 períodos de 2.048 frames), garantindo 85,3 ms de resiliência contra oscilações de jitter de rede.

---

## 3. Arquitetura de Sincronização Ponta a Ponta

```
[ PC Host: PipeWire / PulseAudio ]
  │
  ├─ Sink: Raspberry_Pi_HDMI_Audio (float32le 2ch 192000Hz)
  │
  ├─ Native Audio Recorder (96-192 kHz PCM S16LE)
  │     │
  │     ├─ UDP 5004: Chunks PCM (256 frames = 1.33 ms)
  │     └─ UDP 5006: FFT Spectrum Telemetry (24 Bins + RMS VU Meter)
  ▼
[ Raspberry Pi Zero W: ext-receiver ]
  │
  ├─ Audio Ingress (UDP 5004) ──► ALSA IEC958 Subframe Encoder ──► /dev/snd/pcmC0D0p (192 kHz)
  │                                                                       │
  ├─ Telemetry Ingress (UDP 5006) ──► update_audio_spectrum()             ▼
  │                                         │                      [ HDMI TV Speakers ]
  │                                         ▼
  └─ Level 2 Visualizer Engine ─────► Blit /dev/fb0 (30 FPS) ──────► [ HDMI TV Screen ]
```

---

## 4. Testes Unitários Implementados e Validados

Foram adicionados testes unitários cobrindo 100% dos cenários críticos:
1. `test_iec958_rate_codes`: Validação determinística dos códigos IEC 60958-3 para 32k, 44.1k, 48k, 88.2k, 96k, 176.4k e 192k.
2. `test_audio_buffer_and_period_sizing`: Cálculo e dimensionamento de buffer ALSA por taxa de amostragem.
3. `test_audio_status_json_and_state_mutations`: Mutações atômicas de volume, mudo, sample rate e transporte.
4. `test_audio_transport_serialization`: Enums e conversões entre string, u8 e formato de transporte.
5. `test_evaluate_explicit_standby_transport`: Garantia de colapso de displays e transição para standby via API.
6. `test_parse_control_audio_rate_192khz`: Parsing e hot-apply de taxa de 192 kHz via socket de controle.

**Resultado da suíte na Workspace:** **224 testes passando (0 falhas).**
