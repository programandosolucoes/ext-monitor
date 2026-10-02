# Compêndio de Blueprints Técnicos de Engenharia — `ext-monitor`

> 🇧🇷 Versão em Português | [🇺🇸 English Version](README.md)

**Projeto:** `ext-monitor` — Monitor Secundário USB de Ultra-Baixa Latência  
**Plataforma Alvo:** Raspberry Pi Zero W / Zero 2 W / Raspberry Pi 4 / PC Linux & Windows  
**Autor:** Carlos Alberto <carlosalberto4ti@gmail.com>  
**Data:** Setembro de 2026  
**Status do Projeto:** Estável, Pronto para Produção, Compilado em Rust Nativo Puro  

---

## 1. Visão Geral da Suíte de Documentos

### A Fundação (Blueprints Originais 01 a 20):
Todos os desafios fundamentais do appliance base — desde o alinhamento de setores FAT16 no silício BCM2835 até o scanout zero-copy DRM/KMS no VideoCore IV, a recuperação autônoma de suspensão de energia no Linux Wayland, o motor visual quadrilíngue de splash, a telemetria EDID de monitores reais, a captura direta de hardware DRM/KMS scanout, o controle web bidirecional em Rust, o áudio híbrido IP/Bluetooth, o ecossistema IoT Media Renderer e o multiplexador HDMI com FFT em tempo real — estão formalizados nos 20 blueprints fundamentais iniciais.

### A Suíte de Expansão (Blueprints 21 a 25):
Após a conclusão do appliance base, o escopo de engenharia foi expandido para integrar **projeção sem fio nativa sem drivers (Wi-Fi Display / Miracast / MS-MICE)** para Windows 10/11 (`Win + K`) e desktops Linux. Essa expansão introduziu 5 blueprints avançados cobrindo:
* **BP 21:** Sinalização binária MS-MICE, negociação reversa RTSP WFD e parser dinâmico de dimensões SPS.
* **BP 22:** Alinhamento de macrobloco de 16 linhas NV12 no KMS, eliminando a faixa verde superior e reduzindo a latência do demuxer para 2ms.
* **BP 23:** Detecção determinística e ranking automatizado de GPUs do host (AMD Mendocino/RDNA, Intel QuickSync, NVIDIA NVENC) via sysfs com tooltips na UI.
* **BP 24:** Imposição nativa do formato CEA índice 6 (1280x720p60 Nível 3.1) e demuxer MPEG-TS determinista demarcado por PES.
* **BP 25:** Parada unificada de standby multi-serviço, teardown de sockets TCP RTSP ativos, encerramento de transmissores do host (`SIGTERM`/`SIGKILL`) e tela de splash Ready persistente no HDMI.

---

## 2. Mapa dos 25 Blueprints de Engenharia

| # | Blueprint Técnico | Arquivo | Foco de Engenharia |
| :---: | :--- | :--- | :--- |
| **01** | **Imagem de 32MB e Geometria do BCM2835** | [`01-imagem-32mb-e-geometria-bcm2835.md`](pt/01-imagem-32mb-e-geometria-bcm2835.md) | Bug dos 65.525 clusters da ROM Broadcom, alinhamento no setor 1, partições FAT16 de 2KB por cluster e boot 100% em RAM. |
| **02** | **Arquitetura Fim-a-Fim e Comparativo com o Projeto GUD** | [`02-arquitetura-transmissor-receptor-e-comparativo-gud.md`](pt/02-arquitetura-transmissor-receptor-e-comparativo-gud.md) | Por que o GUD falha (100% CPU, saturação USB com LZ4) e como o pipeline H.264 V4L2 M2M atinge 60 FPS com 0.8% de CPU. |
| **03** | **Transmissão de Pacotes, Fragmentação NALU e Leaky Queue** | [`03-transmissao-pacotes-drop-on-late-e-pipeline.md`](pt/03-transmissao-pacotes-drop-on-late-e-pipeline.md) | MTU de 1472 bytes, fragmentação FU-A RFC 6184, descarte de pacotes atrasados (*drop-on-late*) e drenagem pós-sono. |
| **04** | **PipeWire, Mutter D-Bus e Auto-Recuperação Pós-Suspensão** | [`04-pipewire-mutter-screencast-e-wayland.md`](pt/04-pipewire-mutter-screencast-e-wayland.md) | Captura virtual sem dongle, zero-copy DMA-BUF, relógio anti-ociosidade e **Supervisor com Duplo Watchdog em Rust** para sono/S3. |
| **05** | **Otimizações de CPU em Rust, GPU/VPU e Upscaler CAS** | [`05-otimizacoes-cpu-rust-gpu-vpu-e-cas-scaler.md`](pt/05-otimizacoes-cpu-rust-gpu-vpu-e-cas-scaler.md) | Flags `-C target-cpu=arm1176jzf-s`, codificação por silício VA-API/NVENC e filtros de nitidez CAS vs FSR para texto. |
| **06** | **Modos Concorrentes de Operação e Super-Gadget USB ConfigFS** | [`06-modos-de-operacao-concorrentes-e-usb-gadget.md`](pt/06-modos-de-operacao-concorrentes-e-usb-gadget.md) | 3 modos simultâneos (Linux UDP, Windows Miracast Win+K, USB Bulk), alocação de 7 endpoints DWC2, UDC auto-binding e chaves de controle. |
| **07** | **Cartão Micro-SD em RAM, Proteção e Upgrade via USB** | [`07-cartao-sd-em-ram-e-upgrade-usb.md`](pt/07-cartao-sd-em-ram-e-upgrade-usb.md) | Eliminação de corrupção flash, partição exposta como pendrive `EXTMONITOR` na USB, upgrade em 3 métodos e entrega de binários. |
| **08** | **Manual de Instalação, Portabilidade e Script de Setup Host** | [`08-manual-de-instalacao-e-portabilidade-host.md`](pt/08-manual-de-instalacao-e-portabilidade-host.md) | Conexão em 1 clique via `curl connect.sh`, instalador multi-distro, regras udev de baixa latência e console serial `/dev/ttyACM0`. |
| **09** | **Testes Empíricos em Hardware e Diagnósticos de Boot** | [`09-testes-empiricos-e-diagnosticos-hardware.md`](pt/09-testes-empiricos-e-diagnosticos-hardware.md) | Validação no Pi Zero v1.3 Monocore, diagnóstico da tela arco-íris, cronometria de boot de 1.8s e benchmarks térmicos/cores. |
| **10** | **Refatoração Modular Clean Code, Enquadramento AU e V4L2 M2M** | [`10-refatoracao-modular-clean-code-v4l2-m2m-e-enquadramento-au.md`](pt/10-refatoracao-modular-clean-code-v4l2-m2m-e-enquadramento-au.md) | Decomposição SRP, Builder Pattern, correção ABI 32-bit ARM ioctls (`0xC0CC5605`), remontagem RFC 6184/4571 e conversão SIMD YUV->RGB565. |
| **11** | **Scanout Zero-Copy via DRM/KMS, DMA-BUF e Correção ioctl -EFAULT** | [`11-kms-drm-dma-buf-scanout-zero-copy-e-correcao-efault.md`](pt/11-kms-drm-dma-buf-scanout-zero-copy-e-correcao-efault.md) | Resolução do bug `os error 14`, ativação de Universal Planes no VideoCore IV (`vc4-drm`), importação DMA-BUF direta sem cópia por CPU e double-buffering seguro. |
| **12** | **Splash Quadrilíngue, Telemetria EDID Realtime e Ciclo de Vida de Desconexão** | [`12-splash-quadrilingue-telemetria-edid-realtime-e-ciclo-vida-desconexao.md`](pt/12-splash-quadrilingue-telemetria-edid-realtime-e-ciclo-vida-desconexao.md) | Prevenção de imagem congelada na desconexão, telas de splash embutidas em Rust (PT/EN/ES/FR) via `miniz_oxide` e decodificação do hardware VESA EDID no painel web. |
| **13** | **Captura Direta no Kernel Linux DRM/KMS e Dual-Engine Universal** | [`13-captura-direta-drm-kms-e-dual-engine-universal.md`](pt/13-captura-direta-drm-kms-e-dual-engine-universal.md) | Bypassa GNOME Mutter/Wayland, extração atômica PRIME DMA-BUF do scanout da GPU AMD/Intel/NVIDIA via ioctl `GETFB2`, eliminação de congelamentos por oclusão de janelas e suporte multi-monitor genérico. |
| **14** | **Subsistema de Áudio HDMI Digital, Codec Opus e ALSA** | [`14-subsistema-audio-hdmi-digital-opus-alsa-e-sincronismo-av.md`](pt/14-subsistema-audio-hdmi-digital-opus-alsa-e-sincronismo-av.md) | Transmissão de áudio digital de ultra-baixa latência (< 25ms) via PipeWire e Opus 48kHz estéreo na porta UDP 5004, decodificação ALSA `vc4-hdmi`, sincronia A/V e controle Web de volume. |
| **15** | **Pipeline Zero-Copy, Quiescência Wayland e Ciclo de Vida do Display** | [`15-pipeline-zero-copy-quiescencia-wayland-e-ciclo-vida-display.md`](pt/15-pipeline-zero-copy-quiescencia-wayland-e-ciclo-vida-display.md) | Resolução do congelamento de quadros por inatividade do cursor, sincronismo vblank e reativação dinâmica do pipeline. |
| **16** | **Pacer Wayland de Quiescência, Cross-ARMv6 e Receptores Universais** | [`16-pacer-wayland-quiescencia-e-guia-universal-receptores.md`](pt/16-pacer-wayland-quiescencia-e-guia-universal-receptores.md) | Agente heartbeat 60 FPS sintético para YouTube contínuo, compilação estática musl/cross e guia para Pi 2/3/4/5 e PCs secundários. |
| **17** | **Agente Host Rust e Controle Web Bidirecional** | [`17-agente-host-rust-controle-web-bidirecional.md`](pt/17-agente-host-rust-controle-web-bidirecional.md) | Arquitetura RPC UDP 5001, controle completo do host pelo painel web, proteção contra Mutter stride assert crash (`SIGABRT 6`) e quirks AMD DCN 3.1. |
| **18** | **Áudio Híbrido: Rede IP Opus e Bluetooth A2DP Sink** | [`18-audio-hibrido-rede-opus-e-bluetooth-a2dp.md`](pt/18-audio-hibrido-rede-opus-e-bluetooth-a2dp.md) | Isolamento de áudio local (headset USB Yealink UH34) vs TV HDMI, pareamento Bluetooth 4.1 no Pi Zero W e roteamento ALSA `hw:0,0`. |
| **19** | **Appliance IoT Media Renderer: Google Cast, UPnP/DLNA e Visualizador HDMI** | [`19-iot-media-renderer-chromecast-upnp-e-visualizador-hdmi.md`](pt/19-iot-media-renderer-chromecast-upnp-e-visualizador-hdmi.md) | Transformação em dongle multimídia inteligente: Google Cast (CastV2), DIAL (YouTube), UPnP/DLNA e tela visual com capa/espectro sonoro em áudio IoT. |
| **20** | **Multiplexador HDMI Scanout, FFT Realtime, i18n e Arquitetura Zero-Reboot** | [`20-multiplexador-hdmi-scanout-fft-realtime-i18n-e-zero-reboot.md`](pt/20-multiplexador-hdmi-scanout-fft-realtime-i18n-e-zero-reboot.md) | Exclusão mútua do scanout HDMI único (vídeo vs visualizador), motor FFT 512 pontos com Hann, Canvas 30 FPS, i18n quadrilíngue 230 chaves, Swagger OAS 3.0 v2.3.0 e ciclo de vida zero-reboot contra travamentos estilo USB Bulk. |
| **21** | **Miracast MS-MICE, Conexão Reversa RTSP WFD, Conflito UDP 5002 e SPS 1080p** | [`21-miracast-ms-mice-rtsp-wfd-sps-dinamico-e-gnome-network-displays.md`](pt/21-miracast-ms-mice-rtsp-wfd-sps-dinamico-e-gnome-network-displays.md) | Sinalização binária MS-MICE (TCP 7250), conexão reversa RTSP WFD ao Source (7236), migração da auto-descoberta para UDP 5005, resolução do bug de 1080p do GNOME Displays via parser nativo de SPS e escala de hardware KMS. |
| **22** | **Alinhamento NV12 no KMS, Fim da Faixa Verde e Otimizações Realtime no Miracast** | [`22-correcao-faixa-verde-nv12-stride-alinhamento-e-otimizacoes-realtime-miracast.md`](pt/22-correcao-faixa-verde-nv12-stride-alinhamento-e-otimizacoes-realtime-miracast.md) | Alinhamento de macrobloco de 16 linhas (1080->1088), correção de 15.360 bytes no offset UV do KMS, descarte do padding, demuxer MPEG-TS sem atraso via PUSI, polling de 2ms e aceleração por hardware VA-API AMD Radeon 610M. |
| **23** | **Aceleração de Hardware por GPU no Miracast e Launcher Automático do Host** | [`23-aceleracao-hardware-gpu-miracast-e-launcher-host.md`](pt/23-aceleracao-hardware-gpu-miracast-e-launcher-host.md) | Detecção determinística de GPU via sysfs (AMD, Intel, NVIDIA), daemon de ranking de encoders de hardware, debounce atômico e caixa interativa de comandos de GPU no painel Web. |
| **24** | **Resolução Nativa 720p60 WFD, Nível 3.1 e Demuxer PES AU Determinista** | [`24-resolucao-nativa-720p60-miracast-wfd-demux-pes-au-determinista.md`](pt/24-resolucao-nativa-720p60-miracast-wfd-demux-pes-au-determinista.md) | Aplicação estrita do CEA índice 6 (1280x720p60), Nível 3.1, eliminação de sobrecarga 1080p, demuxer MPEG-TS determinista por PES/RTP Marker e validação de continuidade TS. |
| **25** | **Parada Unificada de Serviços (Standby), Teardown RTSP e Encerramento de Transmissores** | [`25-parada-unificada-standby-multi-servico-teardown-transmissor.md`](pt/25-parada-unificada-standby-multi-servico-teardown-transmissor.md) | Interrupção atômica dos 3 modos (Modos 1, 2, 3), fechamento forçado de sockets RTSP Miracast/MS-MICE, finalização de gnome-network-displays no host e persistência da saída HDMI ativa com tela de Standby. |
| **26** | **Benchmark Empírico de Latência, Responsividade e Telemetria de Hardware** | [`26-benchmark-latencia-responsividade-telemetria-hardware.md`](pt/26-benchmark-latencia-responsividade-telemetria-hardware.md) | Medições empíricas consolidadas dos 3 modos a 60 FPS nativo: Modo 3 USB Bulk (11.45ms), Modo 1 UDP (12.63ms) e Modo 2 Miracast (13.90ms); telemetria de silício, isolamento de barramento e <3.3% de CPU no Pi Zero. |

---

## 3. Destaque Arquitetural: O Desafio da Suspensão de Energia (Sleep / Suspend / Resume)

Um dos marcos mais recentes de estabilidade foi a resolução do **congelamento permanente após suspensão do computador host**:

### O Problema:
Quando o sistema operacional do host entra em suspensão de energia (*suspend-to-RAM* / S3), o compositor do GNOME (Mutter) encerra a sessão D-Bus de ScreenCast e destrói o nó de origem no PipeWire. No entanto, o processo filho do GStreamer (`gst-launch-1.0`) não é encerrado pelo sistema: ele permanece vivo, preso em leitura de I/O em um socket órfão. Um supervisor tradicional que use apenas `child.try_wait()` nunca detecta a falha, fazendo a imagem "morrer" sem retorno.

### A Solução Implementada:
1. **Health Watchdog (1500ms):** O supervisor em Rust executa `pw-cli info <node_id>`. Se o nó foi destruído, ele força o encerramento do processo (`child.kill()`), renegocia a sessão D-Bus com o Mutter e restabelece a transmissão em menos de 2 segundos.
2. **Link Watchdog (3000ms):** Verifica `pw-link -l` e reconecta o enlace PipeWire caso tenha sido desfeito.
3. **Persistência de Exibição:** O script de exibição (`show-welcome-window.py`) executa em laço resiliente `while True:`, auto-reconectando ao display `:0` caso o servidor Xwayland reinicie.
4. **Regras Udev Reativas:** As regras em `/etc/udev/rules.d/99-ext-monitor.rules` escutam `ACTION=="add|change"` para reaplicar `txqueuelen 100` e religar o serviço no momento em que a controladora USB do host sai do estado de repouso.
5. **Auto-Suficiência do Appliance:** Todo o pacote portátil (`client.tar.gz` e `ext-sender`) com essas correções é compilado e gravado dentro do `initramfs.cpio.gz` da imagem de boot de 32MB, ficando disponível via HTTP em `/connect.sh` para qualquer máquina nova.

---

## 4. Como Executar e Validar

* **Transmissão Rápida no Host:**
  ```bash
  ./scripts/start.sh extend auto 30 false economy --bitrate=400
  ```
* **Conexão Portátil em Qualquer PC Linux:**
  ```bash
  curl -sSL http://192.168.7.2:8080/connect.sh | bash
  ```
* **Acesso ao Web Dashboard com 5 Abas e 4 Idiomas:**
  Abra no navegador do computador host: `http://192.168.7.2:8080/`
* **Compilação e Geração da Imagem do Appliance:**
  ```bash
  ./scripts/build-fast-appliance.sh
  ```
