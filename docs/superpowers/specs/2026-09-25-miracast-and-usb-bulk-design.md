# Especificação Técnica: Wireless Display (Miracast / MS-MICE) e Modo 2 (USB Bulk Direct)

**Data:** 25 de Setembro de 2026  
**Status:** Aprovado para Implementação  
**Alvos:** Raspberry Pi Zero (armv6l) e Host Linux / Windows 10/11  

---

## 1. Visão Geral

Este documento detalha o design técnico e a implementação de duas capacidades fundamentais para o ecossistema `ext-monitor`:

1. **Fase 1: Receptor Wireless Display Nativo (Miracast / MS-MICE) no Pi Zero**  
   Permite que qualquer máquina Windows 10 ou Windows 11 conecte nativamente através do atalho de teclado `Win + K` (Transmitir / Conectar Tela Sem Fio), sem necessidade de instalar nenhum driver, executável ou configuração no Windows. O Pi Zero atua como um receptor Wi-Fi Display (WFD Sink) com aceleração de hardware Broadcom VideoCore IV (H.264 M2M).

2. **Fase 2: Modo 2 — Dispositivo USB Display Direto (USB Bulk Transfer via FunctionFS)**  
   Elimina completamente a camada TCP/IP, UDP e sockets de rede entre o host Linux e o Pi Zero. Utiliza o subsistema de Gadget USB do Linux (`configfs` + `FunctionFS` / `f_fs`) para expor endpoints USB Bulk dedicados. No host Linux, o `ext-sender` utiliza `rusb` (bindings Rust para `libusb-1.0`) para escrever os quadros H.264 codificados por hardware VA-API diretamente no endpoint USB. O sistema mantém alternância (Dual-Mode) transparente com o Modo 1 (Rede USB IP).

---

## 2. Fase 1: Arquitetura Miracast / MS-MICE (Windows Nativo Win + K)

### 2.1 Protocolo MS-MICE e WFD

O Windows 10 (versão 1703+) e o Windows 11 utilizam o padrão **Miracast over Infrastructure (MS-MICE)** quando estão na mesma rede IP (seja via cabo USB RNDIS/CDC-ECM ou via Wi-Fi):

```
+------------------------+                        +------------------------+
|       Host Windows     |                        |  Raspberry Pi Zero     |
| (Transmissor / Source) |                        |   (Receptor / Sink)    |
+------------------------+                        +------------------------+
            |                                                 |
            | ----- 1. mDNS Query (_display._tcp) ----------> |
            | <---- 2. mDNS Announce ("Pi Zero Display") ---- | (Avahi / Port 7236)
            |                                                 |
            | ===== 3. TCP Connect (Port 7236) =============> | (wfd-sink daemon)
            | ----- M1: OPTIONS ----------------------------> |
            | <---- RTSP 200 OK (Supported Methods) --------- |
            | <---- M2: OPTIONS ----------------------------- |
            | ----- RTSP 200 OK ----------------------------> |
            | ----- M3: GET_PARAMETER (wfd_video_formats) --> |
            | <---- RTSP 200 OK (H264 1080p/720p, Port 5002)- |
            | ----- M4: SET_PARAMETER (Selects Mode) -------> |
            | <---- RTSP 200 OK ----------------------------- |
            | ----- M5: SET_PARAMETER (Trigger SETUP) ------> |
            | <---- RTSP 200 OK ----------------------------- |
            | <---- M6: SETUP rtsp://... (UDP Port 5002) ---- |
            | ----- RTSP 200 OK (Session: 12345678) --------> |
            | <---- M7: PLAY rtsp://... --------------------- |
            | ----- RTSP 200 OK ----------------------------> |
            |                                                 |
            | ===== 4. RTP / MPEG-TS Stream (UDP:5002) =====> |
            |    [Intel QuickSync / NVENC / AMD AMF]          | GStreamer Hardware:
            |                                                 | v4l2h264dec -> kmssink
```

### 2.2 Descoberta (mDNS / DNS-SD)
- Serviço publicado no Avahi da Pi Zero:
  - Tipo: `_display._tcp` e `_miracast._tcp`
  - Porta: `7236`
  - Registros TXT:
    - `name=Pi Zero Wireless Display`
    - `source-id=<MAC Address>`
    - `flags=0x01`
    - `wfd=0006001000000000` (Tipo: Primary Sink, Session Available)

### 2.3 Handshake RTSP WFD Daemon (`wfd-sink.py`)
- Escuta TCP na porta `7236` em todas as interfaces (`0.0.0.0`).
- Processa as mensagens M1 a M7:
  - **M1**: Responde métodos suportados (`org.wfa.wfd1.0`, `GET_PARAMETER`, `SET_PARAMETER`, etc.).
  - **M2**: Envia `OPTIONS` para o Windows e obtém confirmação.
  - **M3**: Informa resoluções CEA/VESA suportadas (1920x1080p60/30, 1280x720p60), H.264 Constrained Baseline / Main Profile, porta RTP `5002`, áudio opcional LPCM/nenhum, e proteção HDCP desativada (`wfd_content_protection: none`).
  - **M4/M5**: Confirmação de parâmetros e gatilho de `SETUP`.
  - **M6**: Envia `SETUP` com `Transport: RTP/AVP/UDP;unicast;client_port=5002`.
  - **M7**: Envia `PLAY` iniciando a sessão.
  - **TEARDOWN**: Encerra o pipeline de vídeo e libera recursos.

### 2.4 Pipeline GStreamer para Miracast
O Windows encapsula o vídeo H.264 dentro de MPEG-TS sobre pacotes RTP (`application/x-rtp,encoding-name=MP2T`):
```bash
gst-launch-1.0 -v udpsrc port=5002 buffer-size=524288 caps="application/x-rtp,media=video,clock-rate=90000,encoding-name=MP2T" \
  ! rtpmp2tdepay \
  ! tsdemux \
  ! h264parse \
  ! v4l2h264dec capture-io-mode=dmabuf output-io-mode=dmabuf qos=true \
  ! queue max-size-buffers=1 max-size-bytes=0 max-size-time=0 leaky=downstream \
  ! kmssink sync=false qos=true skip-vsync=true
```

### 2.5 Tratamento de Operação Headless no DRM KMS
Quando o Pi Zero está conectado sem monitor físico HDMI:
- O driver DRM KMS desliga o connector `card0-HDMI-A-1`.
- Para evitar que o `kmssink` falhe com erro `Could not get allowed GstCaps of device / driver does not provide mode settings configuration`:
  - O daemon do receptor assegura automaticamente `echo on > /sys/class/drm/card0-HDMI-A-1/status`.
  - O connector assume status `connected` com resolução forçada (1024x768 ou 1920x1080), permitindo inicialização contínua e estável da decodificação por hardware mesmo sem monitor físico.

---

## 3. Fase 2: Arquitetura Modo 2 (USB Bulk Direct via FunctionFS)

### 3.1 Gadget USB do Linux (`configfs` + `FunctionFS`)
Ao contrário do driver GUD (que enviava pixels RGB565 descompactados e saturava a CPU ARM1176 do Pi Zero em 100%), nosso Modo 2 transfere o **fluxo H.264 já comprimido** pelo hardware VA-API do host:

```
[Host Linux]                                              [Raspberry Pi Zero]
+-------------------------+                               +------------------------+
| ext-sender (Rust)       |                               | ext-usb-receiver (Rust)|
| - VA-API H.264 Encoder  |                               | - FunctionFS ep1 reader|
| - rusb (libusb-1.0)     |                               | - V4L2 M2M H.264 dec   |
+-------------------------+                               | - KMS DRM display      |
            |                                             +------------------------+
            | Bulk OUT (Endpoint 1: 512 bytes / microframe)            |
            | =======================================================> |
            |                                                          |
            | <======================================================= |
            | Bulk IN (Endpoint 2: Telemetria / FPS / Drop ACK)        |
```

### 3.2 Estrutura do Gadget USB
- **ConfigFS:** `/sys/kernel/config/usb_gadget/g_display/`
- **Vendor ID:** `0x1d6b` (Linux Foundation)
- **Product ID:** `0x0104` (Multifunction / Custom Display Gadget)
- **Função:** `ffs.display` montada em `/dev/usb-ffs/display`
- **Descritores FunctionFS:**
  - Interface 0: `Vendor Specific`
  - Endpoint 1 (OUT): Bulk, MaxPacketSize 512 bytes (High-Speed USB 2.0 480 Mbps)
  - Endpoint 2 (IN): Bulk, MaxPacketSize 512 bytes
- **Largura de Banda e Latência:**
  - Largura de banda teórica USB 2.0: 480 Mbps (~40-45 MB/s efetivos).
  - Fluxo H.264: 5 Mbps a 15 Mbps (< 2 MB/s).
  - Overhead de barramento: < 5% da capacidade USB 2.0.
  - Latência de transferência: ~0.1 ms (eliminando completamente buffers de stack de rede do kernel).

### 3.3 Alternância entre Modos (Dual-Mode Switcher)
- Modo 1 (Padrão): `network` (`g_multi` / `g_ether` - RNDIS + CDC-ECM + Web UI + RTSP Miracast).
- Modo 2: `usb-bulk` (`g_display` - FunctionFS com endpoints dedicados).
- Um comando simples (`ext-mode switch <network|usb-bulk>`) e um botão no Painel Web executam a troca de modo desacoplando e religando o UDC (`dwc2`).

---

## 4. Estratégia de Verificação e Testes Headless

1. **Validação Miracast M1-M7:**
   - Script de teste em Python emulando cliente RTSP Windows para verificar sequência M1->M7 e recebimento do RTP stream.
   - Verificação mDNS com `avahi-browse -r _display._tcp`.
2. **Validação do Pipeline Hardware Headless:**
   - Forçamento de status do conector DRM HDMI (`echo on > /sys/class/drm/card0-HDMI-A-1/status`).
   - Verificação de logs do `ext-receiver` e `wfd-sink` via journalctl.
3. **Validação FunctionFS e rusb:**
   - Criação do gadget `f_fs` no Pi Zero e abertura dos endpoints.
   - Teste de transferência de pacotes de benchmark e NAL units via `ext-sender --transport=usb`.
