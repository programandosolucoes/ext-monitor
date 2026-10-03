# Blueprint 31: Diagnóstico de Mismatch Visual na UI Web e Sincronização de Telemetria (Modo 1 Rede UDP vs Modo 3 USB Bulk)

> 🇧🇷 Versão em Português | [🇺🇸 English Version](../en/31-ui-transport-mismatch-diagnosis-and-telemetry-sync-mode1-vs-mode3.md)

*Data: 2026-10-02*  
*Status: Aprovado em Produção e Sincronizado*  
*Autor: Carlos Alberto <carlosalberto4ti@gmail.com>*  

---

## 1. Contexto e Pergunta do Usuário

Durante a transmissão estável e contínua de vídeo e áudio (Star Trek na Pluto TV), o usuário observou uma discrepância no painel web (`http://192.168.7.2:8080`):
* O assistente informou que a transmissão estava ocorrendo via **Modo 1: Rede UDP (portas 5000 para H.264 e 5004 para PCM)**.
* No entanto, ao abrir o Painel Web, a interface exibia como selecionado / ativo o **Modo 3: USB Bulk Direct**, e o cartão do Modo 3 continha a tag roxa *"Ligado (USB Bulk)"*.
* O usuário solicitou validar onde estava o erro de interpretação.

---

## 2. Diagnóstico Completo da Causa Raiz

A auditoria detalhada no código-fonte e no ambiente de execução revelou **três causas concomitantes** que geravam a falsa impressão visual de que o USB Bulk estava ativo:

### 2.1 Causa Raiz 1: HTML com Classes `active` Hardcoded no Modo 3
No arquivo `receiver/src/web_ui.rs`, a estrutura estática do HTML embutido na aplicação foi compilada com classes de estado ativo amarradas diretamente ao Modo 3:
```html
<!-- Linha 741 (Aba 1) -->
<button class="btn-toggle active" id="btnTransport_mode3" onclick="setActiveTransport('mode3_usb_bulk')">

<!-- Linha 901 (Card Modo 3) -->
<button class="btn-toggle active" id="btnMode3Extend" ...>

<!-- Linha 948 e 951 (Aba 2 - Host Remote Control) -->
<span class="control-value" id="valHostTransport">USB Bulk Direct (Mode 3)</span>
<button class="btn-toggle active" id="btnHostTransport_mode3" onclick="setActiveTransport('mode3_usb_bulk')">
```
Quando qualquer navegador carrega o painel, antes de qualquer script rodar, a tela renderiza imediatamente o botão do Modo 3 como destacado em verde/roxo e com o texto "USB Bulk Direct (Mode 3)".

### 2.2 Causa Raiz 2: Variável Inicial no JavaScript e Atraso no `pollTelemetry`
No bloco JavaScript de inicialização (`receiver/src/web_ui.rs`):
```javascript
// Linha 3181: variável iniciava com Modo 3
let currentTransport = 'mode3_usb_bulk';

// Linhas 4426-4429: pollTelemetry só rodava após 2 segundos de intervalo
pollNetworkStatus();
setInterval(pollTelemetry, 2000);
```
O método `pollTelemetry()` não era invocado imediatamente no carregamento (`DOMContentLoaded` ou fim do script). Por 2 segundos completos (ou mais se o backend demorasse a responder), os botões permaneciam no estado estático do HTML inicial.

### 2.3 Causa Raiz 3: Ambiguidade Conceitual entre "Daemon Ouvindo" vs "Stream em Trânsito"
No Raspberry Pi Zero, para fornecer comutação sem fricção (*instant zero-reboot transport switching*), múltiplos daemons ficam em modo de escuta (*listening*):
* Modo 1: Socket UDP ouvindo nas portas 5000 (vídeo) e 5004 (áudio).
* Modo 2: Daemon Miracast ouvindo em TCP 7236.
* Modo 3: Ouvinte USB FunctionFS.

Nos cartões de controle de cada modo (Card 1, Card 2, Card 3), havia uma tag de status do daemon:
* Card 1: `badgeMode1` -> *"Ligado (UDP 5000)"*
* Card 2: `badgeMode2` -> *"Ligado (TCP 7236)"*
* Card 3: `badgeMode3` -> *"Ligado (USB Bulk)"*

Um usuário lendo o cartão 3 via a tag *"Ligado (USB Bulk)"* e deduzia que o fluxo atual de dados estava passando por USB Bulk, quando na verdade isso indicava apenas que o hardware listener estava habilitado. O fluxo real de vídeo ativo é indicado unicamente no banner superior (`activeStreamBanner`) e na telemetria retornada por `/api/status`.

---

## 3. Evidência Técnica do Transporte Real em Execução

A inspeção do sistema host e do Pi Zero comprovou categoricamente o funcionamento no Modo 1 (Rede UDP):

1. **Processos Transmissores no Host Linux:**
   * **Vídeo:** PID 1200419 — `gst-launch-1.0 ... udpsink host=192.168.7.2 port=5000`
   * **Áudio:** PID 1172744 — `gst-launch-1.0 ... udpsink host=192.168.7.2 port=5004`
2. **API do Receptor Pi Zero (`curl http://192.168.7.2:8080/api/status`):**
   ```json
   {
     "active_transport": "mode1_udp",
     "active_mode": {
       "id": "mode1_udp",
       "name": "Mode 1: UDP Network (Linux Wayland / X11)",
       "icon": "🐧",
       "protocol": "RTP H.264 / RFC 4571",
       "port": 5000,
       "details": "UDP Port 5000 • Latency < 15ms • VA-API/M2M Pipeline"
     },
     "stream_state": "active"
   }
   ```
3. **Inexistência de Nó de Hardware USB Bulk no Pi:**
   * `/dev/usb-display-bulk` **não existe** no Pi Zero.
   * O cabo USB está configurado como gadget de rede virtual de alta velocidade (**USB CDC-ECM / RNDIS** com sub-rede `192.168.7.0/24`). O tráfego de 6.1 Mbps é composto integralmente de datagramas UDP fluindo pela interface de rede `usb0` através do cabo USB físico.

---

## 4. Correções Implementadas

No arquivo [receiver/src/web_ui.rs](file:///home/carlos/ide/ext-monitor/receiver/src/web_ui.rs):

1. **Ajuste dos Defaults no HTML:**
   * `btnTransport_mode1` e `btnHostTransport_mode1` agora possuem `class="btn-toggle active"`.
   * `btnTransport_mode3` e `btnHostTransport_mode3` agora possuem `class="btn-toggle"` (inativo).
   * `btnMode1Extend` configurado como `class="btn-toggle active"`.
   * `valHostTransport` inicializado como `"Network UDP (Mode 1)"`.
2. **Ajuste da Variável JavaScript:**
   * `let currentTransport = 'mode1_udp';`.
3. **Sincronização Imediata no Boot:**
   * Inclusão de chamadas imediatas de `updateModeAndTopologyButtons();` e `pollTelemetry();` logo no fim do carregamento da página, garantindo sincronização sub-100ms com os valores reais da API `/api/status`.
4. **Compilação e Validação:**
   * Compilado com sucesso via `cargo build --release -p ext-receiver --target arm-unknown-linux-musleabihf`.
   * O streaming ao vivo permaneceu intacto sem interrupções durante todo o processo.
