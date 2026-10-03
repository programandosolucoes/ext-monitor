# Blueprint 33: Certificação TLS Estrita Chromium Cast, Handshake WebRTC Mirroring e Eliminação de Conflitos SSDP/DLNA

## 1. Visão Geral e Motivação
Com a introdução da funcionalidade de transmissão estilo Chromecast (`RaspCast`) na porta TLS 8009 e anúncio mDNS `_googlecast._tcp.local`, os testes reais utilizando o navegador Google Chrome no laptop revelaram dois problemas críticos de integração e experiência do usuário:

1. **Dispositivo Duplicado / Incompatível no Menu Transmitir do Chrome:** O Chrome exibia duas entradas simultâneas na busca de dispositivos Cast:
   - `RaspCast` (Compatível, funcionando via Google Cast V2);
   - `Ext-Monitor TV & Sound (192.168.7.2)` (Incompatível, rotulado pelo Chrome como *"Disponível em sites específicos"*).
2. **Tela Cinza / Vazia e Comportamento da Janela de Seleção de Área:**
   - Ao iniciar a transmissão pelo Chrome, a tela da TV/Pi Zero exibia apenas um buffer cinza estático.
   - O usuário esperava a caixa de diálogo nativa de seleção de tela ou janela, mas o Chrome transmitia diretamente sem abrir o seletor.

Este Blueprint documenta a engenharia reversa do código-fonte do Chromium, a implementação do handshake WebRTC Mirroring, a correção de ativação lógica no compositor GNOME Mutter e a resolução definitiva de conflitos SSDP.

---

## 2. Arquitetura e Engenharia Reversa

### 2.1 Requisitos Estritos de Certificados no Chromium Cast (`cast_auth_util.cc` & `cast_cert_validator.cc`)
A engenharia reversa do código do Chromium revelou regras de segurança criptográfica ultra-estritas para conexões Cast autenticadas:
1. **Validade Máxima de 4 Dias:** O Chromium impõe `kMaxSelfSignedCertLifetimeInDays = 4`. Certificados auto-assinados com validade superior (ex: 365 dias) são sumariamente rejeitados. O gerador de certificados em [`sender/src/cast_cert.rs`](file:///home/carlos/ide/ext-monitor/sender/src/cast_cert.rs) foi configurado para `-days 3`.
2. **Extensões X.509 Críticas Obrigatórias:**
   - `basicConstraints = critical, CA:FALSE`
   - `keyUsage = critical, digitalSignature, keyEncipherment`
   - `extendedKeyUsage = clientAuth, serverAuth`
3. **Casamento de Chave Pública sob Mutex (`CERT_MUTEX`):** O certificado e a chave RSA efêmera gerados devem possuir chaves públicas idênticas (`cert_pkey.public_eq(&rsa_pkey)`), sob proteção de mutex para evitar concorrência em testes unitários.
4. **Algoritmo de Assinatura:** O handshake de desafio criptográfico exige assinatura digital `RSASSA_PKCS1v15` com SHA-256 (`sig_algo = 1`).

### 2.2 Eliminação do Dispositivo Duplicado / Incompatível (Conflito SSDP vs Cast V2)
O Raspberry Pi Zero inclui um motor UPnP/DLNA `MediaRenderer` para reprodução direta via tocadores de mídia locais. O servidor de anúncios SSDP no receptor ([`receiver/src/media_renderer.rs`](file:///home/carlos/ide/ext-monitor/receiver/src/media_renderer.rs)) enviava notificações periódicas `NOTIFY` com `ST: urn:schemas-upnp-org:device:MediaRenderer:1` e nome `Ext-Monitor TV & Sound`.

Como o Google Chrome monitora a porta UDP 1900, ele interpretava esse descritor UPnP genérico como um destino de mídia restrito (*"Disponível em sites específicos"*).

**Solução Implementada:**
1. Em `receiver/src/media_renderer.rs`, suprimimos anúncios `NOTIFY` periódicos de `MediaRenderer` e `upnp:rootdevice` no link USB (`usb0`), mantendo apenas os anúncios do protocolo DIAL (`urn:dial-multiscreen-org:device:dial:1`).
2. Em consultas `M-SEARCH` com `ST: ssdp:all`, o receptor responde apontando exclusivamente para o descritor DIAL do `RaspCast` (`/dial/dd.xml`).
3. Renomeamos o descritor UPnP interno para `RaspCast DLNA (192.168.7.2)` para clientes dedicados que façam busca explícita por `MediaRenderer`.
4. Na ponte local do host ([`sender/src/discovery.rs`](file:///home/carlos/ide/ext-monitor/sender/src/discovery.rs)), restringimos as respostas para filtrar requisições genéricas e encaminhar apenas consultas DIAL.
5. **Resultado:** O Chrome passou a listar exclusivamente o **`RaspCast`** nativo.

### 2.3 Handshake WebRTC Mirroring (`urn:x-cast:com.google.cast.webrtc`)
Ao clicar em transmitir tela ou guia, o Chrome envia uma oferta de espelhamento WebRTC via Cast V2:
```json
{"offer":{"castMode":"mirroring","supportedStreams":[{"codecName":"opus","index":0,"ssrc":26593,...},{"codecName":"vp9","index":1,"ssrc":91897,...}]},"seqNum":1,"type":"OFFER"}
```
Implementamos o parser e gerador de resposta síncrona `ANSWER` em [`sender/src/cast_server.rs`](file:///home/carlos/ide/ext-monitor/sender/src/cast_server.rs):
```rust
let answer_json = serde_json::json!({
    "type": "ANSWER",
    "seqNum": seq_num,
    "result": "ok",
    "answer": {
        "castMode": "mirroring",
        "receiverGetStatus": true,
        "sendIndexes": send_indexes,
        "ssrcs": ssrcs,
        "udpPort": 5000
    }
}).to_string();
```
Adicionalmente, implementamos respostas para a mensagem `GET_STATUS` retornando status de conectividade WiFi.

### 2.4 Resolução da Imagem Cinza: Falso Positivo no GNOME Mutter
A imagem cinza na TV ocorria devido a uma falha na detecção da configuração lógica de monitores no GNOME Wayland via D-Bus:
1. Em `sender/src/pipewire.rs`, a função `ensure_gnome_displays()` verificava se a tela já estava estendida através de `stdout.contains("('HDMI-1'")`.
2. **Causa Raiz:** O método `GetCurrentState` do Mutter retorna uma tupla com dois blocos:
   - Primeiro bloco: conectores físicos conectados (`(('HDMI-1', 'LRX'...)`) com seus modos suportados;
   - Segundo bloco: arranjo de monitores lógicos ativos (`[(0, 0, 1.0, 0, true, [('eDP-1'...)]), (1920, 0, 1.0, 0, false, [('HDMI-1'...)]]`).
3. Como `HDMI-1` estava fisicamente conectado via override de EDID, o método retornava falso positivo de que a tela já estava ativa no GNOME, pulando a chamada necessária `ApplyMonitorsConfig`.
4. Ao invocar `MutterScreenCastSession::create_and_start("HDMI-1")`, o Mutter rejeitava a captura:
   ```
   RecordMonitor('HDMI-1') failed: Failed to record monitor: Monitor not active. Fallback: RecordVirtual...
   ```
5. O fallback `RecordVirtual` criava uma tela virtual vazia e sem janelas alocadas, transmitindo o buffer cinza padrão.
6. **Correção:** Alteramos a checagem para `stdout.contains("[('HDMI-1'")`. O prefixo `[('` é exclusivo da tupla de monitores lógicos ativos no Mutter. Com isso, o comando `ApplyMonitorsConfig` é executado com precisão atômica, ativando o monitor `HDMI-1` antes da captura e renderizando a tela estendida perfeitamente na TV.

### 2.5 Guia de Seleção de Área e Tela no Chrome
Explicamos e documentamos o comportamento do Google Chrome:
- **Modo Padrão ("Transmitir guia"):** Captura diretamente a aba atual via DOM/canvas sem exibir janela de seleção do sistema operacional.
- **Modo Tela/Janela ("Fontes ➔ Transmitir tela"):** Dispara a chamada nativa do XDG Desktop Portal / Wayland no GNOME, exibindo a janela interativa para o usuário escolher entre a tela primária (`eDP-1`), o monitor estendido (`HDMI-1`) ou uma janela específica de aplicativo.

### 2.6 Decisão Interativa do Usuário com 4 Modos de Operação
Para garantir que o usuário tenha total controle e todas as opções de compartilhamento:
1. **Diálogo Gráfico Nativo Wayland (`zenity --list --radiolist`):**
   - Disparado automaticamente na tela do laptop ao receber conexão de transmissão ou comando `LAUNCH`:
     - 🖥️ `extend`: **Estender Área de Trabalho** (Ativa tela secundária virtual `HDMI-1` na TV).
     - 💻 `clone`: **Espelhar Tela Inteira do PC** (Clona a tela primária `eDP-1` do laptop na TV).
     - 🪟 `window`: **Transmitir Janela de Aplicativo** (Abre o Web Caster interativo com seletor de janelas do GNOME).
     - 🌐 `tab`: **Transmitir Aba do Navegador** (Abre o Web Caster `/cast` no Chrome com áudio e WebCodecs).
     - ❌ **Cancelar / Fechar / Timeout (45s)**: Aborta a transmissão sem alterar as telas do GNOME.
2. **Controle via Painel Web (`http://192.168.7.2:8080`):**
   - Grade com 3 opções sob *Modo de Exibição do Monitor*:
     - `❓ Perguntar Sempre (Diálogo)` (`mode: 'ask'`)
     - `🖥️ Extended (HDMI-1 TV)` (`mode: 'extend'`)
     - `💻 Cloned (eDP-1 Notebook)` (`mode: 'clone'`)
3. **Controle via Linha de Comando:**
   - Suporte aos parâmetros `--mode=ask`, `--mode=extend` e `--mode=clone`.

### 2.7 Correção do Congelamento de Seleção na Dock do GNOME (Damage Pacer)
- **Diagnóstico da Causa Raiz:** O laço de pacer (`damage_pacer`) criava uma janela X11 em Xwayland nas coordenadas `(3198, 718)` pulsando `clear_area` a cada 16ms mesmo em Standby com o monitor `HDMI-1` recolhido. Como essa coordenada ficava fora da geometria ativa da tela primária (`eDP-1`), o rastreador de janelas do GNOME Shell (`ShellWindowTracker` / Mutter) perdia a referência de foco, fazendo com que cliques em aplicativos abertos na dock (como o Antigravity) abrissem novas instâncias em vez de trazê-los para o primeiro plano.
- **Solução Implementada:** Como a engine padrão é DRM/KMS direto via hardware scanout DMA-BUF, o pacer de janelas X11 é completamente desnecessário. Tornamos o `damage_pacer` estritamente opcional (apenas sob `--enable-damage-pacer` e engine Mutter). A janela fantasma `0x1000001` foi erradicada, normalizando imediatamente o clique e a alternância de janelas na dock.

### 2.8 Fim da Imagem Congelada na TV ao Parar a Transmissão (Zero Frozen Frame)
- **Diagnóstico:** Quando o usuário interrompia a transmissão (no Chrome ou via STOP), o host encerrava o GStreamer, mas não notificava o receptor. No receptor, o watchdog de inatividade UDP aguardava 15 segundos antes de restaurar o splash, mantendo o último frame decodificado congelado na TV.
- **Solução Dual:**
  1. **Notificação Atômica via HTTP:** Ao receber `StopStreaming` no host ou Cast `STOP`, o `ext-sender` executa imediatamente uma chamada assíncrona para `POST http://192.168.7.2:8080/api/stream/stop`.
  2. **Watchdog Reduzido no Receptor:** Em `receiver/src/ingress/udp.rs`, o timeout de inatividade foi reduzido de 15s para **1,5s**.
  3. Ao parar a transmissão, a TV retorna instantaneamente (< 100ms) para a tela de espera (*Ready Splash* com IP e QR Code), liberando o uso sem congelamentos.

---

## 3. Validação e Resultados

1. **Testes Unitários:**
   - `ext-sender`: 77 testes passando (`cargo test -p ext-sender`).
   - `ext-receiver`: 72 testes passando (`cargo test -p ext-receiver`).
2. **Deploy Dual Completo:**
   - Binário `ext-sender` compilado em release e instalado em `/usr/local/bin/ext-sender`.
   - Serviço de usuário `ext-monitor-sender.service` ativo e respondendo aos comandos de `LAUNCH` e `STOP` do Cast.
   - Binário `ext-receiver` cross-compilado para ARMv6 musl, gravado na partição de boot do Pi Zero e aplicado ao vivo na memória RAM.
3. **Validação Funcional:**
   - O menu Cast do Chrome lista exclusivamente o dispositivo `RaspCast`.
   - O espelhamento inicia com negociação WebRTC instantânea e tela estendida real sem buffer cinza.
