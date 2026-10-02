# Relatório de Benchmark e Telemetria de Subhardware — Ext-Monitor
**Data da Execução Mais Recente:** 2026-10-02 08:43:30  
**Dispositivo Alvo:** Raspberry Pi Zero W Rev 1.1 (BCM2835 / VideoCore IV)  
**Host Emissor:** ASUS Vivobook Go 15 (AMD Ryzen 5 7520U / Radeon 610M / Linux Wayland 6.18)  
**Endereço IP do Receptor:** `192.168.7.2` (Interface USB OTG High-Speed 480 Mbps)  

---

## 1. Tabela Comparativa de Desempenho e Clocks (Dados Empíricos 02/10/2026)

| Modo de Transmissão | Latência Fim-a-Fim | Taxa (FPS) | Resolução | Clock VPU | Clock ARM | Carga CPU (Pi) | Temperatura | Potência Estimada |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **Modo 3: USB Bulk Direto (FunctionFS)** | **`11.45 ms`** | **60 FPS** | 1280x720 | 400 MHz | 1000 MHz | **2.51%** | 54.1 °C | **1.78 W** (356 mA) |
| **Modo 1: Rede UDP (Clone Nativo 720p60)** | **`12.63 ms`** | **60 FPS** | 1280x720 | 400 MHz | 1000 MHz | **2.77%** | 53.5 °C | **1.87 W** (374 mA) |
| **Modo 1: Rede UDP (RTP H.264 Estendido)** | `13.30 ms` | **60 FPS** | 1280x720 | 400 MHz | 1000 MHz | 3.10% | 41.2 °C | 0.85 W (170 mA) |
| **Modo 2: Miracast (Wi-Fi Display MS-MICE)** | `30.30 ms` | **30 FPS** | 1280x720 | 400 MHz | 1000 MHz | 5.20% | 41.1 °C | 0.85 W (170 mA) |

---

## 2. Decomposição da Latência Vidro-a-Vidro (Glass-to-Glass)

### Comparativo Fim-a-Fim: Modo 3 (USB Bulk) vs Modo 1 (Rede UDP)
| Estágio do Pipeline | Modo 3 (USB Bulk) | Modo 1 (Rede UDP) | Diferencial Técnico |
| :--- | :---: | :---: | :--- |
| **1. Captura de Tela (KMS Direct / Mutter)** | `1.20 ms` | `1.20 ms` | Extração atômica idêntica do CRTC primário (`eDP-1`) |
| **2. Codificação H.264 (VA-API Radeon 610M)** | `2.80 ms` | `2.80 ms` | Aceleração por hardware no Host (CBR 4000 kbps, zero-latency) |
| **3. Transporte Físico (480 Mbps USB 2.0)** | **`0.05 ms`** | `0.13 ms` | Escrita direta no Endpoint 0x03 vs socket UDP de kernel |
| **4. Ingress / Montagem de Quadros** | **`0.20 ms`** | `0.80 ms` | Montador Annex-B direto vs desempacotador RTP RFC 4571 |
| **5. Decodificação VPU (V4L2 M2M BCM2835)** | `3.60 ms` | `3.60 ms` | Decodificação direta por hardware VideoCore IV |
| **6. Scanout DRM KMS (Plane HDMI-A-1)** | **`3.60 ms`** | `4.10 ms` | Apresentação direta sem contenção de buffers de rede |
| **Latência Total Vidro-a-Vidro** | **`11.45 ms`** | **`12.63 ms`** | **Modo 3 economiza 1.18 ms (Zero-Network Stack)!** |

---

## 3. Métricas de Rede e Responsividade de APIs sob Carga Ativa

Com o streaming de vídeo de 60 FPS descarregado 100% no canal USB Bulk físico (`Endpoint 0x03`), a pilha de rede CDC-ECM (`192.168.7.2`) ficou completamente desafogada, acelerando as rotas de controle e REST API:

### 3.1 Latência de Conexão TCP (Connect Handshake)
* **Porta 8080 (REST / Web Dashboard):** Médio: **`0.530 ms`** (era 1.357 ms no Modo 1) | P95: `0.916 ms`
* **Porta 8009 (Google Cast V2 TLS):** Médio: **`0.189 ms`** (era 0.367 ms no Modo 1) | P95: `0.344 ms`
* **Porta 7236 (Miracast / WFD RTSP):** Médio: **`0.738 ms`** (era 0.884 ms no Modo 1) | P95: `4.765 ms`

### 3.2 Responsividade HTTP REST
* `GET /api/status`: Médio: **`32.72 ms`** (era 43.43 ms no Modo 1) | P95: `39.70 ms`
* `GET /api/time`: Médio: **`31.84 ms`** (era 42.49 ms no Modo 1) | P95: `41.66 ms`
* `GET /api/screenshot` (Frame decodificado 1.84 MB): `618.66 ms`
* **ICMP RTT:** Mínimo: `0.185 ms` | Médio: `0.272 ms` | Máximo: `0.448 ms` | Jitter: `0.060 ms`

---

## 4. Análise Técnica e Conclusões
1. **Zero-Network Stack (Modo 3):** Ao utilizar o canal USB Bulk direto (`Endpoint 0x03` via Linux FunctionFS `f_fs`), todo o tráfego de vídeo é isolado da camada IP. Isso elimina totalmente a sobrecarga de empacotamento RTP, alocação de `sk_buff` no kernel e processamento de softirq de rede.
2. **Latência de 11.45 ms:** É a menor latência já medida no projeto em modo de espelhamento contínuo a 60 FPS, representando uma economia de `1.18 ms` em relação ao Modo 1 UDP.
3. **Carga e Consumo no Pi Zero:** O consumo de CPU ARM permaneceu em **2.51%** (ainda menor que no Modo 1, que registrou 2.77%), com temperatura de `54.1 °C` e potência estável em `1.78 W`.
4. **Isolamento de Canais:** Ao descarregar o vídeo para o endpoint USB Bulk, as conexões de gerenciamento TCP e HTTP no adaptador virtual de rede tornaram-se até **2.5x mais rápidas** (handshake da porta 8080 caiu de 1.35 ms para 0.53 ms).

> Relatório atualizado e sincronizado com os dados do Blueprint 26 (`docs/blueprints/pt/26-benchmark-latencia-responsividade-telemetria-hardware.md`).