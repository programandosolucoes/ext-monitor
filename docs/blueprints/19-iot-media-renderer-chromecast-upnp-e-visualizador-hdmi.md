# Blueprint 19: Appliance IoT Media Renderer — Google Cast, UPnP/DLNA e Visualizador Gráfico HDMI

*Data: 2026-09-29*  
*Status: Concluído e Operacional na Release v2.3.0 (Congelado em 2026-09-29)*  
*Autor: Carlos Alberto <psncarlosalberto4ti@gmail.com>*  

---

## 1. Visão Geral: Da Segunda Tela ao Dispositivo de Mídia IoT Inteligente

O `ext-monitor` foi concebido inicialmente como monitor secundário USB de alta fidelidade e baixa latência. No entanto, sua arquitetura física (Raspberry Pi Zero W conectado a uma TV via HDMI, com aceleração de hardware VideoCore IV, Wi-Fi 802.11n, Bluetooth 4.1 e servidor web embutido) confere-lhe todas as capacidades de hardware de um **Dongle de Mídia Inteligente (estilo Chromecast / Apple TV / Fire TV)**.

### A Demanda: "Não Ficar Só com Som"
Quando utilizado como receptor de áudio IoT (seja via Bluetooth A2DP ou streaming de rede), manter a TV ligada apenas com tela preta gera desperdício visual e sensação de tela inativa.
A visão é dotar o `ext-monitor` de:
1. **Visualizador Gráfico Dinâmico e Capa de Álbum na TV:** Enquanto toca áudio, o HDMI exibe metadados em tempo real (Nome da Faixa, Artista, Álbum, Capa via AVRCP/DIDL) e barras de espectro/VU meter renderizadas via GPU.
2. **Integração com o Ecossistema Google Home / Google Cast (CastV2 & DIAL):** O dispositivo surge no app Google Home do celular para reprodução de áudio e vídeo em grupo.
3. **UPnP / DLNA MediaRenderer:** Capacidade de receber comandos "Transmitir para Dispositivo" nativos do Windows, Linux e apps como VLC, BubbleUPnP e smart TVs.

---

## 2. Análise de Viabilidade Técnica no Raspberry Pi Zero W (BCM2835)

É perfeitamente viável! O silício do BCM2835 possui os blocos necessários, desde que respeitadas as seguintes restrições de arquitetura:

| Funcionalidade | Mecanismo de Implementação | Carga de CPU no Pi Zero | Viabilidade |
| :--- | :--- | :--- | :--- |
| **Google Cast Audio (CastV2)** | Daemon Rust mDNS (`_googlecast._tcp`) + TLS porta 8009 (Protobuf) | ~1.8% CPU | **Totalmente Viável** |
| **YouTube Cast (DIAL Protocol)** | SSDP na porta UDP 1900 + REST na porta 8080/8008 | < 0.5% CPU | **Totalmente Viável** |
| **UPnP / DLNA A/V Renderer** | SSDP (`urn:schemas-upnp-org:device:MediaRenderer:1`) + HTTP Control | ~1.0% CPU | **Totalmente Viável** |
| **Visualizador de Espectro / Capa** | Extração de metadados BlueZ AVRCP + Textura DRM/KMS Overlay | ~0.8% CPU | **Totalmente Viável** |
| **Decodificação de Vídeos MP4/H.264** | Descarregamento no V4L2 M2M (`/dev/video10`) | ~1.5% CPU | **Totalmente Viável** |

---

## 3. Arquitetura do Subsistema IoT Media Renderer

```
+-----------------------------------------------------------------------------------+
|                            REDE WI-FI / ETHERNET                                  |
|                                                                                   |
|  [ Smartphone Android / iOS ]            [ PC Windows / Linux ]                   |
|   - Google Home (Cast V2)                 - "Cast to Device" (DLNA UPnP)          |
|   - Botão Cast no YouTube / Spotify       - Envio de Vídeo/Áudio MP4 / WebM       |
+--------------------------+--------------------------------+-----------------------+
                           | mDNS / SSDP                    |
                           v                                v
+-----------------------------------------------------------------------------------+
|                        RASPBERRY PI ZERO W (APPLIANCE)                            |
|                                                                                   |
|  +-----------------------------------------------------------------------------+  |
|  |             Camada de Descoberta e Controle (Rust Userspace)                |  |
|  |  - mDNS Avahi Daemon (_googlecast._tcp / _googlezone._tcp)                  |  |
|  |  - SSDP Responder (UDP 1900 para UPnP MediaRenderer e DIAL YouTube)        |  |
|  |  - CastV2 TLS Server (TCP 8009 - Handshake, Session, MediaChannel Protobuf)|  |
|  +--------------------------------------+--------------------------------------+  |
|                                         |                                         |
|                 +-----------------------+-----------------------+                 |
|                 | Fluxo de Vídeo                                | Fluxo de Áudio  |
|                 v                                               v                 |
|  +-------------------------------+             +-------------------------------+  |
|  |   V4L2 M2M Decoder Hardware   |             |    ALSA HDMI Direct (hw:0,0)  |  |
|  |   H.264 / MPEG4 (/dev/video10)|             |    Opus / AAC / PCM / MP3     |  |
|  +---------------+---------------+             +---------------+---------------+  |
|                  |                                             |                  |
|                  | DMA-BUF                                     v                  |
|                  v                              +------------------------------+  |
|  +-------------------------------+              |  BlueZ AVRCP / ID3 Extractor |  |
|  | DRM Plane 0: Vídeo em Tela    |              |  - Título / Artista / Álbum  |  |
|  +-------------------------------+              |  - Espectro Sonoro FFT       |  |
|                  ^                              +--------------+---------------+  |
|                  | Renderização Visual                         |                  |
|                  +---------------------------------------------+                  |
|                   (DRM Overlay Plane 1: UI / Capa / Espectro)                     |
|                                                                                   |
|                                         │                                         |
|                                         ▼ Saída HDMI Única                        |
|                         [ TELEVISOR / MONITOR COM CAIXAS ]                        |
+-----------------------------------------------------------------------------------+
```

---

## 4. O Visualizador Gráfico HDMI para Áudio IoT ("Não Ficar Só com Som")

Quando uma música toca via Bluetooth do smartphone ou streaming de rede IoT:
1. **Extração de Metadados via BlueZ D-Bus:**
   O daemon escuta a interface D-Bus `org.bluez.MediaPlayer1`:
   - `Track`: Nome da música (ex: *"Sultans of Swing"*).
   - `Artist`: Nome do artista (ex: *"Dire Straits"*).
   - `Album`: Nome do álbum e ano.
   - `Status`: Playing / Paused.
2. **Buffer Gráfico na TV HDMI:**
   O `ext-receiver` aloca um plano de sobreposição DRM (`/dev/dri/card0` Plane 1):
   - Fundo em gradiente suave com arte visual estilizada (ou capa do álbum baixada via cache).
   - Tipografia moderna com o título da música e tempo decorrido.
   - **VU Meter / Espectro de Frequências:** Um analisador FFT leve de 16 bandas em Rust calcula a intensidade sonora das frequências graves/médias/agudas e anima barras sonoras na base da TV a 30 FPS.
   - A TV deixa de ser um "buraco negro" sonoro e se transforma em uma elegante central de música ambiente.

---

## 5. Integração com Google Home e Cast V2

Para aparecer no aplicativo **Google Home**:
1. **Anúncio mDNS:**
   O serviço anuncia o serviço `_googlecast._tcp.local` com TXT records contendo:
   - `fn`: "Ext-Monitor Sala de Estar"
   - `md`: "Chromecast Audio / Video"
   - `ic`: Ícone representativo
   - `st`: 0 (Idle) ou 1 (Busy)
2. **Canal de Controle TLS (Porta 8009):**
   Implementa a troca de mensagens Protobuf do protocolo Google Cast (`CastMessage`):
   - `urn:x-cast:com.google.cast.receiver`: Responde com `RECEIVER_STATUS` reportando volume e aplicativo ativo.
   - `urn:x-cast:com.google.cast.media`: Gerencia comandos Play, Pause, Seek e Volume diretamente do app do smartphone.

---

## 6. Suporte a UPnP / DLNA MediaRenderer

Para permitir que qualquer computador ou celular envie arquivos de vídeo ou música com um clique direito ("Reproduzir em..."):
1. **SSDP Multicast (239.255.255.250:1900):** Responde a buscas `M-SEARCH` para `urn:schemas-upnp-org:device:MediaRenderer:1`.
2. **XML de Descrição do Dispositivo (`/upnp/desc.xml`):** Exposto pelo servidor web na porta 8080.
3. **Serviço AVTransport & RenderingControl:**
   - Recebe URLs de vídeos MP4, MKV ou áudios MP3/FLAC via método SOAP `SetAVTransportURI`.
   - Dispara o pipeline de hardware V4L2 M2M para reprodução direta sem sobrecarregar a CPU.

---

## 7. Roadmap de Implementação para o Próximo Congelamento

- [ ] **Módulo `receiver/src/iot_media.rs`:** Parser de metadados BlueZ AVRCP e gerador visual de espectro sonoro no DRM framebuffer.
- [ ] **Módulo `receiver/src/ssdp.rs`:** Respondedor SSDP UDP 1900 para anúncio automático como UPnP MediaRenderer e DIAL.
- [ ] **Integração no Painel Web:** Nova aba ou card "IoT Media Player" exibindo o status de reprodução do Google Home / DLNA.
- [ ] **Validação com YouTube e Spotify:** Testes empíricos de transmissão sem fio direta do celular para a TV.

---

*Blueprint 19 elaborado e formalizado como especificação de engenharia oficial para o próximo ciclo de release.*
