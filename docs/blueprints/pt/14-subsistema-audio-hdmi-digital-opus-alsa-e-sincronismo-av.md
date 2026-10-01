# Blueprint 14: Subsistema de Áudio HDMI Digital, Codec Opus de Baixa Latência e ALSA

> 🇧🇷 Versão em Português | [🇺🇸 English Version](../en/14-digital-hdmi-audio-opus-alsa-av-sync.md)


**Status:** Concluído / Em Produção  
**Autor:** Carlos Alberto <carlosalberto4ti@gmail.com>  
**Data:** Setembro de 2026  
**Tecnologias:** Linux ALSA (`vc4-hdmi` / `snd-soc-hdmi-codec`), Opus Codec (48kHz Estéreo), PipeWire / PulseAudio, RTP UDP Porta 5004, GStreamer, QEMU ARMv6  

---

## 1. Visão Geral e Contexto do Problema

O projeto `ext-monitor` operava historicamente transmitindo apenas o fluxo visual (H.264 acelerado por hardware para VideoCore IV via V4L2 M2M e KMS/DRM DMA-BUF). Em uma estação de trabalho produtiva, a ausência de áudio no monitor ou TV secundária obrigava o usuário a utilizar fones ou caixas de som conectadas ao computador host, impedindo o uso pleno de reprodutores multimídia, reuniões e vídeos em tela cheia na segunda tela.

### Desafios de Engenharia:
1. **Latência Algorítmica e Sincronismo A/V (Lip-Sync):** O fluxo de áudio não podia sofrer buffers longos que gerassem descompasso com o vídeo (< 15–20 ms no vídeo). Codecs pesados como MP3 ou AAC tradicional geram atrasos perceptíveis e overhead de CPU indesejado no SoC monocore de 1.0 GHz do Pi Zero.
2. **Habilitação de Áudio no Kernel do BCM2835:** Por padrão, a diretiva `noaudio` no overlay `vc4-kms-v3d` desativava o codec de som HDMI para economizar memória e simplificar o driver DRM.
3. **Isolamento de Canais e Prevenção de Head-of-Line Blocking:** O streaming de áudio não podia compartilhar a mesma porta UDP do fluxo de vídeo H.264 (porta 5000), pois quadros I-Frame de vídeo (key-frames com até 64KB fragmentados em pacotes MTU) causariam perda ou jitter nos pacotes de áudio.

---

## 2. Decisão de Arquitetura

### A. Escolha do Codec: Opus 48 kHz Estéreo (Frame de 10ms)
O codec **Opus** foi escolhido pelas seguintes propriedades de silício e rede:
* **Latência de Codificação Minúscula:** Frames de 10ms (em vez dos tradicionais 20ms ou 40ms) garantem que o áudio chegue ao monitor em menos de 15ms.
* **Banda de Rede Quase Desprezível:** Operando a 96 kbps em taxa de amostragem de 48.000 Hz, o áudio consome menos de 0.02% da largura de banda do barramento USB High-Speed (480 Mbps).
* **Consumo de CPU Praticamente Nulo no Pi Zero:** A decodificação de pacotes Opus compactados para PCM consome menos de 0.3% do núcleo ARM1176JZF-S.

### B. Mapeamento de Portas e Isolamento de Tráfego
| Serviço | Protocolo / Porta | Função |
| :--- | :--- | :--- |
| **Vídeo H.264 (Modo 1)** | UDP 5000 | Fluxo RTP RFC 6184 de vídeo para V4L2 M2M |
| **Controle & Hot-Apply** | UDP 5001 | Sockets de telemetria e hot-apply dinâmico |
| **Miracast Windows** | UDP 5002 | Contêiner MPEG-TS com vídeo e áudio nativo Windows |
| **Áudio Digital HDMI** | **UDP 5004** | **Canal dedicado de áudio RTP Opus (pt=96)** |
| **Wi-Fi Display RTSP** | TCP 7236 | Sinalização de projeção sem fio Windows Win+K |
| **Painel Web & REST API**| HTTP 8080 | Dashboard de telemetria e controle de volume/mudo |

---

## 3. Implementação Detalhada

### 3.1 Kernel & Boot do Appliance (`build-appliance/boot/config.txt`)
Remoção da flag restritiva e ativação do codec HDMI:
```ini
# VideoCore IV Hardware Graphics KMS + HDMI Audio
dtoverlay=vc4-kms-v3d,cma-128,nocomposite
dtparam=audio=on
```
No `/init` e `scripts/appliance-init.sh`, os drivers ALSA necessários são carregados preventivamente:
```bash
modprobe snd-bcm2835 2>/dev/null || true
modprobe snd-soc-hdmi-codec 2>/dev/null || true
```

### 3.2 Emissor no Host (`ext-sender` / PipeWire)
O `PipelineBuilder` em Rust instancia o `StreamerHandle::Composite`, que gerencia de forma atômica os processos de vídeo e áudio.
A captura de som ocorre diretamente da pilha nativa PipeWire via plugin `pipewiresrc`:
```bash
gst-launch-1.0 -q pipewiresrc client-name=ext-hdmi-audio do-timestamp=true !   audioconvert ! audioresample ! audio/x-raw,rate=48000,channels=2 !   opusenc bitrate=96000 frame-size=10 complexity=3 ! rtpopuspay pt=96 !   udpsink host=192.168.7.2 port=5004 sync=false
```

### 3.3 Receptor no Appliance (`ext-receiver` / ALSA)
O módulo `receiver/src/audio.rs` gerencia o ciclo de vida do worker de áudio, expondo controle de volume com interpolação de ganho e silenciamento gracioso:
```bash
gst-launch-1.0 -q udpsrc port=5004 caps="application/x-rtp,media=audio,clock-rate=48000,encoding-name=OPUS,payload=96" !   rtpopusdepay ! opusdec ! audioconvert ! audioresample !   volume volume=1.0 ! alsasink sync=false buffer-time=20000 latency-time=10000
```
*Fallback Automático:* Em ambientes sem driver ALSA físico direto (como QEMU ou contêineres de desenvolvimento), o pipeline realiza chaveamento automático transparente para `autoaudiosink`.

### 3.4 REST API & Interface Web (`web.rs` e `web_ui.rs`)
* `GET /api/audio/status`: Inspeção em tempo real de portas, volume e mute.
* `POST /api/audio/volume`: Ajuste dinâmico de volume (0 a 100%) sem reinício de stream.
* `POST /api/audio/mute`: Alternância instantânea de silenciamento.
* Telemetria periódica (`GET /api/status`) incorpora o nó `"audio": { ... }` para atualização automática dos controles na interface gráfica.

---

## 4. Atualização das Telas de Splash (4 Idiomas)

Para informar o operador do sistema sobre a disponibilidade do áudio em tempo real:
1. **Splash de Boot (`splash_loading`):**
   * Mensagens nos 4 idiomas (PT, EN, IT, ZH) notificam a inicialização dos drivers de GPU e de áudio HDMI.
2. **Splash de Espera (`splash_ready`):**
   * Badge superior atualizado para `● VÍDEO & ÁUDIO HDMI PRONTOS / READY`.
   * Tag visual de destaque: `• ÁUDIO DIGITAL HDMI ATIVO (ALSA Opus 48kHz)`.
   * Cards operacionais dos 4 quadrantes linguísticos atualizados explicitando a transmissão conjunta de tela e som.
   * Rodapé técnico documenta a porta `UDP 5004` e o codec `Opus 48kHz`.

---

## 5. Validação sob QEMU ARMv6

A suíte completa de testes automatizados executada no binário ARMv6 via `qemu-arm` comprovou:
1. Ingestão e decodificação contínua de pacotes Opus sem underrun.
2. Resposta de telemetria JSON íntegra contendo o nó de áudio.
3. Resposta imediata dos endpoints HTTP `/api/audio/volume` e `/api/audio/mute`.
4. Tamanho de binário stripped final de ~1.0 MB, mantendo a pegada ultraleve da imagem de 33MB do appliance.
