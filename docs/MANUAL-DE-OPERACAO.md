# Manual de Operação Definitivo: ext-monitor
**Sistema de Segunda Tela HDMI via USB Hardware-Offloaded para Raspberry Pi Zero (BCM2835) e Host Linux/Windows**

*Autor: Carlos Alberto ([carlosalberto4ti@gmail.com](mailto:carlosalberto4ti@gmail.com) | [LinkedIn](https://www.linkedin.com/in/carlosalberto4ti) | [Blog](https://carloslopes.programandosolucoes.com.br))*  
*Versão da Documentação: 2.0.0 (Clean Rust & KMS DRM DMA-BUF Zero-Copy)*  
*Compatibilidade: Raspberry Pi Zero (v1.2, v1.3, W, Zero 2 W) | Ubuntu 24.04 / GNOME 46 Wayland | Windows 10/11*

---

## Sumário
1. [Visão Geral e Arquitetura de Hardware](#1-visão-geral-e-arquitetura-de-hardware)
2. [Conexão Física e Guia Anti-Erro](#2-conexão-física-e-guia-anti-erro)
3. [Os Três Modos de Operação](#3-os-três-modos-de-operação)
   - [Modo 1: Linux Wayland (UDP RTP Porta 5000 + Dashboard Web 8080)](#modo-1-linux-wayland-udp-rtp-porta-5000--dashboard-web-8080)
   - [Modo 2: Miracast / Wi-Fi Display (WFD RTSP Porta 7236)](#modo-2-miracast--wi-fi-display-wfd-rtsp-porta-7236)
   - [Modo 3: USB Bulk Direct (FunctionFS / Raw 480 Mbps)](#modo-3-usb-bulk-direct-functionfs--raw-480-mbps)
4. [Integração Completa no GNOME (Wayland & Mutter)](#4-integração-completa-no-gnome-wayland--mutter)
   - [Monitores Virtuais no Mutter](#monitores-virtuais-no-mutter)
   - [Captura PipeWire e Cursor Embutido](#captura-pipewire-e-cursor-embutido)
   - [Miracast Nativo no Linux via GNOME Network Displays](#miracast-nativo-no-linux-via-gnome-network-displays)
5. [Guia Completo de Scripts e Argumentos da CLI](#5-guia-completo-de-scripts-e-argumentos-da-cli)
   - [`scripts/start.sh` (Início de Transmissão e Parâmetros)](#scriptsstartsh-início-de-transmissão-e-parâmetros)
   - [`scripts/stop.sh` (Parada Limpa)](#scriptsstopsh-parada-limpa)
   - [`scripts/status.sh` (Diagnóstico de 4 Pontos)](#scriptsstatussh-diagnóstico-de-4-pontos)
   - [`scripts/connect.sh` (Script One-Click para Clientes)](#scriptsconnectsh-script-one-click-para-clientes)
   - [`scripts/switch-mode.sh` (Chaveador de Modos do Receptor)](#scriptsswitch-modesh-chaveador-de-modos-do-receptor)
   - [`scripts/monitor-latency.py` (Telemetria e Medição de Latência)](#scriptsmonitor-latencypy-telemetria-e-medição-de-latência)
   - [`scripts/setup-autoconnect.sh` (Plug & Play Automático via udev)](#scriptssetup-autoconnectsh-plug--play-automático-via-udev)
   - [`scripts/serial-console.sh` (Terminal de Resgate Serial /dev/ttyACM0)](#scriptsserial-consolesh-terminal-de-resgate-serial-devttyacm0)
   - [`scripts/reconfigure-appliance.sh` (Reconfiguração de Rede Sem Regravação)](#scriptsreconfigure-appliancesh-reconfiguração-de-rede-sem-regravação)
6. [Painel de Controle Web (Dashboard Telemetria 8080)](#6-painel-de-controle-web-dashboard-telemetria-8080)
7. [Instalação e Gravação Manual do Cartão Micro-SD](#7-instalação-e-gravação-manual-do-cartão-micro-sd)
8. [Desativação de Economia de Energia em Navegadores (Chrome e Firefox)](#8-desativação-de-economia-de-energia-em-navegadores-chrome-e-firefox)
9. [Diagnóstico e Solução de Problemas (Troubleshooting)](#9-diagnóstico-e-solução-de-problemas-troubleshooting)
10. [Subsistema de Áudio HDMI Digital (ext-audio)](#10-subsistema-de-áudio-hdmi-digital-ext-audio)

---

## 1. Visão Geral e Arquitetura de Hardware

O `ext-monitor` é uma solução de engenharia de ponta que converte um **Raspberry Pi Zero** em uma placa de captura e exibição de monitor secundário conectada ao PC por um único cabo Micro-USB comum:

```
[ HOST PC: Linux / Windows ]
       │
       │ Cabo Micro-USB OTG (5V Alimentação + Dados 480 Mbps)
       ▼
[ RASPBERRY PI ZERO (BCM2835 SoC / VideoCore IV VPU) ]
       │
       │ Cabo mini-HDMI -> HDMI
       ▼
[ MONITOR SECUNDÁRIO / TV (1280x720@60Hz ou 1600x900@30Hz) ]
```

### Princípios Chave de Performance:
- **Zero-Copy KMS DRM Scanout:** Os quadros decodificados pela GPU Broadcom (`/dev/video10` V4L2 M2M `bcm2835-codec`) são entregues como descritores DMA-BUF diretamente ao plano primário do KMS (`/dev/dri/card0`), sem conversão de espaço de cores na CPU ARM1176.
- **Carga de CPU no Pi Zero:** Menor que 1%.
- **Latência de Ponta a Ponta:** < 15ms em modo estendido.
- **Sistema 100% em RAM:** A imagem de 33MB descompacta o `initramfs.cpio.gz` na memória RAM. O cartão SD não sofre escritas e pode ser desligado puxando o cabo sem corrupção de sistema de arquivos.

---

## 2. Conexão Física e Guia Anti-Erro

O Raspberry Pi Zero possui duas portas Micro-USB idênticas lado a lado. A conexão correta é crucial:

```
                      [ Slot do Cartão Micro-SD ]
  +------------------------------------------------------------------+
  |                                 [ BCM2835 SoC ]                  |
  +-------[ mini-HDMI ]---------[ Micro-USB OTG ]-----[ PWR IN ]----+
                 │                      │                  │
                 │                      │                  └── [NUNCA CONECTAR!]
                 │                      │                      (Evite loop de terra e queima)
                 │                      │
                 │                      └── Conectar ao PC / Notebook
                 │                          (Fornece 5V + Enlace USB 480 Mbps)
                 │
                 └── Conectar ao Monitor Secundário ou TV
```

> [!CAUTION]
> **NUNCA conecte uma fonte de celular na porta "PWR IN" enquanto o Pi estiver conectado ao PC pela porta OTG!** A porta OTG central já fornece toda a energia necessária (~0.8W). Conectar duas fontes em paralelo causa sobretensão e pode danificar a porta USB do seu computador.

---

## 3. Os Três Modos de Operação

O receptor `ext-receiver` opera de forma híbrida e concorrente, oferecendo três modos distintos:

### Modo 1: Linux Wayland (UDP RTP Porta 5000 + Dashboard Web 8080)
- **Destinado a:** Linux (Ubuntu 24.04, Fedora, Arch, Debian) rodando GNOME Wayland ou KDE Plasma.
- **Protocolo:** H.264 empacotado em RTP sobre UDP porta 5000.
- **Latência:** Sub-15ms com decimação *Drop-on-Late*.
- **Como usar:**
  ```bash
  ./scripts/start.sh extend
  ```
  Ou em qualquer máquina cliente sem o repositório clonado:
  ```bash
  curl -sSL http://192.168.7.2:8080/connect.sh | bash
  ```

### Modo 2: Miracast / Wi-Fi Display (WFD RTSP Porta 7236)
- **Destinado a:**
  1. **Windows 10 e Windows 11:** Projeção nativa sem drivers.
  2. **Linux GNOME:** Através do aplicativo nativo `gnome-network-displays`.
- **Protocolo:** RTSP WFD (TCP 7236) + Streaming de Vídeo MPEG-TS sobre UDP (porta 5002).
- **Descoberta:** Anunciado automaticamente na rede local via Avahi mDNS (`_display._tcp.local`).
- **Como conectar no Windows:**
  1. Conecte o Pi Zero na porta USB do computador Windows.
  2. Pressione o atalho de teclado **`Win + K`**.
  3. No menu lateral "Transmitir" (*Cast*), clique em **"Raspberry Pi Miracast"** ou **"ExtMonitor-Pi0"**.
  4. Escolha entre: "Estender" (segunda tela de trabalho), "Duplicar" ou "Somente segunda tela".
- **Como conectar no Linux via GNOME:**
  1. Abra o aplicativo **GNOME Network Displays** (disponível no menu de aplicativos ou execute `gnome-network-displays` no terminal).
  2. O sink `Raspberry Pi Miracast` será listado automaticamente.
  3. Clique sobre ele para iniciar o espelhamento/projeção.

### Modo 3: USB Bulk Direct (FunctionFS / Raw 480 Mbps)
- **Destinado a:** Ambientes sem rede TCP/IP, corporativos ultra-restritos ou de latência absoluta (< 1ms no barramento).
- **Protocolo:** Transferência direta de pacotes brutos USB Bulk via endpoints `0x02` (OUT) e `0x82` (IN) usando `libusb`.
- **Como ativar no Pi Zero:**
  ```bash
  # Pelo terminal do Pi ou pelo Dashboard Web:
  ./scripts/switch-mode.sh usb-bulk
  ```
- **Como transmitir do Host:**
  ```bash
  ./scripts/start.sh extend auto 30 false full --transport=usb
  ```

---

## 4. Integração Completa no GNOME (Wayland & Mutter)

O `ext-monitor` integra-se diretamente com o compositor do GNOME (Mutter) sem emuladores virtuais lentos (como `evdi` ou `v4l2loopback`).

### Monitores Virtuais no Mutter
O GNOME Wayland gerencia monitores através da interface D-Bus `org.gnome.Mutter.DisplayConfig`.
O `ext-sender` solicita a criação de uma saída virtual (geralmente identificada como `HDMI-1` ou `Virtual-1`):
1. O monitor aparece imediatamente em **Configurações do GNOME -> Telas**.
2. Você pode arrastar a tela para o lado direito, esquerdo, superior ou inferior do seu monitor principal.
3. Resoluções suportadas nativamente: `1280x720@60Hz` (padrão de menor latência) e `1600x900@30Hz`.

### Captura PipeWire e Cursor Embutido
- O `ext-sender` cria uma sessão de ScreenCast via D-Bus `org.gnome.Mutter.ScreenCast`.
- A captura de vídeo é obtida diretamente do buffer gráfico do Mutter em formato DMA-BUF via PipeWire.
- **Cursor do Mouse:** O cursor é renderizado diretamente pelo hardware gráfico na ponta do transmissor (`cursor-mode=embedded`), garantindo sincronia perfeita sem o efeito de cursor fantasma.

### Miracast Nativo no Linux via GNOME Network Displays
Se preferir usar o protocolo padrão Wi-Fi Display no Linux sem linha de comando:
1. Certifique-se de que o pacote `gnome-network-displays` está instalado:
   ```bash
   sudo apt install -y gnome-network-displays
   ```
2. Abra o aplicativo:
   ```bash
   gnome-network-displays
   ```
3. O Raspberry Pi Zero será listado imediatamente como um receptor de exibição sem fio compatível.
4. Clique em "Conectar".

---

## 5. Guia Completo de Scripts e Argumentos da CLI

Todos os scripts de controle estão localizados no diretório `scripts/`. Abaixo está a especificação completa de cada um.

---

### `scripts/start.sh` (Início de Transmissão e Parâmetros)

Inicia o pipeline transmissor `ext-sender` no Host.

#### Sintaxe Geral:
```bash
./scripts/start.sh [MODO] [ENCODER] [FPS] [HUD] [PERFIL_COR] [FLAGS_ADICIONAIS...]
```

#### Argumentos Posicionais:
| Argumento | Opções | Padrão | Descrição |
| :--- | :--- | :--- | :--- |
| **`$1` - MODO** | `extend`, `mirror`, `clone` | `extend` | `extend`: cria área de trabalho estendida independente.<br>`mirror` / `clone`: espelha o monitor principal. |
| **`$2` - ENCODER** | `auto`, `vaapi`, `nvenc`, `qsv`, `cpu` | `auto` | `auto`: detecta GPU e escolhe melhor hardware.<br>`vaapi`: AMD Radeon ou Intel.<br>`nvenc`: NVIDIA GeForce.<br>`qsv`: Intel QuickSync.<br>`cpu`: codificação x264 via software. |
| **`$3` - FPS** | `15`, `30`, `60` | `30` | Taxa de atualização desejada (30 FPS recomendado para economia de banda; 60 FPS para máxima fluidez). |
| **`$4` - HUD** | `false`, `true`, `hud` | `false` | Se `true` ou `hud`, ativa overlay com métricas de telemetria na tela. |

#### Perfis de Cor e Flags Nomeadas:
- **`economy` / `256` / `--colors=256`:** Reduz a amostragem de cor e fixa o bitrate em 800 kbps (ótimo para texto e produtividade).
- **`gray` / `bw` / `--gray`:** Modo monocromático (preto e branco) para máxima legibilidade de código e menor uso de banda.
- **`full` / `truecolor` / `24bit`:** Modo RGB completo com alta fidelidade de cores.
- **`--bitrate=N` ou `-b=N`:** Define o bitrate alvo em kbps (ex: `--bitrate=400`, `--bitrate=1500`, `--bitrate=3000`).
- **`--ip=X.X.X.X`:** Define o IP de destino (padrão: `192.168.7.2`).
- **`--port=N`:** Define a porta UDP de destino (padrão: `5000`).
- **`--transport=usb` ou `--usb-bulk`:** Transmite via USB Bulk Direct (Modo 3) em vez de rede UDP.
- **`--drop-only`:** Descarta quadros duplicados quando a tela estiver estática, poupando a GPU e o barramento.
- **`--skip-to-first`:** Descarta pacotes até receber o primeiro IDR (Keyframe) completo.
- **`--key-int-max=N`:** Define o intervalo máximo entre quadros-chave H.264 (padrão: 30).

#### Exemplos Práticos de Uso:
```bash
# 1. Modo Estendido padrão (VA-API/AMD, 30 FPS, recomendação de ouro para notebook):
./scripts/start.sh extend

# 2. Modo Ultra-Rápido 60 FPS Contínuo (YouTube fluido, anti-congelamento padrão):
./scripts/start.sh extend vaapi 60 false full --bitrate=2500

# 3. Modo USB Bulk Direct Contínuo (sem rede IP, fluxo constante):
./scripts/start.sh extend auto 30 false full --transport=usb

# 4. Modo Economia Opcional (descarta estáticos, economiza até 95% de bateria):
./scripts/start.sh extend auto 30 false economy --economy --bitrate=400

# 5. Modo Espelhamento / Apresentação (Clone do monitor primário):
./scripts/start.sh clone auto 60
```

---

### `scripts/stop.sh` (Parada Limpa)

Finaliza o transmissor de vídeo e limpa as instâncias em execução no host.
```bash
./scripts/stop.sh
```
- Envia sinal `SIGTERM` ao `ext-sender`.
- O compositor Mutter detecta o encerramento do ScreenCast e destrói o monitor virtual de forma graciosa, retornando as janelas abertas para a tela primária sem travamentos.

---

### `scripts/status.sh` (Diagnóstico de 4 Pontos)

Verifica a saúde de todo o ecossistema em tempo real:
```bash
./scripts/status.sh
```
Executa 4 verificações consecutivas:
1. **Conexão USB-IP:** Testa se o Pi Zero responde ao ping ICMP em `192.168.7.2` com RTT < 1ms.
2. **Conector HDMI do Host:** Exibe o status da porta virtual no subsistema DRM do Linux.
3. **Processo `ext-sender` no Host:** Confirma se a GPU está ativamente codificando o stream.
4. **Daemon `ext-receiver` no Pi Zero:** Checa a resposta do serviço via SSH ou socket de controle.

---

### `scripts/connect.sh` (Script One-Click para Clientes)

Permite que qualquer máquina Linux conectada ao Pi Zero pela primeira vez se configure e comece a transmitir com um único comando sem precisar clonar o repositório git:
```bash
curl -sSL http://192.168.7.2:8080/connect.sh | bash
```
O script baixa o binário pré-compilado de `ext-sender` diretamente da memória do Pi Zero, instala as regras de permissão udev e inicia a segunda tela imediatamente.

---

### `scripts/switch-mode.sh` (Chaveador de Modos do Receptor)

Permite alternar a prioridade de entrada de vídeo no Raspberry Pi Zero.

#### Uso:
```bash
# Alterna para Modo 1 (Rede UDP + Miracast WFD + Dashboard):
./scripts/switch-mode.sh network

# Alterna para Modo 3 (USB Bulk Direct):
./scripts/switch-mode.sh usb-bulk

# Exibe o status atual dos 3 subsistemas (Serial, Rede, Vídeo):
./scripts/switch-mode.sh status
```

---

### `scripts/monitor-latency.py` (Telemetria e Medição de Latência)

Ferramenta interativa em Python para medição contínua de latência de ponta a ponta e taxa de transferência:
```bash
python3 scripts/monitor-latency.py
```
**O que é exibido:**
- RTT da camada física USB (Min / Méd / Máx / Desvio Padrão).
- Taxa de transmissão instantânea em kbps e MB/s na interface USB.
- Temperatura do chip Broadcom BCM2835 (°C).
- Carga de CPU e memória RAM livre no Pi Zero.
- Taxa de quadros decodificados pela VPU VideoCore IV.

---

### `scripts/setup-autoconnect.sh` (Plug & Play Automático via udev)

Configura o sistema operacional do Host para iniciar a extensão de tela automaticamente no instante em que o cabo USB do Pi Zero for inserido:
```bash
./scripts/setup-autoconnect.sh
```
- Instala a regra `/etc/udev/rules.d/99-ext-monitor.rules`.
- Habilita o serviço de usuário systemd `ext-monitor-watcher.service`.
- Otimiza o buffer de rede do kernel para `txqueuelen 100`, eliminando buffer-bloat.

---

### `scripts/serial-console.sh` (Terminal de Resgate Serial /dev/ttyACM0)

O gadget USB do Pi Zero expõe uma porta serial CDC-ACM nativa em `/dev/ttyACM0`. Se a rede falhar ou você estiver em uma máquina sem driver de rede USB:
```bash
./scripts/serial-console.sh
```
Abre uma sessão de terminal root direta no Raspberry Pi Zero a 115200 bps sem exigir SSH ou IP.

---

### `scripts/reconfigure-appliance.sh` (Reconfiguração de Rede Sem Regravação)

Permite alterar as configurações de IP estático, subnet ou credenciais de Wi-Fi salvas no cartão do Pi Zero remotamente ou via montagem local do `/boot`.
```bash
./scripts/reconfigure-appliance.sh --ip 192.168.7.2 --netmask 255.255.255.0
```

---

## 6. Painel de Controle Web (Dashboard Telemetria 8080)

O Pi Zero roda um servidor web nativo de alta performance escrito em Rust embutido no `ext-receiver`. Para acessá-lo, abra no seu navegador:

👉 **`http://192.168.7.2:8080`**

### Recursos Disponíveis no Dashboard:
1. **Aba Telemetria:** Indicadores gráficos de temperatura do SoC, uso de RAM, FPS de decodificação e switches individuais para ligar/desligar Modo 1, Modo 2 ou Modo 3.
2. **Aba Stream & Ajustes:** Sliders em tempo real para ajuste de Bitrate (150 kbps a 15 Mbps), seletor de FPS (15, 30, 60), troca dinâmica de perfis de cor e botão de pausa.
3. **Aba Downloads de Clientes:** Baixe os binários compilados `ext-sender` para Linux x86_64, scripts de conexão e pacote portátil.
4. **Aba SD Card & Firmware Upgrade:** Permite montar a partição `/boot` com um clique via USB e fazer upload de novos binários do receptor sem retirar o cartão micro-SD do Pi Zero!

---

## 7. Instalação e Gravação Manual do Cartão Micro-SD

A imagem do appliance foi reduzida a **33 MB** e utiliza uma partição FAT16 especial alinhada no **Setor 1** para máxima compatibilidade com a Boot ROM do BCM2835.

### Como Gravar Manualmente via Terminal:

1. **Identifique a unidade do cartão Micro-SD:**
   Insira o cartão no leitor USB do seu computador e execute:
   ```bash
   lsblk
   ```
   *Exemplo: o cartão aparece como `/dev/sdc` (com partição `sdc1`). Certifique-se de usar a letra correta da unidade e NÃO o seu SSD principal (`/dev/sda` ou `/dev/nvme0n1`).*

2. **Desmonte as partições do cartão:**
   ```bash
   sudo umount /dev/sdc* 2>/dev/null || true
   ```

3. **Grave a imagem do appliance usando `dd`:**
   ```bash
   # A partir da raiz do projeto ext-monitor:
   sudo dd if=build-appliance/ext-monitor-pi0-appliance.img of=/dev/sdc bs=4M status=progress conv=fsync
   ```

4. **Sincronize e descarregue o cache:**
   ```bash
   sudo sync
   ```

5. **Pronto para Uso:**
   Retire o cartão do computador, insira-o no slot do Raspberry Pi Zero, conecte o cabo mini-HDMI ao monitor e o cabo Micro-USB central ao PC. O Pi Zero iniciará em aproximadamente **1.8 segundos**.

---

## 8. Desativação de Economia de Energia em Navegadores (Chrome e Firefox)

Para evitar que navegadores descartem abas inativas (Tab Discarding), diminuam o FPS de animações ou congelem vídeos rodando na segunda tela, foram configuradas políticas corporativas definitivas no sistema:

### Google Chrome (Políticas Globais de Sistema)
Arquivo de política instalado em: `/etc/opt/chrome/policies/managed/performance.json`
- `HighEfficiencyModeEnabled`: **`false`** (Desativa totalmente o Memory Saver / Economia de Memória).
- `BatterySaverModeAvailability`: **`0`** (Desativa o modo de economia de energia e redução de taxa de quadros).
- `IntensiveWakeUpThrottlingEnabled`: **`false`** (Impede o estrangulamento de timers JavaScript em abas em segundo plano).
- `BackgroundModeEnabled`: **`true`** (Mantém execução em segundo plano ativa).

### Mozilla Firefox (Políticas Globais e Profile Locking)
Arquivos configurados em: `/etc/firefox/policies/policies.json` e `~/snap/firefox/common/.mozilla/firefox/*.default/user.js`
- `browser.tabs.unloadOnLowMemory`: **`false`** (Bloqueia o descarte forçado de abas da memória RAM).
- `dom.suspend_inactive.enabled`: **`false`** (Impede o congelamento de renderização de abas inativas).
- `media.suspend-bkgnd-video.enabled`: **`false`** (Impede o congelamento ou pausa de vídeos ao trocar de janela).
- `dom.min_background_timeout_value`: **`4`** (Executa timers de abas secundárias na mesma frequência de abas ativas).
- `browser.low_commit_space_threshold_mb`: **`0`** (Desativa gatilhos de corte de uso de memória).

---

## 9. Diagnóstico e Solução de Problemas (Troubleshooting)

### P: O monitor secundário fica preto ou sem sinal ao ligar o Pi Zero.
- **R:** Verifique se o monitor HDMI estava ligado antes ou durante o boot do Pi Zero. O BCM2835 lê o EDID na inicialização. No arquivo `/boot/config.txt`, a diretiva `hdmi_force_hotplug=1` garante que o sinal seja gerado mesmo sem leitura de EDID.

### P: Apareceu o erro `Bad address (os error 14)` no log do KMS.
- **R:** Esse erro ocorria quando o Linux DRM ioctl `DRM_IOCTL_MODE_GETRESOURCES` recebia ponteiros nulos na segunda chamada. A versão atual do `ext-receiver` aloca todos os arrays previamente com `DRM_CLIENT_CAP_UNIVERSAL_PLANES = 2`, eliminando essa falha.

### P: O Windows não encontra o Pi Zero ao pressionar `Win + K`.
- **R:** Certifique-se de que a interface de rede do Pi Zero no Windows obteve o IP `192.168.7.1` (o Pi atribui via DHCP integrado). O serviço Miracast WFD escuta na porta TCP 7236. Se o firewall do Windows perguntar, permita o tráfego de rede para a rede privada.

### P: Como alternar a tela para o lado esquerdo no Ubuntu?
- **R:** Abra **Configurações -> Telas**. Você verá duas telas retangulares: uma com o número 1 e outra com o número 2. Clique e arraste o retângulo do monitor secundário para a esquerda do primário e clique no botão verde "Aplicar".

### P: O Pi Zero pode ser desligado puxando o cabo USB diretamente?
- **R:** **Sim!** O `ext-monitor` roda 100% na memória RAM. O cartão Micro-SD é montado em modo somente-leitura e desacoplado logo após o carregamento do `initramfs`. Não há risco de quebra de partição ou perda de dados.


---

## 10. Subsistema de Áudio HDMI Digital (ext-audio)

O `ext-monitor` incorpora transmissão e reprodução de **áudio digital estéreo em alta fidelidade** diretamente para os alto-falantes da TV ou monitor secundário via cabo HDMI do Raspberry Pi Zero, com sincronismo de lábios (lip-sync) perfeito e latência < 25ms.

### 10.1 Arquitetura de Transmissão e Codec Opus
* **Captura no Host:** Através da infraestrutura moderna do **PipeWire** (`pipewiresrc client-name=ext-hdmi-audio`), capturando o áudio do sistema com `do-timestamp=true`.
* **Codificação:** **Opus 48.000 Hz Estéreo**, taxa de bits de 96 kbps, com tamanho de quadro de **10 ms** (algoritmo de latência ultrabaixa).
* **Porta de Rede Exclusiva:** **UDP 5004** (RTP payload type 96). O canal de áudio opera em porta segregada para não competir nem sofrer *head-of-line blocking* dos quadros de vídeo H.264 (UDP 5000).

### 10.2 Reprodução no Hardware do Pi Zero (ALSA vc4-hdmi)
* O kernel do appliance inicializa o driver ALSA `snd-soc-hdmi-codec` / `vc4-hdmi` através do parâmetro `dtparam=audio=on` no `config.txt`.
* O `ext-receiver` executa um worker de áudio em tempo real com desmultiplexador e decodificador Opus integrado diretamente ao dispositivo ALSA (`hw:0,0` ou `default`), com chaveamento de segurança para `autoaudiosink` em ambientes virtuais/QEMU.

### 10.3 Argumentos CLI no Host
```bash
# Transmissão completa com Áudio HDMI ativo (Padrão):
./scripts/start.sh extend auto 30 false full

# Transmitir apenas vídeo (desativar envio de áudio):
./scripts/start.sh extend auto 30 false full --no-audio

# Especificar porta personalizada para o canal de áudio:
./scripts/start.sh extend auto 30 false full --audio-port=5004
```

### 10.4 Controle Remoto de Volume e Silenciamento (REST API)
O painel web (`http://192.168.7.2:8080`) e aplicativos externos podem controlar o som dinamicamente:
* **Consultar Status:**
  ```bash
  curl http://192.168.7.2:8080/api/audio/status
  # Resposta: {"enabled":true,"active":true,"volume":100,"muted":false,"port":5004}
  ```
* **Alterar Volume (0 a 100%):**
  ```bash
  curl -X POST -H "Content-Type: application/json" -d '{"volume": 85}' http://192.168.7.2:8080/api/audio/volume
  ```
* **Silenciar (Mudo) ou Desmutar:**
  ```bash
  curl -X POST -H "Content-Type: application/json" -d '{"muted": true}' http://192.168.7.2:8080/api/audio/mute
  ```

### 10.5 Isolamento Anti-Hijack (WirePlumber), Supressão de Silêncio e Fast Cutoff (<800ms)
* **Proteção Anti-Hijack:** Tanto o streamer de áudio Opus quanto o monitor de espectro FFT rodam com `PULSE_PROP="stream.dont-route=true node.dont-reconnect=true"`. Isso bloqueia o WirePlumber do PipeWire de desviar o monitor para fones de ouvido ou caixas locais quando a saída do GNOME é comutada.
* **Zero Pacotes em Silêncio:** Quando o sinal acústico cai abaixo de -55 dB, a emissão de pacotes UDP na porta 5006 é suspensa totalmente (0 pacotes/s).
* **Restauração Imediata da Tela Pronta:** Se o áudio cessar por mais de 800ms, o visualizador é desligado e o appliance restaura imediatamente a tela oficial multilíngue `SplashEngine::show_ready()` sem telas pretas.
* **Web UI Transparente:** O canvas Web UI não utiliza ondas simuladas; em repouso, as barras e medidores VU permanecem em zero absoluto.

---

## 11. Catálogo Exaustivo de Todos os Scripts (`scripts/`)

O projeto dispõe de 33 scripts especializados para automação, deploy, telemetria e controle de hardware:

| Script | Função Principal | Sintaxe de Uso |
| :--- | :--- | :--- |
| **`start.sh`** | Inicializa o emissor `ext-sender` no host com seleção de modo, resolução, FPS, encoder e áudio. Auto-inicia o Damage Pacer no Wayland. | `./scripts/start.sh [extend\|mirror] [auto\|vaapi] [30\|60] [hud] [full\|256]` |
| **`stop.sh`** | Finaliza todos os processos do `ext-sender`, streamers de áudio, pipelines GStreamer e o daemon `wayland-damage-pacer`. | `./scripts/stop.sh` |
| **`status.sh`** | Executa verificação em 4 pontos: interface de rede USB, conectividade IP `192.168.7.2`, portas abertas e resposta do dashboard. | `./scripts/status.sh` |
| **`connect.sh`** | Script de download e inicialização rápida para máquinas clientes (utilizado pelo endpoint `curl \| bash`). | `./scripts/connect.sh` |
| **`install-host.sh`** | Instalador do sistema Host Linux: compilação, `setcap cap_sys_admin`, drivers gráficos VA-API, regras udev e atalho no menu. | `sudo ./scripts/install-host.sh` |
| **`wayland-damage-pacer.py`** | Daemon Wayland/Xwayland invisível que emite pulso de dano a 60 Hz no monitor estendido, impedindo que o GNOME Mutter durma. | `python3 scripts/wayland-damage-pacer.py [X] [Y]` |
| **`setup-autoconnect.sh`** | Configura serviço udev e systemd para iniciar a transmissão automaticamente ao plugar o cabo Micro-USB. | `./scripts/setup-autoconnect.sh` |
| **`autoconnect.sh`** | Script disparado pela regra udev na conexão do Pi Zero, iniciando o streaming após aguardar a subida da rede. | `./scripts/autoconnect.sh` |
| **`usb-watcher.sh`** | Monitor de barramento USB em background que reinicia o pipeline caso a porta USB caia ou seja reconectada. | `./scripts/usb-watcher.sh` |
| **`switch-mode.sh`** | Alterna o modo de operação do receptor entre Modo 1 (UDP), Modo 2 (Miracast) e Modo 3 (USB Bulk). | `./scripts/switch-mode.sh [udp\|miracast\|usb-bulk]` |
| **`flash-appliance.sh`** | Grava a imagem compacta `.img` do appliance no cartão Micro-SD via `dd` com checagem de integridade. | `sudo ./scripts/flash-appliance.sh /dev/sdX` |
| **`build-fast-appliance.sh`** | Constrói a imagem do appliance `initramfs.cpio.gz` a partir da árvore do buildroot e overlays. | `./scripts/build-fast-appliance.sh` |
| **`reconfigure-appliance.sh`** | Monta a partição FAT16 do cartão SD para alterar IPs, modo de boot ou Wi-Fi sem regravar todo o sistema. | `sudo ./scripts/reconfigure-appliance.sh [opções]` |
| **`serial-console.sh`** | Conecta instantaneamente ao console serial CDC-ACM (`/dev/ttyACM0`) do Pi Zero para diagnóstico root sem rede. | `./scripts/serial-console.sh` |
| **`monitor-latency.py`** | Mede latência fim-a-fim e jitter através de marcação de timestamp na imagem capturada. | `python3 scripts/monitor-latency.py` |
| **`hud-pi.sh`** | Liga ou desliga remotamente o HUD de telemetria na tela do Pi Zero enviando comando UDP para a porta 5001. | `./scripts/hud-pi.sh [on\|off]` |
| **`scan-pis.sh`** | Varre a rede local em busca de dispositivos Raspberry Pi Zero ativos na subnet. | `./scripts/scan-pis.sh` |
| **`launch-browser.sh`** | Inicia navegadores (Chrome/Firefox) com aceleração gráfica ativa e políticas anti-suspensão de abas. | `./scripts/launch-browser.sh [chrome\|firefox] [url]` |
| **`test-wfd-client.py`** | Simula um cliente Miracast negociando parâmetros RTSP com o servidor do Pi Zero. | `python3 scripts/test-wfd-client.py 192.168.7.2` |
| **`test_render_frame.py`** | Validador de teste de decodificação H.264 local com GStreamer e frames sintéticos. | `python3 scripts/test_render_frame.py` |
| **`qemu-run-pi0.sh`** | Executa o kernel e initramfs do appliance dentro do emulador QEMU ARMv6 para testes sem hardware físico. | `./scripts/qemu-run-pi0.sh` |
| **`setup-composite-gadget.sh`** | Script interno do initramfs que cria o gadget USB 3-em-1 (ACM Serial + NCM Rede + FunctionFS). | `./scripts/setup-composite-gadget.sh` |
| **`setup-usb-bulk.sh`** | Configura os descritores de endpoint USB FunctionFS para o modo Bulk de 480 Mbps. | `./scripts/setup-usb-bulk.sh` |
| **`appliance-init.sh`** | Script raiz de inicialização do appliance no boot (montagem de `/proc`, `/sys`, drivers e rede). | `Executado pelo initramfs` |
| **`show-splash.sh`** | Renderiza imediatamente uma imagem raw no framebuffer `/dev/fb0` do Pi Zero. | `sudo ./scripts/show-splash.sh /etc/splash_ready.raw.gz` |
| **`show-welcome-window.py`** | Janela gráfica de notificação pop-up informando que o monitor estendido está ativo e pronto. | `python3 scripts/show-welcome-window.py` |
| **`generate_splash.py`** | Gera e compila as imagens de splash quadrilíngue nos formatos PNG e RAW compactado RGB565. | `python3 scripts/generate_splash.py` |
| **`generate_demo_gif.py`** | Captura e gera animações GIF de demonstração de baixa taxa para documentação. | `python3 scripts/generate_demo_gif.py` |
| **`deploy-receiver.sh`** | Atualiza remotamente o binário `ext-receiver` no Raspberry Pi via scp/curl sem retirar o cartão SD. | `./scripts/deploy-receiver.sh 192.168.7.2` |
| **`99-ext-monitor.rules`** | Regra udev do Host que detecta o Vendor `0x1d50` Product `0x614d`, configura permissões e fila `txqueuelen 100`. | `/etc/udev/rules.d/99-ext-monitor.rules` |
| **`ext-monitor-autoconnect.service`** | Serviço systemd acionado na inserção do cabo USB para conexão do display. | `systemctl enable ext-monitor-autoconnect` |
| **`ext-monitor-watcher.service`** | Serviço systemd de monitoramento contínuo em segundo plano. | `systemctl enable ext-monitor-watcher` |
| **`miracast.service`** | Serviço systemd para inicialização do daemon de Miracast no receptor. | `systemctl enable miracast` |

---

## 12. Guia de Compilação para Raspberry Pi Zero (ARMv6)

O SoC Broadcom BCM2835 (utilizado no Raspberry Pi Zero v1.2, v1.3 e W) possui arquitetura **ARMv6 (ARM1176JZF-S com VFPv2)**. Compilações normais em computadores x86_64 geram binários incompatíveis.

### Método Recomendado: Usando `cross` (Docker)
```bash
# 1. Instalar ferramenta cross:
cargo install cross --git https://github.com/cross-rs/cross

# 2. Compilar ext-receiver em modo estático Musl (independente de versão de libc):
cd ~/ide/ext-monitor/receiver
cross build --target arm-unknown-linux-musleabihf --release

# 3. O binário final estará em:
# target/arm-unknown-linux-musleabihf/release/ext-receiver
```

### Método com Toolchain GNU Nativo:
```bash
# 1. Instalar cross-compiler no Ubuntu/Debian:
sudo apt-get install -y gcc-arm-linux-gnueabihf g++-arm-linux-gnueabihf

# 2. Adicionar target do Rust:
rustup target add arm-unknown-linux-gnueabihf

# 3. Configurar linker no ~/.cargo/config.toml:
cat << 'EOF' >> ~/.cargo/config.toml
[target.arm-unknown-linux-gnueabihf]
linker = "arm-linux-gnueabihf-gcc"
EOF

# 4. Compilar:
cargo build --target arm-unknown-linux-gnueabihf --release
```

---

## 13. Guia de Instalação em Modelos Raspberry Pi Não-OTG (Pi 2, 3, 4, 5)

Os modelos **Raspberry Pi 2, 3, 4B e 5** não suportam modo USB Gadget OTG nas suas portas USB comuns (controladas por chips VL805 ou hubs PCIe). Nesses modelos, o transporte é feito via **Cabo de Rede Ethernet ou Wi-Fi**.

### 13.1 Ajuste Obrigatório dos Parâmetros de Boot no Cartão SD

Ao gravar a imagem no cartão SD desses modelos, abra o cartão no seu computador e edite:

#### 1. Arquivo `config.txt`:
Comente ou remova o overlay do driver OTG `dwc2`:
```ini
# DESATIVAR DWC2 GADGET (não suportado em portas USB tipo A normais):
# dtoverlay=dwc2

# ATIVAR CONTROLADOR DE DISPLAY NATIVO:
dtoverlay=vc4-kms-v3d,cma-128
dtparam=audio=on
hdmi_force_hotplug=1
```

#### 2. Arquivo `cmdline.txt`:
Remova o parâmetro `modules-load=dwc2` da linha de comando do kernel:
```text
console=tty3 quiet loglevel=0 vt.global_cursor_default=0 ip=dhcp
```

### 13.2 Executando e Conectando pela Rede Local:
1. Ligue o Raspberry Pi 3/4/5 no roteador via cabo Ethernet ou configure Wi-Fi.
2. Descubra o IP atribuído a ele pelo roteador (ex: `192.168.1.150`).
3. No Host Linux, inicie a transmissão apontando para o IP do Pi:
   ```bash
   ./scripts/start.sh extend auto 60 false full 192.168.1.150:5000
   ```

---

## 14. Guia: Transformar um PC / Laptop Convencional em Segunda Tela

Você pode reaproveitar qualquer computador velho ou notebook com Linux como uma segunda tela externa de alto desempenho.

### 14.1 No Computador Receptor (PC Secundário):
Instale os pacotes mínimos do GStreamer:
```bash
sudo apt-get update && sudo apt-get install -y gstreamer1.0-tools gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-libav
```

Em seguida, execute o pipeline de recepção em tela cheia:
```bash
gst-launch-1.0 -v udpsrc port=5000 buffer-size=524288 \
    caps="application/x-rtp,media=video,clock-rate=90000,encoding-name=H264,payload=96" ! \
    rtph264depay ! h264parse ! avdec_h264 ! autovideosink sync=false
```

### 14.2 Como Direcionar a Transmissão para a Tela 1 ou 2 em PCs com Múltiplos Monitores

Se o computador receptor possui mais de um monitor conectado (ex: Monitor 1 tela interna e Monitor 2 HDMI externo) e você quer projetar exclusivamente na Tela 2:

#### Opção A: No Terminal Direto KMS DRM (`kmssink`)
```bash
# 1. Liste os conectores conectados no PC receptor:
modetest -c | grep -E "id|name|status"

# 2. Execute passando o conector exato (exemplo: connector-id=45 correspondente ao HDMI):
gst-launch-1.0 udpsrc port=5000 caps="application/x-rtp,media=video,clock-rate=90000,encoding-name=H264,payload=96" ! \
    rtph264depay ! h264parse ! avdec_h264 ! \
    kmssink connector-id=45 sync=false
```

#### Opção B: Em Sessão Gráfica com `ffplay` Posicionado
```bash
# Posiciona a janela em tela cheia na coordenada do segundo monitor (ex: x=1920):
ffplay -left 1920 -top 0 -x 1920 -y 1080 -fs -flags low_delay -framedrop rtp://0.0.0.0:5000
```

---

## 15. O Pacer Wayland de 60 Hz e Prevenção Definitiva de Quiescência

O **GNOME Mutter no Wayland** utiliza uma arquitetura estritamente orientada a danos (*damage-driven rendering*). Quando não há movimento de janelas ou quando o cursor do mouse não está sobre o monitor secundário, o compositor entra em dormência e suspende a gravação de frames no PipeWire.

### A Solução Definitiva do ext-monitor:
O mecanismo de pacing integrado no `ext-sender`:
1. Cria uma janela invisível de 1x1 pixel com transparência total (`RGBA 0,0,0,0`), sem foco e sem barra de tarefas.
2. Posiciona essa janela no canto inferior direito da tela estendida (ex: `x=3518, y=898`).
3. **Máscara 100% Click-Through (`cairo.Region()` vazia):** A janela aplica `gdk_win.input_shape_combine_region(cairo.Region(), 0, 0)`. Isso garante que cliques em logotipos, barras de endereço, abas ou botões (como o ícone inicial do YouTube no Firefox) nunca sejam interceptados.
4. Dispara um pulso de redesenho a cada 16.6 ms (60 Hz).
5. O Mutter é forçado a manter o `ClutterFrameClock` ativo e transmitir a 60 FPS contínuos.
6. **Resultado:** Vídeos em navegadores (YouTube, streaming), dashboards, terminais e relógios rodam com fluidez absoluta e contínua, sem pausar quando o mouse está parado ou sobre a tela do notebook.

---

## 16. Alternativas ao GStreamer em Ambientes Não-GNOME (KDE, XFCE, i3, Windows, macOS)

O fluxo de vídeo transmitido pelo `ext-sender` é composto por pacotes **RTP H.264 padrão RFC 4571** com payload 96. Ele não tem dependência com o GNOME ou com o GStreamer na ponta receptora.

### 16.1 FFmpeg / ffplay (Universal, Ultra-baixo atraso, Multiplataforma)
O `ffplay` está disponível em qualquer distribuição Linux, Windows e macOS:

#### Comando com Zero Buffer e Descarte de Quadros Atrasados:
```bash
ffplay -fflags nobuffer -flags low_delay -framedrop -strict experimental \
       -an -sn -sync ext -protocol_whitelist file,udp,rtp \
       -i rtp://0.0.0.0:5000
```

#### Com Aceleração de Hardware na GPU (Intel/AMD VA-API no Linux):
```bash
ffplay -vcodec h264_vaapi -hwaccel vaapi -hwaccel_device /dev/dri/renderD128 \
       -fflags nobuffer -flags low_delay -framedrop -an \
       -protocol_whitelist file,udp,rtp -i rtp://0.0.0.0:5000
```

#### Direcionar para a Tela 1 ou Tela 2 no FFmpeg:
```bash
# Monitor 1 (Primário):
ffplay -left 0 -top 0 -fs -fflags nobuffer -flags low_delay -protocol_whitelist file,udp,rtp rtp://0.0.0.0:5000

# Monitor 2 (Secundário à direita do primário a 1920x0):
ffplay -left 1920 -top 0 -fs -fflags nobuffer -flags low_delay -protocol_whitelist file,udp,rtp rtp://0.0.0.0:5000
```

### 16.2 MPV Player (Excelente sincronismo, OpenGL/Vulkan nativo)
```bash
# Execução com latência mínima:
mpv --no-cache --untimed --no-correct-pts --fps=60 --profile=low-latency --hwdec=auto rtp://0.0.0.0:5000

# Seleção direta de tela no MPV:
mpv --fs --screen=1 --profile=low-latency rtp://0.0.0.0:5000
```

### 16.3 VLC Player
```bash
cvlc --network-caching=0 --clock-jitter=0 --no-audio rtp://@:5000
```

---

## 17. Modo Padrão USB Bulk Direto com Fallback Automático para Rede

A partir da versão v2.1.0, o `ext-monitor` prioriza o canal USB direto de mais alta velocidade:
1. **Padrão (Default):** Conexão direta via **USB Bulk** (`1d50:614d`). A imagem do Pi Zero vem pré-configurada em `/boot/mode.txt` com `usb-bulk`.
2. **Fallback Automático:** Caso o dispositivo USB Bulk não seja encontrado (ex: cabo conectado em porta sem dados, ou conectando a um receptor via Wi-Fi/Ethernet como Pi 4 ou PC secundário), o `ext-sender` detecta a ausência e comuta automaticamente e em tempo de execução para **Rede UDP** (`192.168.7.2:5000` ou IP especificado), sem interrupções.


