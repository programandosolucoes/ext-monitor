# Blueprint 08: Manual de Instalação, Portabilidade e Script de Setup no Host PC

> 🇧🇷 Versão em Português | [🇺🇸 English Version](../en/08-host-installation-and-portability-manual.md)


**Projeto:** `ext-monitor`  
**Autor:** Carlos Alberto <carlosalberto4ti@gmail.com>  
**Arquivos de Referência:** `scripts/connect.sh`, `scripts/install-host.sh`, `scripts/99-ext-monitor.rules`  
**Data:** Setembro de 2026  

---

## 1. Visão Geral

Este blueprint descreve a infraestrutura de portabilidade do `ext-monitor`, projetada para que qualquer usuário possa pegar o Raspberry Pi Zero, conectá-lo a um novo computador (Linux ou Windows) e colocá-lo para funcionar em questão de segundos sem precisar compilar o projeto do zero.

---

## 2. Portabilidade Imediata: Conexão em 1-Clique via `curl`

Quando o Raspberry Pi Zero é conectado a uma nova máquina Linux através do cabo USB, a interface de rede CDC-ECM (`usb0` / `enx...`) é ativada e recebe automaticamente o IP `192.168.7.1` via servidor DHCP embutido do Pi Zero.

O usuário pode simplesmente abrir o terminal no novo computador e executar:

```bash
curl -sSL http://192.168.7.2:8080/connect.sh | bash
```

### O Que o Script `connect.sh` Faz Automaticamente:
1. Faz ping no Raspberry Pi Zero em `192.168.7.2` para confirmar que o cabo USB está conectado na porta de dados correta.
2. Cria o diretório de ferramentas portátil `~/.local/share/ext-monitor/`.
3. Baixa o pacote completo de ferramentas (`client.tar.gz`) diretamente do servidor web embutido no Pi Zero.
4. Descompacta o binário `ext-sender` e o script de inicialização `start.sh`.
5. Inicia imediatamente a extensão de tela em modo de economia de banda (400 kbps, 30 FPS) com supervisão de duplo watchdog ativa (recuperação automática de sleep / suspend-to-RAM em menos de 2 segundos).

---

## 3. Instalação Permanente no Sistema Host (`scripts/install-host.sh`)

Para usuários que desejam instalar o driver e as ferramentas de forma definitiva no sistema operacional (com integração aos menus gráficos e regras de permissão):

```bash
./scripts/install-host.sh
```

### 3.1 Etapas Executadas Pelo Instalador:

#### 1. Resolução Automática de Dependências Multi-Distro e Aceleração Gráfica
O instalador detecta a distribuição Linux em uso e instala os pacotes necessários para streaming e codificação acelerada por hardware (AMD, Intel e NVIDIA):
* **Debian / Ubuntu / Pop!_OS / Linux Mint:**
  ```bash
  sudo apt-get install -y gstreamer1.0-tools pipewire pipewire-bin ffmpeg libcap2-bin vainfo mesa-va-drivers
  # Se GPU Intel:
  sudo apt-get install -y intel-media-va-driver-non-free || sudo apt-get install -y intel-media-va-driver
  ```
* **Fedora / RHEL:**
  ```bash
  sudo dnf install -y gstreamer1-tools pipewire pipewire-utils ffmpeg libcap mesa-va-drivers libva-utils
  ```
* **Arch Linux / Manjaro:**
  ```bash
  sudo pacman -S --noconfirm gstreamer pipewire pipewire-media-session ffmpeg libva-mesa-driver libva-utils
  ```

#### 2. Instalação dos Binários e Concessão de Capacidades KMS (`cap_sys_admin`)
* `ext-sender`: Binário em Rust com arquitetura Dual-Engine (`kms` e `mutter`).
* `ext_kms_probe`: Utilitário para varredura e diagnóstico das placas de vídeo e conectores DRM/KMS.
* `ext-monitor-start`: Script de controle com suporte a `--kms`, `--mutter`, `--monitor=<nome>` e taxas de 15 a 60 FPS.
* `ext-monitor-connect`: Conector rápido universal em 1 clique.

**Segurança e Operação sem Root:**
O instalador executa:
```bash
sudo setcap cap_sys_admin+ep /usr/local/bin/ext-sender
sudo setcap cap_sys_admin+ep /usr/bin/ffmpeg
sudo usermod -a -G video,render $USER
```
Isso permite que o `ext-sender` interaja diretamente com o subsistema DRM/KMS do Kernel Linux (`/dev/dri/card*`) e exporte buffers PRIME DMA-BUF do scanout da GPU sem exigir execução como `root` ou `sudo`.

#### 3. Regras Udev de Baixa Latência e Retomada de Suspensão (`/etc/udev/rules.d/99-ext-monitor.rules`)
```udev
# 99-ext-monitor.rules: Configuração automática da interface de rede USB e recuperação de sleep
ACTION=="add|change", SUBSYSTEM=="net", KERNEL=="enx*|usb*", ATTRS{idVendor}=="1d50", ATTRS{idProduct}=="614d", \
    RUN+="/sbin/ip link set dev %k txqueuelen 100", RUN+="/sbin/ip link set dev %k mtu 1500"
ACTION=="add|change", SUBSYSTEM=="net", KERNEL=="enx*|usb*", ATTRS{idVendor}=="1d6b", ATTRS{idProduct}=="0104", \
    RUN+="/sbin/ip link set dev %k txqueuelen 100", RUN+="/sbin/ip link set dev %k mtu 1500"

ACTION=="add", SUBSYSTEM=="usb", ATTR{idVendor}=="1d50", ATTR{idProduct}=="614d", TAG+="systemd", \
    RUN+="/usr/bin/systemctl --no-block --user restart ext-monitor-autoconnect.service"
ACTION=="add", SUBSYSTEM=="usb", ATTR{idVendor}=="1d6b", ATTR{idProduct}=="0104", TAG+="systemd", \
    RUN+="/usr/bin/systemctl --no-block --user restart ext-monitor-autoconnect.service"
```
* **`txqueuelen 100`:** Reduz a fila de transmissão da placa de rede de 1000 para 100 pacotes. Isso elimina completamente o acúmulo de buffers (buffer bloat) e reduz a latência do mouse em mais de 15ms.
* **`add|change` na Retomada de Suspensão:** Garante que quando o PC acorda do modo de economia de energia (*sleep / suspend-to-RAM*), as regras de baixa latência e os daemons de extensão de tela sejam reativados instantaneamente.

#### 4. Atalho no Menu de Aplicativos (Desktop Entry)
Cria o arquivo `/usr/share/applications/ext-monitor.desktop`, permitindo que o usuário inicie a segunda tela com um único clique no menu de programas ou pressione a tecla Super e digite "Ext-Monitor".

---

## 4. Conexão em Ambientes Windows 10 e Windows 11

No Windows, **não é necessário instalar absolutamente nenhum executável ou driver**:

```
[ Pressione no Teclado: Win + K ]
              │
              ▼
[ Menu Lateral "Transmitir" Abre no Windows ]
              │
              ▼
[ Selecione: "ExtMonitor-Pi0" ]
              │
              ▼
[ O Windows Estende a Área de Trabalho Instantaneamente ]
```

* **Tecnologia Utilizada:** Miracast Nativo (Wi-Fi Display / RTSP WFD) implementado em Rust pelo `ext-receiver` na porta TCP 7236.
* O Windows reconhece o Raspberry Pi como uma tela sem fio certificada, codificando os frames via hardware (NVENC/Intel QuickSync) diretamente para a porta USB/Rede.

---

## 5. Console Serial de Emergência (/dev/ttyACM0)

Se a interface de rede do computador host estiver com firewall bloqueado ou se o usuário precisar depurar o Raspberry Pi Zero sem rede:

1. Conecte o cabo USB ao computador.
2. Abra o terminal serial a 115200 baud:
   ```bash
   screen /dev/ttyACM0 115200
   # ou: minicom -D /dev/ttyACM0 -b 115200
   ```
3. Pressione Enter para receber o shell root do Busybox do appliance.
4. É possível inspecionar o log do decodificador (`cat /tmp/receiver.log`), verificar o uso de CPU (`top`) e gerenciar interfaces de rede em tempo real.
