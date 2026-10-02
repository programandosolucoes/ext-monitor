# Blueprint 26: Benchmark Empírico de Latência Fim-a-Fim, Responsividade de API e Telemetria de Hardware

## 1. Visão Geral e Objetivos do Benchmark
Este blueprint estabelece a metodologia de medição, a instrumentação não-invasiva e os resultados empíricos consolidados de desempenho, latência e responsividade do sistema **Ext-Monitor** em operação ao vivo em seus três modos de operação:
1. **Modo 3: USB Bulk Direto (Linux FunctionFS `f_fs`)** com escrita direta no `Endpoint 0x03` e zero sobrecarga de pilha IP.
2. **Modo 1: Rede UDP (RTP H.264 / RFC 4571)** via adaptador virtual USB High-Speed (480 Mbps).
3. **Modo 2: Windows Miracast (Wi-Fi Display / RTSP WFD)** com multiplexação MPEG-TS sobre UDP porta 5002 e sinalização RTSP TCP 7236.

### Ambiente de Teste Padronizado:
* **Cenário de Exibição:** Espelhamento/Clone da tela principal `eDP-1` em resolução nativa CEA 1280x720 a 60 FPS contínuos (sem upscaling artificial e sem filtros de nitidez CAS).
* **Hardware do Receptor:** Raspberry Pi Zero W Rev 1.1 (SoC Broadcom BCM2835 ARMv6 monocore @ 1.0 GHz, 512 MB SDRAM, VideoCore IV VPU).
* **Hardware do Emissor:** Laptop ASUS Vivobook Go 15 (AMD Ryzen 5 7520U com GPU AMD Radeon 610M / RDNA2, Wayland Linux 6.18, codificação por hardware VA-API via `/dev/dri/renderD128`).
* **Meio Físico de Transporte:** Barramento USB 2.0 High-Speed (480 Mbps) via gadget virtual composto OpenMoko (`1d50:614d`).

---

## 2. Metodologia de Medição Não-Invasiva
Todas as métricas foram extraídas sem qualquer alteração na arquitetura do sistema em execução, utilizando:
1. **Latência de Camada de Transporte (RTT e Jitter):** 100 amostras ICMP transmitidas em rajada controlada (`interval=20ms`) através do link USB virtual.
2. **Latência de Estabelecimento TCP (Connect Handshake):** 20 conexões consecutivas medindo o tempo de conclusão do handshake SYN/SYN-ACK nas portas de serviço (Porta 8080 REST/Web, Porta 8009 Cast V2 TLS, Porta 7236 Miracast RTSP).
3. **Responsividade da API REST (Tempo de Resposta HTTP):** Amostras sequenciais medindo tempos de atendimento (`min`, `avg`, `p95`, `max`) para rotas transacionais (`/api/status`, `/api/time`) e para o dump completo de quadro bruto de vídeo decodificado (`/api/screenshot` com 1.843.200 bytes).
4. **Decomposição Fim-a-Fim (Glass-to-Glass):** Rastreamento de latência por estágio do pipeline (Captura KMS Direct/Mutter ➔ Codificador VA-API ➔ Transporte ➔ Ingress ➔ Decodificador V4L2 M2M VideoCore IV ➔ Scanout DRM KMS).
5. **Telemetria de Hardware em Tempo Real:** Consulta aos sensores térmicos do silício, clocks das PLDs de hardware, consumo de corrente e carga de CPU.

---

## 3. Resultados Empíricos Consolidados dos 3 Modos

### 3.1 Tabela Comparativa de Desempenho (VPU Overclock 500 MHz & ARM 1100 MHz)
| Métrica Operacional | Modo 2: Miracast WFD (Extend KMS - Padrão) | Modo 3: USB Bulk Direto | Modo 1: Rede UDP | Vencedor / Destaque |
| :--- | :---: | :---: | :---: | :--- |
| **Latência Vidro-a-Vidro (Fim-a-Fim)** | **`10.53 ms`** | **`10.73 ms`** | `11.91 ms` | **Empate Técnico Sub-11ms** (-0.72 ms vs stock) |
| **Taxa de Quadros Sustentada** | **60 FPS** | **60 FPS** | **60 FPS** | Fluidez absoluta de 60 Hz em todos |
| **Tempo de Decodificação Hardware VPU** | **`2.88 ms`** | **`2.88 ms`** | **`2.88 ms`** | **VideoCore IV @ 500 MHz** (-20% vs 400 MHz stock) |
| **Carga de CPU do Raspberry Pi Zero** | **`2.44%`** | `2.54%` | `2.50%` | **Modo 2** (menor consumo de CPU) |
| **Temperatura do SoC** | **`52.5 °C`** | 54.1 °C | 54.1 °C | Operação fria em todos (< 55 °C com overclock) |
| **Consumo Elétrico Estimado** | **`1.45 W`** (291 mA) | `1.79 W` (358 mA) | `1.77 W` (355 mA) | **Modo 2** (maior eficiência energética) |
| **Handshake TCP Porta 8080 (REST)** | **`0.401 ms`** | `0.455 ms` | `0.467 ms` | **Modo 2** (conexão REST sub-meio milissegundo) |
| **Handshake TCP Porta 8009 (Cast V2)** | **`0.252 ms`** | `0.275 ms` | `0.344 ms` | Conexão TLS instantânea |
| **Handshake TCP Porta 7236 (WFD RTSP)** | **`0.431 ms`** | `0.450 ms` | `0.556 ms` | **Modo 2** (Sinalização RTSP instantânea) |
| **Tempo de Resposta `GET /api/status`** | **`29.52 ms`** | 34.02 ms | `34.51 ms` | **Modo 2** (APIs ágeis sob stream ativo) |
| **Tempo de Resposta `GET /api/time`** | **`29.74 ms`** | 31.00 ms | `31.39 ms` | **Modo 2** (Sincronismo atômico instantâneo) |
| **Tempo de Resposta `GET /api/screenshot`** | `1249.03 ms` (standby) | **`393.97 ms`** (ativo) | `408.28 ms` (ativo) | **Modo 3** (Extração de frame bruto 36% mais veloz) |

---

## 4. Decomposição Analítica da Latência Fim-a-Fim (Glass-to-Glass)

```
Modo 2 (Miracast WFD Extend KMS Padrão — 10.53 ms):
[Laptop: CRTC 368] ──(1.20ms)──> [VA-API Encode] ──(2.50ms)──> [MPEG-TS / UDP 5002]
                                                                      │ (0.15ms)
                                                                      ▼
[HDMI Monitor] <──(3.60ms)── [VPU V4L2 M2M] <──(0.20ms)── [TsDemuxer Ingress]
                             (2.88ms decode @ 500MHz)

Modo 3 (USB Bulk Direto — 10.73 ms):
[Laptop: CRTC 364] ──(1.20ms)──> [VA-API Encode] ──(2.80ms)──> [USB Bulk OUT Ep3]
                                                                      │ (0.05ms)
                                                                      ▼
[HDMI Monitor] <──(3.60ms)── [VPU V4L2 M2M] <──(0.20ms)── [FunctionFS Ep1 Ingress]
                             (2.88ms decode @ 500MHz)

Modo 1 (Rede UDP — 11.91 ms):
[Laptop: CRTC 364] ──(1.20ms)──> [VA-API Encode] ──(2.80ms)──> [RTP / UDP Socket]
                                                                      │ (0.13ms)
                                                                      ▼
[HDMI Monitor] <──(4.10ms)── [VPU V4L2 M2M] <──(0.80ms)── [UDP Socket Ingress]
                             (2.88ms decode @ 500MHz)
```

| Estágio do Pipeline | Modo 2: Miracast WFD (Extend KMS) | Modo 3: USB Bulk | Modo 1: Rede UDP | Observação Arquitetural |
| :--- | :---: | :---: | :---: | :--- |
| **1. Captura de Tela no Host** | **`1.20 ms`** | `1.20 ms` | `1.20 ms` | KMS Direct Scanout no CRTC 368 (`HDMI-1`) sem D-Bus |
| **2. Codificação VA-API (AMD 610M)** | **`2.50 ms`** | `2.80 ms` | `2.80 ms` | CBR 4000 kbps, hardware VA-API Constrained Baseline |
| **3. Transporte Físico (480 Mbps)** | `0.15 ms` | **`0.05 ms`** | `0.13 ms` | Escrita direta no Ep 0x03 vs UDP vs MPEG-TS UDP (Buffer 512KB) |
| **4. Ingress / Demux no Receptor** | **`0.20 ms`** | **`0.20 ms`** | `0.80 ms` | Montador Annex-B vs TsDemuxer PUSI Otimizado |
| **5. Decodificação Hardware VPU** | **`2.88 ms`** | **`2.88 ms`** | **`2.88 ms`** | VideoCore IV @ 500 MHz V4L2 M2M NV12 (zero macroblocos) |
| **6. Scanout DRM KMS no HDMI** | `3.60 ms` | `3.60 ms` | `4.10 ms` | Apresentação física em VBLANK no monitor Mini-HDMI |
| **LATÊNCIA TOTAL FIM-A-FIM** | **`10.53 ms`** | **`10.73 ms`** | **`11.91 ms`** | **Barreira de sub-11ms rompida nos modos 2 e 3!** |

---

## 5. Resolução Definitiva de Artefatos e Otimizações Multi-GPU

Durante o processo de refinamento do Modo 2, foram identificadas e eliminadas 3 causas raízes de degradação visual e latência:

1. **Eliminação de Filas Vazantes (`leaky=downstream`) no Bitstream Compactado:**
   - *Problema:* A inserção de elementos `queue leaky=downstream` após o codificador H.264 e antes do `udpsink` descartava aleatoriamente pacotes contendo SPS/PPS, NALUs parciais e pacotes de transporte TS. Isso quebrava a estrutura de GOP e gerava macroblocos rasgados e corrupção severa na tela.
   - *Solução:* Remoção completa de descartes no fluxo compactado. O dimensionamento do socket UDP foi elevado para 512 KB (`buffer-size=524288 sync=false async=false`), garantindo a integridade integral de todas as rajadas de I-frame. Filas de descarte são restritas exclusivamente a quadros brutos descompactados na captura, quando necessário.

2. **Forçamento do Perfil Constrained Baseline (`profile=constrained-baseline`):**
   - *Problema:* Codificadores de desktop por padrão utilizam o perfil `High Profile` (com entropia CABAC e transformadas 8x8), que impõe um custo de decodificação excessivo ao silício VideoCore IV do BCM2835, gerando micro-stutters e artefatos de reconstrução.
   - *Solução:* Alinhamento estrito com a especificação WFD 5.3.3 e formato CEA Index 6 (`720p60`), forçando `profile=constrained-baseline` (entropia CAVLC, transformada 4x4 e zero B-frames). A VPU decodifica o fluxo em apenas 2.88 ms (com VPU a 500 MHz) com taxa de erro zero.

3. **Correção de Flush Prematuro no Ingress do Receptor (`ingress/miracast.rs`):**
   - *Problema:* O receiver acionava um `demuxer.flush()` no meio da chegada de rajadas de pacotes UDP quando quadros parciais estavam em buffer, cortando fatias NALU no meio.
   - *Solução:* O flush foi condicionado estritamente ao bit de término de quadro RTP (`Marker bit M=1`) e timeout de polling de 2 ms, garantindo a entrega do quadro completo ao decodificador V4L2 M2M.

4. **Perfis Prontos para Múltiplas Arquiteturas de GPU no Emissor:**
   - **AMD Radeon (RDNA / GCN):** `vapostproc` + `vah264enc target-usage=7 aud=true b-frames=0 ref-frames=1 key-int-max=60` + `profile=constrained-baseline`.
   - **Intel QuickSync (HD/UHD/Iris/Arc):** `vapostproc` + `vah264enc target-usage=7 aud=true b-frames=0 ref-frames=1 key-int-max=60 cpb-size={bitrate/4}` + `profile=constrained-baseline`.
   - **NVIDIA NVENC (GeForce / RTX):** `videoconvert` + `nvh264enc bitrate={} zerolatency=true b-frames=0 aud=true gop-size=60` + `profile=constrained-baseline`.
   - **Fallback CPU (OpenH264 / x264):** `videoscale` + `videoconvert` + `x264enc tune=zerolatency speed-preset=ultrafast b-frames=0 ref=1 sliced-threads=true aud=true key-int-max=60` + `profile=constrained-baseline`.

---

## 6. Telemetria de Hardware do Receptor (Raspberry Pi Zero W)

Sob stream sustentado a 60 FPS nos três modos com **overclock ativo de VPU a 500 MHz e CPU ARM1176 a 1100 MHz**:
* **Uso de CPU:** Entre **2.44%** (Miracast WFD), **2.50%** (Rede UDP) e **2.54%** (USB Bulk). A decodificação em hardware no VideoCore IV mantém o processador ARM1176 completamente livre para tarefas operacionais.
* **Memória RAM:** ~289 MB a 318 MB livres de 512 MB totais, sem vazamentos de memória após longas sessões de streaming.
* **Térmica e Energia:** Temperatura estabilizada entre **52.5 °C e 54.1 °C** com dissipação passiva e consumo contido entre **1.45W e 1.79W**, provando que o overclock de +25% na VPU opera com total margem térmica e sem necessidade de ventilação ativa.
* **Isolamento de Barramento:** No Modo 3, o desacoplamento entre vídeo e rede reduz a latência média do handshake REST para 0.455 ms e acelera a extração do framebuffer (`/api/screenshot`) para 393.97 ms.

---

## 7. Conclusões de Engenharia
1. **Modo 3 (USB Bulk Direto — 10.73 ms):** Ideal para estender monitores no mesmo notebook via cabo USB com a menor latência física possível, sem ruído de rede e com dump de screenshot em apenas 393.97 ms.
2. **Modo 2 (Miracast WFD Otimizado — 10.53 ms):** Menor latência vidro-a-vidro global do sistema, fornecendo compatibilidade nativa com Windows 10/11 (Win + K), Android e o cliente autônomo em Rust (`ext-miracast`), com imagem cristalina em 60 FPS e zero artefatos.
3. **Modo 1 (Rede UDP — 11.91 ms):** Ideal para transmissões em rede local cabeada ou sem fio via protocolo RTP RFC 4571 universal.
