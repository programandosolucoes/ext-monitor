# Blueprint 24: Resolução Nativa 720p60 WFD, Nível 3.1 e Demuxer MPEG-TS PES Determinista

## 1. Visão Geral e Contexto do Problema

No **Modo 2 (Miracast / Wi-Fi Display / MS-MICE)**, observavam-se dois sintomas críticos reportados pelo usuário:
1. **Lentidão Severa e Atraso (Lag):** A imagem demorava múltiplos quadros para atualizar, com latência perceptível de centenas de milissegundos a segundos, em forte contraste com o **Modo 1 (Rede UDP)** e o **Modo 3 (USB Bulk Direct)**, que operam a < 15ms @ 60 FPS com suavidade de mouse e vídeo em tempo real.
2. **Artefatos Visuais e Corrupção de Macroblocos:** Falhas e quebras parciais da imagem a cada atualização de tela ("qualquer update falha a imagem", faixas verdes ou macroblocos rasgados).

O usuário destacou com precisão a premissa de arquitetura:
> *"Até onde eu tinha entendido era só colocar o protocolo do miracast aplicando em cima da lógica usada no modelo de rede e de usb bulk."*

A investigação detalhada do código-fonte e da engenharia reversa do protocolo revelou as **duas causas-raiz exatas** que diferenciavam o Miracast dos modos 1 e 3.

---

## 2. Causa-Raiz 1: Sobrecarga de Resolução (1080p60 vs. 720p60 Nativo)

### 2.1 Análise de Throughput e Barramento do BCM2835
No Raspberry Pi Zero W (SoC Broadcom BCM2835, CPU ARMv6 single-core de 1 GHz, 512 MB de RAM LPDDR2 unificada compartilhada entre CPU e GPU com largura de banda de ~1.6 GB/s):
- **Modo 1 e Modo 3:** Operam com resolução estrita de **1280x720 @ 60 FPS**.
  - Taxa de pixels: `1280 * 720 * 60 = 55.296.000` pixels/s (~55 Mpix/s).
  - Tamanho do buffer de quadro NV12: `1.382.400` bytes (1.38 MB).
  - Escalonamento no monitor: 1:1 direto para o CRTC HDMI (1280x720).
- **Modo 2 (Anterior):** O parâmetro `WFD_VIDEO_FORMATS` anunciava bitmaps CEA `0001deff` e VESA `157cff5f`. Esses bitmaps continham os bits correspondentes a **1920x1080 @ 60 FPS** e **1920x1080 @ 30 FPS**.
  - Clientes Miracast padrão (como o `gnome-network-displays` ou o Projeção Sem Fio do Windows 10/11) selecionavam automaticamente a maior resolução anunciada pelo receptor: **1920x1080 @ 60 FPS**.
  - Taxa de pixels em 1080p60: `1920 * 1080 * 60 = 124.416.000` pixels/s (**124.4 Mpix/s — 2,25 vezes maior**).
  - Tamanho do buffer NV12: `3.133.440` bytes (3.13 MB por frame).
  - O hardware VideoCore IV do Pi Zero é dimensionado para até 1080p30 ou 720p60. Decodificar 1080p a 60 quadros por segundo satura a fila de DMA da VPU, estoura o barramento de memória e gera atraso cumulativo inaceitável.

### 2.2 Estrutura Binária de `WFD_VIDEO_FORMATS` e Correção
A especificação Wi-Fi Display (WFD 1.1.0, Tabela 5-11 e 5-12) define a string RTSP `wfd_video_formats`:
```text
[Native Index] [Preferred Mode] [Profile] [Level] [CEA Bitmap] [VESA Bitmap] [HH Bitmap] [Latency] [Min Slice] [Slice Enc] [FRC] none none
```

A engenharia reversa no binário do `gnome-network-displays` revelou a rotina de desempacotamento de resolução nativa:
```assembly
1f994: mov %r14d, %eax        # Carrega 1º byte hex de wfd_video_formats
1f997: sar $0x3, %eax         # Desloca 3 bits para a direita (índice da tabela)
1f99a: cmp %ecx, %eax         # Compara com o tamanho da tabela CEA (27 entradas)
```
- Bits `[2..0]`: Seleção da Tabela (`000` = CEA-861, `001` = VESA, `010` = HH).
- Bits `[7..3]`: Índice da resolução na tabela selecionada (`Índice << 3`).
- Na tabela CEA do WFD:
  - Índice 0: 640x480p60
  - Índice 3: 720x576p50
  - Índice 5: 1280x720p30
  - **Índice 6: 1280x720p60** -> Em binário: `00110` (6). Deslocado 3 bits: `00110 000b` = **0x30**.

A configuração otimizada e implementada em [`receiver/src/wfd.rs`](file:///home/carlos/ide/ext-monitor/receiver/src/wfd.rs):
```rust
pub const WFD_VIDEO_FORMATS: &str =
    "30 00 01 02 00000069 00000000 00000000 00 0000 0000 00 none none";
```
1. **`30`**: Seleciona expressamente a resolução nativa **CEA Índice 6 (1280x720 @ 60 Hz)**.
2. **`01`**: Perfil H.264 *Constrained Baseline Profile* (CBP), sem quadros B, garantindo latência zero.
3. **`02`**: Nível H.264 *Level 3.1*, cujo limite padrão de macroblocos por segundo proíbe transmissões em 1080p, forçando o codificador do host a enviar 720p.
4. **`00000069`**: Máscara CEA habilitando apenas 720p60 (`0x40`), 720p30 (`0x20`), 576p50 (`0x08`) e 480p60 (`0x01`). Nenhuma resolução 1080p é autorizada.
5. **`00000000`**: Máscaras VESA e Handheld zeradas, impedindo 1080p via VESA.

---

## 3. Causa-Raiz 2: Demultiplexação de Pacotes MPEG-TS e Montagem de Quadros (PES vs. Annex-B)

### 3.1 A Fragilidade do `AnnexBAssembler` em Fluxos MPEG-TS
Anteriormente, o `TsDemuxer` extraía os bytes de Elementary Stream de cada pacote TS e os repassava para o `AnnexBAssembler`.
O `AnnexBAssembler` foi concebido para fluxos contínuos de bytes sem demarcação de pacotes (como leitura contínua de pipes USB Bulk). Por isso, ele utilizava heurísticas como:
- Varredura byte-a-byte em busca de códigos de início `00 00 01` / `00 00 00 01`.
- Inspeção do cabeçalho da fatia para inferir início de quadro: `(nal_body[1] & 0x80) != 0` (`first_mb_in_slice == 0`).
- Retenção do quadro em buffer até a chegada do próximo quadro ou estouro de timeout do `poll(15ms)`.

**Falhas induzidas por essa abordagem no Miracast:**
1. Em quadros P únicos (sem AUD ou SPS/PPS recorrente), o `AnnexBAssembler` não conseguia confirmar o encerramento do quadro até que o próximo quadro chegasse ou até que o timeout de 15ms disparasse. Isso adicionava **15 a 33 ms de atraso artificial a cada quadro**.
2. Quando pacotes chegavam em rajada, o timeout de 15ms disparava no meio de fatias ou cortava fatias subsequentes, limpando o acumulador e descartando o final da imagem.
3. Isso alimentava a VPU VideoCore IV com bitstreams truncados, provocando **corrupção de macroblocos e faixas verdes persistentes**.

### 3.2 O Princípio Determinista da WFD (Seção 5.3.3)
Segundo a especificação formal do Wi-Fi Display (WFA WFD 1.1.0, Seção 5.3.3):
> *"The WFD Source shall encapsulate each video Access Unit (AU) in one PES packet."*
*(A fonte WFD encapsulará cada Access Unit (quadro completo) de vídeo em exatamente um pacote PES).*

Portanto, em fluxos de Miracast:
1. O bit **PUSI = 1** (*Payload Unit Start Indicator*) no cabeçalho do pacote TS indica o início exato de um novo pacote PES e, portanto, **o fim estrito do quadro anterior**.
2. O bit **RTP Marker (M = 1)** no pacote UDP de transporte sinaliza a transmissão do último pacote TS pertencente àquele quadro.
3. Não há necessidade alguma de heurísticas de busca de início de NAL ou de teste de bits Exp-Golomb. O quadro anterior já está 100% completo e pode ser emitido de imediato para a decodificação de hardware.

### 3.3 Nova Arquitetura do `TsDemuxer`
O novo [`receiver/src/stream/ts.rs`](file:///home/carlos/ide/ext-monitor/receiver/src/stream/ts.rs) implementa:
1. **Descarte de Heurística:** Eliminação total do `AnnexBAssembler`.
2. **Delimitação Direta por PES e RTP Marker:**
   - Ao receber `PUSI = 1` no PID de vídeo: se `current_au` contiver bytes e não estiver corrompido, emite o quadro imediatamente para `frames_out`, limpa o acumulador e inicia o novo quadro a partir de `9 + pes_header_data_len`.
   - Ao processar pacote RTP com `Marker = 1`: emite o quadro imediatamente ao término do processamento dos pacotes TS contidos no datagrama.
   - No método `flush()`: emite o quadro pendente em caso de tela estática.
3. **Validação de `Continuity Counter` (CC):**
   - Rastreia o contador cíclico de 4 bits (`0..15`) em cada pacote TS com payload.
   - Detecta perda de pacotes na rede UDP (`cc != (prev_cc + 1) & 0x0F`).
   - Ao detectar perda, marca `frame_corrupted = true` e descarta o quadro incompleto, impedindo que macroblocos danificados entrem na VPU do Raspberry Pi e poluam os quadros seguintes.

---

## 4. Otimização do Ciclo de Decodificação e Splash Screen

1. **Dimensões Padrão 720p em [`miracast.rs`](file:///home/carlos/ide/ext-monitor/receiver/src/ingress/miracast.rs):**
   - Inicializa `current_dims` em `(1280, 720)` e cria `V4l2DecoderSession::new(1280, 720)`.
   - O plano KMS DRM (Plano 86 no CRTC 97) é configurado imediatamente em 1280x720 NV12, eliminando reconfigurações de decodificador no primeiro quadro recebido.
2. **Timeout de Poll Reduzido:**
   - Reduzido de 15ms para 5ms para atualização imediata de telas estáticas (mouse parado).
3. **Proteção da Splash Screen:**
   - Nos trabalhadores [`udp.rs`](file:///home/carlos/ide/ext-monitor/receiver/src/ingress/udp.rs) e [`usb.rs`](file:///home/carlos/ide/ext-monitor/receiver/src/ingress/usb.rs), a exibição da tela de Ready Splash ao encerrar a rotina agora verifica se o modo ativo continua sendo o respectivo modo. Isso impede que o encerramento do Modo 1 sobrescreva a tela de guia do Miracast (Modo 2).

---

## 5. Verificação e Resultados

1. **Compilação e Testes Unitários:**
   - 18 testes unitários do `ext-receiver` aprovados com 100% de sucesso, incluindo o teste de demarcação PES e montagem de AU (`test_ts_demuxer_pes_demarcation`).
2. **Atualização OTA do Appliance:**
   - Novo `initramfs.cpio.gz` gerado com target `arm-unknown-linux-musleabihf` (SHA256: `b107db0b...`).
   - Gravado no cartão SD e inicializado na RAM do Pi Zero.
   - Binário verificado em execução: `/usr/local/bin/ext-receiver` com SHA256 idêntico (`9fe21c42...`).
3. **Descoberta e Conexão:**
   - Avahi mDNS anunciando serviço `_display._tcp` na porta 7250 em `pi-zero.local:7250`.
   - `gnome-network-displays` localizou o "Raspberry Pi Miracast" via interface de rede `192.168.7.2`.
   - Splash dedicado do Miracast renderizado com sucesso no monitor HDMI (`print_pi_zero_miracast_standby_new.png`).
