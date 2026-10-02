# Blueprint 27: Arquitetura Orientada a Fluxos em Microblocos, Desacoplamento Físico e Suporte a Codecs Genéricos (H.264 / HEVC H.265)

## 1. Visão Geral e Princípios Fundamentais

Este blueprint formaliza a reestruturação arquitetural do ecossistema **Ext-Monitor** para garantir modularidade extrema, manutenibilidade independente e desacoplamento absoluto entre camadas.

### O Problema do Monobloco:
Anteriormente, funções de captura, codificação, empacotamento de transporte (USB vs UDP vs RTSP) e decodificação residiam frequentemente em arquivos extensos com alto acoplamento contextual. Uma alteração no tratamento de pacotes ZLP do USB podia inadvertidamente afetar o fluxo de pacotes MPEG-TS ou o depayloader RTP.

### A Filosofia de Microblocos Orientados ao Fluxo:
O sistema é decomposto em **microblocos funcionais e autodocumentados**. O nome de cada arquivo, struct e função espelha exatamente a sua posição e função na esteira unidirecional de dados:
```
[Captura Scanout] ➔ [Codificação GPU] ➔ [Egress Transporte] ➔ [Link Físico] ➔ [Ingress Transporte] ➔ [Demux / Depayload] ➔ [Decodificação Hardware] ➔ [Apresentação KMS]
```

Cada microbloco possui:
1. **Contrato de Interface Mínima (Deep Module):** Uma interface enxuta com implementação profunda e robusta.
2. **Tratamento de Borda Autônomo:** Validação de cabeçalhos, filtragem de dados corrompidos, timeout explícito e recuperação de falha sem interromper os módulos adjacentes.
3. **Isolamento de Egress e Ingress:** Nenhuma dependência cruzada entre entrada e saída; o USB não sabe o que é H.264 e a VPU não sabe se os bytes vieram de um cabo USB ou de um socket UDP.

---

## 2. Mapa do Fluxo de Dados e Topologia de Microblocos

### 2.1 Lado Emissor (Host / Transmitter):
```
sender/src/flow/
├── capture_kms_direct.rs       # Captura de quadro zero-copy no CRTC KMS primário/secundário
├── capture_pipewire_mutter.rs  # Captura de tela via portal screencast PipeWire D-Bus
├── encoder_types.rs            # Tipos universais de vídeo, codecs (H264, HEVC), perfis e taxas
├── encoder_gpu_selector.rs     # Despacho automático de encoder de hardware (VA-API / NVENC / QSV / CPU)
├── egress_usb_bulk.rs          # Escrita atômica em endpoint Bulk OUT (0x03) com tratamento de ZLP
├── egress_network_udp.rs       # Transmissão de datagramas UDP RFC 4571 para socket receptor
└── egress_miracast_wfd.rs      # Orquestrador de sessão RTSP WFD e multiplexação MPEG-TS
```

### 2.2 Lado Receptor (Appliance / Receiver):
```
receiver/src/flow/
├── ingress_usb_bulk.rs         # Leitura contínua de Endpoint USB FunctionFS (0x01/0x03)
├── ingress_network_udp.rs      # Receptor de sockets UDP (portas 5000 / 5002)
├── ingress_rtsp_wfd.rs         # Servidor de sinalização RTSP WFD (porta 7236 / 7250)
├── demux_annexb_assembler.rs   # Montador de NALUs Annex-B, SPS/PPS e detecção de IDR
├── demux_rtp_depayloader.rs    # Desempacotador RFC 6184 (H.264) e RFC 7798 (HEVC H.265)
├── demux_mpegts_parser.rs      # Parser MPEG-TS de 188 bytes, PAT, PMT, PES e sincronismo PCR
├── codec_types.rs              # Definições universais de CodecKind (H264, HevcH265, Av1)
├── codec_v4l2_m2m.rs           # Decodificador de hardware VideoCore IV / rpivid V4L2 M2M
├── codec_vaapi.rs              # Decodificador de hardware VA-API para receptores x86_64
├── scanout_frame_pacer.rs      # Pacing determinístico CFR (60/30 FPS) e mitigação de congelamento
└── scanout_drm_kms.rs          # Importação e scanout de buffers DMA-BUF no plano DRM do HDMI
```

---

## 3. Especificação do Suporte a Codecs Genéricos (H.264 e HEVC / H.265)

### 3.1 Motivação e Matriz de Hardware

| Plataforma / Hardware | Bloco H.264 (AVC) | Bloco HEVC (H.265) | Eficiência de Banda | Caso de Uso |
| :--- | :---: | :---: | :---: | :--- |
| **Raspberry Pi Zero / W (BCM2835)** | ✅ VideoCore IV (ASIC) | ❌ Inexistente (CPU chokes) | Padrão baseline (100%) | **Padrão Obrigatório** |
| **Raspberry Pi 4 / 400 (BCM2711)** | ✅ VideoCore VI | ✅ Hardware 4K60 (`rpivid`) | **-45% de banda** | Alta resolução / Wi-Fi |
| **Raspberry Pi Zero 2 W (BCM2710A1)**| ✅ VideoCore IV | ⚠️ Software multithread 4 cores | -40% de banda (720p) | Transição eficiente |
| **PC / Notebook Receptor (x86_64)**  | ✅ VA-API / NVDEC | ✅ VA-API / NVDEC (10-bit) | **-50% de banda** | PC como tela remota |
| **PlayStation 5 / Chiaki / Moonlight** | ✅ Hardware | ✅ Hardware nativo | **-50% de banda** | Streaming de baixa latência |

### 3.2 Enum Universal de Codecs (`codec_types.rs`)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum CodecKind {
    /// H.264 / AVC (Advanced Video Coding - ISO/IEC 14496-10)
    H264,
    /// H.265 / HEVC (High Efficiency Video Coding - ISO/IEC 23008-2)
    HevcH265,
    /// AOMedia Video 1 (AV1)
    Av1,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StreamNegotiation {
    pub supported_codecs: Vec<CodecKind>,
    pub preferred_codec: CodecKind,
    pub max_width: u32,
    pub max_height: u32,
    pub max_fps: u32,
}
```

### 3.3 Regras de Auto-Negociação e Fallback
1. O receptor anuncia suas capacidades no handshake inicial (`GET /api/status` ou OPTIONS RTSP).
2. Se o receptor for um **Raspberry Pi Zero W**, ele restringe o suporte a `CodecKind::H264`.
3. Se o receptor for um **Pi 4**, **PC x86_64** ou cliente com suporte a HEVC declarado, o emissor negocia preferencialmente `CodecKind::HevcH265`.
4. Em caso de falha de decodificação no primeiro GOP de HEVC, ocorre fallback atômico em menos de 100 ms para `CodecKind::H264`.

---

## 4. Isolamento Estrito da Fronteira USB (OTG / FunctionFS)

### 4.1 A Fronteira de Ingress (`ingress_usb_bulk.rs`)
O microbloco de entrada USB tem **responsabilidade única**:
* Ler buffers do endpoint `/dev/usb-ffs/display/ep1` (ou `ep3`).
* Descartar pacotes de tamanho zero (ZLP - Zero-Length Packet).
* Validar cabeçalhos de enquadramento (RFC 4571 / cabeçalho de framing).
* Entregar fatias de bytes limpas para o canal sem bloqueio (`crossbeam_channel` ou `sync_channel`) do demuxer.
* **Proibição expressa:** O microbloco USB não decodifica vídeo, não analisa NALUs e não conhece o plano KMS.

### 4.2 A Fronteira de Egress (`egress_usb_bulk.rs`)
O microbloco de saída USB no Host tem **responsabilidade única**:
* Receber fatias codificadas do encoder.
* Adicionar prefixo RFC 4571 de 2 bytes (comprimento da fatia em Big-Endian).
* Transmitir via chamada USB Bulk no endpoint `0x03`.
* Se o tamanho da transmissão for múltiplo exato de 512 bytes, emitir imediatamente um ZLP para flush do FIFO do hardware do Pi Zero.
* Capturar e recuperar automaticamente erros de pipe (`rusb::Error::Pipe` / Endpoint Stall) sem derrubar o pipeline de captura.

---

## 5. Overclock e Clocks Travados do Silício

Conforme validado no hardware BCM2835:
```ini
# --- Overclock Travado no Raspberry Pi Zero W ---
arm_freq=1050       # CPU ARM1176JZF-S (+5% headroom)
arm_freq_min=1050   # Fixa frequência eliminando atraso do governor de energia
core_freq=500      # VideoCore IV VPU & L2 Cache (+25% no throughput do decodificador)
core_freq_min=500  # Fixa frequência da VPU em 500 MHz
sdram_freq=500     # LPDDR2 DMA throughput (+11% a +25% de largura de banda)
over_voltage=2     # +0.05V para estabilidade contínua
force_turbo=1      # Fixa clocks permanentemente sem oscilação
```

Com esses parâmetros:
* O tempo de decodificação V4L2 M2M por quadro cai para sub-3.0 ms.
* O barramento de memória sustenta transferência DMA-BUF zero-copy para o DRM KMS a 60 FPS contínuos.

---

## 6. Critérios de Aceite e Verificação Automatizada (TDD)
1. **Compilação e Verificação Limpa:** Todo o workspace compila com zero avisos (`cargo check --workspace`).
2. **Suíte de Testes Unitários dos Microblocos:**
   - Teste de enquadramento ZLP e RFC 4571 no USB.
   - Teste de parsing e depayload de pacotes RTP (H.264 e HEVC).
   - Teste de detecção de tipo de codec e auto-negociação.
   - Teste de montagem de fatias Annex-B a partir de fluxos fragmentados.
3. **Compatibilidade Regressiva Total:** Os binários existentes (`ext-sender`, `ext-receiver`, `ext-miracast`) continuam funcionando com a mesma sintaxe de linha de comando e comportamento operacional.
