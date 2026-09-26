# Blueprint 06: Modos Concorrentes de Operação e o Super-Gadget USB ConfigFS

**Projeto:** `ext-monitor`  
**Autor:** Carlos Alberto & Antigravity  
**Arquivos de Referência:** `build-appliance/initramfs/init`, `receiver/src/wfd.rs`, `receiver/src/pipeline.rs`  
**Data:** Setembro de 2026  

---

## 1. Visão Geral

Um dos maiores diferenciais técnicos do `ext-monitor` é a sua capacidade de operar de forma **universal e concorrente**:
* Um único dispositivo Raspberry Pi Zero conectado a qualquer computador (seja Linux, Windows ou Mac) disponibiliza simultaneamente os serviços de rede, terminal de controle serial e unidade de disco para atualização.
* O firmware em Rust escuta simultaneamente nos canais de transmissão do **Linux (UDP)** e do **Windows Miracast (RTSP/WFD)**, permitindo alternância instantânea de computadores sem necessidade de reiniciar o appliance.

---

## 2. A Arquitetura do Super-Gadget USB ConfigFS

O Linux USB Gadget Subsystem é configurado no boot pelo script `/init` utilizando a interface de sistema de arquivos virtual **ConfigFS** (`/sys/kernel/config/usb_gadget/`):

```
/sys/kernel/config/usb_gadget/ext_composite/
├── idVendor (0x1d50 - Openmoko / Experimental)
├── idProduct (0x614d - Multifunction Hub)
├── strings/0x409/ (Fabricante: Raspberry Pi | Produto: High-Speed Display Hub)
├── configs/c.1/ (Configuração de Energia: 500mA)
│    ├── acm.usb0 ----------> [ Link para Função 1: Serial ACM ]
│    ├── ecm.usb0 ----------> [ Link para Função 2: Rede CDC-ECM ]
│    └── mass_storage.0 ----> [ Link para Função 3: Disco Removível SD ]
└── functions/
     ├── acm.usb0 (Terminal de depuração /dev/ttyGS0 -> /dev/ttyACM0 no PC)
     ├── ecm.usb0 (Rede de dados USB Ethernet 480 Mbps)
     └── mass_storage.0 (Partição FAT16 /dev/mmcblk0p1 exportada como pendrive)
```

### 2.1 Análise de Endpoints no Controlador DWC2 OTG
A controladora USB OTG Broadcom DWC2 do BCM2835 possui **8 endpoints de hardware** (EP0 de controle + 7 endpoints configuráveis IN/OUT):
* `acm.usb0`: Consome 2 endpoints (1 Bulk IN, 1 Bulk OUT) + 1 Interrupt IN.
* `ecm.usb0`: Consome 2 endpoints (1 Bulk IN, 1 Bulk OUT) + 1 Interrupt IN.
* `mass_storage.0`: Consome 2 endpoints (1 Bulk IN, 1 Bulk OUT).
* **Total:** Exatamente 7 endpoints alocados + EP0 de controle. O super-gadget opera 100% dentro dos limites do hardware de silício sem nenhum conflito ou colisão de barramento!

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
Latência: < 18ms            Latência: ~35ms                Latência: < 15ms
```

---

### 3.1 Modo 1: Linux Display de Alta Performance (UDP 5000)
* **Público-Alvo:** Computadores rodando Linux com GNOME Wayland ou X11.
* **Mecanismo:** O host executa o script `./scripts/start.sh extend` ou o comando universal `connect.sh`. O fluxo de tela é capturado pelo PipeWire e transmitido via RTP H.264 diretamente para o socket UDP `192.168.7.2:5000`.
* **Desempenho:** 60 FPS fluidos, resposta de cursor instantânea, suporte a hot-apply de taxa de bits (400 kbps a 6000 kbps) via Web Dashboard.

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
* **Público-Alvo:** Ambientes de computação embarcada ou redes corporativas restritas onde pilhas TCP/IP ou firewalls bloqueiam tráfego UDP local.
* **Mecanismo:** A transferência de pacotes de vídeo ocorre diretamente através de chamadas de leitura e escrita nos arquivos de dispositivo do USB Gadget FunctionFS (`/dev/ffs-display/ep1` e `ep2`), contornando por completo a pilha de rede do sistema operacional.

---

## 4. Gerenciamento e Transição de Modos em Tempo Real

O receptor implementa uma máquina de estados com transição atômica gerenciada pelo `PipelineManager`:
* Se o receptor estiver ocioso exibindo a tela de boas-vindas e pacotes chegarem na porta UDP 5000, o Modo 1 é ativado imediatamente.
* Se um computador Windows iniciar uma negociação Miracast via RTSP 7236, o receptor pausa o ouvinte UDP e comuta a decodificação para o canal WFD.
* Ao desconectar, o receptor retorna automaticamente ao estado de espera, pronto para a próxima conexão sem necessidade de reboot.
