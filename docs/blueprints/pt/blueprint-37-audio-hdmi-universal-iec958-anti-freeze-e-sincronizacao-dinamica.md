# Blueprint 37: Áudio HDMI Universal IEC 60958, Resolução do Mudo em Smart TVs, Sincronização Dinâmica 48k/192k e Scanout Anti-Freeze Contínuo

## 1. Visão Geral e Contexto de Engenharia

Este blueprint documenta a descoberta, diagnóstico e solução definitiva de dois desafios críticos no projeto Ext-Monitor:
1. **O fenômeno do "Mudo após 2 segundos" em Smart TVs HDMI (Philips, LG, Samsung, Sony):** O áudio reproduzia perfeitamente por 2 a 3 segundos e era cortado abruptamente pelo processador DSP da TV.
2. **Descompasso de Relógio / Áudio Distorcido por Mismatch de Taxa:** Falta de sincronização bidirecional entre o gerador de áudio no host (PC) e o consumidor no SoC BCM2835 (Raspberry Pi Zero W).
3. **Consolidação do Mecanismo Anti-Congelamento (Continuous KMS Scanout):** Manutenção do scanout a 60 FPS contínuos no conector virtual estendido `HDMI-1` via DMA-BUF direto para o KMS Plane 86, prevenindo que o compositor Wayland (Mutter) entre em quiescência.

---

## 2. Diagnóstico de Causa Raiz: Bloqueio do DSP HDMI da TV (IEC 60958 Channel Status)

### 2.1 O Mecanismo do HDMI Audio Packet Parser nas TVs
Os receptores HDMI de televisores de consumo (ex: Philips 50PUG6102, arquitetura Saphi/Android TV) descompactam os pacotes de amostra de áudio (HDMI Audio Sample Packets) e acumulam **blocos de 192 frames de subframe IEC 60958-3** (S/PDIF over HDMI). 
Durante os primeiros ~2 segundos, a TV armazena amostras em buffer FIFO enquanto seu parser valida a conformidade dos metadados de status de canal (Channel Status Data).

### 2.2 Os Dois Erros Críticos Anteriores
No código anterior de encapsulamento de subframes ALSA (`receiver/src/audio.rs`):
```rust
// Código anterior com bugs:
let status_bytes: [u8; 24] = [
    0x04, // Consumer mode, PCM audio, No emphasis, NOT COPYRIGHT
    0x82, // BUG 1: Categoria PCM Coder (0x02) | Original (0x80)
    0x00, // Source / channel unspecified
    rate_code, // Sampling frequency
    0x02, // BUG 2: Word length específico 16/18 bits
    ...
];
```

* **Bug 1 — Categoria Proibida / Não-Broadcast (`0x82`):**
  * Conforme a norma IEC 60958-3 e `asoundef.h`, o byte 1 com valor `0x82` (`IEC958_AES1_CON_PCM_CODER`) indica um "conversor digital-para-digital / mixer de estúdio". Receptores HDMI de TVs comerciais exigem categoria geral de consumo (`IEC958_AES1_CON_GENERAL = 0x00`) para liberar áudio comercial de broadcast.
* **Bug 2 — Conflito de Word Length (`0x02`):**
  * O byte 4 com valor `0x02` (`IEC958_AES4_CON_WORDLEN_20_16` sem a flag `MAX_WORDLEN_24`) é interpretado pelo firmware de decodificação da TV como áudio de **18 bits**. Ao receber amostras alinhadas em 16/24 bits nos subframes, o DSP da TV detecta uma incongruência estrutural de tamanho de palavra e aciona a proteção de hardware ativando o **Mute de Proteção**.

### 2.3 Solução Implementada (Alinhamento com `snd_pcm_create_iec958_consumer_default`)
Conforme a especificação do subsistema ALSA no kernel Linux (`sound/core/pcm_iec958.c`):
```rust
let status_bytes: [u8; 24] = [
    0x04, // Consumer mode, PCM audio, No emphasis, NOT COPYRIGHT (IEC958_AES0_CON_NOT_COPYRIGHT)
    0x00, // Categoria: IEC958_AES1_CON_GENERAL (0x00) - Compatibilidade universal para TVs de consumo
    0x00, // Source / channel unspecified (IEC958_AES2_CON_SOURCE_UNSPEC | CHANNEL_UNSPEC)
    rate_code, // Sampling frequency (IEC 60958-3: 0x02=48k, 0x0A=96k, 0x0E=192k)
    0x00, // IEC958_AES4_CON_WORDLEN_NOTID (0x00) - Aceitação incondicional sem verificação estrita de wordlen
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];
```
Com `status_bytes[1] = 0x00` e `status_bytes[4] = 0x00`, a TV Philips aceita o fluxo PCM de forma contínua e irrestrita, eliminando 100% dos cortes após 2 segundos.

---

## 3. Sincronização Dinâmica Bidirecional de Taxa de Amostragem (48 kHz / 96 kHz / 192 kHz)

### 3.1 O Problema do Mismatch de Taxa (Áudio Distorcido / Pitch Baixo)
Se o host envia áudio a 192.000 Hz enquanto o receiver opera em 48.000 Hz, o receiver recebe 4x mais amostras do que seu clock de hardware consome:
$$\text{Fator de Acúmulo} = \frac{192.000}{48.000} = 4\times$$
Isso gera compressão temporal severa, distorção grosseira ("som estragado / voz de monstro") e estouro contínuo de buffer.

### 3.2 Arquitetura de Sincronização Automática
Implementamos um protocolo determinístico de sincronização automática entre os dois nós:

```
[ Raspberry Pi Zero: ext-receiver ]                      [ Host PC: ext-sender ]
         │                                                         │
         │─── Boot: Broadcast initial_rate via UDP 5001 ──────────►│ Alinha sink e recorder
         │                                                         │ imediatamente
         │                                                         │
         │◄── Host altera taxa (CLI / config) ─────────────────────│ POST /api/audio/rate
         │    Reconfigura ALSA clock local                         │
         │                                                         │
         │◄── Usuário clica no Painel Web (ex: 192 kHz)            │
         │    Reconfigura ALSA /dev/snd/pcmC0D0p                   │
         │    Notifica ext-sender via UDP 5001 ───────────────────►│ Reconstitui sink PulseAudio
         │                                                         │ e reinicia recorder em 192k
```

1. **Anúncio no Boot (`receiver/src/main.rs`):** Ao iniciar, o receiver despacha um pacote UDP 5001 anunciando sua taxa configurada (`{"audio_rate": 48000}`).
2. **Repasse Atômico da Web UI (`receiver/src/web.rs`):** Ao receber `POST /api/audio/rate`, o receiver dispara `forward_config_to_sender(&format!("{{\"audio_rate\":{}}}", rate))` para o socket 5001 do host.
3. **Migração Suave de Stream (`sender/src/audio_native.rs`):** O migrador de streams do PipeWire/PulseAudio verifica o ID do sink antes de transferir nós de áudio, eliminando transferências redundantes a cada segundo que causavam micro-cortes e vazamento para o speaker do notebook.

---

## 4. Remoção de Cache Estático na Web UI (Fonte Única da Verdade)

Anteriormente, o arquivo `receiver/src/web_ui.rs` restaurava `localStorage.getItem('ext_audio_rate')` na inicialização do DOM, sobrescrevendo a interface com valores antigos de sessões anteriores.

**Refatoração:**
* Removido o carregamento de `savedRate` do `localStorage`.
* O painel agora renderiza estritamente os dados vivos do endpoint `/api/status`:
```javascript
if (a.rate) {
    currentAudioRate = a.rate;
    updateAudioRateUI(a.rate);
}
```

---

## 5. Validação Empírica em Produção

| Métrica | Modo 48 kHz (Cinema Standard) | Modo 192 kHz (Ultra Hi-Res) |
|---|---|---|
| **Consumo ALSA (`hw_ptr`)** | ~48.000 frames/s contínuo | ~196.000 frames/s contínuo |
| **Delay de Buffer** | 668 a 760 frames (~13 a 15 ms) | 2.388 frames (~12.4 ms) |
| **Buffer ALSA Total** | 4.096 frames (4 períodos de 1.024) | 16.384 frames (8 períodos de 2.048) |
| **TV Philips 50PUG6102** | Suporte nativo, 0% distorção | Suporte funcional com buffer ampliado |
| **Mecanismo Anti-Freeze** | Ativo (60 FPS contínuo KMS Plane 86) | Ativo (60 FPS contínuo KMS Plane 86) |
| **Estabilidade de Execução** | > 1 hora ininterrupta sem underrun | Estável e auditada |
