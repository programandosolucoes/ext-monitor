# Blueprint 16: Pacer Wayland de Quiescência, Compilação Cross-ARMv6 e Receptores Universais (Pi Não-OTG e PC Secundário)

*Data: 2026-09-29*  
*Status: Aprovado em Produção e Validado com YouTube 60 FPS Sem Interrupção*  
*Autor: Carlos Alberto <psncarlosalberto4ti@gmail.com>*  

---

## 1. O Problema da Quiescência do GNOME Mutter e a Solução Damage Pacer

### O Fenômeno Físico
No GNOME Wayland (Mutter), o compositor desliga o ciclo de pintura (`stage_painted`) quando nenhuma região do monitor sofre dano (*damage-driven rendering*).
Isso fazia com que:
1. O mouse parado ou fora da tela secundária parasse a geração de frames no PipeWire (0 FPS).
2. Vídeos, animações de navegadores (como YouTube) ou tarefas em segundo plano pausassem na tela secundária assim que o cursor deixava a geometria do monitor.
3. O mesmo sintoma ocorre nativamente no Miracast oficial do GNOME (`gnome-network-displays`).

### A Solução: Wayland Damage Pacer (`scripts/wayland-damage-pacer.py`)
Em vez de hacks de duplicação na CPU (como `imagefreeze`, que destruía o zero-copy e inflava a latência para > 200ms), implementamos um **agente de batimento cardíaco gráfico sintético**:
- Cria uma superfície invisível de 1x1 pixel 100% transparente (`RGBA 0.0, 0.0, 0.0, 0.0`), sem bordas, sem foco (`accept_focus = false`) e com `GDK_BACKEND=x11` para mapeamento exato nas coordenadas globais da tela estendida (`x=1920, y=0`).
- A cada 16.6 ms (60 Hz), emite um pulso de dano (`queue_draw`).
- O Mutter detecta a região suja e é forçado a executar `clutter_stage_schedule_update()` -> `stage_painted()` a 60 FPS ininterruptos.
- **Resultado:** Vídeos do YouTube, clocks e terminais rodam a 60 FPS cravados e ultra-fluidos com menos de 15ms de latência, sem intervenção do mouse.

---

## 2. Guia de Compilação para Raspberry Pi Zero (ARMv6)

O SoC BCM2835 do Raspberry Pi Zero v1.2/v1.3/W requer a arquitetura `armv6l` (ARM1176JZF-S com VFPv2). Binários padrão `armv7` ou `aarch64` falham com `Illegal instruction`.

### 2.1 Compilação com `cross` (Docker)
Recomendado para compilações limpas e reprodutíveis:
```bash
# 1. Instalar o cross tool
cargo install cross --git https://github.com/cross-rs/cross

# 2. Compilar o ext-receiver em modo release estático
cd ~/ide/ext-monitor/receiver
cross build --target arm-unknown-linux-musleabihf --release

# 3. Binário resultante:
# target/arm-unknown-linux-musleabihf/release/ext-receiver
```

### 2.2 Compilação com Toolchain Nativo GNU
```bash
# Instalar toolchain no Ubuntu/Debian
sudo apt-get install -y gcc-arm-linux-gnueabihf g++-arm-linux-gnueabihf

# Adicionar target no Rust
rustup target add arm-unknown-linux-gnueabihf

# Configurar linker em ~/.cargo/config.toml:
# [target.arm-unknown-linux-gnueabihf]
# linker = "arm-linux-gnueabihf-gcc"

# Compilar
cargo build --target arm-unknown-linux-gnueabihf --release
```

---

## 3. Guia de Instalação em Modelos Raspberry Pi Não-OTG (Pi 2, 3, 4, 5)

Modelos tradicionais como Raspberry Pi 2, 3, 4B e 5 utilizam portas USB conectadas através de controladores de hub PCIe/USB (como VL805) e não possuem suporte a modo Gadget OTG de dispositivo periférico na maioria de suas portas padrão. Nesses modelos, a conexão é feita via **Rede Local (Ethernet Gigabit ou Wi-Fi)**.

### 3.1 Correção dos Parâmetros de Boot no Cartão SD

Ao gravar a imagem ou o initramfs em um Raspberry Pi sem OTG:

#### Arquivo `config.txt`:
Remova ou comente a linha do driver dwc2 de gadget:
```ini
# DESATIVAR OTG DWC2 EM MODELOS NÃO-OTG:
# dtoverlay=dwc2

# ATIVAR CONTROLADOR KMS DRM PADRÃO:
dtoverlay=vc4-kms-v3d,cma-128
dtparam=audio=on
```

#### Arquivo `cmdline.txt`:
Remova o carregamento forçado do módulo `dwc2`:
```text
# Configuração para Pi 2/3/4/5 (Removido modules-load=dwc2):
console=tty3 quiet loglevel=0 vt.global_cursor_default=0 ip=dhcp
```

### 3.2 Conexão de Rede e Execução:
Nesses modelos, o `ext-receiver` opera ouvindo na porta UDP 5000 do IP atribuído pelo roteador (ex: `192.168.1.150`):
```bash
# No Host Linux:
./scripts/start.sh extend auto 60 false full 192.168.1.150:5000
```

---

## 4. Guia: Transformar um PC / Laptop Convencional em Segunda Tela

Você pode reutilizar qualquer computador ou notebook antigo rodando Linux (Ubuntu, Debian, Fedora, Arch) como monitor secundário de altíssima velocidade.

### 4.1 Compilação e Execução do `ext-receiver` no PC Secundário
No PC secundário (receptor):
```bash
# 1. Instalar GStreamer
sudo apt-get install -y gstreamer1.0-tools gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-vaapi

# 2. Compilar ou rodar o ext-receiver
cargo build --release --bin ext-receiver
./target/release/ext-receiver
```

### 4.2 Pipeline GStreamer Direto (Alternativa Ultraleve sem Compilar)
No PC secundário, execute um terminal com o pipeline de recepção:
```bash
gst-launch-1.0 -v udpsrc port=5000 buffer-size=524288 \
    caps="application/x-rtp,media=video,clock-rate=90000,encoding-name=H264,payload=96" ! \
    rtph264depay ! h264parse ! avdec_h264 ! autovideosink sync=false
```

---

## 5. Como Direcionar a Transmissão para a Tela 1 ou 2 em PCs Multi-Monitor

Caso o PC que está atuando como receptor possua mais de um monitor físico conectado (ex: Monitor 1 interno do notebook e Monitor 2 HDMI), você pode direcionar a janela ou o scanout KMS diretamente para a saída desejada:

### Opção A: No Modo Direto KMS DRM (`kmssink`)
O elemento `kmssink` permite especificar o identificador exato do conector:
```bash
# 1. Listar conectores disponíveis no PC receptor:
modetest -c | grep -E "id|name"

# 2. Exemplo: Se HDMI-A-1 possui connector-id=42:
gst-launch-1.0 udpsrc port=5000 ... ! kmssink connector-id=42 sync=false
```

### Opção B: Em Sessão Gráfica X11 / Wayland
Abra o player com tela cheia posicionada na geometria da segunda tela:
```bash
# Usando ffplay posicionado no monitor 2 (x=1920):
ffplay -left 1920 -top 0 -x 1920 -y 1080 -fs -flags low_delay -framedrop rtp://0.0.0.0:5000
```
