# Blueprint 23: Aceleração de Hardware por GPU no Miracast e Launcher Automático do Host

> 🇧🇷 Versão em Português | [🇺🇸 English Version](../en/23-miracast-gpu-hardware-acceleration-and-host-launcher.md)

*Data: 2026-10-01*  
*Status: Implementado, Validado em Hardware Real (AMD Radeon 610M / RDNA2 / Mendocino) e Operacional*  
*Autor: Carlos Alberto <carlosalberto4ti@gmail.com>*  

---

## 1. Visão Geral e Contexto do Problema

Ao projetar a tela de um computador Linux para o appliance Raspberry Pi Zero W em **Modo 2 (Miracast / Wi-Fi Display)** utilizando o utilitário nativo de rede do GNOME (`gnome-network-displays`), o GStreamer por padrão seleciona a codificação por software em CPU (`x264enc`), a menos que modificadores de ranking de plugins sejam fornecidos. Isso gerava uma sobrecarga severa no processador do laptop host, induzindo estrangulamento térmico (thermal throttling) e introduzindo latência perceptível no movimento do cursor e na atualização de frames.

Para obter projeção sem fio em tempo real com codificação H.264 por hardware nas mais diversas plataformas de laptop, dois requisitos centrais foram levantados:
1. **Orientações Interativas no Painel Web:** Tooltips claros e uma caixa de comandos dedicada para GPU na interface Web do receptor (`http://192.168.7.2:8080`), com suporte a cópia de comandos otimizados para **AMD**, **Intel**, **NVIDIA** e **Auto**.
2. **Disparo Automático sem Fricção:** Ao clicar em "Conectar Miracast" no painel Web, o Raspberry Pi Zero sinaliza o agente em segundo plano do laptop (`ext-sender`) via porta UDP `5001`. O agente host detecta automaticamente o fabricante da GPU instalada, aplica o ranking ideal de aceleração por hardware do GStreamer, atualiza o atalho `.desktop` do usuário e abre o `gnome-network-displays` já configurado.

---

## 2. Detecção Determinística de GPU via Sysfs

Em vez de depender da execução de shells lentos ou ferramentas externas, o agente host detecta a placa de vídeo do laptop inspecionando diretamente a hierarquia DRM do sysfs do kernel Linux em `/sys/class/drm/card*/device/vendor`:

| Fabricante | PCI Vendor ID | Mecanismo de Hardware | Encoders GStreamer Priorizados | Variável de Ranking Aplicada |
| :--- | :--- | :--- | :--- | :--- |
| **AMD** | `0x1002` | VA-API / AMDGPU (RDNA, Vega, Polaris) | `vaapih264enc`, `vah264enc` | `GST_PLUGIN_FEATURE_RANK=vaapih264enc:MAX,vah264enc:MAX` |
| **Intel** | `0x8086` | Intel Media SDK / VA-API / QSV | `vaapih264enc`, `vah264enc`, `qsvh264enc` | `GST_PLUGIN_FEATURE_RANK=vaapih264enc:MAX,vah264enc:MAX,qsvh264enc:MAX` |
| **NVIDIA** | `0x10de` | NVENC / CUDA / VDPAU | `nvh264enc`, `vaapih264enc` | `GST_PLUGIN_FEATURE_RANK=nvh264enc:MAX,vaapih264enc:MAX` |
| **Híbrida** | Múltiplos | NVIDIA dGPU + AMD/Intel Integrada | `nvh264enc`, `vaapih264enc`, `vah264enc` | `GST_PLUGIN_FEATURE_RANK=nvh264enc:MAX,vaapih264enc:MAX,vah264enc:MAX,qsvh264enc:MAX` |

Um fallback automático via `lspci` é mantido caso o sysfs não esteja acessível.

---

## 3. Arquitetura de Controle Ponta a Ponta

```
 ┌──────────────────────────────────────────────────────────────────┐
 │                    Raspberry Pi Zero Appliance                   │
 │                                                                  │
 │  Web UI (http://192.168.7.2:8080)                                │
 │    - Tooltips & Seletor de GPU (AMD / Intel / NVIDIA / Auto)     │
 │    - Botão "Conectar Miracast (Win+K / Linux GPU)"               │
 │                           │                                      │
 │                           ▼                                      │
 │         POST /api/host/control {"action": "launch_miracast"}     │
 │                           │                                      │
 │                           ▼                                      │
 │         UdpSocket::send_to("192.168.7.1:5001")                   │
 └───────────────────────────┬──────────────────────────────────────┘
                             │
                      Datagrama UDP 5001
                             │
 ┌───────────────────────────▼──────────────────────────────────────┐
 │                       Laptop Host (Linux)                        │
 │                                                                  │
 │  ext-sender (Rust Daemon - ext-monitor-sender.service)           │
 │    - Listener na porta UDP 5001 (control.rs)                     │
 │    - Ação: ControlAction::LaunchMiracast                         │
 │    - Manipulador: miracast_launcher::launch_gnome_network_...()  │
 │    - Debounce Atômico (< 2.5s) previne instâncias duplicadas     │
 │    - Preserva DISPLAY, WAYLAND_DISPLAY, XDG_RUNTIME_DIR          │
 │    - Atualiza ~/.local/share/applications/org.gnome.NetworkDisplays│
 │                           │                                      │
 │                           ▼                                      │
 │    Executa: env GST_PLUGIN_FEATURE_RANK=vaapih264enc:MAX,...     │
 │             gnome-network-displays                               │
 └──────────────────────────────────────────────────────────────────┘
```

---

## 4. Detalhes de Implementação

### 4.1 Launcher Miracast no Agente Host (`sender/src/miracast_launcher.rs`)
- **Varredura em Sysfs:** Lê `/sys/class/drm/card[0-9]*/device/vendor` em microssegundos sem gerar subprocessos.
- **Persistência no Atalho Desktop:** Grava ou substitui `~/.local/share/applications/org.gnome.NetworkDisplays.desktop`, garantindo que execuções futuras a partir do menu do GNOME Shell também utilizem a aceleração de hardware detectada.
- **Injeção do Ambiente de Sessão:** Vincula-se com segurança à sessão gráfica do usuário (`DISPLAY=:0`, `WAYLAND_DISPLAY=wayland-0`, `XDG_RUNTIME_DIR=/run/user/<UID>`).
- **Debounce Atômico:** Utiliza `AtomicU64` com timestamp de época para evitar condições de corrida caso múltiplos datagramas cheguem em intervalo inferior a 2,5 segundos.

### 4.2 Aprimoramentos na Interface Web e API (`receiver/src/web_ui.rs`, `receiver/src/web.rs`)
- **Caixa de Comandos Interativa para GPU:** Adiciona botões tipo chip (`AMD`, `Intel`, `NVIDIA`, `Auto`), pré-visualização em tempo real e botão de cópia rápida para o clipboard.
- **Tooltips Contextuais:** Informa ao usuário diretamente nos botões de conexão o comando de aceleração necessário caso deseje executar via terminal.
- **Deduplicação de Pacotes:** Evita envio duplicado para a interface `192.168.7.1:5001` quando o navegador está acessando diretamente pela rede USB.

---

## 5. Validação em Hardware Real

Teste empírico conduzido em laptop com AMD Ryzen Mendocino (GPU AMD Radeon 610M):
1. **Disparo:** Clique no botão "Conectar Miracast" na Web UI em `http://192.168.7.2:8080`.
2. **Log do Daemon Host (`journalctl --user -u ext-monitor-sender`):**
   ```text
   [*] Web Command: Lançar Miracast (GNOME Network Displays) com aceleração de GPU
   [miracast-launcher] GPU Detectada: AMD Radeon (VA-API / RDNA / Vega / Mendocino) (Encoder: vaapih264enc / vah264enc)
   [miracast-launcher] Aplicando: GST_PLUGIN_FEATURE_RANK=vaapih264enc:MAX,vah264enc:MAX
   [miracast-launcher] gnome-network-displays iniciado com sucesso (PID: 817033) com aceleração por GPU!
   ```
3. **Auditoria das Variáveis de Ambiente:**
   ```bash
   $ tr '\0' '\n' < /proc/817033/environ | grep -E "GST_|NETWORK_DISPLAYS"
   GST_PLUGIN_FEATURE_RANK=vaapih264enc:MAX,vah264enc:MAX
   NETWORK_DISPLAYS_H264_ENC=vaapih264enc
   ```

---

## 6. Arquivos e Entregáveis

- `sender/src/miracast_launcher.rs`: Detecção de GPU, atualização de entrada desktop e execução do `gnome-network-displays`.
- `sender/src/control.rs`: Parser da ação de controle `LaunchMiracast`.
- `sender/src/main.rs`: Integração do módulo `miracast_launcher` e roteamento de controle.
- `receiver/src/web.rs`: Encaminhamento de controle para UDP 5001 com deduplicação de IP.
- `receiver/src/web_ui.rs`: Chips de seleção de GPU, caixa de comando, cópia de atalho e tooltips.
- `release/frozen-v0.3.0/ext-receiver`: Recompilado e empacotado para ARMv6 musl.
- `build-appliance/boot/initramfs.cpio.gz`: Empacotado e gravado via OTA no cartão SD do Raspberry Pi Zero.
