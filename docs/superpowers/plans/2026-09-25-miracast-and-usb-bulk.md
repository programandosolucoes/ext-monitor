# Plano de Implementação: Wireless Display (Miracast) e Modo 2 (USB Bulk Direct)

> **Para executores:** Este plano segue a metodologia passo a passo, cobrindo a implementação da Fase 1 (Windows Wireless Display nativo via Win + K) e Fase 2 (Modo 2 USB Bulk Direct via FunctionFS), mantendo compatibilidade com operação headless e aceleração de hardware.

**Meta:** Implementar recepção nativa de Wireless Display (Miracast/MS-MICE) para Windows sem drivers e implementar o Modo 2 (USB Bulk Direct via FunctionFS) eliminando a pilha de rede.

**Arquitetura:**
- **Fase 1:** Avahi mDNS (`_display._tcp`) + Daemon RTSP WFD (port 7236) no Pi Zero respondendo M1-M7 e gerando pipeline GStreamer `udpsrc ! rtpmp2tdepay ! tsdemux ! h264parse ! v4l2h264dec ! kmssink`.
- **Fase 2:** USB Gadget `configfs` com `FunctionFS` (`f_fs`), expondo endpoints Bulk OUT (vídeo H.264) e Bulk IN (telemetria). `ext-sender` no Linux com `rusb` e receptor dedicado no Pi Zero.

**Stack Tecnológica:** Python 3 (RTSP WFD / mDNS), Rust (`ext-receiver`, `ext-sender`, `rusb`), GStreamer 1.0 (V4L2 M2M, KMS DRM), Linux Kernel ConfigFS / FunctionFS.

---

### Tarefa 1: Proteção Headless DRM HDMI no Pi Zero (`ext-receiver`)

**Arquivos:**
- Modificar: `receiver/src/main.rs`
- Modificar: `/boot/firmware/cmdline.txt` (no Pi Zero)

- [ ] **Passo 1.1:** Adicionar rotina de verificação e ativação forçada do conector HDMI no `receiver/src/main.rs` caso esteja `disconnected`, garantindo que `kmssink` consiga alocar modos e prerollar mesmo sem tela física.
- [ ] **Passo 1.2:** Compilar e instalar a versão atualizada no Pi Zero (`/usr/local/bin/ext-receiver`).
- [ ] **Passo 1.3:** Reiniciar `ext-receiver.service` e validar via `journalctl -u ext-receiver` que o pipeline entra em estado `PLAYING` de forma estável.

---

### Tarefa 2: Descoberta mDNS WFD para Windows (`Win + K`)

**Arquivos:**
- Criar: `scripts/miracast.service`
- Alvo no Pi Zero: `/etc/avahi/services/miracast.service`

- [ ] **Passo 2.1:** Criar definição XML do serviço Avahi com registros `_display._tcp` e `_miracast._tcp`, porta `7236`, flags e sub-elementos WFD.
- [ ] **Passo 2.2:** Copiar para o Pi Zero e recarregar `avahi-daemon`.
- [ ] **Passo 2.3:** Validar anúncio mDNS na rede local usando `avahi-browse -r -t _display._tcp`.

---

### Tarefa 3: Daemon RTSP WFD (Handshake M1 a M7) no Pi Zero

**Arquivos:**
- Criar: `receiver/wfd_sink.py`
- Alvo no Pi Zero: `/opt/ext-monitor/wfd_sink.py`

- [ ] **Passo 3.1:** Implementar servidor RTSP TCP na porta 7236 tratando:
  - M1: `OPTIONS`
  - M2: `OPTIONS` enviado pelo sink
  - M3: `GET_PARAMETER` com formatos H.264 (1080p60/720p60), áudio e porta RTP `5002`
  - M4: `SET_PARAMETER`
  - M5: `SET_PARAMETER` (wfd_trigger_method: SETUP)
  - M6: `SETUP`
  - M7: `PLAY` e disparo do subprocesso GStreamer (`rtpmp2tdepay ! tsdemux ! h264parse ! v4l2h264dec ! kmssink`)
  - `TEARDOWN`: finalização limpa do GStreamer.
- [ ] **Passo 3.2:** Testar execução local do script no Pi Zero.

---

### Tarefa 4: Serviço Systemd e Integração com Web UI para Miracast

**Arquivos:**
- Criar: `scripts/ext-wfd.service`
- Modificar: `web/server.py` e `web/static/index.html`

- [ ] **Passo 4.1:** Criar unidade systemd `ext-wfd.service` e habilitar no Pi Zero.
- [ ] **Passo 4.2:** Adicionar status do serviço Miracast e telemetria na API do Web Server (`web/server.py`).
- [ ] **Passo 4.3:** Adicionar card informativo no Painel Web (`Windows Cast: Pronto / Conectado`).

---

### Tarefa 5: Teste Automatizado de Handshake Miracast

**Arquivos:**
- Criar: `scripts/test-wfd-client.py`

- [ ] **Passo 5.1:** Criar script cliente que simula a conexão de um PC Windows (`Win + K`), executando as requisições M1 a M7 via TCP 7236 e enviando stream de teste RTP MP2T.
- [ ] **Passo 5.2:** Executar teste contra o Pi Zero e verificar logs no `ext-wfd.service`.

---

### Tarefa 6: Setup do Gadget USB FunctionFS (Modo 2) e Switcher de Modo

**Arquivos:**
- Criar: `scripts/setup-usb-bulk.sh`
- Criar: `scripts/switch-mode.sh`
- Alvo no Pi Zero: `/usr/local/bin/ext-mode`

- [ ] **Passo 6.1:** Criar script de configuração do Gadget `configfs` com suporte a `FunctionFS` (`f_fs`) em `/sys/kernel/config/usb_gadget/g_display`.
- [ ] **Passo 6.2:** Criar utilitário `switch-mode.sh` permitindo alternar instantaneamente entre `network` (Modo 1) e `usb-bulk` (Modo 2).
- [ ] **Passo 6.3:** Testar montagem do diretório FunctionFS `/dev/usb-ffs/display`.

---

### Tarefa 7: Receptor USB Bulk Direct no Pi Zero (`ext-usb-receiver`)

**Arquivos:**
- Criar: `receiver/src/usb_bulk.rs`
- Modificar: `receiver/Cargo.toml`
- Modificar: `receiver/src/main.rs`

- [ ] **Passo 7.1:** Implementar inicialização de descritores FunctionFS no `ep0` e leitura assíncrona/em loop de blocos H.264 do `ep1` (Bulk OUT).
- [ ] **Passo 7.2:** Conectar o fluxo lido do `ep1` à entrada do decodificador `v4l2h264dec` via stdin de GStreamer (`fdsrc` / `appsrc`).
- [ ] **Passo 7.3:** Compilar e validar funcionamento.

---

### Tarefa 8: Transporte USB Bulk no Host Linux (`ext-sender`)

**Arquivos:**
- Modificar: `sender/Cargo.toml` (adicionar `rusb`)
- Criar: `sender/src/usb_transport.rs`
- Modificar: `sender/src/main.rs`

- [ ] **Passo 8.1:** Adicionar suporte a `--transport=usb` no `ext-sender`.
- [ ] **Passo 8.2:** Implementar abertura do dispositivo USB (`VID: 0x1d6b, PID: 0x0104`), reivindicação de interface e `bulk_write` no endpoint OUT.
- [ ] **Passo 8.3:** Compilar e validar no host Linux.

---

### Tarefa 9: Documentação no Obsidian e Commits no Git

**Arquivos:**
- Modificar: `/home/carlos/ide/obsidian-estudos/Projetos/Pi-Zero-Monitor-GPU-Offload-Architecture.md`
- Git commit & push em `ext-monitor` e `obsidian-estudos`.
