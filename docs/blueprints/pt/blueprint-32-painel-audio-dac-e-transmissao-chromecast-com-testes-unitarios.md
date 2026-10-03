# Blueprint 32: Painel de Áudio Digital DAC na Aba Principal, Transmissão Estilo Chromecast e Testes Unitários de Chaveamento

## 1. Visão Geral e Motivação
Na versão anterior, os controles avançados de áudio (Volume, Mute, perfis de amostragem Hi-Res de 96kHz / 192kHz / 48kHz / 44.1kHz e arquitetura de transporte de áudio Modo 1 UDP, Modo 2 Gadget UAC2 e Modo 3 USB Bulk) estavam localizados na **Aba 2 (Configurações)**, distantes da visualização direta do usuário que opera principalmente na **Aba 1 (Monitoramento & Telemetria)**.
Além disso, o modo de compartilhamento web antigo se chamava ambiguamente de "Central de Mídia IoT & Visualizador HDMI", com placeholders e sem um fluxo claro de espelhamento com 1 clique.
Por fim, era necessário garantir via **testes unitários formais em Rust** que a máquina de estados de chaveamento de modos (`evaluate_mode_switch`) cumpra rigorosamente:
1. Desligar todos os modos entra imediatamente em Standby e pausa a transmissão;
2. Ativar qualquer modo transfere a transmissão para o serviço selecionado;
3. Desligar o modo ativo faz fallback automático para outro modo habilitado;
4. Desligar um modo secundário mantém a transmissão ativa no modo principal.

---

## 2. Arquitetura e Implementação

### 2.1 Testes Unitários de Chaveamento de Serviços (`receiver/src/web.rs`)
Implementamos a função pura e determinística:
```rust
pub fn evaluate_mode_switch(
    body: &str,
    current_active_transport: &str,
    mut mode1: bool,
    mut mode2: bool,
    mut mode3: bool,
) -> (bool, bool, bool, ModeSwitchResult)
```
E adicionamos 7 testes unitários em `receiver/src/web.rs` com 100% de aprovação (`cargo test -p ext-receiver`):
1. `test_evaluate_all_modes_disabled_enters_standby`: Valida que ao desligar todos os 3 modos, o resultado é `ModeSwitchResult::EnterStandby`.
2. `test_evaluate_explicit_transport_activation_mode1`: Valida troca direta para `mode1_udp`.
3. `test_evaluate_explicit_transport_activation_mode2`: Valida troca direta para `mode2_miracast`.
4. `test_evaluate_explicit_transport_activation_mode3`: Valida troca direta para `mode3_usb_bulk`.
5. `test_evaluate_mode_toggle_transfers_active_stream`: Valida que ao habilitar um modo pelo checkbox ou botão, a transmissão é imediatamente transferida para ele.
6. `test_evaluate_active_mode_disabled_fallback_to_other_mode`: Valida que se o modo ativo for desabilitado mas outro estiver ligado, ocorre fallback suave para o modo restante.
7. `test_evaluate_secondary_mode_disabled_does_not_switch_active`: Valida que desabilitar um listener secundário não interrompe a transmissão ativa.

### 2.2 Cartão de Áudio Digital HDMI & Hardware DAC na Aba 1
Consolidamos no frontend principal (`receiver/src/web_ui.rs` na Aba 1):
- **Slider de Volume Master (0% a 100%)** com botão de Mute em tempo real.
- **Botão de Teste Real de Chime HDMI**: Dispara um pulso sonoro no pipeline ALSA de hardware da TV via `/api/media/test_sound`.
- **Canvas de Espectro de Frequência 30 FPS**: Visualizador dinâmico de 24 bandas gerado a partir do stream PCM ALSA de hardware.
- **Botoeira de Perfil de Clock Master**:
  - `96 kHz / 24-bit`: Hi-Res Studio (Padrão)
  - `192 kHz / 24-bit`: Ultra Hi-Res
  - `48 kHz / 16-bit`: Cinema Standard
  - `44.1 kHz / 16-bit`: Fidelidade CD
- **Seletor de Arquitetura de Transporte**:
  - `Modo 1`: Rede UDP (Porta 5004, sub-5ms)
  - `Modo 2`: USB Audio Class (Gadget UAC2 plug-and-play)
  - `Modo 3`: USB Bulk Mux (/dev/usb-display-bulk)

### 2.3 Cartão de Transmissão Web & Compartilhamento Estilo Chromecast
Substituímos o box confuso de IoT por uma experiência de Chromecast real:
- **Status do Google Cast V2**: Porta 8008/8009 com descoberta mDNS ativa na rede.
- **Transmissor Web com 1 Clique (`/cast`)**: Botão proeminente que abre a página de captura nativa HTML5 (`getDisplayMedia` + WebCodecs) para espelhar qualquer aba do Chrome/Firefox, janela de reunião (Meet/Teams/Zoom) ou tela inteira.
- **Transmitir URL de Vídeo Direto (Play on TV)**: Campo de texto para colar URLs de streams (MP4, WebM, HLS m3u8) com botão "▶ Transmitir na TV".
- **Painel de Metadados / Now Playing**: Exibe título, artista e status de streaming em tempo real.

---

## 3. Validação e Resultados
1. **72 Testes Unitários Passando**: Todos os testes do receiver executados com sucesso (`72 passed; 0 failed`).
2. **Atualização OTA em RAM**: O appliance Pi Zero foi regravado via OTA em menos de 10 segundos sem corrupção de cartão SD.
3. **Validação de API Live**:
   - `{"rate":192000}` e `{"rate":96000}` testados e aplicados com sucesso.
   - Desativação de todos os modos coloca o sistema em `stream_state: "paused"` e `active_transport: "standby"`.
   - Reativação do Modo 1 restaura a transmissão ao vivo (`active_transport: "mode1_udp"`).
4. **Verificação Visual no Framebuffer**: Captura de screenshot confirmou streaming contínuo a 60 FPS com áudio sincronizado.
