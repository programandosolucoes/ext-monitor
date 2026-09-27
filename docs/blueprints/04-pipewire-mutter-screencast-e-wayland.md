# Blueprint 04: Integração com PipeWire, D-Bus Mutter Screencast e Wayland

**Projeto:** `ext-monitor`  
**Autor:** Carlos Alberto <psncarlosalberto4ti@gmail.com>  
**Arquivos de Referência:** `sender/src/screencast.rs`, `sender/src/pipewire.rs`, `sender/src/pipeline.rs`, `sender/src/main.rs`  
**Data:** Setembro de 2026 (Atualizado com Refatoração Modular e RAII Drop Cleanup)  

---

## 1. Visão Geral

Em ambientes gráficos Linux modernos com o servidor **Wayland** e compositor **GNOME Mutter**, aplicações de usuário não possuem permissão de acesso direto à memória de vídeo de outras janelas ou da área de trabalho por razões de segurança e isolamento.

O `ext-monitor` resolve esse desafio integrando-se nativamente com a API de D-Bus do GNOME Mutter (`org.gnome.Mutter.ScreenCast`) e com o servidor de multimídia **PipeWire**, permitindo a criação de **monitores virtuais estendidos sem necessidade de qualquer hardware adicional (como HDMI dummy plugs)**.

---

## 2. A Sequência de Handshake D-Bus com o GNOME Mutter

Para criar e capturar o monitor estendido de forma programática, o `ext-sender` executa a seguinte sequência de chamadas D-Bus:

```
[ ext-sender ]                          [ org.gnome.Mutter.ScreenCast ]
      │                                                │
      ├─── 1. CreateSession(properties) ──────────────>│
      │<─── Retorna Objeto de Sessão (/org/gnome/...) ─┤
      │                                                │
      ├─── 2. RecordVirtual(session, properties) ─────>│
      │       (Cria monitor virtual HDMI-1 ou VIRTUAL) │
      │<─── Retorna Objeto de Stream ──────────────────┤
      │                                                │
      ├─── 3. OpenPipeWireRemote() ───────────────────>│
      │<─── Retorna File Descriptor Unix (fd) ─────────┤
      │                                                │
      ├─── 4. Start() ────────────────────────────────>│
      │<─── Sinal: PipeWire Node ID (ex: 45) ──────────┤
```

### 2.1 Detalhes das Propriedades de Sessão D-Bus
* **Destino:** `org.gnome.Mutter.ScreenCast`
* **Caminho do Objeto:** `/org/gnome/Mutter/ScreenCast`
* **Interface:** `org.gnome.Mutter.ScreenCast`
* **Método `CreateSession`:**
  * `remote-desktop`: tipo de sessão para controle ou captura.
* **Método `RecordVirtual`:**
  * Solicita ao Mutter a instanciação de uma saída virtual com resolução nativa (ex: `1280x720` a 60Hz).
  * O compositor do GNOME adiciona essa tela à sua topologia multimonitor (visível nas Configurações de Telas do GNOME).

---

## 3. Roteamento Zero-Copy com PipeWire e DMA-BUF

Uma vez obtido o File Descriptor de conexão com o PipeWire e o identificador numérico do nó (`node_id`), o pipeline de vídeo é estabelecido:

```
[ Mutter Virtual Display ]
         │ (Renderização por Hardware GPU)
         ▼
[ DMA-BUF Memory Pages ] <── Buffer mapeado diretamente na VRAM
         │
         ▼ (Passagem de ponteiro sem cópia de CPU)
[ PipeWire Stream Node ]
         │
         ▼ (pipewiresrc path=NODE_ID)
[ Codificador VA-API / NVENC ]
```

### 3.1 Vantagem do DMA-BUF Zero-Copy
* A memória de vídeo nunca é copiada para o espaço de usuário da CPU (sem chamadas `memcpy`).
* O encoder de hardware (VA-API/NVENC) lê os mesmos blocos de memória em que a GPU do sistema renderizou a área de trabalho.
* O consumo de CPU no computador host permanece praticamente nulo (< 2%).

### 3.2 Linkagem Determinística por ID Numérico (`pw-link`)
Em ambientes com múltiplos dispositivos de áudio e vídeo, nomes de nós do PipeWire (como `Mutter Screencast` ou `GNOME Virtual`) podem sofrer variações ou colisões de strings.
* O script `start.sh` e o `ext-sender` identificam o nó PipeWire estritamente pelo seu **ID numérico inteiro** (ex: `node.id=48`).
* A conexão com o elemento consumidor é feita de forma atômica via `pw-link <SOURCE_ID> <SINK_ID>`.

---

## 4. O Fenômeno de "Damage Tracking" no Mutter e a Solução do Relógio Virtual

Um dos diagnósticos mais importantes realizados no projeto envolve o mecanismo de **Damage Tracking** (rastreamento de áreas modificadas) do GNOME Mutter.

### 4.1 O Problema da Tela Estática
* O Mutter foi projetado para economizar energia em notebooks e desktops.
* **Se o usuário não move o mouse e nenhuma janela é alterada no monitor virtual `HDMI-1`, o Mutter simplesmente interrompe o envio de buffers para o PipeWire.**
* O encoder de vídeo e o pipeline GStreamer interpretam a ausência de buffers como interrupção da transmissão, podendo sofrer timeout ou fechamento de portas de rede.

### 4.2 A Solução: Janela de Atividade de Baixo Custo (`show-welcome-window.py`)
Para manter o canal PipeWire ativo e o decodificador do Raspberry Pi Zero sincronizado mesmo quando a tela estendida está em repouso:
1. Um pequeno script em Python/GTK ou thread nativa (`scripts/show-welcome-window.py`) exibe um relógio ou micro-indicador visual na tela `HDMI-1`.
2. O relógio atualiza um bloco de 10x10 pixels a cada **500 milissegundos**.
3. Essa micro-alteração de pixels gera um evento de dano (*damage event*) legítimo no Mutter.
4. O Mutter dispara um novo buffer DMA-BUF para o PipeWire, garantindo que o fluxo H.264 permaneça fluindo continuamente sem nenhum congelamento ou perda de conexão.

---

## 5. Resiliência a Suspensão/Hibernação de Energia (Sleep / Suspend / Resume)

Um dos desafios mais complexos em drivers de vídeo virtuais no Linux é a **preservação do fluxo após o computador entrar em suspensão de energia (sleep / suspend-to-RAM)**.

### 5.1 A Causa Raiz da "Morte da Imagem" Pós-Suspensão
1. **Destruição da Sessão pelo Mutter:** Quando o sistema entra em modo de suspensão, o GNOME Mutter encerra sumariamente todas as sessões ativas de ScreenCast D-Bus e destrói o nó de origem do PipeWire para economizar energia.
2. **Bloqueio Invisível em I/O Wait:** O processo filho (`gst-launch-1.0`) não é encerrado pelo sistema operacional; em vez disso, ele permanece vivo em segundo plano, **bloqueado indefinidamente** em espera de leitura (*I/O wait*) em um socket de PipeWire órfão.
3. **Falha de Detecção:** Se o supervisor do transmissor monitorar apenas se o processo filho terminou (`child.try_wait()`), ele nunca detectará a queda, pois o filho nunca sai do ar por conta própria. Ao acordar, o usuário encontra a tela secundária congelada ou preta.

### 5.2 A Arquitetura de Duplo Watchdog em Rust

Para garantir recuperação 100% autônoma, o `ext-sender` implementa duas sondas de monitoramento contínuo no laço de supervisão principal:

```
                  [ Laço de Supervisão (ext-sender) ]
                                 │
           ┌─────────────────────┴─────────────────────┐
           ▼                                           ▼
 [ Health Watchdog: 1500ms ]                 [ Link Watchdog: 3000ms ]
           │                                           │
  is_pipewire_node_alive(id) ?                is_sender_linked() ?
     ├── SIM: Stream saudável                    ├── SIM: Conexão íntegra
     └── NÃO: NÓ DESAPARECEU!                   └── NÃO: Link desfeito!
           │                                           │
           ▼                                           ▼
   1. Mata processo filho órfão                Reexecuta pw-link atômico
   2. Aguarda estabilização (1.5s)
   3. Break para laço externo
   4. Cria nova sessão Mutter via D-Bus
   5. Obtém novo Node ID
   6. Spawna novo pipeline GStreamer
   7. Reconecta e retoma em < 2 segundos!
```

#### Código da Sonda de Verificação no PipeWire:
```rust
fn is_pipewire_node_alive(node_id: u32) -> bool {
    let output = match Command::new("pw-cli")
        .arg("info")
        .arg(node_id.to_string())
        .output()
    {
        Ok(o) => o,
        Err(_) => return false,
    };
    let s = String::from_utf8_lossy(&output.stdout);
    let err = String::from_utf8_lossy(&output.stderr);
    !s.is_empty() && !s.contains("unknown global") && !err.contains("unknown global")
}

fn is_sender_linked() -> bool {
    if let Ok(output) = Command::new("pw-link").arg("-l").output() {
        let s = String::from_utf8_lossy(&output.stdout);
        return s.contains("ext-hdmi-sender:input_1");
    }
    true
}
```

### 5.3 Resiliência do Relógio Virtual e Regras Udev
* **Auto-Recuperação do Relógio (`show-welcome-window.py`):** O script em Python agora encapsula a inicialização do Tkinter em um laço infinito `while True:` com captura de exceções, garantindo que se o servidor Xwayland reiniciar na volta do sono, a janela se reconecte ao display `:0` em menos de 2 segundos.
* **Retomada de Barramento Udev (`99-ext-monitor.rules`):** O evento `ACTION=="add|change"` para o subsistema de rede USB reaplica automaticamente o parâmetro `txqueuelen 100` e dispara o serviço `ext-monitor-autoconnect.service`, assegurando que a placa de rede emulada volte à operação de baixa latência imediatamente após a desativação da suspensão.

