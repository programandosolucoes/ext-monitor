# Blueprint 06: Modos Concorrentes de Operação e o Super-Gadget USB ConfigFS

**Projeto:** `ext-monitor`  
**Autor:** Carlos Alberto <psncarlosalberto4ti@gmail.com>  
**Arquivos de Referência:** `build-appliance/initramfs/init`, `receiver/src/wfd.rs`, `receiver/src/usb_bulk.rs`, `sender/src/usb_transport.rs`, `receiver/src/web_ui.rs`  
**Data:** Setembro de 2026  

---

## 1. Visão Geral

Um dos maiores diferenciais técnicos do `ext-monitor` é a sua capacidade de operar de forma **universal e concorrente**:
* Um único dispositivo Raspberry Pi Zero conectado a qualquer computador (seja Linux, Windows ou Mac) disponibiliza simultaneamente os serviços de rede, terminal de controle serial e unidade de disco para atualização.
* O firmware em Rust escuta simultaneamente nos canais de transmissão do **Linux (UDP)** e do **Windows Miracast (RTSP/WFD)**, permitindo alternância instantânea de computadores sem necessidade de reiniciar o appliance.
* O painel Web Dashboard disponibiliza **flags de controle (chaves de ligar/desligar)** independentes para cada um dos 3 modos, permitindo ao usuário desativar modos concorrentes e operar com isolamento total em um canal exclusivo.

---

## 2. A Arquitetura do Super-Gadget USB ConfigFS

O Linux USB Gadget Subsystem é configurado no boot pelo script `/init` utilizando a interface de sistema de arquivos virtual **ConfigFS** (`/sys/kernel/config/usb_gadget/`):

```
/sys/kernel/config/usb_gadget/ext_composite/
├── idVendor (0x1d50 - Openmoko / Experimental / 0x1d6b Linux Foundation)
├── idProduct (0x614d - Multifunction Hub / 0x0104 Composite Gadget)
├── strings/0x409/ (Fabricante: Raspberry Pi | Produto: High-Speed Display Hub)
├── configs/c.1/ (Configuração de Energia: 500mA)
│    ├── acm.usb0 ----------> [ Link para Função 1: Serial ACM ]
│    ├── ecm.usb0 ----------> [ Link para Função 2: Rede CDC-ECM ]
│    └── ffs.usb0 ----------> [ Link para Função 3: FunctionFS Display ]
└── functions/
     ├── acm.usb0 (Terminal de depuração /dev/ttyGS0 -> /dev/ttyACM0 no PC)
     ├── ecm.usb0 (Rede de dados USB Ethernet 480 Mbps)
     └── ffs.usb0 (USB Bulk Direct montado em /dev/ffs-display)
```

### 2.1 Análise de Endpoints no Controlador DWC2 OTG
A controladora USB OTG Broadcom DWC2 do BCM2835 possui **8 endpoints de hardware** (EP0 de controle + 7 endpoints configuráveis IN/OUT):
* `acm.usb0`: Consome 2 endpoints (1 Bulk IN, 1 Bulk OUT) + 1 Interrupt IN.
* `ecm.usb0`: Consome 2 endpoints (1 Bulk IN, 1 Bulk OUT) + 1 Interrupt IN.
* `ffs.usb0`: Consome 2 endpoints (1 Bulk OUT para vídeo, 1 Bulk IN para retorno de controle).
* **Total:** Exatamente 7 endpoints alocados + EP0 de controle. O super-gadget opera 100% dentro dos limites do hardware de silício sem nenhum conflito ou colisão de barramento!

### 2.2 Descoberta Dinâmica de Endpoints FunctionFS e Prevenção de Conflitos
Em gadgets multifunção com CDC-ACM Serial e FunctionFS, endpoints físicos compartilham números de interface adjacentes.
No `sender/src/usb_transport.rs`, o transmissor implementa varredura inteligente por descritores USB:
1. Compatibilidade com duplo par VID/PID: aceita tanto o identificador experimental `1d50:614d` quanto o padrão Linux Foundation `1d6b:0104`.
2. Itera todas as interfaces do descritor de configuração ativa procurando especificamente por `interface.class_code() == 0xFF` (Vendor Specific / FunctionFS).
3. Reivindica a interface correspondente (`claim_interface`) e extrai o endpoint com direção `Direction::Out` e tipo `TransferType::Bulk`, garantindo que o fluxo de vídeo H.264 jamais colida com a interface serial CDC-ACM.

### 2.3 Auto-Binding UDC no Receptor
No `receiver/src/usb_bulk.rs`, a função `bind_udc_if_needed()` monitora `/sys/kernel/config/usb_gadget/g1/UDC` e vincula o controlador OTG `20980000.usb` imediatamente após a escrita dos descritores em `ep0`, eliminando janelas de corrida onde a USB reportava estado desconectado ao host.

---

## 3. Os Três Modos de Operação do Receptor

O binário `ext-receiver` em Rust mantém ouvintes ativos em três portas e subsistemas de forma concorrente:

```
                                [ ext-receiver ]
                                       │
        ┌──────────────────────────────┼──────────────────────────────┐
        ▼                              ▼                              ▼
  [ MODO 1: LINUX ]            [ MODO 2: WINDOWS ]             [ MODO 3: BULK ]
Canal: UDP Porta 5000       Canal: RTSP Porta 7236         Canal: USB FunctionFS
Protocolo: RTP H.264        Protocolo: Wi-Fi Display / WFD Protocolo: Raw Packet Stream
Cliente: ext-sender         Cliente: Windows Nativo (Win+K)Cliente: Driver USB Direto
Latência: < 15ms            Latência: ~30ms                Latência: < 1ms
```

---

### 3.1 Modo 1: Linux Display de Alta Performance (UDP 5000)
* **Público-Alvo:** Computadores rodando Linux com GNOME Wayland ou X11.
* **Mecanismo:** O host executa `./scripts/start.sh extend` ou o comando universal `connect.sh`. O fluxo de tela é capturado pelo PipeWire D-Bus Mutter e transmitido via RTP H.264 diretamente para o socket UDP `192.168.7.2:5000`.
* **Desempenho:** 60 FPS fluidos, resposta de cursor instantânea, suporte a hot-apply de taxa de bits (400 kbps a 6000 kbps) e alternância de paleta de cores (24-bit TrueColor, 256 cores, monocromático).

---

### 3.2 Modo 2: Windows Miracast Nativo sem Drivers (`Win + K`)
* **Público-Alvo:** Qualquer notebook ou desktop com Windows 10 ou Windows 11.
* **Mecanismo:**
  1. O usuário pressiona o atalho nativo do teclado: **`Win + K`** (Cast / Conectar).
  2. O Windows pesquisa telas sem fio na rede e localiza o **"ExtMonitor-Pi0"**.
  3. O módulo `wfd.rs` do receptor em Rust responde à negociação RTSP M1 a M7 na porta TCP 7236.
  4. O Windows passa a codificar sua tela secundária usando o codificador de hardware da placa de vídeo (Intel/AMD/Nvidia) e transmite pacotes MPEG-TS H.264 para o Raspberry Pi.
  5. O Pi Zero decodifica e exibe a imagem no HDMI **sem que o usuário precise instalar absolutamente nenhum software no Windows**!

---

### 3.3 Modo 3: USB Bulk Direct (FunctionFS / Raw Endpoints)
* **Público-Alvo:** Ambientes de computação de ultra-baixa latência (< 1ms no transporte) ou onde pilhas de rede locais/firewalls bloqueiem tráfego UDP.
* **Mecanismo:**
  1. A transferência de pacotes de vídeo ocorre diretamente através de chamadas assíncronas de escrita em endpoint USB 2.0 High-Speed Bulk OUT (`ep1` em `/dev/usb-ffs/display/ep1`).
  2. **Isolamento de Interfaces:** O transmissor `ext-sender` (`sender/src/usb_transport.rs`) inspeciona os descritores de configuração ativa filtrando estritamente pela classe `bInterfaceClass == 0xFF` (Vendor Specific / FunctionFS). Isso previne categoricamente que o fluxo de vídeo capture ou colida com as interfaces de controle do terminal serial CDC ACM (`/dev/ttyACM0`) ou da placa de rede CDC ECM (`usb0`).
  3. **Auto-Montagem e Ativação Dinâmica:** O receptor `ext-receiver` (`receiver/src/usb_bulk.rs`) conta com `ensure_functionfs_gadget()`, que verifica a existência de `/dev/usb-ffs/display/ep0` e cria dinamicamente o diretório e links no ConfigFS se necessário, escrevendo os descritores em `ep0` e vinculando o controlador UDC.
  4. **Modo de Boot Persistente (`mode.txt`):** Se o arquivo `/mnt/boot/mode.txt` contiver `usb-bulk` ou `3`, o script de inicialização do appliance (`/init`) passa automaticamente a flag `--mode=usb-bulk` para o `ext-receiver`, iniciando diretamente no Modo 3 desde o primeiro segundo de alimentação.
  5. **Comando de Teste no Host:**
     ```bash
     ./scripts/start.sh extend auto 30 false economy --transport=usb
     ```

---

## 4. Chaves de Controle e Persistência de Estado (F5)

O Web Dashboard (`http://192.168.7.2:8080`) disponibiliza interruptores do tipo toggle switch para isolamento granular de modos:
1. **Comutação Isolada e Concorrente:** Ao acionar a chave do Modo 3 (USB Bulk), o painel envia `POST /api/modes` com `mode3: true`. O receptor ativa a pipeline de decodificação `PipelineKind::UsbBulkPipe` sem derrubar a interface de rede USB `usb0` ou o servidor HTTP na porta 8080, permitindo monitorar o status do hardware e métricas em tempo real.
2. **Persistência Completa de Sessão:** Toda alteração de chave (Ligar/Desligar) ou perfil de cor (24-bit TrueColor vs 256 cores) é persistida em `localStorage` no navegador e transmitida via `POST /api/modes` e `POST /api/config`. Ao pressionar F5, a interface reidrata instantaneamente o estado exato configurado pelo usuário sem resetar para padrões.
3. **API REST para Automação:**
   * `POST /api/modes`: Comuta flags dos modos (`mode1`, `mode2`, `mode3`).
   * `POST /api/mode`: Endpoint direto para comutação rápida (`{"mode": "usb-bulk"}` ou `{"mode": "network"}`).
   * `POST /api/system/update`: Aciona atualização de imagem de firmware OTA (`initramfs.cpio.gz`) pela rede local com gravação direta no micro-SD e reboot seguro.

