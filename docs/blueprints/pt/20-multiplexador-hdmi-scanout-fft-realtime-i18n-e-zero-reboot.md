# Blueprint 20: Multiplexador HDMI de Porta Única, Engine FFT Realtime, i18n Simétrico e Arquitetura Zero-Reboot (v2.3.0)

> 🇧🇷 Versão em Português | [🇺🇸 English Version](../en/20-single-hdmi-scanout-multiplexer-realtime-fft-i18n-zero-reboot.md)


*Data: 2026-09-29*  
*Status: Implementado, Validado em Hardware Real e Congelado (v2.3.0)*  
*Autor: Carlos Alberto <carlosalberto4ti@gmail.com>*  

---

## 1. Visão Geral e Desafios de Engenharia

A versão **v2.3.0** do `ext-monitor` consolida a convergência final entre monitor secundário profissional de ultra-baixa latência e central de mídia inteligente de hardware. Quatro desafios de alta complexidade técnica foram resolvidos nesta entrega:

1. **Multiplexação Física de Porta Única HDMI (`HDMI-A-1`):** O SoC Broadcom BCM2835 possui apenas uma controladora física de saída HDMI conectada ao televisor/monitor. Se o transmissor de vídeo para PC e o visualizador de áudio tentarem escrever simultaneamente no framebuffer (`/dev/fb0`), ocorre corrupção visual e quebra de scanout.
2. **Cálculo de Espectro Real de Hardware (Sem Dados Fictícios):** Eliminação completa de tabelas senoidais sintéticas (`SINE_LUT`) e simulações. A telemetria de áudio e visualização de barras agora reflete a transformada rápida de Fourier (FFT) em tempo real do áudio real processado pela GPU e pelo sistema de som.
3. **Internacionalização Robusta (i18n) sem Vazamento:** Garantia de que a interface abra em inglês nativo puro (`EN`) como padrão mundial, com dicionários simétricos de 230 chaves em Inglês, Português, Italiano e Chinês.
4. **Arquitetura Zero-Reboot para Troca e Desligamento de Serviços:** Resolução definitiva do padrão de deadlock vivenciado no chaveamento de USB Bulk e FunctionFS, assegurando que qualquer serviço (UDP, USB Bulk, Miracast, Áudio, Visualizador) possa ser ligado, desligado ou alternado dinamicamente **sem necessitar de reinicialização do Raspberry Pi ou do Host PC**.

---

## 2. Multiplexador de Scanout de Porta Única HDMI

### 2.1 A Regra de Exclusão Mútua
Como existe apenas um display HDMI físico, o pipeline do receptor (`ext-receiver`) adota uma máquina de estados estrita:

```text
                                  +-----------------------+
                                  |    Ocioso / Standby   |
                                  | Splash Screen 4 Idiom |
                                  +-----------+-----------+
                                              |
                     +------------------------+------------------------+
                     | Vídeo Ativo (PC)                                | Áudio Ativo (sem Vídeo)
                     v                                                 v
         +-----------------------+                         +-----------------------+
         |      V4L2 M2M         |                         |   Visualizador HDMI   |
         |  H.264 Hardware Dec   |                         |  Espectro 24 Bandas   |
         |  Prioridade Total     |                         |  30 FPS no /dev/fb0   |
         |  (Visualizador Dorme) |                         |  (Evita TV Apagada)   |
         +-----------+-----------+                         +-----------+-----------+
                     |                                                 |
                     +------------------------+------------------------+
                                              | Desconexão / Fim do Stream
                                              v
                                  +-----------------------+
                                  |   Retorno Automático  |
                                  |   Splash Ready 4 Idiom|
                                  +-----------------------+
```

1. **Vídeo de Desktop Ativo:** O pipeline de vídeo H.264 (via V4L2 M2M ou GStreamer) detém 100% do scanout HDMI. A thread do visualizador adormece (`was_drawing = false`), desocupando a GPU e o framebuffer. O áudio digital do PC toca simultaneamente nos alto-falantes da TV via canal HDMI digital PCM/Opus.
2. **Vídeo Inativo + Áudio Ativo (IoT / Música):** Se o usuário estiver ouvindo música via Bluetooth A2DP, DLNA ou áudio do PC sem espelhamento de tela, o visualizador de hardware acorda e desenha a 30 FPS no `/dev/fb0` as 24 barras de frequência reais calculadas pelo sinal sonoro, impedindo que o televisor desligue por inatividade ou apresente tela preta.
3. **Ambos Inativos:** A tela exibe a Splash Screen de Prontidão (*Ready Screen*) com endereço IP, modo e instruções em 4 idiomas.

---

## 3. Engine de Áudio FFT Realtime no Host PC (`ext-sender`)

### 3.1 FFT Cooley-Tukey de 512 Pontos em Rust Puro
O módulo `sender/src/pipeline.rs` monitora diretamente o sink virtual de áudio `Raspberry_Pi_HDMI_Audio.monitor` via `parec` a 48.000 Hz, 16-bit estéreo:

1. **Janelamento de Hann:** Amostras de 512 pontos (2048 bytes $\approx 10.7\text{ ms}$) recebem ponderação $w[n] = 0.5 \times (1 - \cos(2\pi n / N))$ para erradicar dispersão espectral.
2. **Cálculo da Potência RMS:** $RMS_{dB} = 20 \times \log_{10}(RMS)$, quantizado em byte único (0 a 255) representando escala de $-60\text{ dB}$ a $0\text{ dBFS}$.
3. **Agrupamento Logarítmico em 24 Bandas (94 Hz a 24 kHz):**
   - Graves: 94 Hz a 375 Hz (bandas 0 a 3);
   - Médios: 375 Hz a 3.000 Hz (bandas 4 a 11);
   - Agudos e Presença: 3.000 Hz a 24.000 Hz (bandas 12 a 23).
4. **Transmissão UDP de Baixa Sobrecarga:** Pacote binário ultraleve de 25 bytes (24 bytes de barras normalizadas + 1 byte de RMS) enviado diretamente à porta UDP `5006` do Raspberry Pi a 50 FPS.

---

## 4. Visualizador Dinâmico 30 FPS no Painel Web (`ext-receiver`)

- Componente `<canvas id="audioVisualizerCanvas">` responsivo acoplado a um loop contínuo de `requestAnimationFrame`.
- Gradiente cromático dinâmico (Roxo `#8b5cf6` $\rightarrow$ Ciano `#06b6d4` $\rightarrow$ Esmeralda `#10b981`).
- Marcadores de queda de pico (*peak decay*) com taxa suave de descida de $0.02/\text{frame}$.
- Medidor VU Meter Estéreo L/R independente com alertas de saturação em amarelo/vermelho.
- Botão "Test Sound Output" acionando `/api/media/test_sound` para verificação ponta a ponta da cadeia de áudio digital.

---

## 5. Arquitetura Zero-Reboot para Troca e Desligamento de Serviços

### 5.1 A Lição Histórica do USB Bulk (Commit `5f9858d`)
No desenvolvimento anterior, a tentativa de transição entre o Modo 3 (USB Bulk) e o Modo 2 (Miracast) causava o travamento completo do receptor. A causa raiz foi identificada na chamada síncrona `libc::read` no endpoint `/dev/usb-ffs/display/ep1`: quando o host cessava a transmissão, a thread ficava eternamente bloqueada no driver de kernel `dwc2`. A thread de controle chamava `worker.join()` e congelava o processo, exigindo reinicialização física (reboot).

### 5.2 A Solução Universal Anti-Deadlock

1. **Polling Não-Blocante com Timeout (`libc::poll` a 100ms):**
   Toda leitura de barramento físico ou socket (USB Bulk, UDP, Miracast) é precedida por `poll` ou `set_read_timeout` de 10ms a 100ms. O laço acorda ciclicamente para avaliar a flag atômica `running.load()`. Se o serviço for desligado ou trocado, a thread de trabalho encerra em milissegundos sem bloquear o supervisor.
2. **Gerenciamento de Ciclo de Vida do Processo Filho (`Child::kill` & `wait`):**
   No transmissor (`ext-sender`), a struct `StreamerHandle::Composite` agora armazena explicitamente o processo filho do `parec` (`audio_spectrum`) ao lado do `gst-launch-1.0` de áudio e do processo de vídeo. Ao pausar ou alternar modos, ambos são finalizados via `kill()` e coletados via `wait()`, impedindo o acúmulo de processos zumbis ou canais de áudio órfãos.
3. **Desmapeamento e Liberação Limpa de Recursos (`munmap` e `libc::close`):**
   O `FramebufferSink` implementa `Drop` desfazendo o mapeamento de memória (`libc::munmap`) de `/dev/fb0`. O `NativeV4l2Decoder` fecha o descritor `/dev/video10`. Isso permite que outro serviço assuma a saída de vídeo imediatamente.
4. **Deduplicação de Módulos de Áudio PipeWire:**
   Antes de carregar `module-null-sink` com o nome `Raspberry_Pi_HDMI_Audio`, o sistema inspeciona `pactl list sinks short`. Se o sink já existir, ele é reaproveitado sem criar instâncias duplicadas no servidor de som.
5. **Alocação Dinâmica de Monitores Virtuais GNOME Mutter (`RecordVirtual`):**
   Se o monitor físico local não estiver configurado no Mutter, o `ext-sender` recorre automaticamente ao método `RecordVirtual` com flag `remote-desktop: true`. O compositor Wayland cria um monitor estendido dinâmico sem tocar no hardware físico e sem demandar reinicialização de sessão.

---

## 6. Documentação Swagger OpenAPI 3.0 (v2.3.0)

O endpoint `http://192.168.7.2:8080/swagger` disponibiliza a interface interativa Swagger UI baseada na especificação OpenAPI 3.0.3, documentando integralmente:
- Operações de Mídia e Espectro (`/api/media/*`);
- Controle do Host PC (`/api/host/*`);
- Roteamento e Pareamento Bluetooth (`/api/bluetooth/*`);
- Controle de Áudio, Volume e Mudo (`/api/audio/*`);
- Configurações de Rede Ethernet/OTG (`/api/network`).

---

## 7. Verificação em Hardware e Conclusão

- **Raspberry Pi Zero W:** SoC BCM2835 operando a $48.7^\circ\text{C}$, consumo de 1.10 W (220 mA), decodificando H.264 por hardware VideoCore IV sem quedas de frames.
- **Transmissor Host PC:** AMD Radeon 610M executando VA-API em modo contínuo anti-freeze (CFR) a 60 FPS com motor padrão KMS Direct.
- **Troca a Quente de Serviços:** Comandos via Web UI iniciam, pausam, alternam resoluções e modos sem qualquer necessidade de reinicialização da placa ou do computador.

---

## 8. Protocolo de Auto-Discovery de Rede (Wi-Fi e Ethernet LAN) via UDP Broadcast (Porta 5002)

Para eliminar a necessidade de configuração manual de endereços IP quando o receptor opera conectado à rede Wi-Fi ou através de adaptadores Ethernet USB:
1. **Beacon Responder no Receptor (`receiver/src/discovery.rs`):**
   - O receptor escuta na porta UDP `5002` por datagramas contendo a string de probe `EXT-MONITOR-DISCOVER`.
   - Ao receber o probe, responde imediatamente ao emissor com o payload `EXT-MONITOR-OFFER http_port=8080 stream_port=5000 version=2.3.0`.
   - Adicionalmente, emite um broadcast periódico a cada 10 segundos em `255.255.255.255:5002` anunciando sua presença na rede local.
2. **Probing Dinâmico no Transmissor (`sender/src/discovery.rs`):**
   - Na inicialização do `ext-sender`, caso o IP USB padrão `192.168.7.2` não esteja alcançável, o transmissor dispara automaticamente o broadcast `EXT-MONITOR-DISCOVER` na porta 5002.
   - Ao receber a resposta `EXT-MONITOR-OFFER`, extrai o endereço IP da interface Wi-Fi/Ethernet e reconecta o pipeline de vídeo RTP e áudio Opus sem intervenção manual do usuário.

---

## 9. Isolamento Estrito Anti-Hijack de Áudio (WirePlumber), Supressão de Silêncio e Fast Cutoff (<800ms)

Para eliminar interferências indesejadas de áudio pessoal do usuário (fones de ouvido, YouTube no PC) e evitar dados falsos na TV e Web UI:
1. **Bloqueio Anti-Hijack no PipeWire (`sender/src/pipeline.rs`):**
   - Injetam-se as variáveis `PULSE_SOURCE="Raspberry_Pi_HDMI_Audio.monitor"` e `PULSE_PROP="media.role=filter stream.dont-route=true node.dont-reconnect=true"` nas chamadas de `parec` e `gst-launch-1.0` (Opus).
   - O WirePlumber fica impedido de transferir o monitor para outro sink quando a saída de som do GNOME é comutada.
2. **Zero-Packet Streaming no Host:**
   - Durante silêncio (`rms_db < -55.0 dB`), transmitem-se 3 quadros de transição e cessa-se totalmente o tráfego UDP 5006 (0 pacotes por segundo).
3. **Corte Rápido (<800ms) e Restauração Instantânea do Splash no Receptor:**
   - Inatividade maior que 800ms zera barras, picos e RMS (`-60.0 dB`).
   - Se o visualizador estava ativo, invoca imediatamente `SplashEngine::show_ready()`, garantindo que a TV exiba o Splash de Prontidão em 4 idiomas sem dados falsos ou telas pretas.
4. **Eliminação de Ondas Simuladas na Web UI:**
   - Removidas animações senoidais sintéticas ociosas. Barras e VU meters ficam estritamente em zero quando o áudio está desligado.

