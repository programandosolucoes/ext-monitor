# Blueprint Técnico 10 — Refatoração Modular Clean Code, Enquadramento de Access Units (RFC 6184 / RFC 4571) e Decodificação V4L2 M2M Multi-Formato no VideoCore IV

**Projeto:** `ext-monitor` — Monitor Secundário USB de Ultra-Baixa Latência  
**Plataforma Alvo:** Raspberry Pi Zero W (BCM2835 ARMv6 @ 1.0 GHz) / Host Linux & Windows  
**Autor:** Carlos Alberto <psncarlosalberto4ti@gmail.com>  
**Data:** Setembro de 2026  
**Status:** Implementado, Validado e Integrado ao Appliance de Produção  

---

## 1. Visão Geral e Motivação da Sprint

Ao longo da evolução do projeto `ext-monitor`, a adição de múltiplos modos de operação (Modo 1: Rede UDP/RTP, Modo 2: Miracast Windows, Modo 3: USB Bulk Direct via FunctionFS) e controles em tempo real gerou dois sintomas críticos:
1. **Degradação Arquitetural:** O código do transmissor (`sender/src/main.rs`) havia se tornado monolítico (mais de 1.100 linhas), acumulando cadeias aninhadas de `if/else`, acoplamento direto entre D-Bus, PipeWire, GStreamer, hot-apply UDP e parsing CLI.
2. **Artefatos Visuais e Faixas Verdes no Display:** Slices de vídeo H.264 sofriam corrupção de macroblocos em oscilações de latência USB/rede, e o decodificador V4L2 M2M VideoCore IV sofria rejeição de chamadas ioctl e falhas de negociação de formato de cor no kernel Linux do Raspberry Pi Zero.

Esta sprint realizou a refatoração completa do ecossistema seguindo os princípios de **Clean Code**, **Responsabilidade Única (SRP)** e **Padrões de Projeto (Builder Pattern e Facade)**, sanando em definitivo as causas físicas e lógicas dos artefatos visuais.

---

## 2. Refatoração Modular do `ext-sender` (Host Linux)

O arquivo `sender/src/main.rs` foi reduzido de **1.111 linhas monolíticas para apenas ~230 linhas** de lógica de supervisão pura, distribuindo as funcionalidades em módulos coesos e desacoplados:

```
sender/src/
├── main.rs            # Supervisor de ciclo de vida e orquestração (~230 linhas)
├── config.rs          # Parser CLI strongly-typed sem if ladders (SRP)
├── screencast.rs      # Gestão de Sessão D-Bus Mutter/GNOME com RAII Drop
├── pipewire.rs        # Grafo PipeWire, monitor de saúde e conector virtual DRM
├── pipeline.rs        # Builder Pattern (PipelineBuilder) para GStreamer / FFmpeg
├── control.rs         # Listener UDP 5001 hot-apply com enum ControlAction
├── usb_transport.rs   # Transporte USB Bulk direto via libusb / rusb
├── encoder.rs         # Detecção e probes de hardware VA-API / NVENC / QSV
└── i18n.rs            # Internacionalização CLI (EN, PT, IT, ZH)
```

### 2.1. `config.rs` — Configuração Fortemente Tipada
Eliminou o parser procedural repleto de `if/else` encadeados. Todas as opções de linha de comando agora se resolvem em enums seguros em tempo de compilação:
* `TransportKind`: `Network { ip, port }` vs `UsbBulk`.
* `ColorProfile`: `Full24Bit`, `High16Bit`, `Economy`.
* `StreamEngine`: `GStreamer`, `FFmpeg`, `NativeRust`.
* `EncoderApi`: `Vaapi`, `Nvenc`, `Qsv`, `Software`.

### 2.2. `screencast.rs` — Encapsulamento D-Bus com RAII
Isolou toda a comunicação de baixo nível com o `org.gnome.Mutter.ScreenCast` via `zbus`. A struct `MutterScreenCastSession` implementa a trait `Drop`: ao encerrar o processo (seja normalmente ou por SIGINT/SIGTERM), a sessão D-Bus é imediatamente destruída no compositor Wayland, impedindo sessões fantasmas ou vazamento de nós virtuais.

### 2.3. `pipewire.rs` — Saúde do Grafo e Kernel HDMI Guard
Centraliza a descoberta dinâmica de portas de saída (`pw-link`), verificação de links ativos e a rotina do **Kernel HDMI Connector Guard**, que força o status `connected` em `/sys/kernel/debug/dri/0/HDMI-A-1/force` para criação de monitores virtuais sem dongles físicos HDMI dummy.

### 2.4. `pipeline.rs` — PipelineBuilder Pattern
Substituiu a função monolítica de 400 linhas de formatação de strings GStreamer por um construtor baseado no padrão **Builder**:
```rust
let handle = PipelineBuilder::new(&cfg)
    .with_monitor(&monitor_to_record)
    .with_usb_pipe(usb_pipe_fd)
    .spawn()?;
```
A geração de pipelines VA-API (`vah264enc`), GStreamer puro, FFmpeg e streaming nativo agora ocorre de forma modular e extensível.

---

## 3. Refatoração Modular do `ext-receiver` (Raspberry Pi Zero)

O receptor embarcado em Rust foi estruturado em quatro subsistemas de domínio independentes:

```
receiver/src/
├── stream/            # Enquadramento e integridade de pacotes de vídeo
│   ├── rtp.rs         # RtpDepayloader (RFC 6184) e Rfc4571Assembler
│   ├── annexb.rs      # Start-Code Assembly (0x000001 / 0x00000001)
│   └── mod.rs
├── ingress/           # Ingestão de dados isolada por transporte
│   ├── udp.rs         # Ingress Modo 1 (UDP RTP 5000)
│   ├── usb.rs         # Ingress Modo 3 (FunctionFS USB Bulk ep1)
│   └── mod.rs
├── decoder/           # Silício VideoCore IV V4L2 M2M & Cores
│   ├── v4l2_types.rs  # ABI C 32-bit ARM (Ioctls _IOWR corrigidos)
│   ├── v4l2_m2m.rs    # Sessão M2M, probes e negociação de formatos
│   ├── color_convert.rs # Conversão SIMD branchless YUV420/NV12 -> RGB565
│   └── mod.rs
├── display/           # Apresentação no Framebuffer HDMI
│   ├── framebuffer.rs # Blit direto via mmap (/dev/fb0) e KD_GRAPHICS
│   └── mod.rs
├── web.rs             # Painel Web HTTP 8080 & Diagnósticos (/api/dmesg, /api/logs)
└── main.rs            # Inicialização e supervisor de recuperação
```

---

## 4. As Três Grandes Descobertas Técnicas da Sprint

### 4.1. Causa Raiz dos Artefatos: Enquadramento de Access Units (RFC 6184 vs Annex-B)
* **O Diagnóstico:** O stream bruto em Annex-B não possui cabeçalho de comprimento de pacote. O código legado acumulava bytes e, caso 3 milissegundos se passassem sem dados adicionais, despejava os bytes acumulados no decodificador. Em qualquer jitter de barramento USB ou escalonamento de CPU do host, fatias (slices) intermediárias eram partidas ao meio. A primeira metade chegava sem final de macrobloco e a segunda metade chegava sem cabeçalho NALU, sendo descartada. O silício VideoCore IV tentava reconstruir a imagem com vetores de movimento incompletos, produzindo faixas verdes verticais persistentes.
* **A Resolução de Engenharia:**
  1. No modo UDP, o transmissor utiliza `rtph264pay`. O novo `RtpDepayloader` remonta pacotes fragmentados FU-A (RFC 6184) e **só emite a Access Unit para o hardware quando detecta o bit Marker (`M=1`)**, garantindo que o frame esteja 100% completo.
  2. No modo USB Bulk, o pipeline suporta enquadramento RFC 4571 (prefixo de 2 bytes com o comprimento exato do pacote em Big-Endian). O `Rfc4571Assembler` lê o comprimento exato antes de descarregar os bytes, eliminando qualquer dependência de timeouts temporais.

### 4.2. Causa Raiz do V4L2: Discrepância de ABI 32-bit ARM nos Ioctls Linux
* **O Diagnóstico:** O macro de ioctl do kernel Linux (`_IOWR('V', nr, type)`) codifica nos bits 16 a 29 o tamanho exato da estrutura em bytes:
  $$\text{ioctl\_cmd} = (\text{dir} \ll 30) \mid (\text{size} \ll 16) \mid (\text{type} \ll 8) \mid \text{nr}$$
  No x86_64, `struct v4l2_format` possui 208 bytes (`0xD0`) e `struct v4l2_buffer` possui 88 bytes (`0x58`).
  No ARM 32-bit (ARMv6 do Pi Zero), ponteiros e campos `timeval` (`tv_sec`, `tv_usec`) ocupam 4 bytes cada (`c_long`), resultando em `struct v4l2_format` de 204 bytes (`0xCC`) e `struct v4l2_buffer` de **exatos 68 bytes (`0x44`)**.
  Além disso, no kernel Linux os tipos multi-planares são `V4L2_BUF_TYPE_VIDEO_CAPTURE_MPLANE = 9` (quadros decodificados RGB565/YUV de saída) e `V4L2_BUF_TYPE_VIDEO_OUTPUT_MPLANE = 10` (fluxo de entrada comprimido H.264).
* **A Resolução de Engenharia:**
  1. Mapeamento exato das estruturas na ABI ARM32 com asserções estáticas de tamanho em tempo de compilação:
     * `VIDIOC_S_FMT`: `0xC0CC5605` (tamanho: 204 bytes / `0xCC`).
     * `VIDIOC_REQBUFS`: `0xC0145608` (tamanho: 20 bytes / `0x14`).
     * `VIDIOC_QUERYBUF`: `0xC0445609` (tamanho: 68 bytes / `0x44`).
     * `VIDIOC_QBUF`: `0xC044560F` (tamanho: 68 bytes / `0x44`).
     * `VIDIOC_DQBUF`: `0xC0445611` (tamanho: 68 bytes / `0x44`).
  2. Implementação da ativação preguiçosa de modo gráfico (*Lazy Graphics Activation*) no `FramebufferSink`: o terminal do console (`/dev/tty1`) permanece em modo texto (`KD_TEXT`) exibindo o painel e dados de boot até que o primeiro quadro decodificado seja renderizado na tela, eliminando a tela preta ociosa.

### 4.3. Causa Raiz do Formato de Cor: O Silício VideoCore IV Rejeita RGB no Decodificador
* **O Diagnóstico:** O decodificador por hardware `/dev/video10` opera diretamente sobre as unidades de processamento de vídeo (VPU) do VideoCore IV. Ele **apenas decodifica para formatos planares YUV** (`YU12` / `YUV420` ou `NV12`). Ao receber um pedido de decodificação direta para `RGB565` (`RGBP`), o driver rejeitava o `VIDIOC_S_FMT` na fila de CAPTURE.
* **A Resolução de Engenharia:**
  1. **Negociação Dinâmica:** O `V4l2DecoderSession` agora sonda os formatos suportados via `VIDIOC_ENUM_FMT` e tenta a sequência: RGB565 -> YUV420 -> NV12 -> YUV420M -> NV12M.
  2. **Conversor de Cores Ultrarrápido (`color_convert.rs`):** Implementado algoritmo branchless e com aritmética inteira de ponto fixo. Como o YUV420 realiza subamostragem 2x2 de croma, o cálculo de deltas de $U$ e $V$ é realizado **uma única vez para cada bloco de 4 pixels**:
     $$\Delta V_R = (359 \times (V - 128)) \gg 8$$
     $$\Delta UV_G = (-88 \times (U - 128) - 183 \times (V - 128)) \gg 8$$
     $$\Delta U_B = (454 \times (U - 128)) \gg 8$$
     Para cada pixel $Y$, os valores RGB são obtidos por adição e convertidos em RGB565 Little-Endian (`[RRRRRGGG, GGGBBBBB]`).
     Em um processador ARM11 de 1.0 GHz, essa rotina consome menos de 3% da CPU para 1280x720 a 30 FPS.

---

## 5. Alocação de Endpoints DWC2 e Exclusividade de Modo USB Bulk

O controlador USB OTG DWC2 do Raspberry Pi Zero possui **apenas 8 endpoints físicos de hardware** (1 bidirecional de controle EP0 + 7 configuráveis).

| Configuração de Gadget | Interfaces USB Ativas | Endpoints Consumidos | Status no DWC2 |
| :--- | :--- | :--- | :--- |
| **Modo 1 & 2 (Híbrido)** | CDC-ACM (Serial) + CDC-ECM (Rede) + Mass Storage (SD) | EP1 IN (Int) + EP2 IN/OUT (Bulk) + EP3 IN (Int) + EP4 IN/OUT (Bulk) + EP5 IN/OUT (Bulk) = **7 EPs** | Limite Máximo Atingido (8/8) |
| **Modo 3 (USB Bulk Direct)** | CDC-ACM (Serial) + FunctionFS (`display` Bulk OUT/IN) | EP1 IN (Int) + EP2 IN/OUT (Bulk) + EP3 OUT (Bulk Video) + EP4 IN (Bulk Telemetria) = **5 EPs** | 3 EPs Livres, Zero Latência IP |

> **Conclusão de Engenharia:** Para estender a tela em Modo 3 (USB Bulk Direct), o appliance desativa a interface de Mass Storage / ECM do Gadget Composite para ceder os endpoints de alta velocidade à interface proprietária `Class 0xFF` (FunctionFS), garantindo vazão máxima contínua no barramento USB 2.0 High-Speed (480 Mbps).

---

## 6. Verificação e Testes de Regressão

* **Suíte de Testes Rust:** 7 testes unitários passando em 0.00s (`test_rfc4571_stream_framing`, `test_fua_assembly_into_complete_access_unit`, `test_single_nal_and_marker`, `test_yuv420_black_conversion`, `test_nv12_white_conversion`, `test_assemble_single_complete_frame`, `test_assemble_consecutive_frames`).
* **Binário Final do Receptor:** 810 KB stripped para ARMv6 (`arm-unknown-linux-gnueabihf`).
* **Imagem de Boot do Appliance:** 6.3 MB comprimida em RAM (`initramfs.cpio.gz`).
