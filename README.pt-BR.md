# ext-monitor: Motor Universal de Monitor Secundário USB de Ultra-Baixa Latência & Central de Mídia IoT

> 🇧🇷 Versão em Português (Brasil) | [🇺🇸 English Version](README.md)

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Language: Rust](https://img.shields.io/badge/Language-Rust_100%25-orange.svg)](https://www.rust-lang.org/)
[![Hardware: BCM2835 / VideoCore IV](https://img.shields.io/badge/Hardware-Broadcom_BCM2835-red.svg)](https://www.raspberrypi.com/)
[![Latency: Sub-Millisecond](https://img.shields.io/badge/Latency-%3C1ms_USB_Bulk-brightgreen.svg)]()
[![OS: Linux & Windows](https://img.shields.io/badge/OS-Linux_Wayland_%26_Windows_10%2F11-blueviolet.svg)]()
[![API: OpenAPI 3.0](https://img.shields.io/badge/API-OpenAPI_3.0_Swagger-green.svg)](http://192.168.7.2:8080/swagger)
[![Version: v2.3.0](https://img.shields.io/badge/Release-v2.3.0_Frozen-purple.svg)](https://github.com/programandosolucoes/ext-monitor/releases/tag/v2.3.0-final)

Um sistema de alta performance, 100% em Rust nativo puro, concebido para transformar um modesto **Raspberry Pi Zero (v1.2 / v1.3 / W / Zero 2 W)** conectado por um único **cabo Micro-USB comum** em um **monitor secundário HDMI de hardware com latência zero, sink de áudio digital, visualizador de espectro sonoro em tempo real e central multimídia IoT** para Linux (Wayland / GNOME Mutter) e Windows 10/11 (Miracast nativo / `Win + K`).

Entrega fluidos **30 a 60 FPS** em resolução nativa (1280x720 / 1600x900) com **latência sub-milissegundo (< 1 ms via USB Bulk / < 15 ms via UDP)** e **~0.8% de carga de CPU** no Pi Zero através de offload zero-copy para a GPU (decodificador por hardware Broadcom VideoCore IV V4L2 M2M + scanout KMS/DRM, impulsionado por codificação em hardware AMD VA-API, NVIDIA NVENC ou Intel QSV no host).

---

![Raspberry Pi Zero Dual Monitor Desk Setup](docs/assets/hero-setup.jpg)

---

## 💡 Motivação do Autor, O Desafio do Silício & O Que o Projeto Oferece

### A Motivação
Desenvolvedores, engenheiros e profissionais remotos lutam constantemente com falta de espaço de tela. Monitores portáteis USB comerciais custam entre R$ 1.200 e R$ 2.500, ocupam espaço excessivo na mochila e soluções comuns baseadas em espelhamento por software (como Duet Display ou Spacedesk) não possuem suporte nativo de baixo nível para Linux/Wayland, sofrem com atraso no mouse ou consomem CPU abusiva da máquina hospedeira.

O objetivo deste projeto foi provar que, através de engenharia de sistemas rigorosa em Rust no nível do silício, **uma placa extremamente barata e acessível de R$ 70 a R$ 100 pode ser transformada em um monitor secundário profissional de latência imperceptível e central multimídia IoT**.

### O Desafio Extremo do Silício
O Raspberry Pi Zero W é equipado com o SoC Broadcom BCM2835:
* **Núcleo único ARM1176JZF-S** a 1.0 GHz (arquitetura ARMv6 de 32 bits).
* **Apenas 512 MB de RAM compartilhada** entre CPU e GPU.
* **Controladora USB 2.0 (`dwc2`)** concorrente com interrupções do sistema.

Rodar uma distribuição Linux comum (como Raspberry Pi OS com X11 ou Wayfire) consome mais de 350 MB de RAM apenas ociosa, travando a CPU em 100% e descartando frames.

Para atingir 30 a 60 FPS com áudio digital e resposta instantânea, **todas as abstrações convencionais foram eliminadas**:
1. **Sem X11, Sem Wayland no Receptor:** O Pi Zero roda Linux direto no metal com scanout DRM/KMS no kernel e buffers DMA-BUF zero-copy.
2. **Cabo Único USB Gadget:** Alimentação elétrica e dados em alta velocidade (480 Mbps) trafegam pelo mesmo cabo micro-USB.
3. **Execução 100% em RAM:** Todo o sistema inicializa a partir de um `initramfs` comprimido de 32 MB. O cartão micro-SD é desacoplado após o boot, eliminando qualquer risco de queima ou corrupção ao puxar o cabo.
4. **Motor de FFT de Áudio em Tempo Real:** Uma FFT Cooley-Tukey de 512 pontos com janelamento de Hann processa o áudio do host e projeta um visualizador de 24 bandas na TV quando o vídeo do PC é pausado.

### O Que o ext-monitor Oferece:
* **Extensão Real de Desktop:** Expõe uma saída HDMI virtual no GNOME Wayland / Mutter ou Projeção Sem Fio do Windows (`Win + K`). Janelas e espaços de trabalho se comportam de forma nativa.
* **Áudio Digital HDMI via PipeWire:** Roteamento de som estéreo 48kHz Opus direto para os alto-falantes da TV ou monitor secundário via HDMI.
* **Multiplexador de Scanout de Porta Única HDMI:** Exclusão mútua em nível de hardware alterna automaticamente a saída HDMI entre a área de trabalho do PC, visualizador dinâmico de espectro a 30 FPS e tela de splash em 4 idiomas.
* **Central de Mídia IoT:** Suporta transmissão de streams de mídia (Google Cast / DLNA / UPnP) diretamente para o televisor.
* **Documentação Interativa Swagger / OpenAPI 3.0:** Interface interativa acessível via navegador em `http://192.168.7.2:8080/swagger`.
* **Arquitetura Zero-Reboot:** Troca não-bloqueante entre modos USB Bulk, UDP e Wi-Fi sem necessidade de reiniciar os dispositivos.

---

## 🔌 Ligação de Hardware e Anti-Erro de Portas

<p align="center">
  <img src="docs/assets/hardware-macro.jpg" alt="Raspberry Pi Zero BCM2835 com Conexões USB e HDMI" width="760" />
</p>

### Pinout das Portas & Guia de Proteção Elétrica

```
                            [ Barra de Pinos 40-Pin GPIO ]
  +------------------------------------------------------------------------+
  | [Slot Micro-SD]                                                        |
  | (Cartão SanDisk)              [ SoC BCM2835 ]                          |
  |                                                                 [CSI]  |
  +---------[ mini-HDMI ]-----------[ Micro-USB OTG ]---------[ PWR IN ]---+
                   │                        │                     │
                   │                        │                     └── [DEIXE VAZIA!] NÃO CONECTAR
                   │                        │                         (O PC fornece 5V via OTG)
                   │                        └── Micro-USB na Porta USB do Notebook / PC
                   │                            (Alimentação 5V + Transporte de Dados a 480 Mbps)
                   │
                   └── Cabo mini-HDMI para o Monitor Secundário ou TV
```

* **Porta mini-HDMI (Esquerda):** Saída de vídeo e áudio dedicada para a TV ou monitor secundário.
* **Porta Micro-USB Central (OTG / Dados + Energia):** Conectada diretamente ao notebook ou computador host. Fornece 5V de energia e trafega todos os dados (480 Mbps).
* **Porta Micro-USB Direita (PWR IN):** **DEVE PERMANECER VAZIA!** Não conecte carregadores externos quando conectado ao computador para evitar loops de terra.
* **Consumo de Energia:** Apenas ~0.8W (dentro da especificação de qualquer porta USB 2.0 padrão).
* **Temperatura de Operação:** ~44.5°C contínuos sob carga (zero thermal throttling).

---

## 🚀 Como Conectar e Rodar no Ato

### No Linux (Wayland / GNOME / Ubuntu / Fedora / Arch)

O binário nativo em Rust `ext-sender` é auto-suficiente:
```bash
# 1. Extensão de Tela Imediata (O melhor comando de ouro - 60 FPS CFR, 6000 kbps, VA-API, Áudio HDMI):
ext-sender

# 2. Modo 3 USB Bulk Direto (< 1ms de latência física):
ext-sender --usb

# 3. Espelhamento de Tela (Clone):
ext-sender clone

# 4. Parar transmissões ativas:
ext-sender stop

# 5. Telemetria e status ao vivo:
ext-sender status
```

### No Windows 10 / Windows 11 (Sem Nenhum Driver)
1. Pressione no teclado: **`Win + K`** (Transmitir / Conectar).
2. Selecione **"ExtMonitor-Pi0"**.
3. O Windows projeta a tela secundária via hardware com áudio e vídeo instantâneos.

---

## 📖 Manuais de Operação & Blueprints Técnicos

* **[Manual de Operação em Português (docs/MANUAL-DE-OPERACAO.md)](docs/MANUAL-DE-OPERACAO.md):** Guia operacional exaustivo cobrindo todos os modos, roteamento PipeWire, telemetria e arquitetura zero-reboot.
* **[English Operation Manual (docs/OPERATION-MANUAL.md)](docs/OPERATION-MANUAL.md):** Manual completo de operação em inglês.
* **[O Livro do Ext-Monitor: 20 Blueprints de Engenharia (docs/LIVRO-EXT-MONITOR.md)](docs/LIVRO-EXT-MONITOR.md):** Livro técnico consolidado em 20 capítulos detalhando engenharia reversa de silício, geometria matemática e arquitetura fundamental do appliance.
* **[Catálogo Completo dos 25 Blueprints (docs/blueprints/README.pt-BR.md)](docs/blueprints/README.pt-BR.md):** Índice mestre detalhado com todos os 25 blueprints de engenharia em português e inglês.

### A Fundação (Blueprints Originais 01 a 20)
Os 20 blueprints fundamentais documentam a arquitetura de silício e o motor essencial do appliance: geometria FAT16 no Broadcom BCM2835, decodificação acelerada V4L2 M2M, scanout zero-copy no VideoCore IV via DRM/KMS, watchdogs duplos para recuperação de suspensão S3, agente host bidirecional em Rust e multiplexação HDMI com visualizador de áudio FFT.

1. **[Blueprint 01: Imagem de 32MB e Geometria do BCM2835](docs/blueprints/pt/01-imagem-32mb-e-geometria-bcm2835.md):** Limite de 65.525 clusters da ROM Broadcom, alinhamento no setor 1 e FAT16 de 2KB.
2. **[Blueprint 02: Arquitetura Fim-a-Fim e Comparativo com GUD](docs/blueprints/pt/02-arquitetura-transmissor-receptor-e-comparativo-gud.md):** Por que o GUD falha (100% CPU) e como o H.264 V4L2 M2M atinge 60 FPS com 0.8% de CPU.
3. **[Blueprint 03: Transmissão de Pacotes & Drop-on-Late](docs/blueprints/pt/03-transmissao-pacotes-drop-on-late-e-pipeline.md):** MTU de 1472 bytes, fragmentação FU-A RFC 6184 e fila leaky anti-lag.
4. **[Blueprint 04: PipeWire & Auto-Recuperação Pós-Suspensão S3](docs/blueprints/pt/04-pipewire-mutter-screencast-e-wayland.md):** Captura virtual sem dongle e supervisor com duplo watchdog em Rust para sleep/retomada.
5. **[Blueprint 05: Otimizações de CPU em Rust e Scaler CAS](docs/blueprints/pt/05-otimizacoes-cpu-rust-gpu-vpu-e-cas-scaler.md):** Flags ARM1176, codificação VA-API/NVENC e filtro de nitidez adaptativo CAS para tipografia.
6. **[Blueprint 06: Modos Concorrentes & Super-Gadget USB ConfigFS](docs/blueprints/pt/06-modos-de-operacao-concorrentes-e-usb-gadget.md):** 3 modos simultâneos (Linux UDP, Windows Miracast Win+K, USB Bulk) e alocação de 7 endpoints DWC2.
7. **[Blueprint 07: Proteção do Cartão SD e Upgrade via USB](docs/blueprints/pt/07-cartao-sd-em-ram-e-upgrade-usb.md):** Execução 100% em RAM, zero desgaste flash e partição exposta como pendrive `EXTMONITOR`.
8. **[Blueprint 08: Manual de Instalação e Portabilidade Host](docs/blueprints/pt/08-manual-de-instalacao-e-portabilidade-host.md):** Conexão em 1 clique via `curl connect.sh`, regras udev de baixa latência e console serial.
9. **[Blueprint 09: Testes Empíricos e Diagnósticos de Boot](docs/blueprints/pt/09-testes-empiricos-e-diagnosticos-hardware.md):** Diagnóstico da tela arco-íris, cronometria de boot de 1.8s e ensaios térmicos.
10. **[Blueprint 10: Refatoração Modular e Enquadramento AU V4L2 M2M](docs/blueprints/pt/10-refatoracao-modular-clean-code-v4l2-m2m-e-enquadramento-au.md):** Decomposição SRP, Builder Pattern, correção ABI 32-bit ARM ioctls e montagem RFC 6184/4571.
11. **[Blueprint 11: Scanout Zero-Copy via DRM/KMS e Resolução -EFAULT](docs/blueprints/pt/11-kms-drm-dma-buf-scanout-zero-copy-e-correcao-efault.md):** Correção errno 14 (-EFAULT), planos universais no VideoCore IV e importação DMA-BUF direta.
12. **[Blueprint 12: Splash Quadrilíngue, Telemetria EDID e Desconexão](docs/blueprints/pt/12-splash-quadrilingue-telemetria-edid-realtime-e-ciclo-vida-desconexao.md):** Prevenção de congelamento na desconexão, splash em 4 línguas e telemetria VESA EDID real.
13. **[Blueprint 13: Captura Direta no Kernel Linux DRM/KMS](docs/blueprints/pt/13-captura-direta-drm-kms-e-dual-engine-universal.md):** Extração atômica PRIME DMA-BUF do scanout da GPU via ioctl GETFB2, imune a oclusão de janelas.
14. **[Blueprint 14: Subsistema de Áudio HDMI Digital, Opus e ALSA](docs/blueprints/pt/14-subsistema-audio-hdmi-digital-opus-alsa-e-sincronismo-av.md):** Áudio digital estéreo de ultra-baixa latência (< 25 ms) na porta UDP 5004 com reprodução ALSA.
15. **[Blueprint 15: Pipeline Zero-Copy e Quiescência Wayland](docs/blueprints/pt/15-pipeline-zero-copy-quiescencia-wayland-e-ciclo-vida-display.md):** Eliminação de cópias de CPU com `imagefreeze`, retenção de frame no plano KMS e watchdog de 15s.
16. **[Blueprint 16: Pacer Wayland e Receptores Universais](docs/blueprints/pt/16-pacer-wayland-quiescencia-e-guia-universal-receptores.md):** Pacing contínuo a 60 FPS para YouTube contínuo, compilação cruzada ARMv6 e suporte a Raspberry Pi 2/3/4/5.
17. **[Blueprint 17: Agente Host Rust e Controle Web Bidirecional](docs/blueprints/pt/17-agente-host-rust-controle-web-bidirecional.md):** Protocolo RPC UDP 5001, controle web total e prevenção contra crash de stride no GNOME Mutter.
18. **[Blueprint 18: Áudio Híbrido: Rede IP Opus e Bluetooth A2DP](docs/blueprints/pt/18-audio-hibrido-rede-opus-e-bluetooth-a2dp.md):** Isolamento de fone de ouvido pessoal vs som da TV HDMI e pareamento Bluetooth 4.1 no Pi Zero W.
19. **[Blueprint 19: Appliance IoT Media Renderer: Cast e Visualizador HDMI](docs/blueprints/pt/19-iot-media-renderer-chromecast-upnp-e-visualizador-hdmi.md):** Dongle multimídia inteligente: Google Cast (CastV2), DIAL (YouTube), UPnP/DLNA e visualizador gráfico HDMI.
20. **[Blueprint 20: Multiplexador HDMI de Porta Única e Zero-Reboot](docs/blueprints/pt/20-multiplexador-hdmi-scanout-fft-realtime-i18n-e-zero-reboot.md):** Exclusão mútua na porta HDMI física, FFT de 512 pontos com Hann, i18n simétrico e ciclo de vida zero-reboot.

### A Suíte de Expansão Miracast (Blueprints 21 a 25)
Para viabilizar projeção de tela sem fio nativa e sem necessidade de instalar drivers ou aplicativos adicionais, o escopo foi expandido com 5 novos blueprints dedicados ao ecossistema Wi-Fi Display (Miracast / MS-MICE) no Windows 10/11 (`Win + K`) e desktops Linux:

21. **[Blueprint 21: Miracast MS-MICE, Conexão Reversa RTSP WFD e SPS Dinâmico](docs/blueprints/pt/21-miracast-ms-mice-rtsp-wfd-sps-dinamico-e-gnome-network-displays.md):** Sinalização binária MS-MICE (TCP 7250), reversão RTSP WFD para o Source (7236), migração para UDP 5005 e parser puro em Rust de SPS dinâmico.
22. **[Blueprint 22: Alinhamento NV12 no KMS, Fim da Faixa Verde e Otimizações Realtime](docs/blueprints/pt/22-correcao-faixa-verde-nv12-stride-alinhamento-e-otimizacoes-realtime-miracast.md):** Correção matemática de stride de 16 linhas (1080->1088), remoção de padding de 15.360 bytes no plano UV do KMS, demuxer MPEG-TS zero-delay via PUSI e aceleração VA-API AMD Radeon 610M.
23. **[Blueprint 23: Aceleração por Hardware GPU no Miracast e Launcher do Host](docs/blueprints/pt/23-aceleracao-hardware-gpu-miracast-e-launcher-host.md):** Detecção direta via sysfs de GPUs (AMD, Intel, NVIDIA), daemon de ranking de encoders, debounce atômico e caixa interativa de comandos no painel Web.
24. **[Blueprint 24: Resolução Nativa 720p60 WFD, Nível 3.1 e Demuxer PES AU Determinista](docs/blueprints/pt/24-resolucao-nativa-720p60-miracast-wfd-demux-pes-au-determinista.md):** Imposição do CEA índice 6 (1280x720p60), Nível 3.1 H.264, demuxer MPEG-TS determinista por PES/RTP Marker e validação de continuidade TS.
25. **[Blueprint 25: Parada Unificada de Serviços (Standby), Teardown RTSP e Encerramento de Transmissores](docs/blueprints/pt/25-parada-unificada-standby-multi-servico-teardown-transmissor.md):** Parada limpa dos 3 modos (Modo 1 UDP, Modo 2 Miracast, Modo 3 USB Bulk), encerramento forçado de conexões RTSP ativas, finalização de transmissores no host (`SIGTERM`/`SIGKILL`) e exibição contínua da tela Ready via KMS no HDMI.

---

## 🛠️ Compilação e Geração da Imagem do Appliance

Para compilar o sistema e gerar uma imagem bootável de 32MB para cartão micro-SD utilizando a ferramenta nativa em Rust:
```bash
# Compilar imagem completa universal do appliance (Pi Zero 1 & Zero 2 W):
ext-tool build --image

# Gravar diretamente no cartão micro-SD (substitua /dev/sdX pelo leitor de cartão):
sudo ext-tool flash /dev/sdX
```

### 🧪 Bateria de Testes & Ambiente Isolado em Docker
Execute testes unitários nativos em Rust ou inicialize um container Docker isolado com repasse de barramento USB e GPU DRI (protege a sessão do GNOME Shell / desktop do hospedeiro contra qualquer queda ou recarregamento inesperado):
```bash
# Executar todos os testes unitários 100% nativos em Rust (ext-sender, ext-receiver, ext-tool):
ext-tool test

# Diagnosticar o Pi Zero USB Display Gadget conectado via sysfs direto:
ext-tool test --usb

# Executar testes isolados no container Docker com /dev/bus/usb e /dev/dri:
ext-tool test --docker
```

---

## 📄 Licença & Atribuição

Distribuído sob a **Licença MIT**. Consulte [LICENSE](LICENSE) para mais detalhes.

### Autor & Contato
* **Autor:** Carlos Alberto
* **E-mail:** [carlosalberto4ti@gmail.com](mailto:carlosalberto4ti@gmail.com)
* **LinkedIn:** [linkedin.com/in/carlosalberto4ti](https://www.linkedin.com/in/carlosalberto4ti)
* **Blog Pessoal & Portfólio:** [carloslopes.programandosolucoes.com.br](https://carloslopes.programandosolucoes.com.br)
* **Website:** [programandosolucoes.com.br](https://programandosolucoes.com.br)
