# Compêndio de Blueprints Técnicos de Engenharia — `ext-monitor`

**Projeto:** `ext-monitor` — Monitor Secundário USB de Ultra-Baixa Latência  
**Plataforma Alvo:** Raspberry Pi Zero W / Zero 2 W / Raspberry Pi 4 / PC Linux & Windows  
**Autor:** Carlos Alberto & Antigravity  
**Data:** Setembro de 2026  
**Status do Projeto:** Estável, Pronto para Produção, Compilado em Rust Nativo Puro  

---

## 1. Visão Geral da Suíte de Documentos

Esta pasta reúne a documentação de engenharia reversa, decisões de arquitetura e especificações de baixo nível que tornaram possível transformar um dispositivo de computação de US$ 10 (Raspberry Pi Zero de núcleo único a 1.0 GHz) em um monitor secundário profissional de 60 FPS com latência inferior a 18 milissegundos.

Todos os desafios fundamentais — desde o alinhamento de setores FAT16 no silício BCM2835 até a recuperação autônoma de suspensão de energia no Linux Wayland — estão formalizados nos 8 blueprints técnicos a seguir:

---

## 2. Mapa dos 8 Blueprints de Engenharia

| # | Blueprint Técnico | Arquivo | Foco de Engenharia |
| :---: | :--- | :--- | :--- |
| **01** | **Imagem de 32MB e Geometria do BCM2835** | [`01-imagem-32mb-e-geometria-bcm2835.md`](01-imagem-32mb-e-geometria-bcm2835.md) | Bug dos 65.525 clusters da ROM Broadcom, alinhamento no setor 1, partições FAT16 de 2KB por cluster e boot 100% em RAM. |
| **02** | **Arquitetura Fim-a-Fim e Comparativo com o Projeto GUD** | [`02-arquitetura-transmissor-receptor-e-comparativo-gud.md`](02-arquitetura-transmissor-receptor-e-comparativo-gud.md) | Por que o GUD falha (100% CPU, saturação USB com LZ4) e como o pipeline H.264 V4L2 M2M atinge 60 FPS com 0.8% de CPU. |
| **03** | **Transmissão de Pacotes, Fragmentação NALU e Leaky Queue** | [`03-transmissao-pacotes-drop-on-late-e-pipeline.md`](03-transmissao-pacotes-drop-on-late-e-pipeline.md) | MTU de 1472 bytes, fragmentação FU-A RFC 6184, descarte de pacotes atrasados (*drop-on-late*) e drenagem pós-sono. |
| **04** | **PipeWire, Mutter D-Bus e Auto-Recuperação Pós-Suspensão** | [`04-pipewire-mutter-screencast-e-wayland.md`](04-pipewire-mutter-screencast-e-wayland.md) | Captura virtual sem dongle, zero-copy DMA-BUF, relógio anti-ociosidade e **Supervisor com Duplo Watchdog em Rust** para sono/S3. |
| **05** | **Otimizações de CPU em Rust, GPU/VPU e Upscaler CAS** | [`05-otimizacoes-cpu-rust-gpu-vpu-e-cas-scaler.md`](05-otimizacoes-cpu-rust-gpu-vpu-e-cas-scaler.md) | Flags `-C target-cpu=arm1176jzf-s`, codificação por silício VA-API/NVENC e filtros de nitidez CAS vs FSR para texto. |
| **06** | **Modos Concorrentes de Operação e Super-Gadget USB ConfigFS** | [`06-modos-de-operacao-concorrentes-e-usb-gadget.md`](06-modos-de-operacao-concorrentes-e-usb-gadget.md) | 3 modos simultâneos (Linux UDP, Windows Miracast Win+K, USB Bulk), alocação de 7 endpoints DWC2 e ciclo USB Suspend/Resume. |
| **07** | **Cartão Micro-SD em RAM, Proteção e Upgrade via USB** | [`07-cartao-sd-em-ram-e-upgrade-usb.md`](07-cartao-sd-em-ram-e-upgrade-usb.md) | Eliminação de corrupção flash, partição exposta como pendrive `EXTMONITOR` na USB, upgrade em 3 métodos e entrega de binários. |
| **08** | **Manual de Instalação, Portabilidade e Script de Setup Host** | [`08-manual-de-instalacao-e-portabilidade-host.md`](08-manual-de-instalacao-e-portabilidade-host.md) | Conexão em 1 clique via `curl connect.sh`, instalador multi-distro, regras udev de baixa latência e console serial `/dev/ttyACM0`. |

---

## 3. Destaque Arquitetural: O Desafio da Suspensão de Energia (Sleep / Suspend / Resume)

Um dos marcos mais recentes de estabilidade foi a resolução do **congelamento permanente após suspensão do computador host**:

### O Problema:
Quando o sistema operacional do host entra em suspensão de energia (*suspend-to-RAM* / S3), o compositor do GNOME (Mutter) encerra a sessão D-Bus de ScreenCast e destrói o nó de origem no PipeWire. No entanto, o processo filho do GStreamer (`gst-launch-1.0`) não é encerrado pelo sistema: ele permanece vivo, preso em leitura de I/O em um socket órfão. Um supervisor tradicional que use apenas `child.try_wait()` nunca detecta a falha, fazendo a imagem "morrer" sem retorno.

### A Solução Implementada:
1. **Health Watchdog (1500ms):** O supervisor em Rust executa `pw-cli info <node_id>`. Se o nó foi destruído, ele força o encerramento do processo (`child.kill()`), renegocia a sessão D-Bus com o Mutter e restabelece a transmissão em menos de 2 segundos.
2. **Link Watchdog (3000ms):** Verifica `pw-link -l` e reconecta o enlace PipeWire caso tenha sido desfeito.
3. **Persistência de Exibição:** O script de exibição (`show-welcome-window.py`) executa em laço resiliente `while True:`, auto-reconectando ao display `:0` caso o servidor Xwayland reinicie.
4. **Regras Udev Reativas:** As regras em `/etc/udev/rules.d/99-ext-monitor.rules` escutam `ACTION=="add|change"` para reaplicar `txqueuelen 100` e religar o serviço no momento em que a controladora USB do host sai do estado de repouso.
5. **Auto-Suficiência do Appliance:** Todo o pacote portátil (`client.tar.gz` e `ext-sender`) com essas correções é compilado e gravado dentro do `initramfs.cpio.gz` da imagem de boot de 32MB, ficando disponível via HTTP em `/connect.sh` para qualquer máquina nova.

---

## 4. Como Executar e Validar

* **Transmissão Rápida no Host:**
  ```bash
  ./scripts/start.sh extend auto 30 false economy --bitrate=400
  ```
* **Conexão Portátil em Qualquer PC Linux:**
  ```bash
  curl -sSL http://192.168.7.2:8080/connect.sh | bash
  ```
* **Acesso ao Web Dashboard com 5 Abas e 4 Idiomas:**
  Abra no navegador do computador host: `http://192.168.7.2:8080/`
* **Compilação e Geração da Imagem do Appliance:**
  ```bash
  ./scripts/build-fast-appliance.sh
  ```
