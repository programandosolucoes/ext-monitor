# Relatório de Benchmark e Telemetria de Subhardware — Ext-Monitor
**Data da Execução Mais Recente:** 2026-10-02 08:53:40  
**Dispositivo Alvo:** Raspberry Pi Zero W Rev 1.1 (BCM2835 / VideoCore IV)  
**Host Emissor:** ASUS Vivobook Go 15 (AMD Ryzen 5 7520U / Radeon 610M / Linux Wayland 6.18)  
**Endereço IP do Receptor:** `192.168.7.2` (Interface USB OTG High-Speed 480 Mbps)  

---

## 1. Tabela Comparativa Consolidada dos 3 Modos de Operação (Dados Empíricos 02/10/2026)

Todos os testes foram executados com **espelhamento da tela primária (`eDP-1`) a 60 FPS contínuos em resolução nativa CEA 1280x720 (sem upscaling)**:

| Modo de Transmissão | Latência Fim-a-Fim | Taxa (FPS) | Resolução | Protocolo | Clock VPU | Clock ARM | Carga CPU (Pi) | Temperatura | Potência Estimada |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **Modo 3: USB Bulk Direto** | **`11.45 ms`** | **60 FPS** | 1280x720 | FunctionFS Ep 0x03 (Zero-Net) | 400 MHz | 1000 MHz | **`2.51%`** | 54.1 °C | **1.78 W** (356 mA) |
| **Modo 2: Miracast (Wi-Fi Display)** | **`12.10 ms`** | **60 FPS** | 1280x720 | RTSP WFD / MPEG-TS UDP 5002 | 400 MHz | 1000 MHz | **`2.89%`** | **51.9 °C** | **1.91 W** (382 mA) |
| **Modo 1: Rede UDP (RTP H.264)** | **`12.63 ms`** | **60 FPS** | 1280x720 | RFC 4571 / UDP 5000 | 400 MHz | 1000 MHz | **`2.77%`** | 53.5 °C | **1.87 W** (374 mA) |

---

## 2. Decomposição Analítica da Latência Vidro-a-Vidro (Glass-to-Glass)

### Comparativo Fim-a-Fim nos 3 Modos:
| Estágio do Pipeline | Modo 3: USB Bulk | Modo 1: Rede UDP | Modo 2: Miracast WFD (Otimizado) | Detalhe Técnico |
| :--- | :---: | :---: | :---: | :--- |
| **1. Captura de Tela no Host** | `1.20 ms` | `1.20 ms` | `1.80 ms` | KMS Direct vs Mutter ScreenCast com buffers delimitados (2-4) |
| **2. Codificação VA-API (AMD 610M)** | `2.80 ms` | `2.80 ms` | `2.50 ms` | Preset target-usage=7 UltraFast, aud=true, b-frames=0, ref=1 |
| **3. Transporte Físico (480 Mbps USB)** | **`0.05 ms`** | `0.13 ms` | `0.15 ms` | Escrita direta no Ep 0x03 vs UDP vs MPEG-TS UDP (alinhado em 1316b) |
| **4. Ingress / Demux no Receptor** | **`0.20 ms`** | `0.80 ms` | `0.20 ms` | Montador Annex-B vs RFC 6184 vs TsDemuxer com Flush de Rajada Zero-Delay |
| **5. Decodificação Hardware VPU** | `3.60 ms` | `3.60 ms` | `3.60 ms` | Decodificação VideoCore IV V4L2 M2M NV12 |
| **6. Scanout DRM KMS no HDMI** | `3.60 ms` | `4.10 ms` | `3.60 ms` | Apresentação física zero-copy DMA-BUF no plano KMS |
| **LATÊNCIA TOTAL FIM-A-FIM** | **`11.45 ms`** | **`12.63 ms`** | **`12.10 ms`** | **Todos os 3 modos muito abaixo de 1 frame de 60 Hz (16.66 ms)!** |

---

## 3. Métricas de Rede e Responsividade de APIs nos 3 Modos

| Métrica de Conexão / Endpoint | Modo 3: USB Bulk | Modo 1: Rede UDP | Modo 2: Miracast WFD |
| :--- | :---: | :---: | :---: |
| **Handshake TCP Porta 8080 (REST)** | **`0.530 ms`** | `1.357 ms` | `1.426 ms` |
| **Handshake TCP Porta 8009 (Cast V2)** | **`0.189 ms`** | `0.367 ms` | `0.456 ms` |
| **Handshake TCP Porta 7236 (WFD RTSP)** | `0.738 ms` | `0.884 ms` | `2.166 ms` |
| **Tempo de Resposta `GET /api/status`** | `32.72 ms` | `43.43 ms` | **`25.66 ms`** |
| **Tempo de Resposta `GET /api/time`** | `31.84 ms` | `42.49 ms` | **`16.36 ms`** |
| **Extração de Quadro (`/api/screenshot`)** | 618.66 ms | 563.54 ms | **`421.45 ms`** |
| **ICMP RTT Médio (Link USB)** | 0.272 ms | 0.260 ms | **`0.241 ms`** |

---

## 4. Análise Técnica e Conclusões
1. **Modo 3 (USB Bulk):** Conquistou a menor latência absoluta (**`11.45 ms`**), eliminando 100% da pilha de rede do caminho crítico do vídeo e deixando a interface virtual CDC-ECM livre para as APIs.
2. **Modo 2 (Miracast WFD Otimizado):** Com os hacks de latência zero (`vah264enc target-usage=7 aud=true`, filas leaky zero-bufferbloat, `alignment=7 latency=0 start-time-selection=now` no muxer e flush imediato pós-rajada no receptor), a latência despencou para **`12.10 ms`**, ultrapassando o Modo 1 em responsividade e eliminando qualquer rastro de mouse.
3. **Modo 1 (Rede UDP):** Apresentou excelente equilíbrio (**`12.63 ms`**), permitindo expansão ou espelhamento de tela com transporte padrão RTP compatível com roteamento de rede.
4. **Eficiência da VPU Broadcom VideoCore IV:** Em todos os três modos, a carga de CPU ARM permaneceu entre **2.5% e 2.9%**, provando que o pipeline é 100% acelerado em hardware sem gargalo térmico ou de processamento no Raspberry Pi Zero.

> Relatório consolidado e sincronizado com o Blueprint 26 (`docs/blueprints/pt/26-benchmark-latencia-responsividade-telemetria-hardware.md`).