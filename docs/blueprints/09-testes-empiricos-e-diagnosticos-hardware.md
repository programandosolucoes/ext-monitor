# Blueprint 09: Testes Empíricos em Hardware e Diagnósticos de Boot

**Projeto:** `ext-monitor`  
**Autor:** Carlos Alberto <carlosalberto4ti@gmail.com>  
**Ambiente de Testes:** Raspberry Pi Zero v1.3 Monocore (ARMv6 1.0 GHz, 512MB RAM, VideoCore IV VPU @ 500MHz)  
**Data:** Setembro de 2026  

---

## 1. Visão Geral

Este documento consolida os dados de testes empíricos, medições osciloscópicas/temporais de inicialização e diagnósticos de hardware realizados diretamente sobre a placa de silício **Raspberry Pi Zero v1.3**.

O objetivo desta documentação é servir como referência técnica aprofundada para entender as nuances da controladora Broadcom BCM2835, a interação entre o firmware do VideoCore IV (`bootcode.bin`/`start.elf`) e o kernel Linux (`kernel.img`), bem como o comportamento das interfaces de rede USB (`usb0`) e saída de vídeo HDMI direta.

---

## 2. Diagnóstico de Boot: A Tela Arco-Íris e a Ativação de Rede USB (`usb0`)

Nos testes práticos realizados no Raspberry Pi Zero v1.3 Monocore, identificou-se e mapeou-se todo o fluxo entre o firmware e o kernel:

```
[ Alimentação 5V USB ] ──▶ [ GPU Bootcode (bootcode.bin) ] ──▶ [ Tela Arco-Íris GPU ]
                                                                       │
                                                                       ▼
[ Desktop Conectado ] ◀── [ Rede USB Ativa (usb0) ] ◀── [ Kernel Linux + Initramfs ]
```

### 2.1 A Tela Arco-Íris (GPU Splash Screen)
* **O que é:** A imagem com padrão em gradiente de 4 cores (arco-íris) é gerada diretamente pela GPU Broadcom VideoCore IV nos primeiros **300 milissegundos** de alimentação, antes de carregar o kernel Linux.
* **Diagnóstico Técnico:**
  1. **Se a tela arco-íris permanece congelada:** A GPU inicializou com sucesso, mas o kernel Linux (`kernel.img`) não foi localizado ou os parâmetros em `config.txt` estão incorretos para o SoC ARMv6.
  2. **Se a tela arco-íris pisca por ~1 segundo e apaga:** Comportamento normal e esperado. A GPU transferiu com sucesso o controle para o kernel Linux, que assume o framebuffer `/dev/fb0` e inicia o `/init` em RAM.

### 2.2 Linha do Tempo de Inicialização do Appliance (0.0s a 1.8s)

| Tempo Relativo | Subsistema | Evento / Estado de Hardware |
| :--- | :--- | :--- |
| **0.00s** | Silício | Aplicação de tensão de 5V na porta micro-USB OTG central. |
| **0.25s** | VideoCore IV | GPU carrega `bootcode.bin` e `start.elf`. Exibe padrão arco-íris no HDMI. |
| **0.65s** | Kernel ARMv6 | Carga do `kernel.img` (5.15-v6) e montagem do `initramfs.cpio.gz` na memória RAM. |
| **1.10s** | ConfigFS | Script `/init` instancia o USB Gadget multifunção (CDC-ECM + ACM + FunctionFS). |
| **1.40s** | DWC2 OTG | Host detecta novo dispositivo USB `1d50:614d` / `1d6b:0104`. Interface `usb0` criada no PC. |
| **1.65s** | DHCP Rust | Servidor DHCP embutido no `ext-receiver` atribui IP `192.168.7.1` ao computador. |
| **1.80s** | HDMI Scanout | Framebuffer ativado, painel Web ouvindo na porta HTTP 8080 e RTP pronto na UDP 5000. |

---

## 3. Matriz de Testes Empíricos de Desempenho e Cores

Durante a homologação em laboratório, foram testados diferentes perfis de codificação de vídeo em tempo real a uma resolução nativa de **1280x720** e **1600x900**:

### 3.1 Perfil de Cores 256 (Economy / Coarse QP 30-44)
* **Comportamento Observado:** O codificador VA-API utiliza quantização agressiva (QP entre 30 e 42) com `target-percentage=50`.
* **Consumo de Banda:** ~400 kbps constantes.
* **Qualidade de Imagem:** Texto de fontes monospace (terminais, VS Code) perfeitamente nítido e legível. Gradientes contínuos sofrem leve banding discreto, com baixo consumo de CPU e RAM.
* **Uso Ideal:** Sessões prolongadas de programação, digitação e monitoramento de servidores via rede com largura de banda restrita.

### 3.2 Perfil 24-bit TrueColor (Full Colors / Dynamic QP 18-34)
### 3.3 Preservação de Salto de Quadros (Frame Skipping) e Varredura Periódica IDR

Durante o refinamento empírico da pipeline GStreamer/VA-API, identificamos que o elemento `videorate` com `drop-only=false` gerava quadros duplicados artificiais mesmo com a tela do desktop 100% estática, saturando a banda com dados redundantes e elevando o atraso residual.

Para sanar o gargalo e permitir sintonia fina dinâmica pelo usuário:
1. **`drop-only=true` (Preservação Damage-Only):**
   * Quando a tela está imóvel, o pipeline não duplica quadros; todo o orçamento de taxa de bits (bitrate) é mantido em repouso.
   * Quando ocorre movimentação (movimento do mouse ou digitação capturada pelo Mutter ScreenCast via D-Bus), o pipeline aloca 100% do orçamento para as áreas com danos atualizados. Economia de até **95% de banda e CPU** em repouso.
2. **`skip-to-first=true` (Latência Zero ao Mover):**
   * Garante a entrega imediata do primeiro quadro atualizado sem aguardar slots de tempo prévios, eliminando a sensação de "arrasto" ou atraso ao retomar a digitação ou mover o cursor.
3. **`key-int-max=N` (Varredura Periódica / Refresh Clean):**
   * Injeta um quadro-chave IDR completo periodicamente (padrão de 30 quadros a cada 1.0s).
   * Atua como uma varredura de limpeza automática, eliminando artefatos residuais e restaurando a fidelidade da imagem na TV/monitor HDMI sem necessidade de reinicializar o stream.

### 3.4 Exposição no Painel Web, Scripts e Persistência

Esses três parâmetros foram totalmente externalizados para controle do usuário:
* **Painel Web (`receiver/src/web_ui.rs`):**
  * Controles visuais para `drop-only` (Ativo/Desativado), `skip-to-first` (Ativo/Desativado) e slider de quadros IDR com presets (15q, 30q, 60q, 120q).
  * Persistência automática em `localStorage` no navegador para que as opções sobrevivam a recarregamentos (`F5`).
  * Sincronização bidirecional em tempo real com `GET /api/config` e `POST /api/config`.
* **Hot-Apply via UDP 5001:** O receptor repassa a configuração instantaneamente para o transmissor `ext-sender` no PC através do soquete de controle `192.168.7.1:5001`.
* **Linha de Comando e Scripts (`scripts/start.sh`):** Suporte nativo aos argumentos `--drop-only`, `--no-drop-only`, `--skip-to-first`, `--no-skip-to-first` e `--key-int-max=<N>`.

### 3.5 Sinalização MS-MICE e GNOME Network Displays (Porta TCP 7250)

Para suporte ao GNOME Network Displays no Linux e Windows Miracast sobre Infraestrutura (MS-MICE):
* O serviço Avahi publica `_display._tcp` e `_miracast._tcp` com o registro TXT `p2pMAC=12:22:33:44:55:66` associado à interface do dispositivo.
* O `ext-receiver` implementa um listener TCP na porta **7250** dedicado ao protocolo MS-MICE.
* Ao receber o sinal `SOURCE_READY` na porta 7250, o receptor aceita o handshake e confirma que o canal RTSP 7236 está pronto para início da sessão de vídeo, viabilizando projeção sem fio nativa em clientes GNOME Wayland.

### 3.6 Teste de Estresse Térmico e Uso de Silício

| Parâmetro Medido | Em Repouso (Aguardando Sinal) | Streaming Ativo 30 FPS | Streaming 60 FPS (Pico) |
| :--- | :--- | :--- | :--- |
| **Temperatura do SoC** | 41.2°C | 44.8°C | 48.5°C |
| **Carga de CPU (ARM1176)** | 0.05 (praticamente nula) | ~0.45% a 1.2% | ~3.8% |
| **Memória RAM Usada** | 138 MB | 148 MB | 154 MB |
| **Memória RAM Livre** | 362 MB | 352 MB | 346 MB |
| **Latência Média de Render** | N/A | **11.4 ms** | **8.2 ms** |

---

## 4. Testes de Transição de Energia e Recuperação (PC Suspend / Sleep)

Nos testes de suspensão de energia do PC (S3 / Suspend-to-RAM):
1. **Comportamento Sem Watchdog:** Ao suspender o PC, o PipeWire destruía o nó ScreenCast `ext-hdmi-sender`. Ao retornar da suspensão, o stream falhava silenciosamente e ficava com tela preta.
2. **Correção Implementada:**
   * Watchdog de nó D-Bus / PipeWire verificando o status a cada 1500ms.
   * Ao detectar que o nó desapareceu, o `ext-sender` finaliza o processo filho, aguarda a restauração dos serviços do GNOME Wayland e recria a sessão atômica de captura.
   * Tempo de recuperação pós-retomada de energia: **~1.5 segundos**, com restauração visual automática no monitor HDMI conectado ao Pi Zero.

---

## 5. Conclusões Técnicas

1. **Aceleração 100% Hardware é Viável no ARMv6:** O chip BCM2835 monocore de 2012 é perfeitamente capaz de reproduzir vídeo em tempo real a 60 FPS, desde que a decodificação seja delegada ao VideoCore IV (`bcm2835-codec`) e o escaneamento de tela utilize KMS DRM overlay direto.
2. **Modo Appliance em RAM Elimina Falhas:** Rodar o sistema operacional em initramfs em RAM garante que oscilações elétricas ou desconexões abruptas da USB nunca causem corrupção no sistema de arquivos do appliance.
