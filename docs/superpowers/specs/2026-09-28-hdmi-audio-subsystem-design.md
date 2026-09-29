# Especificação Técnica: Sub дву Sub-Sistema de Áudio HDMI de Baixa Latência (ext-audio)

**Projeto:** ext-monitor  
**Autor:** Carlos Alberto <psncarlosalberto4ti@gmail.com>  
**Data:** 2026-09-28  
**Status:** Proposto / Planejamento  

---

## 1. Visão Geral e Objetivos

O `ext-monitor` atualmente entrega transmissão de vídeo com aceleração de hardware H.264 para o Raspberry Pi Zero com latência inferior a 15ms. O objetivo desta especificação é dotar o sistema de transmissão e reprodução de **Áudio HDMI Digital em Tempo Real**, transformando o monitor/TV secundário conectado ao Pi Zero em uma saída de áudio completa (áudio e vídeo sincronizados - A/V Sync).

### Objetivos Primários:
1. **Latência Inaudível (< 25 ms):** Manter o áudio sincronizado com o fluxo de vídeo H.264 (lip-sync adequado para vídeos no YouTube, chamadas de vídeo e reprodução multimídia).
2. **Saída Nativa HDMI no BCM2835:** Saída via conector mini-HDMI para alto-falantes da TV/Monitor através do driver ALSA `vc4-hdmi` / `snd-bcm2835`.
3. **Pia Virtual de Áudio no Host (PipeWire / PulseAudio):** Criação de um dispositivo de áudio dedicado `ext-monitor (HDMI Audio)` no Linux, permitindo rotear aplicativos individuais ou todo o sistema operacional.
4. **Codec Ultra-Eficiente (Opus 48kHz Stereo):** Consumo mínimo de CPU (< 1% no monocore ARM1176 do Pi Zero) e economia extrema de banda de rede/USB (64–96 kbps).
5. **Suporte a Múltiplos Modos:**
   - **Modo 1 (Linux Wayland):** RTP Opus via UDP na porta 5002.
   - **Modo 2 (Miracast Windows):** Desmultiplexação do stream de áudio AAC / LPCM do contêiner MPEG-TS do Wi-Fi Display.
   - **Modo 3 (USB Bulk Direct):** Quadros de áudio multiplexados ou endpoint dedicado.

---

## 2. Arquitetura do Sistema de Áudio

```
[ HOST LINUX (PipeWire / PulseAudio) ]
  ├── Dispositivo Virtual de Áudio: "ext-monitor HDMI Audio" (module-null-sink)
  ├── Captor de Fluxo: PipeWire Audio Streamer / GStreamer pulsesrc
  └── Encoder: Opus Encoder (48 kHz, Stereo, 64-96 kbps, Frame 10ms)
        │
        │ UDP RTP Porta 5002 (ou multiplexação USB Bulk)
        ▼
[ RASPBERRY PI ZERO (Appliance 100% RAM) ]
  ├── Receptor Ingress: UDP Socket Porta 5002
  ├── Jitter Buffer Adaptativo: 20-30ms (Prevenção de Underrun)
  ├── Decoder: Opus Decoder / ALSA Demux
  └── ALSA PCM Output: `/dev/snd/pcmC0D0p` (Driver vc4-hdmi -> Cabo HDMI -> TV)
```

---

## 3. Detalhes de Implementação

### 3.1 Appliance Kernel e Boot (`build-appliance/boot/config.txt`)
Atualmente, o `config.txt` contém:
```ini
dtoverlay=vc4-kms-v3d,cma-128,noaudio,nocomposite
```
A diretiva `noaudio` deve ser removida para ativar o codec de áudio HDMI no driver DRM/KMS:
```ini
dtoverlay=vc4-kms-v3d,cma-128,nocomposite
dtparam=audio=on
```
No `initramfs/init` e `scripts/appliance-init.sh`, carregar os módulos ALSA:
- `snd`
- `snd-pcm`
- `snd-timer`
- `snd-bcm2835`
- `snd-soc-hdmi-codec`

### 3.2 Host: Emissor (`ext-sender` & scripts)
1. **Configuração PipeWire / PulseAudio:**
   - Script cria a pia de áudio sob demanda:
     `pactl load-module module-null-sink sink_name=ext_monitor_audio sink_properties=device.description="ext-monitor (HDMI Audio)"`
   - Opção `--audio` ou `--audio=mirror` (espelha áudio principal) vs `--audio=sink` (dispositivo independente).
2. **Pipeline de Áudio GStreamer / Native:**
   ```bash
   pulsesrc device=ext_monitor_audio.monitor ! audioconvert ! audioresample ! audio/x-raw,rate=48000,channels=2 ! opusenc bitrate=96000 frame-size=10 complexity=3 ! rtpopuspay pt=97 ! udpsink host=192.168.7.2 port=5002 sync=false
   ```

### 3.3 Receptor (`ext-receiver`)
1. **Módulo de Áudio (`receiver/src/audio/`):**
   - Ingress UDP escutando na porta 5002.
   - Gerenciamento de volume digital e Mute via API Web (`POST /api/audio/volume`).
   - Pipeline de decodificação ALSA para `hw:0,0` com buffers pequenos (10-20ms) para latência mínima.
2. **Miracast (WFD Windows):**
   - No `receiver/src/stream/ts.rs`, extrair o PID de áudio (AAC ou LPCM) e encaminhar ao pipeline de áudio ALSA.

---

## 4. Próximos Passos
1. Modificação do `config.txt` e verificação da criação do nó ALSA HDMI `/dev/snd/pcmC0D0p`.
2. Criação do módulo de áudio no `sender` e argumentos CLI (`--audio`).
3. Implementação do receptor de áudio no `receiver`.
4. Integração de controle de volume na interface Web (Dashboard 8080).
