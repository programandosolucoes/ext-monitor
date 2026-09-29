# Compêndio de Blueprints Técnicos de Engenharia — `ext-monitor`

**Projeto:** `ext-monitor` — Monitor Secundário USB de Ultra-Baixa Latência  
**Plataforma Alvo:** Raspberry Pi Zero W / Zero 2 W / Raspberry Pi 4 / PC Linux & Windows  
**Autor:** Carlos Alberto <psncarlosalberto4ti@gmail.com>  
**Data:** Setembro de 2026  
**Status do Projeto:** Estável, Pronto para Produção, Compilado em Rust Nativo Puro  

---

## 1. Visão Geral da Suíte de Documentos

Esta pasta reúne a documentação de engenharia reversa, decisões de arquitetura e especificações de baixo nível que tornaram possível transformar um dispositivo de computação de US$ 10 (Raspberry Pi Zero de núcleo único a 1.0 GHz) em um monitor secundário profissional de 60 FPS com latência inferior a 15 milissegundos.

Todos os desafios fundamentais — desde o alinhamento de setores FAT16 no silício BCM2835 até o scanout zero-copy DRM/KMS no VideoCore IV, a recuperação autônoma de suspensão de energia no Linux Wayland, o motor visual quadrilíngue de splash, a telemetria EDID de monitores reais, a captura direta de hardware DRM/KMS scanout, o controle web bidirecional em Rust, o áudio híbrido IP/Bluetooth e o ecossistema IoT Media Renderer (Google Cast / UPnP) — estão formalizados nos 19 blueprints técnicos a seguir:

---

## 2. Mapa dos 19 Blueprints de Engenharia

| # | Blueprint Técnico | Arquivo | Foco de Engenharia |
| :---: | :--- | :--- | :--- |
| **01** | **Imagem de 32MB e Geometria do BCM2835** | [`01-imagem-32mb-e-geometria-bcm2835.md`](01-imagem-32mb-e-geometria-bcm2835.md) | Bug dos 65.525 clusters da ROM Broadcom, alinhamento no setor 1, partições FAT16 de 2KB por cluster e boot 100% em RAM. |
| **02** | **Arquitetura Fim-a-Fim e Comparativo com o Projeto GUD** | [`02-arquitetura-transmissor-receptor-e-comparativo-gud.md`](02-arquitetura-transmissor-receptor-e-comparativo-gud.md) | Por que o GUD falha (100% CPU, saturação USB com LZ4) e como o pipeline H.264 V4L2 M2M atinge 60 FPS com 0.8% de CPU. |
| **03** | **Transmissão de Pacotes, Fragmentação NALU e Leaky Queue** | [`03-transmissao-pacotes-drop-on-late-e-pipeline.md`](03-transmissao-pacotes-drop-on-late-e-pipeline.md) | MTU de 1472 bytes, fragmentação FU-A RFC 6184, descarte de pacotes atrasados (*drop-on-late*) e drenagem pós-sono. |
| **04** | **PipeWire, Mutter D-Bus e Auto-Recuperação Pós-Suspensão** | [`04-pipewire-mutter-screencast-e-wayland.md`](04-pipewire-mutter-screencast-e-wayland.md) | Captura virtual sem dongle, zero-copy DMA-BUF, relógio anti-ociosidade e **Supervisor com Duplo Watchdog em Rust** para sono/S3. |
| **05** | **Otimizações de CPU em Rust, GPU/VPU e Upscaler CAS** | [`05-otimizacoes-cpu-rust-gpu-vpu-e-cas-scaler.md`](05-otimizacoes-cpu-rust-gpu-vpu-e-cas-scaler.md) | Flags `-C target-cpu=arm1176jzf-s`, codificação por silício VA-API/NVENC e filtros de nitidez CAS vs FSR para texto. |
| **06** | **Modos Concorrentes de Operação e Super-Gadget USB ConfigFS** | [`06-modos-de-operacao-concorrentes-e-usb-gadget.md`](06-modos-de-operacao-concorrentes-e-usb-gadget.md) | 3 modos simultâneos (Linux UDP, Windows Miracast Win+K, USB Bulk), alocação de 7 endpoints DWC2, UDC auto-binding e chaves de controle. |
| **07** | **Cartão Micro-SD em RAM, Proteção e Upgrade via USB** | [`07-cartao-sd-em-ram-e-upgrade-usb.md`](07-cartao-sd-em-ram-e-upgrade-usb.md) | Eliminação de corrupção flash, partição exposta como pendrive `EXTMONITOR` na USB, upgrade em 3 métodos e entrega de binários. |
| **08** | **Manual de Instalação, Portabilidade e Script de Setup Host** | [`08-manual-de-instalacao-e-portabilidade-host.md`](08-manual-de-instalacao-e-portabilidade-host.md) | Conexão em 1 clique via `curl connect.sh`, instalador multi-distro, regras udev de baixa latência e console serial `/dev/ttyACM0`. |
| **09** | **Testes Empíricos em Hardware e Diagnósticos de Boot** | [`09-testes-empiricos-e-diagnosticos-hardware.md`](09-testes-empiricos-e-diagnosticos-hardware.md) | Validação no Pi Zero v1.3 Monocore, diagnóstico da tela arco-íris, cronometria de boot de 1.8s e benchmarks térmicos/cores. |
| **10** | **Refatoração Modular Clean Code, Enquadramento AU e V4L2 M2M** | [`10-refatoracao-modular-clean-code-v4l2-m2m-e-enquadramento-au.md`](10-refatoracao-modular-clean-code-v4l2-m2m-e-enquadramento-au.md) | Decomposição SRP, Builder Pattern, correção ABI 32-bit ARM ioctls (`0xC0CC5605`), remontagem RFC 6184/4571 e conversão SIMD YUV->RGB565. |
| **11** | **Scanout Zero-Copy via DRM/KMS, DMA-BUF e Correção ioctl -EFAULT** | [`11-kms-drm-dma-buf-scanout-zero-copy-e-correcao-efault.md`](11-kms-drm-dma-buf-scanout-zero-copy-e-correcao-efault.md) | Resolução do bug `os error 14`, ativação de Universal Planes no VideoCore IV (`vc4-drm`), importação DMA-BUF direta sem cópia por CPU e double-buffering seguro. |
| **12** | **Splash Quadrilíngue, Telemetria EDID Realtime e Ciclo de Vida de Desconexão** | [`12-splash-quadrilingue-telemetria-edid-realtime-e-ciclo-vida-desconexao.md`](12-splash-quadrilingue-telemetria-edid-realtime-e-ciclo-vida-desconexao.md) | Prevenção de imagem congelada na desconexão, telas de splash embutidas em Rust (PT/EN/ES/FR) via `miniz_oxide` e decodificação do hardware VESA EDID no painel web. |
| **13** | **Captura Direta no Kernel Linux DRM/KMS e Dual-Engine Universal** | [`13-captura-direta-drm-kms-e-dual-engine-universal.md`](13-captura-direta-drm-kms-e-dual-engine-universal.md) | Bypassa GNOME Mutter/Wayland, extração atômica PRIME DMA-BUF do scanout da GPU AMD/Intel/NVIDIA via ioctl `GETFB2`, eliminação de congelamentos por oclusão de janelas e suporte multi-monitor genérico. |
| **14** | **Subsistema de Áudio HDMI Digital, Codec Opus e ALSA** | [`14-subsistema-audio-hdmi-digital-opus-alsa-e-sincronismo-av.md`](14-subsistema-audio-hdmi-digital-opus-alsa-e-sincronismo-av.md) | Transmissão de áudio digital de ultra-baixa latência (< 25ms) via PipeWire e Opus 48kHz estéreo na porta UDP 5004, decodificação ALSA `vc4-hdmi`, sincronia A/V e controle Web de volume. |
| **15** | **Pipeline Zero-Copy, Quiescência Wayland e Ciclo de Vida do Display** | [`15-pipeline-zero-copy-quiescencia-wayland-e-ciclo-vida-display.md`](15-pipeline-zero-copy-quiescencia-wayland-e-ciclo-vida-display.md) | Resolução do congelamento de quadros por inatividade do cursor, sincronismo vblank e reativação dinâmica do pipeline. |
| **16** | **Pacer Wayland de Quiescência, Cross-ARMv6 e Receptores Universais** | [`16-pacer-wayland-quiescencia-e-guia-universal-receptores.md`](16-pacer-wayland-quiescencia-e-guia-universal-receptores.md) | Agente heartbeat 60 FPS sintético para YouTube contínuo, compilação estática musl/cross e guia para Pi 2/3/4/5 e PCs secundários. |
| **17** | **Agente Host Rust e Controle Web Bidirecional** | [`17-agente-host-rust-controle-web-bidirecional.md`](17-agente-host-rust-controle-web-bidirecional.md) | Arquitetura RPC UDP 5001, controle completo do host pelo painel web, proteção contra Mutter stride assert crash (`SIGABRT 6`) e quirks AMD DCN 3.1. |
| **18** | **Áudio Híbrido: Rede IP Opus e Bluetooth A2DP Sink** | [`18-audio-hibrido-rede-opus-e-bluetooth-a2dp.md`](18-audio-hibrido-rede-opus-e-bluetooth-a2dp.md) | Isolamento de áudio local (headset USB Yealink UH34) vs TV HDMI, pareamento Bluetooth 4.1 no Pi Zero W e roteamento ALSA `hw:0,0`. |
| **19** | **Appliance IoT Media Renderer: Google Cast, UPnP/DLNA e Visualizador HDMI** | [`19-iot-media-renderer-chromecast-upnp-e-visualizador-hdmi.md`](19-iot-media-renderer-chromecast-upnp-e-visualizador-hdmi.md) | Transformação em dongle multimídia inteligente: Google Cast (CastV2), DIAL (YouTube), UPnP/DLNA e tela visual com capa/espectro sonoro em áudio IoT. |

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
