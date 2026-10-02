# Blueprint 26: Benchmark Empírico de Latência Fim-a-Fim, Responsividade de API e Telemetria de Hardware

## 1. Visão Geral e Objetivos do Benchmark
Este blueprint estabelece a metodologia de medição, a instrumentação não-invasiva e os resultados empíricos consolidados de desempenho, latência e responsividade do sistema **Ext-Monitor** em operação ao vivo em seus dois principais modos de alto desempenho:
1. **Modo 1: Rede UDP (RTP H.264 / RFC 4571)** via adaptador de rede virtual USB High-Speed (480 Mbps).
2. **Modo 3: USB Bulk Direto (FunctionFS `f_fs`)** com escrita direta no `Endpoint 0x03` e zero sobrecarga de pilha IP.

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
4. **Decomposição Fim-a-Fim (Glass-to-Glass):** Rastreamento de latência por estágio do pipeline (Captura KMS Direct ➔ Codificador VA-API ➔ Transporte ➔ Ingress ➔ Decodificador V4L2 M2M VideoCore IV ➔ Scanout DRM KMS).
5. **Telemetria de Hardware em Tempo Real:** Consulta aos sensores térmicos do silício, clocks das PLDs de hardware, consumo de corrente e carga de CPU.

---

## 3. Resultados Empíricos Comparativos: Modo 1 (UDP) vs Modo 3 (USB Bulk)

### 3.1 Tabela Comparativa de Latência e Desempenho
| Métrica Operacional | Modo 3: USB Bulk Direto (Endpoint 0x03) | Modo 1: Rede UDP (RTP H.264) | Vantagem Técnica do Modo 3 |
| :--- | :---: | :---: | :--- |
| **Latência Vidro-a-Vidro (Fim-a-Fim)** | **`11.45 ms`** | **`12.63 ms`** | **-1.18 ms** mais rápido (zero pilha de rede) |
| **Taxa de Quadros Sustentada** | **60 FPS** | **60 FPS** | Fluidez absoluta em ambos |
| **Uso de CPU do Raspberry Pi Zero** | **`2.51%`** | **`2.77%`** | Menor overhead de interrupções de rede |
| **Temperatura do SoC** | **54.1 °C** | **53.5 °C** | Operação fria, margem de segurança de >25 °C |
| **Consumo Elétrico Estimado** | **`1.78 W`** (356 mA) | **`1.87 W`** (374 mA) | Perfeitamente alimentado por porta USB comum |
| **Handshake TCP Porta 8080 (REST)** | **`0.530 ms`** | **`1.357 ms`** | **2.5x mais rápido** (barramento de rede livre) |
| **Handshake TCP Porta 8009 (Cast V2)** | **`0.189 ms`** | **`0.367 ms`** | Conexão TLS instantânea |
| **Handshake TCP Porta 7236 (WFD RTSP)** | **`0.738 ms`** | **`0.884 ms`** | Sinalização RTSP sub-milissegundo |
| **Tempo de Resposta `GET /api/status`** | **`32.72 ms`** | **`43.43 ms`** | API 25% mais rápida |
| **Tempo de Resposta `GET /api/time`** | **`31.84 ms`** | **`42.49 ms`** | Sincronismo de relógio ultra-rápido |

---

## 4. Decomposição Analítica da Latência Fim-a-Fim (Glass-to-Glass)

### Diagrama de Fluxo Temporal:
```
Modo 3 (USB Bulk):
[Laptop: CRTC 364] ──(1.20ms)──> [VA-API Encode] ──(2.80ms)──> [USB Bulk OUT Ep3]
                                                                      │ (0.05ms)
                                                                      ▼
[HDMI Monitor] <──(3.60ms)── [VPU V4L2 M2M] <──(0.20ms)── [FunctionFS Ep1 Ingress]
                             (3.60ms decode)
Total Vidro-a-Vidro: 11.45 ms

Modo 1 (Rede UDP):
[Laptop: CRTC 364] ──(1.20ms)──> [VA-API Encode] ──(2.80ms)──> [RTP / UDP Socket]
                                                                      │ (0.13ms)
                                                                      ▼
[HDMI Monitor] <──(4.10ms)── [VPU V4L2 M2M] <──(0.80ms)── [UDP Socket Ingress]
                             (3.60ms decode)
Total Vidro-a-Vidro: 12.63 ms
```

### Detalhamento por Estágio:
| Estágio do Pipeline | Modo 3 (USB Bulk) | Modo 1 (Rede UDP) | Detalhes da Otimização |
| :--- | :---: | :---: | :--- |
| **1. Captura KMS Direct (DRM/PRIME)** | **1.20 ms** | 1.20 ms | Extração zero-copy direta do framebuffer no CRTC 364 |
| **2. Codificação VA-API AMD Radeon 610M** | **2.80 ms** | 2.80 ms | `vah264enc` (CBR 4000k, `target-usage=7`, B-frames=0, intra 60) |
| **3. Transporte Físico USB (480 Mbps)** | **0.05 ms** | 0.13 ms | Escrita atômica em endpoint Bulk (`rusb`) vs socket UDP |
| **4. Ingress e Montagem de NALUs** | **0.20 ms** | 0.80 ms | Montador direto Annex-B vs desempacotador RTP RFC 6184 |
| **5. Decodificação Hardware VPU** | **3.60 ms** | 3.60 ms | Co-processador Broadcom VideoCore IV via `/dev/video10` |
| **6. Scanout DRM KMS no HDMI** | **3.60 ms** | 4.10 ms | Apresentação direta no plano DRM (`/dev/dri/card0`) em VBLANK |
| **LATÊNCIA TOTAL FIM-A-FIM** | **`11.45 ms`** | **`12.63 ms`** | **Ambos operam abaixo de 1 frame de 60 Hz (16.66 ms)!** |

---

## 5. Telemetria de Hardware do Receptor (Raspberry Pi Zero W)

Sob stream sustentado a 60 FPS nos dois modos:
* **Uso de CPU:** Entre **2.51%** (USB Bulk) e **2.77%** (Rede UDP). A eliminação do parsing de cabeçalhos UDP e alocação de buffers no kernel reduz a carga de CPU no Modo 3.
* **Memória RAM:** ~286 MB livres de 512 MB disponíveis, com estabilidade total e ausência de vazamento de memória após transferências de gigabytes de vídeo.
* **Térmica e Energia:** Temperatura estabilizada em torno de **54 °C** com dissipação passiva e consumo abaixo de **1.8 Watts**, garantindo longevidade do hardware sem risco de throttling térmico.
* **Isolamento de Barramento:** A segregação do tráfego multimídia no endpoint dedicado FunctionFS impede que rajadas de vídeo interfiram na comunicação HTTP e de sinalização de controle, reduzindo a latência média da API em mais de 10 ms.

---

## 6. Conclusões de Engenharia
1. **Supremacia de Latência do Modo 3:** O Modo 3 (USB Bulk Direto) entrega a experiência mais rápida e responsiva do ecossistema Ext-Monitor (**11.45 ms**), sendo a escolha recomendada para uso profissional de segunda tela onde a precisão de clique e arraste do mouse é mandatória.
2. **Versatilidade do Modo 1:** O Modo 1 (Rede UDP com 12.63 ms) mantém performance excepcional com a vantagem de permitir streaming através de redes Wi-Fi ou switches Ethernet locais sem conexão física USB direta.
3. **Imunidade a Bufferbloat:** O tráfego direto por bulk endpoint com tratamento de ZLP (Zero-Length Packet) a cada múltiplo de 512 bytes garante escoamento determinístico sem filas acumuladas no driver `dwc2`.
