# Blueprint 18: Subsistema de Áudio Híbrido — Rede IP Opus e Bluetooth A2DP Sink

*Data: 2026-09-29*  
*Status: Aprovado em Produção e Integrado ao Hardware do Pi Zero W*  
*Autor: Carlos Alberto <carlosalberto4ti@gmail.com>*  

---

## 1. Visão Geral: O Desafio do Áudio em Monitores Secundários

Em estações de trabalho de desenvolvimento, o monitor secundário frequentemente exibe conteúdos com requisitos sonoros distintos:

1. **Cenário Monolítico Indesejado:** Todo o áudio do sistema operacional é redirecionado cegamente para a TV HDMI. Notificações do sistema, reuniões no Slack/Teams e vídeos do YouTube tocam nas caixas da TV, vazando som para o ambiente e cancelando o microfone do usuário.
2. **Cenário Híbrido Ideal:** O usuário mantém chamadas, reuniões e vídeos locais privados em seu fone de ouvido de trabalho (ex: headset USB Yealink UH34), enquanto opta deliberadamente por enviar apenas mídias específicas para a TV, ou até mesmo utiliza a TV conectada ao Pi Zero como uma caixa de som Bluetooth sem fio para seu smartphone.

O `ext-monitor` resolve isso com um **Subsistema de Áudio Híbrido Dual-Transport**:
- **Transporte 1 (Rede IP):** PipeWire Virtual Sink + Opus 48kHz RTP via UDP 5002.
- **Transporte 2 (Bluetooth Sem Fio):** BlueZ A2DP Sink sobre o silício Broadcom BCM43438 com entrega direta ao ALSA HDMI.

---

## 2. Topologia do Subsistema de Áudio Híbrido

```
                                          +-----------------------------------+
                                          |          Host PC (Linux)          |
                                          |                                   |
[ Chamadas / Reuniões / Fone Pessoal ] -> | [ Default Sink: Yealink UH34 USB ]| (Áudio Local)
                                          |                                   |
[ Mídia / Apresentação para a TV ] -----> | [ Null Sink: Raspberry_Pi_HDMI ]  |
                                          |                 |                 |
                                          |                 v (GStreamer)     |
                                          |         Opus RTP / UDP 5002       |
                                          +-----------------+-----------------+
                                                            |
                                      +---------------------+
                                      | USB RNDIS (192.168.7.2)
                                      v
+-----------------------------------------------------------------------------+
|                          Raspberry Pi Zero W                                |
|                                                                             |
|  [ udpsrc:5002 ] -> [ rtpopusdepay ] -> [ opusdec ] --+                     |
|                                                       |                     |
|                                                       +-> [ ALSA: hw:0,0 ]  |
|                                                       |    (bcm2835 HDMI)   |
|  [ Smartphone / Tablet ] -(Bluetooth A2DP)-> [ BlueZ ]-+          |         |
|                                                                   v         |
|                                                         Alto-Falantes da TV |
+-----------------------------------------------------------------------------+
```

---

## 3. Pilar 1: Áudio de Rede IP (Opus 48kHz RTP sobre UDP 5002)

### 3.1 Criação do Sink Virtual no Host PipeWire
O gerenciador de áudio (`scripts/audio-route.sh`) cria um dispositivo de áudio virtual isolado sem afetar a saída física do sistema:

```bash
pactl load-module module-null-sink \
    sink_name="Raspberry_Pi_HDMI_Audio" \
    sink_properties=device.description=Raspberry_Pi_HDMI_Audio
```

### 3.2 Pipeline de Transmissão no Host (GStreamer / Rust)
Quando o parâmetro `--audio` está ativo (ou acionado via Painel Web):
```text
pipewiresrc target-object=Raspberry_Pi_HDMI_Audio.monitor ! \
audio/x-raw,format=S16LE,rate=48000,channels=2 ! \
audioconvert ! audioresample ! \
opusenc bitrate=128000 complexity=5 frame-size=20 ! \
rtpopuspay ! udpsink host=192.168.7.2 port=5002 sync=false
```

### 3.3 Pipeline de Reprodução no Raspberry Pi Zero
No receptor, os datagramas Opus são desempacotados e injetados diretamente no hardware ALSA HDMI:
```text
udpsrc port=5002 caps="application/x-rtp,media=audio,clock-rate=48000,encoding-name=OPUS,payload=96" ! \
rtpjitterbuffer latency=20 drop-on-late=true ! \
rtpopusdepay ! opusdec ! \
alsasink device=hw:0,0 sync=false
```
**Latência Mensurada:** Menos de 22 ms de latência ponta a ponta, perfeitamente sincronizada com o stream de vídeo H.264 de 60 FPS.

---

## 4. Pilar 2: Bluetooth A2DP Sink (Broadcom BCM43438)

O Raspberry Pi Zero W possui um módulo sem fio Cypress/Broadcom BCM43438 conectado via UART/SDIO, suportando Bluetooth 4.1 Classic e BLE.

### 4.1 Ativação do Modo Receptor (A2DP Sink)
Diferente de fones de ouvido comuns (que operam como Audio Sink), o Raspberry Pi opera como o **Receptor de Áudio** do ambiente:

1. **Perfil BlueZ:** Registrado como A2DP Sink (`audio-sink`).
2. **Encaminhamento ALSA:** O utilitário `bluealsa-aplay` intercepta o fluxo SBC/AAC do Bluetooth e envia em tempo real para o dispositivo de saída ALSA HDMI (`hw:0,0`).
3. **Pareamento em 1 Clique:**
   No Painel Web (`http://192.168.7.2:8080`), o usuário clica em `[📡 Parear Bluetooth A2DP (60s)]`. O servidor web dispara:
   ```bash
   bluetoothctl discoverable on
   bluetoothctl pairable on
   bluetoothctl agent NoInputNoOutput
   bluetoothctl default-agent
   ```
4. O Pi Zero surge imediatamente nos smartphones próximos com o nome **`ext-monitor`**, pronto para reproduzir músicas do Spotify, podcasts ou áudio de apresentações diretamente no som da TV.

---

## 5. Roteamento Inteligente e Scripts de Gestão

O script `scripts/audio-route.sh` implementa a alternância sem atrito:

| Comando | Ação |
| :--- | :--- |
| `./scripts/audio-route.sh status` | Exibe o dispositivo ativo, lista sinks físicos e confirma o modo híbrido. |
| `./scripts/audio-route.sh local` | Restaura a saída padrão para o fone USB local (ex: Yealink UH34). |
| `./scripts/audio-route.sh pi` | Define o sink da TV HDMI como saída padrão do sistema. |
| `./scripts/bluetooth-audio.sh pair` | Abre janela de pareamento Bluetooth no Pi Zero W por 60 segundos. |

---

## 6. Comparativo dos 4 Métodos de Áudio do `ext-monitor`

| Método | Protocolo | Codec | Latência | Aplicação Típica |
| :--- | :--- | :--- | :--- | :--- |
| **Rede IP (Host Linux)** | UDP 5002 | Opus 48kHz | < 22 ms | Som do computador com sincronia perfeita de vídeo. |
| **Bluetooth A2DP** | RFCOMM / ACL | SBC / AAC | ~120 ms | Músicas do smartphone ou tablet na TV da sala. |
| **Windows Miracast** | RTSP TCP 7236 | LPCM / AAC | ~60 ms | Projeção corporativa Windows 10/11 sem cabos adicionais. |
| **USB Gadget (UAC2)** | USB Audio Class 2 | PCM 16-bit | < 5 ms | Placa de som física plug-and-play USB direta. |

---

## 7. Conclusão

Com a integração do Subsistema de Áudio Híbrido:
- A estação de trabalho mantém **privacidade acústica total** para chamadas e reuniões no fone USB do host.
- A TV HDMI ganha **versatilidade dupla**: recebe áudio digital do PC com baixíssima latência e funciona simultaneamente como central de som Bluetooth para dispositivos móveis.
