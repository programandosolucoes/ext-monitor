# Blueprint 13: Captura Direta no Kernel Linux DRM/KMS e Arquitetura Dual-Engine Universal

**Projeto:** `ext-monitor`  
**Autor:** Carlos Alberto <psncarlosalberto4ti@gmail.com>  
**Arquivos de Referência:** `sender/src/kms.rs`, `sender/src/screencast.rs`, `sender/src/pipeline.rs`, `sender/src/config.rs`  
**Data:** Setembro de 2026  

---

## 1. Visão Geral e Motivação

O `ext-monitor` foi concebido para transformar qualquer Raspberry Pi Zero em um segundo monitor de altíssima fidelidade (1600x900@60 FPS) com latência sub-15ms.

Tradicionalmente em desktops modernos Linux com **Wayland** e compositor **GNOME Mutter**, a transmissão de tela depende da API D-Bus `org.gnome.Mutter.ScreenCast` intermediada pelo servidor de multimídia **PipeWire**.

Entretanto, esse modelo possui um gargalo intrínseco de projeto:
* **Damage Tracking e Oclusão de Janelas:** O GNOME Mutter foi projetado para economizar bateria e ciclos de GPU. Se o usuário move o ponteiro do mouse para fora do monitor estendido (`HDMI-1`), navegadores baseados em Chromium e Gecko ativam o modo de economia (*Window Occlusion Tracking*), suspendendo os ciclos de renderização do elemento de vídeo (`<video>`). Sem novas pinturas da aplicação e sem o ponteiro do mouse se movimentando, o compositor considera o monitor estático e suspende a emissão de novos buffers DMA-BUF para o PipeWire, congelando a reprodução de vídeo na tela do Pi Zero mesmo com o áudio continuando normalmente.

Para solucionar essa limitação de forma limpa, arquitetural e definitiva — **sem recorrer a janelas fantasmas, scripts de polling ou hacks de interface** —, o `ext-monitor` implementa a arquitetura **Dual-Engine Universal**, permitindo a alternância contínua entre dois motores de captura:
1. **Motor Mutter (Padrão de Alto Nível):** Captura via D-Bus e PipeWire, sem exigir privilégios especiais do sistema operacional.
2. **Motor KMS Direct (Baixo Nível no Kernel):** Captura direta do scanout do CRTC na VRAM da GPU via DRM/KMS (`/dev/dri/card*`), extraindo o buffer de hardware em formato PRIME DMA-BUF, **bypasando completamente o compositor e o gerenciador de janelas**.

---

## 2. Diagrama da Arquitetura Dual-Engine

```
                     ┌────────────────────────────────────────────────────────┐
                     │            APLICAÇÃO (Chrome, IDE, Vídeo)              │
                     └───────────────────────────┬────────────────────────────┘
                                                 │
                               ┌─────────────────┴─────────────────┐
                               │                                   │
                               ▼                                   ▼
             ┌───────────────────────────────────┐ ┌───────────────────────────────────┐
             │       MOTOR 1: GNOME MUTTER       │ │       MOTOR 2: KERNEL DRM/KMS     │
             │   (D-Bus Screencast + PipeWire)   │ │    (Hardware Scanout CRTC Buffer) │
             ├───────────────────────────────────┤ ├───────────────────────────────────┤
             │ • Nível de usuário sem privilégio  │ │ • Acesso direto a /dev/dri/card* │
             │ • Intermediado pelo PipeWire      │ │ • Bypassa Wayland / Mutter / D-Bus│
             │ • Sujeito a damage tracking       │ │ • 60 FPS contínuos do clock VBLANK│
             │ • Ideal para uso padrão de desktop│ │ • Imune a perda de foco de janelas│
             └─────────────────┬─────────────────┘ └─────────────────┬─────────────────┘
                               │                                   │
                               └─────────────────┬─────────────────┘
                                                 │
                                                 ▼
                             ┌───────────────────────────────────────┐
                             │       ZERO-COPY DMA-BUF UNIFICADO     │
                             └───────────────────┬───────────────────┘
                                                 │
                                                 ▼
                             ┌───────────────────────────────────────┐
                             │       HARDWARE VIDEO ENCODER          │
                             │  (AMD VA-API / NVENC / Intel QSV)     │
                             └───────────────────┬───────────────────┘
                                                 │
                                                 ▼
                             ┌───────────────────────────────────────┐
                             │       TRANSPORTE LOSSLESS / RTP       │
                             │   (USB Bulk RFC 4571 ou UDP 5000)     │
                             └───────────────────┬───────────────────┘
                                                 │
                                                 ▼
                             ┌───────────────────────────────────────┐
                             │       RASPBERRY PI ZERO W APPLIANCE   │
                             │   (Broadcom VideoCore IV Hardware)    │
                             └───────────────────────────────────────┘
```

---

## 3. O Mecanismo de Baixo Nível: Kernel Direct DRM/KMS

### 3.1 Descoberta e Mapeamento de Recursos
Através da crate nativa `drm` (`smithay/drm-rs`), o `ext-sender` interage com o subsistema DRM do kernel Linux:

1. **Abertura do Dispositivo:** Abre o nó da controladora gráfica primária (`/dev/dri/card1` no caso de APU AMD Radeon, ou `/dev/dri/card0` em Intel/NVIDIA).
2. **Enumeração de Conectores:** Mapeia todos os conectores de hardware registrados no subsistema DRM:
   * `EmbeddedDisplayPort` (tela do notebook)
   * `HDMIA` (porta HDMI de extensão)
   * `DisplayPort`
   * `Virtual` (conectores virtuais KMS)
3. **Resolução de Encoders e CRTCs:** Localiza o encoder ativo atrelado ao conector desejado e obtém o identificador do CRTC de hardware correspondente (`crtc::Handle`).
4. **Obtenção do Scanout Framebuffer:** Invoca `get_planar_framebuffer` (DRM ioctl `DRM_IOCTL_MODE_GETFB2`) para obter os metadados do buffer que a GPU está ativamente exibindo no monitor:
   * Dimensões reais (`width`, `height`)
   * Formato de quatro caracteres (`DrmFourcc::XR24`, `AR24`, `NV12`, etc.)
   * Pitches e offsets dos planos de memória.

### 3.2 Exportação Atômica para PRIME DMA-BUF
Com o identificador do buffer de hardware (`buffer::Handle`) em mãos, o transmissor chama a ioctl `DRM_IOCTL_PRIME_HANDLE_TO_FD`:
```rust
let prime_fd = card.buffer_to_prime_fd(buffer_handle, 0)?;
```
O kernel retorna um descritor de arquivo de memória compartilhada **DMA-BUF (`OwnedFd`)**, apontando diretamente para o endereço de memória de vídeo VRAM alocado pelo driver da GPU.

### 3.3 Requisitos de Segurança e Capabilities
Para permitir a chamada `DRM_IOCTL_PRIME_HANDLE_TO_FD` em monitores ativos sem necessidade de executar o transmissor como `root`, o binário recebe a permissão de capacidade do kernel Linux:
```bash
sudo setcap cap_sys_admin+ep /usr/local/bin/ext-sender
```
Isso concede estritamente a autorização para exportar descritores de buffers DRM, mantendo todo o resto da execução isolada no usuário padrão.

---

## 4. Suporte Multi-GPU e Detecção Genérica de Hardware

O motor de captura direta e o pipeline de codificação operam de forma heterogênea e genérica para suportar qualquer máquina:

| Fabricante da GPU | Driver do Kernel | Dispositivo DRM | API de Codificação Hardware | Formato de Scanout |
| :--- | :--- | :--- | :--- | :--- |
| **AMD (Radeon / APU Ryzen)** | `amdgpu` | `/dev/dri/card1` ou `card0` | VA-API (`vah264enc` / `vapostproc`) | `XR24` (XRGB8888) |
| **Intel (Core / Iris / Arc)** | `i915` / `xe` | `/dev/dri/card0` | VA-API / QSV (`vah264enc` / `qsvh264enc`) | `AR24` / `NV12` |
| **NVIDIA (GeForce / RTX)** | `nvidia-drm` | `/dev/dri/card0` ou `card1` | NVENC (`nvh264enc` via CUDA) | `NV12` / `XRGB` |
| **Genérico / Fallback** | `vkms` / `vgem` | `/dev/dri/cardX` | CPU OpenH264 / x264 ultrafast | `I420` |

### Detecção Automática da GPU
O `ext-sender` varre `/dev/dri/by-path/` e `/sys/class/drm/`, identificando qual placa possui o conector monitorado ativo, instanciando o codificador correto automaticamente.

---

## 5. Suporte a 1 ou Mais Monitores

O sistema suporta seleção de monitores por nome e por índice:
* `--monitor=auto` (padrão): Localiza o primeiro conector estendido conectado (`HDMI-1`, `DP-1` ou similar).
* `--monitor=HDMI-1`: Força a captura da porta HDMI física.
* `--monitor=eDP-1`: Captura o monitor principal do notebook (modo espelhamento/clone).
* `--monitor=card1-DP-2`: Seleciona monitor específico em configurações com múltiplas saídas DisplayPort.

---

## 6. Telemetria e Monitoramento em Tempo Real

O supervisor do transmissor executa um watchdog contínuo de telemetria a cada 2.5 segundos, exibindo:
1. **Status do Monitor e Resolução:** Conector selecionado e resolução ativa (ex: `1600x900@59.95Hz`).
2. **Estado do Motor de Captura:**
   * No modo Mutter: ID do Nó PipeWire, status do enlace (`CONECTADO` vs `DESCONECTADO`).
   * No modo KMS: Identificador do CRTC de hardware e descritor DMA-BUF exportado.
3. **Taxa de Quadros (FPS) e Modo de Transmissão:** Confirmação se está operando em modo Contínuo (CFR Anti-Freeze) ou Econômico (drop-only).
4. **Alerta de Inibição de Sessão:** Confirmação de que o bloqueio de tela automático e suspensão de energia estão desativados pelo `GnomeSessionInhibitor`.

---

## 7. Renderização do Cursor do Mouse: Hardware Plane vs Scanout Primário

Um dos aspectos mais críticos e não-óbvios da captura direta via Kernel DRM/KMS reside na forma como controladoras gráficas modernas (AMD Radeon, Intel Graphics, NVIDIA GeForce) tratam o ponteiro do mouse:

### 7.1 Separação Física dos Planos DRM
No subsistema DRM/KMS, o hardware gráfico opera com múltiplos planos independentes:
* **Plano Primário (`DRM_PLANE_TYPE_PRIMARY`):** Onde o compositor Wayland (GNOME Mutter) desenha as janelas, papéis de parede, navegadores e vídeos.
* **Plano de Cursor (`DRM_PLANE_TYPE_CURSOR`):** Um plano de hardware de aceleração de altíssima prioridade (geralmente 64x64 ou 32x32 pixels), atualizado em tempo real pelo kernel através de chamadas atômicas ou `drmModeSetCursor` à medida que o mouse se desloca (`crtc_x`, `crtc_y`).

**O display controller da GPU mescla o Plano Primário e o Plano de Cursor no próprio silício na hora de enviar os elétrons/fótons pela porta HDMI/eDP.**

Por consequência direta de projeto do kernel, qualquer captura direta de framebuffer (`kmsgrab`, DMA-BUF prime handle do CRTC) lê exclusivamente o Plano Primário. **O cursor do mouse não existe na memória do framebuffer primário.**

### 7.2 Soluções Implementadas no `ext-monitor`:
1. **Modo GNOME Mutter Screencast (`--mutter`):** O compositor Mutter recebe a propriedade `cursor-mode = 1` (`Embedded`). O compositor desenha o ponteiro do mouse diretamente na textura de vídeo antes de exportar o buffer via PipeWire, assegurando que o ponteiro apareça com formato, animações e hotspot perfeitos.
2. **Modo Kernel KMS com Cursor em Software (`MUTTER_DEBUG_DISABLE_HW_CURSORS=1`):** Ao configurar a variável de ambiente `MUTTER_DEBUG_DISABLE_HW_CURSORS=1` (instalada automaticamente em `~/.config/environment.d/99-mutter-software-cursor.conf` pelo `install-host.sh`), o Mutter é instruído a desativar os planos de hardware da GPU e renderizar o cursor em software diretamente no framebuffer primário via OpenGL. Dessa forma, a captura direta KMS obtém o ponteiro do mouse sem perda de desempenho perceptível.

---

## 8. Suspensão, Retomada de Serviço e Inibição de Bloqueio de Tela

Tanto o transmissor quanto o receptor foram projetados para resistir a ciclos de suspensão de energia (fechamento de tampa do notebook, suspensão do sistema, bloqueio de tela):

### 8.1 Inibição Ativa de Protetor de Tela e Suspensão no Host
Durante a execução ativa da transmissão, o `ext-sender` registra um bloqueio de sessão via D-Bus junto ao `org.gnome.SessionManager`:
* **Flags:** `4 | 8 = 12` (Inibir Suspensão por Inatividade + Inibir Protetor de Tela e Bloqueio de Sessão).
* **RAII Drop Pattern:** No encerramento normal ou interrupção via `Ctrl+C` / `SIGTERM`, o destrutor de `GnomeSessionInhibitor` libera automaticamente o cookie no D-Bus, restaurando as políticas originais de economia de energia.

### 8.2 Supervisor com Auto-Recuperação Pós-Suspensão
Caso o host seja forçado a suspender (ex: fechamento físico da tampa do laptop):
1. O kernel desliga os CRTCs e desconecta temporariamente os conectores virtuais.
2. Ao acordar (resume), o laço supervisor do `ext-sender` detecta o encerramento do pipeline ou falha de leitura, reinvoca `pipewire::ensure_kernel_hdmi_connected()` (re-injetando o EDID caso necessário), re-aplica a geometria de tela via `org.gnome.Mutter.DisplayConfig` e restaura o fluxo de vídeo sem intervenção manual.

### 8.3 Limpeza de Tela no Receptor (Black Screen on Disconnect)
No Raspberry Pi Zero (`ext-receiver`):
* Se o host suspender, desconectar o cabo USB ou encerrar o `ext-sender`, o receptor detecta a ausência de novos pacotes por mais de 2.0 segundos.
* O motor de splash aciona imediatamente `SplashEngine::clear()`, preenchendo o framebuffer `/dev/fb0` com preto absoluto (`0x0000`).
* Isso **impede que imagens residuais do desktop do usuário fiquem congeladas na tela secundária**, garantindo privacidade e aparência profissional. Assim que novos quadros chegam, o pipeline V4L2 M2M retoma a renderização em tempo real.

---

## 9. Arquitetura de Binário Único 100% Rust (Zero Dependência de FFmpeg)

Diferente de soluções convencionais que empacotam scripts ao redor do binário do FFmpeg, o `ext-monitor` adota uma política estrita de auto-suficiência:

* **No Receptor (Pi Zero):** Binário monolítico compilado em Rust (`ext-receiver`), contendo decodificador V4L2 M2M para VideoCore IV, servidor DHCP zero-gateway, servidor RTSP Miracast e painel web embarcado em um único arquivo de ~640 KB.
* **No Transmissor (Host PC):** Binário monolítico em Rust (`ext-sender`), contendo integração nativa de hardware com libva (`/dev/dri/renderD128`), libdrm (`/dev/dri/card*`), NVENC via CUDA, OpenH264 em processo, empacotador RTP RFC 6184 e gerenciador de transporte USB Bulk via `rusb`.
* **Zero CLI de FFmpeg:** Elimina dependências externas pesadas e complexas, garantindo portabilidade para qualquer distribuição Linux (Ubuntu, Debian, Fedora, Arch, Alpine).

---

## 10. Referências Cruzadas com outros Blueprints

* [Blueprint 01](file:///home/carlos/ide/ext-monitor/docs/blueprints/01-imagem-32mb-e-geometria-bcm2835.md): Geometria de Boot do Pi Zero e Inicialização RAM.
* [Blueprint 02](file:///home/carlos/ide/ext-monitor/docs/blueprints/02-arquitetura-transmissor-receptor-e-comparativo-gud.md): Arquitetura Transmissor/Receptor e Comparativo GUD.
* [Blueprint 04](file:///home/carlos/ide/ext-monitor/docs/blueprints/04-pipewire-mutter-screencast-e-wayland.md): Handshake D-Bus com GNOME Mutter e PipeWire.
* [Blueprint 05](file:///home/carlos/ide/ext-monitor/docs/blueprints/05-pipeline-vaapi-gstreamer-hardware.md): Pipeline VA-API de Baixa Latência e Otimizações VBR.
* [Blueprint 08](file:///home/carlos/ide/ext-monitor/docs/blueprints/08-manual-de-instalacao-e-portabilidade-host.md): Manual de Instalação e Portabilidade Host.

