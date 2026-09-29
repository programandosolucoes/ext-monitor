# Plano de Implementação: Sub-Sistema de Áudio HDMI de Baixa Latência (ext-audio)

> **Para executores:** Este plano segue o padrão de desenvolvimento do ext-monitor, dividindo a introdução do áudio HDMI em tarefas atômicas e testáveis, garantindo que o pipeline de vídeo continue funcionando com zero regressões.

**Meta:** Implementar transmissão e reprodução de áudio HDMI digital estéreo de baixa latência (< 25ms) via PipeWire no Host e ALSA HDMI no Raspberry Pi Zero.

**Stack Tecnológica:** PipeWire / PulseAudio, ALSA (`snd-soc-hdmi-codec` / `vc4-hdmi`), Opus Codec (48kHz Stereo), GStreamer / Pure Rust, RTP UDP Porta 5002.

---

### Tarefa 1: Habilitação de Áudio HDMI no Kernel do Appliance Pi Zero

**Arquivos:**
- Modificar: `build-appliance/boot/config.txt`
- Modificar: `build-appliance/initramfs/init`
- Modificar: `scripts/appliance-init.sh`

- [x] **Passo 1.1:** Remover a flag `noaudio` de `dtoverlay=vc4-kms-v3d,cma-128,noaudio,nocomposite` em `build-appliance/boot/config.txt`, habilitando `dtparam=audio=on`.
- [x] **Passo 1.2:** Incluir verificação e carregamento dos módulos ALSA (`snd-pcm`, `snd-bcm2835`, `snd-soc-hdmi-codec`) em `appliance-init.sh` e `initramfs/init`.
- [x] **Passo 1.3:** Testar no Pi Zero a presença do dispositivo de áudio `/dev/snd/pcmC0D0p` ou `aplay -l`.

---

### Tarefa 2: Criador da Pia Virtual de Áudio e Captura no Host (`ext-sender`)

**Arquivos:**
- Modificar: `sender/src/config.rs`
- Modificar: `sender/src/pipeline.rs`
- Modificar: `scripts/start.sh`

- [x] **Passo 2.1:** Adicionar suporte ao argumento `--audio` (valores: `auto`, `sink`, `mirror`, `off`) e porta `--audio-port=5002` no `sender/src/config.rs`.
- [x] **Passo 2.2:** Adicionar rotina para registrar a pia de áudio PulseAudio/PipeWire `ext-monitor-audio` se ela não existir.
- [x] **Passo 2.3:** Implementar o branch de áudio no pipeline de streaming (`pulsesrc` -> `audioconvert` -> `opusenc` -> `rtpopuspay` -> `udpsink port=5002`).
- [x] **Passo 2.4:** Atualizar `scripts/start.sh` com as flags `--audio` e `--no-audio`.

---

### Tarefa 3: Receptor e Decodificador de Áudio no Pi Zero (`ext-receiver`)

**Arquivos:**
- Criar: `receiver/src/audio.rs`
- Modificar: `receiver/src/pipeline.rs`
- Modificar: `receiver/src/main.rs`

- [x] **Passo 3.1:** Criar módulo `receiver/src/audio.rs` gerenciando o pipeline ALSA (`udpsrc port=5002` -> `rtpopusdepay` -> `opusdec` -> `alsasink device=hw:0,0`).
- [x] **Passo 3.2:** Integrar o ciclo de vida do áudio ao `PipelineManager` (início simultâneo com o vídeo e parada graciosa).
- [x] **Passo 3.3:** Adicionar suporte a áudio no modo Miracast (desmultiplexação de áudio AAC/LPCM do contêiner TS).

---

### Tarefa 4: Controles de Áudio no Painel Web e Telemetria

**Arquivos:**
- Modificar: `receiver/src/web.rs`
- Modificar: `receiver/src/web_ui.rs`

- [x] **Passo 4.1:** Adicionar endpoint `POST /api/audio/volume` e `POST /api/audio/mute` na API do receptor.
- [x] **Passo 4.2:** Adicionar controle deslizante de volume (0–100%) e botão Mute na aba de Otimização do Painel Web.
- [x] **Passo 4.3:** Incluir telemetria de áudio (taxa de amostragem, codec, buffer) em `GET /api/status`.

---

### Tarefa 5: Validação Empírica e Benchmark de A/V Sync

- [x] **Passo 5.1:** Executar teste de vídeo de calibração A/V (Beep & Flash) para aferir sincronismo entre HDMI vídeo e HDMI áudio.
- [x] **Passo 5.2:** Medir impacto de CPU no Pi Zero (confirmar permanência de CPU < 2%).
- [x] **Passo 5.3:** Documentar o novo subsistema no `docs/MANUAL-DE-OPERACAO.md` e criar novo blueprint.
