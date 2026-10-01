# Relatório de Benchmark e Telemetria de Subhardware — Ext-Monitor
**Data da Execução:** 2026-09-29 15:10:58  
**Dispositivo Alvo:** Raspberry Pi Zero W Rev 1.1 (BCM2835 / VideoCore IV)  
**Endereço IP:** `192.168.7.2`  

## 1. Tabela Comparativa de Desempenho e Clocks

| Modo de Transmissão | Latência Fim-a-Fim | Taxa (FPS) | Clock H.264 | Clock VPU | Clock ARM | Temperatura | Potência Estimada |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **Modo 1: Rede UDP (RTP H.264)** | `13.3 ms` | **60 FPS** | 200 MHz | 400 MHz | 1000 MHz | 41.2 °C | **0.85 W** (170 mA) |
| **Modo 2: Miracast (Wi-Fi Display MS-MICE)** | `30.3 ms` | **30 FPS** | 200 MHz | 400 MHz | 1000 MHz | 41.1 °C | **0.85 W** (170 mA) |
| **Modo 3: USB Bulk Direto (480 Mbps FunctionFS)** | `11.5 ms` | **60 FPS** | 200 MHz | 400 MHz | 1000 MHz | 41.2 °C | **0.85 W** (170 mA) |

## 2. Análise Técnica e Conclusões
- **Modo 3 (USB Bulk Direct):** Apresentou a menor latência absoluta (~11.5 ms) ao eliminar totalmente o overhead das pilhas TCP/IP, operando direto nos endpoints USB 2.0 High-Speed (480 Mbps).
- **Modo 1 (Rede UDP RTP):** Ideal para computadores Linux Wayland/X11, sustentando 60 FPS estáveis com latência inferior a 15 ms via RTP H.264 (RFC 4571) e decodificação acelerada por hardware V4L2 M2M.
- **Modo 2 (Miracast WFD):** Compatibilidade nativa com emissão Windows 10/11 (Win + K) e Android sem instalar nenhum driver no host emissor.
- **Eficiência Energética:** O consumo total do Raspberry Pi Zero W permaneceu contido abaixo de **1.15W** mesmo sob carga total de vídeo a 60 FPS, permitindo alimentação estável por qualquer porta USB comum sem aquecimento excessivo.

> Documento gerado automaticamente pela suíte de validação contínua do Ext-Monitor.