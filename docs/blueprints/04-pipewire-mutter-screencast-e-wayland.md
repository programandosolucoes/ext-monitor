# Blueprint 04: Integração com PipeWire, D-Bus Mutter Screencast e Wayland

**Projeto:** `ext-monitor`  
**Autor:** Carlos Alberto & Antigravity  
**Arquivos de Referência:** `sender/src/main.rs`, `scripts/start.sh`, `scripts/show-welcome-window.py`  
**Data:** Setembro de 2026  

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
