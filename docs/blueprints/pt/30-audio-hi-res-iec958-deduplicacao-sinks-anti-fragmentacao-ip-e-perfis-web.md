# Blueprint 30: Áudio Hi-Res Direto via ALSA IEC958, Deduplicação de Sinks, Anti-Fragmentação IP e Perfis no Painel Web

> 🇧🇷 Versão em Português | [🇺🇸 English Version](../en/30-hi-res-iec958-audio-sink-deduplication-anti-fragmentation-and-web-profiles.md)

*Data: 2026-10-02*  
*Status: Aprovado em Produção e Validado com Áudio Hi-Res 96kHz Master em Reprodução Contínua na TV*  
*Autor: Carlos Alberto <carlosalberto4ti@gmail.com>*  

---

## 1. Visão Geral e Contexto de Engenharia

Durante a reprodução contínua de streaming de vídeo de alta fidelidade (séries e filmes como Star Trek) em ambiente de rede (Modo 1: UDP RTP) conectado a uma TV via porta Mini-HDMI do Raspberry Pi Zero, foram demandados aprimoramentos arquiteturais profundos no subsistema de som e estabilidade de imagem:

1. **Suporte a Altas Taxas de Amostragem (Hi-Res Audio):** Operação nativa do hardware de áudio ALSA a 96 kHz e 192 kHz com 24-bit/subframe IEC 60958-3, aproveitando a capacidade configurada no firmware (`config.txt`).
2. **Seleção de Perfis no Painel Web:** Possibilidade de forçar e alternar os perfis de clock e fidelidade diretamente na interface web (`http://192.168.7.2:8080`) com feedback visual do clock real negociado.
3. **Resolução dos Dispositivos de Som Duplicados no Host:** Eliminação do conflito que gerava múltiplos sinks virtuais no GNOME.
4. **Eliminação de Latência Inicial e Cortes de Som ("Cortes Longos"):** Remoção de engasgos e clipping no início de frases após momentos de silêncio ou pausas de diálogo.
5. **Eliminação de Flicks e Pulos de Quadro no Vídeo:** Identificação e solução definitiva para instabilidades de vsync e rajadas de rede.

---

## 2. Diagnóstico e Resolução das Causas Raízes

### 2.1 Dispositivos de Som Duplicados no PC Host (`Raspberry_Pi_HDMI_Audio`)
* **Sintoma:** O usuário observava dois dispositivos idênticos de áudio no painel de controle de som do sistema operacional.
* **Causa Raiz:** No transmissor (`sender/src/main.rs`), duas rotinas paralelas eram disparadas na inicialização: `spawn_opus_audio_streamer` e `spawn_audio_spectrum_monitor` (`parec`). Ambas invocavam `ensure_audio_sink_exists()`. Como a consulta ao PulseAudio (`pactl list sinks short`) ocorria concorrentemente antes de o primeiro módulo completar o registro, dois módulos `module-null-sink` eram carregados em paralelo (ex: IDs 536870916 e 536870917).
* **Solução:**
  * Implementação de uma trava estática de exclusão mútua (`static SINK_MUTEX: Mutex<()>`).
  * Rotina de inspeção em `pactl list modules short` que contabiliza instâncias de `Raspberry_Pi_HDMI_Audio`. Se houver mais de uma, descarrega imediatamente os módulos excedentes (`pactl unload-module <id>`).
  * Garantia de **exatamente 1 dispositivo virtual de som ativo** no PC host.

---

### 2.2 Atraso Inicial e Cortes Longos no Áudio (Teardown em Silêncio)
* **Sintoma:** O áudio demorava a iniciar e, durante diálogos pausados do filme, as primeiras palavras de cada frase eram cortadas ou sofriam atraso perceptível.
* **Causa Raiz:** No arquivo `receiver/src/audio.rs`, havia uma lógica de economia que, após 1,5 segundo sem recepção de pacotes UDP, executava:
  ```rust
  if let Some(mut dev) = pcm_device.take() {
      dev.drain();
      dev.drop_playback();
  }
  ```
  Ao destruir o descritor `/dev/snd/pcmC0D0p`, toda vez que um personagem voltava a falar, o receiver era forçado a reabrir o arquivo de dispositivo e reexecutar a sequência completa de ioctls: `SNDRV_PCM_IOCTL_HW_PARAMS`, `SW_PARAMS` e `PREPARE`. No driver ALSA BCM2835 com firmware VideoCore IV, esse handshake leva entre **200 ms e 400 ms**. Durante esse intervalo, dezenas de pacotes de voz eram descartados.
* **Solução:**
  1. **Pré-aquecimento no Boot:** O dispositivo ALSA agora é aberto imediatamente ao inicializar a thread do receiver (`AlsaHdmiDevice::open(initial_rate)`).
  2. **Manutenção do Descritor Aberto:** Em caso de silêncio ou timeout, o `pcm_device` **nunca é destruído**. O hardware permanece em estado pré-aquecido e pronto para gravação imediata.
  3. **Recuperação Instantânea de Underrun com Retry:** Na função `write_frames`, caso ocorra um underrun de buffer (`EPIPE`), o sistema reexecuta `SNDRV_PCM_IOCTL_PREPARE` e imediatamente reescreve o mesmo bloco de amostras no mesmo ciclo:
     ```rust
     if err.raw_os_error() == Some(libc::EPIPE) {
         unsafe { libc::ioctl(self.fd, SNDRV_PCM_IOCTL_PREPARE as _) };
         let retry_ret = unsafe { libc::ioctl(self.fd, SNDRV_PCM_IOCTL_WRITEI_FRAMES as _, &mut xfer) };
         if retry_ret >= 0 { return Ok(()); }
     }
     ```
  4. **Tuning de Latência no Host:** O `pulsesrc` do GStreamer foi calibrado com `buffer-time=20000` (20 ms) e `latency-time=5000` (5 ms), eliminando filas excessivas na captura.

---

### 2.3 Pulos de Quadro e Flicks de Vídeo a Cada 30 Segundos (Jitter de Relógio)
* **Sintoma:** Flicks periódicos e pequenos engasgos na imagem a cada 30 segundos exatos.
* **Causa Raiz:** O daemon `sender/src/time_sync.rs` chamava `POST /api/time/sync` a cada 30 segundos. No receiver (`receiver/src/web.rs`), a rota executava incondicionalmente `libc::settimeofday(&tv, std::ptr::null())` com `tv_usec: 0`. Resetar o relógio de tempo real (`CLOCK_REALTIME`) zera os microssegundos e gera saltos de tempo no kernel Linux, desregulando o timer wheel e os prazos de vsync do decodificador V4L2/KMS.
* **Solução:**
  * O endpoint `POST /api/time/sync` agora verifica a hora atual do sistema:
    ```rust
    if cur_now < 1704067200 || (cur_now as i64 - epoch_secs as i64).abs() > 5 {
        unsafe { libc::settimeofday(&tv, std::ptr::null()) };
    }
    ```
    Durante a reprodução normal, microajustes são ignorados. O relógio só é alterado se o Pi Zero estiver no ano 1970 (boot a frio sem RTC) ou se houver desvio superior a 5 segundos.
  * O intervalo do sender foi estendido de 30 para 300 segundos (5 minutos).

---

### 2.4 Falhas no Som e Flicks por Fragmentação de Pacotes IP na Rede
* **Sintoma:** Mesmo com ALSA aberto, ocorriam estalos ocasionais no som e trepidações concomitantes no vídeo sob tráfego contínuo.
* **Investigação de Baixo Nível via `tcpdump` e Métricas SNMP do Kernel:**
  * O dump de pacotes revelou que o transmissor de áudio enviava pacotes UDP com **tamanho de 2048 bytes**.
  * A interface de rede USB CDC-ECM (`enx...`) possui **MTU de 1500 bytes**.
  * Pacotes de 2048 bytes excedem o limite de 1472 bytes (MTU - cabeçalhos IP/UDP) e sofriam **fragmentação IP obrigatória** em múltiplos fragmentos.
  * A tabela SNMP do Pi Zero (`/proc/net/snmp`) acusou:
    $$\text{ReasmReqds} = 1.004.049 \quad \text{requisições de remontagem de fragmentos}$$
  * A perda de qualquer fragmento de um pacote descartava o quadro inteiro de áudio (gerando estalo/corte).
  * O esforço de remontagem de fragmentos na CPU single-core de 1 GHz atrasava o processamento dos pacotes RTP de vídeo, provocando atraso de entrega e flick na imagem.
* **Solução:**
  * Inclusão do elemento `audiobuffersplit` no pipeline GStreamer do transmissor:
    ```rust
    .arg("audiobuffersplit")
    .arg("output-buffer-size=1024")
    ```
  * Cada pacote UDP passa a ter exatamente **1024 bytes** (256 amostras estéreo a 16-bit).
  * Tamanho total no cabo Ethernet: $1024 + 8\text{ (UDP)} + 20\text{ (IPv4)} = \mathbf{1052\text{ bytes}}$.
  * Como $1052 < 1500\text{ MTU}$, a **taxa de fragmentação IP caiu para ZERO**.
  * Otimização da rotina de ganho unitário (volume 100%) no receiver para evitar aritmética de ponto flutuante no laço crítico de amostras.

### 2.5 Conformidade dos Bits de Status IEC 60958-3 (Anti-Mute SCMS em TVs Philips) e Roteamento PipeWire
* **Sintoma:** Ao selecionar o dispositivo `Raspberry_Pi_HDMI_Audio` no painel de som do GNOME, a TV permanecia em silêncio absoluto.
* **Causas Raízes Identificadas:**
  1. **Flag de Proteção de Cópia / SCMS Ativada por Padrão:** Nos subframes IEC 60958 construídos manualmente em `receiver/src/audio.rs`, o byte 0 do Channel Status estava configurado como `0x00`. Na especificação IEC 60958-3 / CEA-861, o bit 2 representa `IEC958_AES0_CON_NOT_COPYRIGHT`. Quando este bit é 0, o fluxo sinaliza "Cópia Protegida com Restrição SCMS". Receptores HDMI de televisores comerciais (especialmente Philips 50PUG6102) acionam mute de segurança no conversor DAC para evitar violação de proteção de conteúdo sem handshake HDCP.
     * *Correção:* Configuração explícita de `status_bytes[0] = 0x04` (`IEC958_AES0_CON_NOT_COPYRIGHT`) e `status_bytes[1] = 0x82` (`IEC958_AES1_CON_ORIGINAL | IEC958_AES1_CON_PCM_CODER`), idêntico à configuração oficial ALSA de `cards/vc4-hdmi.conf`.
  2. **Persistência de Roteamento de Streams Ativos no PipeWire:** Ao trocar o dispositivo padrão no GNOME, streams de mídia já em execução contínua (ex: aba do Google Chrome com Star Trek) permaneciam atrelados ao sink anterior (`alsa_output...HiFi__Speaker__sink`) devido à tabela `module-stream-restore`.
     * *Correção:* Roteamento forçado do sink-input ativo diretamente para o sink virtual `Raspberry_Pi_HDMI_Audio` (`pactl move-sink-input <id> Raspberry_Pi_HDMI_Audio`) e alinhamento da taxa negociada em 48.000 Hz / 96.000 Hz.

---

## 3. Matriz de Perfis de Áudio e Integração no Painel Web

O painel web do receptor (`receiver/src/web_ui.rs`) conta agora com uma seção dedicada: **HDMI Master Audio Profile & Sample Rate**, permitindo selecionar forçadamente os quatro modos suportados:

| Perfil | Taxa (Hz) | Resolução | Subframe IEC958 | Buffer por Pacote | Latência Teórica |
| :--- | :---: | :---: | :---: | :---: | :---: |
| 🎵 **Hi-Res Studio** (Padrão) | **96.000 Hz** | 24-bit / S16LE | Rate Code `0x0A` | 1024 B (256 frames) | ~2,66 ms / pacote |
| 🚀 **Ultra Hi-Res** | **192.000 Hz** | 24-bit / S16LE | Rate Code `0x0E` | 1024 B (256 frames) | ~1,33 ms / pacote |
| 🎬 **Cinema Standard** | **48.000 Hz** | 16-bit / S16LE | Rate Code `0x02` | 1024 B (256 frames) | ~5,33 ms / pacote |
| 💿 **CD Fidelity** | **44.100 Hz** | 16-bit / S16LE | Rate Code `0x00` | 1024 B (256 frames) | ~5,80 ms / pacote |

### Fluxo de Comunicação Bidirecional:
1. O usuário clica no perfil desejado no Painel Web.
2. O navegador dispara `POST /api/audio/rate {"rate": 96000}`, que reconfigura o clock de hardware ALSA do Raspberry Pi Zero via ioctl `SNDRV_PCM_IOCTL_HW_PARAMS`.
3. Simultaneamente, o painel despacha `sendHostControl({"audio_rate": 96000})` via UDP na porta 5001 para o transmissor `ext-sender`.
4. O `ext-sender` executa `ControlAction::SetAudioRate`, reiniciando a thread do streamer com uma nova flag atômica isolada e reconfigurando o resampling do GStreamer sem derrubar o vídeo.

---

### 3.1 Arquitetura de Transporte de Áudio (Matriz de Modos Físicos)

O painel web disponibiliza agora a seleção das três modalidades de transporte de áudio do ecossistema:

| Modalidade de Transporte | Meio Físico | Protocolo / Porta | Latência | Características Principais |
| :--- | :--- | :--- | :---: | :--- |
| 🌐 **Modo 1: Rede UDP (Low-Latency)** | USB CDC-ECM / LAN | UDP `5004` (RTP/PCM) | < 5 ms | Padrão ativo. Pacotes de 1024 B, zero fragmentação IP, A/V sync automático. |
| 🔌 **Modo 2: USB Audio Class (UAC2)** | USB Gadget `dwc2` | UAC2 Endpoint Isochronous | < 1 ms | Hardware Plug & Play. Reconhecido nativamente como Placa de Som USB sem rede. |
| 📦 **Modo 3: Multiplex USB Bulk** | Tubo `/dev/usb-display-bulk` | Mux PCM + H.264 | < 1 ms | Offline puro. Pacotes de áudio intercalados diretamente no pipe de vídeo Bulk. |

---

## 4. Validação Empírica no Hardware

* **Verificação de Pacotes no Barramento USB/Ethernet:**
  ```text
  21:08:01.363578 IP 192.168.7.1.52908 > 192.168.7.2.5004: UDP, length 1024
  21:08:01.363639 IP 192.168.7.1.52908 > 192.168.7.2.5004: UDP, length 1024
  21:08:01.368583 IP 192.168.7.1.52908 > 192.168.7.2.5004: UDP, length 1024
  ```
  *Status:* 100% dos pacotes medidos em exatos 1024 bytes. **Zero fragmentação IP**.
* **Estado do Hardware de Áudio no Pi Zero (`/proc/asound/card0/pcm0p/sub0/hw_params`):**
  ```text
  access: RW_INTERLEAVED
  format: IEC958_SUBFRAME_LE
  channels: 2
  rate: 96000 (96000/1)
  period_size: 2048
  buffer_size: 8192
  ```
* **Telemetria de Carga do Pi Zero:**
  * Uso de CPU: ~14% no processo `ext-receiver` e 42% de folga total (idle).
  * Temperatura do SoC: Estável em 50,8 °C durante reprodução ativa.
* **Percepção Subjetiva e Estabilidade:**
  * Star Trek reproduzindo sem nenhum flick de imagem e com som cristalino contínuo sem cortes de início de frase.
