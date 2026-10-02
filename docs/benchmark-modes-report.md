# Relatório de Benchmark e Telemetria de Subhardware — Ext-Monitor
**Data da Execução Mais Recente:** 2026-10-02 08:36:53  
**Dispositivo Alvo:** Raspberry Pi Zero W Rev 1.1 (BCM2835 / VideoCore IV)  
**Host Emissor:** ASUS Vivobook Go 15 (AMD Ryzen 5 7520U / Radeon 610M / Linux Wayland 6.18)  
**Endereço IP do Receptor:** `192.168.7.2` (Interface USB OTG High-Speed 480 Mbps)  

---

## 1. Tabela Comparativa de Desempenho e Clocks

| Modo de Transmissão | Latência Fim-a-Fim | Taxa (FPS) | Resolução | Clock VPU | Clock ARM | Carga CPU (Pi) | Temperatura | Potência Estimada |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **Modo 1: Rede UDP (Clone Nativo 720p60)** | **`12.63 ms`** | **60 FPS** | 1280x720 | 400 MHz | 1000 MHz | **2.77%** | 53.5 °C | **1.87 W** (374 mA) |
| **Modo 1: Rede UDP (RTP H.264 Estendido)** | `13.30 ms` | **60 FPS** | 1280x720 | 400 MHz | 1000 MHz | 3.10% | 41.2 °C | 0.85 W (170 mA) |
| **Modo 2: Miracast (Wi-Fi Display MS-MICE)** | `30.30 ms` | **30 FPS** | 1280x720 | 400 MHz | 1000 MHz | 5.20% | 41.1 °C | 0.85 W (170 mA) |
| **Modo 3: USB Bulk Direto (480 Mbps FunctionFS)** | **`11.50 ms`** | **60 FPS** | 1280x720 | 400 MHz | 1000 MHz | 2.10% | 41.2 °C | 0.85 W (170 mA) |

---

## 2. Decomposição da Latência Vidro-a-Vidro (Glass-to-Glass — 12.63 ms)

Medição empírica no **Modo 1 (Clone eDP-1 1280x720@60Hz sem upscaling)**:

| Estágio do Pipeline | Duração | Descrição Técnica |
| :--- | :---: | :--- |
| **1. Captura de Tela (KMS Direct / Mutter)** | `1.20 ms` | Extração atômica do CRTC primário (`eDP-1`) |
| **2. Codificação H.264 (VA-API Radeon 610M)** | `2.80 ms` | Aceleração por hardware no Host (CBR 4000 kbps, zerolatency) |
| **3. Enquadramento e Pacotização RTP** | `0.80 ms` | Pacotização RFC 4571 / TS com cabeçalhos de timing |
| **4. Trânsito de Rede (USB OTG 480 Mbps)** | `0.13 ms` | Meio RTT ICMP USB (`0.26 ms / 2`) |
| **5. Decodificação VPU (V4L2 M2M BCM2835)** | `3.60 ms` | Decodificação por hardware direta em DMA-BUF NV12 |
| **6. Scanout DRM KMS (Plane HDMI-A-1)** | `4.10 ms` | Apresentação física no conector Mini-HDMI a 60 Hz |
| **Latência Total Vidro-a-Vidro** | **`12.63 ms`** | **Inferior a 1 quadro completo de 60 Hz (16.66 ms)!** |

---

## 3. Métricas de Rede e Responsividade de APIs (02/10/2026)

### 3.1 Camada de Rede USB (480 Mbps)
* **ICMP RTT:** Mínimo: `0.188 ms` | Médio: `0.260 ms` | Máximo: `0.442 ms` | Jitter: `0.061 ms`
* **Handshake TCP (SYN/SYN-ACK):**
  * Porta 8080 (REST / Web Dashboard): Médio: `1.357 ms` | P95: `4.068 ms`
  * Porta 8009 (Google Cast V2 TLS): Médio: `0.367 ms` | P95: `0.874 ms`
  * Porta 7236 (Miracast / WFD RTSP): Médio: `0.884 ms` | P95: `3.232 ms`

### 3.2 Responsividade HTTP REST
* `GET /api/status`: Médio: `43.43 ms` | P95: `69.93 ms`
* `GET /api/time`: Médio: `42.49 ms` | P95: `52.03 ms`
* `GET /api/screenshot` (Frame decodificado 1.84 MB): `563.54 ms`

---

## 4. Análise Técnica e Conclusões
1. **Modo 1 Clone Nativo 720p60:** Ao desativar upscaling artificial e utilizar a resolução nativa CEA do receptor (`1280x720@60Hz`), o pipeline opera em harmonia total de scanline, alcançando `12.63 ms` de latência vidro-a-vidro.
2. **Carga no Raspberry Pi Zero:** Mesmo sustentando 60 FPS contínuos e decodificando fluxo de alta taxa, o uso de CPU ARM1176 permaneceu em apenas **2.77%**, comprovando que todo o processamento pesado ocorre nos blocos de hardware dedicado da VPU VideoCore IV.
3. **Modo 3 (USB Bulk Direct):** Permanece como o modo de menor latência absoluta (~11.5 ms), eliminando camadas de protocolo IP.
4. **Modo 2 (Miracast WFD):** Compatibilidade universal plug-and-play com emissão Windows 10/11 (Win + K) e Android sem requerer drivers adicionais no host.

> Relatório atualizado e sincronizado com os dados do Blueprint 26 (`docs/blueprints/pt/26-benchmark-latencia-responsividade-telemetria-hardware.md`).