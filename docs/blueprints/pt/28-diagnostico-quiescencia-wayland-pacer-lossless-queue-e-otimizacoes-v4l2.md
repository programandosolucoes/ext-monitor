# Blueprint 28: Diagnóstico Empírico de Quiescência Wayland, Damage Pacer, Filas Lossless e Matriz de Buffers V4L2 M2M

> 🇧🇷 Versão em Português | [🇺🇸 English Version](../en/28-wayland-quiescence-diagnosis-pacer-lossless-queue-and-v4l2-tuning.md)

*Data: 2026-10-02*  
*Status: Aprovado em Produção e Validado a 60 FPS Contínuos no Modo 3 (USB Bulk)*  
*Autor: Carlos Alberto <carlosalberto4ti@gmail.com>*  

---

## 1. Visão Geral do Problema

Durante a evolução da arquitetura do **Ext-Monitor**, a busca por mitigação de latência levou à experimentação de parâmetros de buffering em dois nós críticos do pipeline:
1. A fila intermediária pós-encoder GStreamer (`queue` e `fdsink`).
2. A alocação de buffers MMAP do decodificador de hardware VideoCore IV (`V4L2 M2M` via `bcm2835-codec`).

Essa experimentação gerou simultaneamente dois sintomas adversos reportados:
* **"Tirar o mouse congela":** O fluxo de vídeo parava instantaneamente assim que o cursor deixava a tela estendida `HDMI-1` ou quando o usuário parava de movimentar o cursor sobre o monitor.
* **"Ficou lento":** O cursor e a renderização apresentavam atrasos pesados (latência acima de 250ms e sensação de 5 a 15 FPS).

Este blueprint consolida a análise de causa raiz baseada nas lições dos Blueprints 10, 15, 16 e 26, a matriz empírica de benchmarks e a solução definitiva aplicada ao código.

---

## 2. Anatomia dos 4 Gargalos e Causas Raízes

### 2.1 Quiescência do GNOME Mutter sob Wayland (O "Tirar o Mouse Congela")
* **Mecanismo Físico:** No GNOME Wayland, o compositor Mutter adota *Damage-Driven Rendering*. Quando uma região do desktop não apresenta alterações gráficas (nenhum vídeo tocando e nenhum cursor se movendo), o Mutter suspende as chamadas `stage_painted()`, derrubando a taxa de quadros emitida no nó PipeWire para **0 FPS**.
* **Solução Arquitetural Definitiva (100% Rust Puro In-Process):** O **Wayland Damage Pacer** nativo (`sender/src/damage_pacer.rs`).
  * Implementação em thread Rust sem dependências externas ou scripts Python.
  * Cria dinamicamente via `libX11` uma janela de 1x1 pixel com `CW_OVERRIDE_REDIRECT = 1` e input shape vazio (`XShapeCombineRectangles(SHAPE_INPUT)` via `libXext`), garantindo 100% de click-through (transparente e sem interceptar cliques/foco).
  * Posicionada no canto inferior da tela secundária (`1920 + 1280 - 2, 720 - 2`).
  * Emite um pulso de dano contínuo a 60 Hz alternando desenho de pixels (`0x00000000` / `0x00010101`) com `XFillRectangle` + `XFlush`, forçando o Mutter a comprometer buffers reais (`wl_surface.commit`) continuamente.
  * O compositor Mutter mantém o clock do PipeWire a 60 FPS estáveis, eliminando o congelamento quando o mouse repousa ou deixa o monitor.
* **Implementação no Host:** Integrado auto-spawn automático in-process no supervisor `ext-sender` sempre que rodando sob sessão Wayland.

### 2.2 Fila Vazante (`leaky=downstream`) no Bitstream Compactado
* **O Erro Conceitual:** A inserção de `queue max-size-buffers=1 leaky=downstream` após o elemento `h264parse` e antes do `fdsink`.
* **Impacto no Decodificador:** Em fluxos brutos (RAW), descartar quadros apenas salta um frame. Em fluxos H.264 comprimidos (Annex-B), descartar NALUs parciais, fatias P-frame ou conjuntos SPS/PPS **destrói a cadeia de referência do GOP**.
* **Efeito no Pi Zero:** O hardware VideoCore IV falha na decodificação de todos os quadros subsequentes, ficando congelado na última imagem válida até a chegada do próximo I-frame (1 segundo depois).
* **Regra Arquitetural de Ouro (Blueprint 26 Item 85):** *Filas com descarte (`leaky`) são terminantemente proibidas no fluxo H.264 compactado.* A fila pós-encoder deve ser estritamente *lossless* (`queue max-size-buffers=4 max-size-bytes=0 max-size-time=0`).

### 2.3 Subdimensionamento de Buffers V4L2 M2M (Starvation do VideoCore IV)
* **O Erro Conceitual:** Reduzir `req_out.count` de 16 para 4 e `req_cap.count` de 8 para 3 sob a suposição de que "menos buffers = menor latência".
* **A Realidade do Hardware Broadcom BCM2835:**
  * No driver `bcm2835-codec`, os buffers de CAPTURE não são uma fila de atraso de rede; são slots DMA para o *Decoded Picture Buffer* (DPB) onde o hardware armazena os quadros de referência para reconstrução temporal.
  * Com apenas 3 buffers CAPTURE: 1 buffer está em exibição no plano KMS, 1 buffer está retido como referência DPB, restando **ZERO buffers livres** para a VPU realizar o decode do próximo quadro.
* **O Colapso da Pipeline:** Sem buffers livres, o loop de ingestão entra no fallback de espera:
  ```rust
  for _ in 0..400 {
      self.drain_decoded_frames(&mut on_frame);
      self.reclaim_output_buffers();
      std::thread::sleep(Duration::from_micros(500)); // 200 ms de atraso por quadro!
  }
  ```
  Isso causava picos de 200ms de latência e queda drástica para 5 FPS durante rajadas.
* **A Solução:** Restaurar a alocação padrão estável de **16 buffers OUTPUT** e **8 buffers CAPTURE**.

### 2.4 Atraso de Agrupamento com `fdsink blocksize=65536`
* **O Erro Conceitual:** Forçar `blocksize=65536` (64 KB) no `fdsink` na tentativa de casar com o buffer de leitura do USB.
* **Impacto na Latência:** A 60 FPS com 6000 kbps, cada P-frame tem em média 10-12 KB, e micro-atualizações do cursor têm apenas 1-2 KB. Forçar 64 KB fazia o GStreamer reter os pacotes em buffer interno por **5 a 30 quadros (80ms a 500ms)** até encher o bloco!
* **A Solução:** Remover o argumento `blocksize` do `fdsink`, permitindo que cada NALU seja escrito atomicamente no pipe no instante exato de sua codificação (`sync=false`).

---

## 3. Matriz de Benchmark: Comparativo de Configurações

| Métrica | Configuração Degradada (Subdimensionada) | Configuração Stock (Sem Pacer) | Configuração Blueprint 28 (Lossless + Pacer) |
| :--- | :---: | :---: | :---: |
| **V4L2 OUTPUT Buffers** | 4 | 16 | **16** |
| **V4L2 CAPTURE Buffers** | 3 | 8 | **8** |
| **Fila Pós-Encoder** | `max-size=1 leaky=downstream` | `max-size=4` (lossless) | **`max-size=4` (lossless)** |
| **fdsink blocksize** | `65536` | Default | **Default (unbuffered atomic)** |
| **Wayland Damage Pacer** | Inativo | Inativo | **Ativo (60 Hz Heartbeat)** |
| **Latência Média Fim-a-Fim** | `> 250 ms` (picos de 500ms) | `11.45 ms` | **`11.45 ms`** |
| **Taxa de Quadros Sustentada** | 5 ~ 15 FPS (intermitente) | 60 FPS (em movimento) / 0 FPS (estático) | **60 FPS Cravados e Contínuos** |
| **Comportamento ao Parar Mouse** | Congelamento imediato | Congelamento após 100ms | **Perfeitamente Fluido e Ativo** |
| **Estabilidade de GOP H.264** | Corrupção frequente de macroblocos | Estável | **100% Íntegro e Cristalino** |

---

## 4. Topologia Final Validada

```
[Host Mutter Wayland]
        │
        ├── [damage_pacer (Rust In-Process)] ──> (Pulso 60 Hz 1x1 transparente)
        ▼
   [PipeWire Node] ──(60 FPS CFR)──> [vapostproc (VA-API)]
                                            │
                                            ▼
                                     [vah264enc 6000 kbps]
                                     (Constrained Baseline)
                                            │
                                            ▼
                                       [h264parse]
                                            │
                                            ▼
                                    [queue max-size=4] (Lossless)
                                            │
                                            ▼
                                     [fdsink sync=false]
                                            │
                                  (Pipe F_SETPIPE_SZ=262144)
                                            │
                                            ▼
                                   [rusb Endpoint 0x03]
                                            │ (0.05ms)
                                            ▼
                             [Pi Zero FunctionFS Endpoint 0x01]
                                            │
                                            ▼
                             [V4L2 M2M: 16 OUT / 8 CAP Buffers]
                             (VideoCore IV Hardware VPU @ 500 MHz)
                                            │
                                            ▼
                               [KMS DRM Scanout @ 60 FPS]
                                            │
                                            ▼
                                     [Monitor HDMI-1]
```

---

## 5. Conclusões e Diretrizes para Engenharia Futura

1. **Nunca use filas com descarte (`leaky`) em fluxos compactados H.264/HEVC.** Descarte de pacotes só pode ocorrer na fase RAW não-comprimida.
2. **Buffers de hardware V4L2 M2M em SoCs embarcados devem respeitar o pipeline de referências DPB.** 16 buffers OUTPUT e 8 buffers CAPTURE são os mínimos empíricos para 60 FPS no BCM2835.
3. **No Wayland (Mutter), o Damage Pacer é indispensável para segunda tela.** Sem um pulso de dano sintético, o compositor interrompe a renderização para economizar ciclos, induzindo a ilusão de travamento da tela.
