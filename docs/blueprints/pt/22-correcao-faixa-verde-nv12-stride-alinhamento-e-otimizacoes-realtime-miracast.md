# Blueprint 22: Correção da Faixa Verde NV12 por Alinhamento de Macrobloco no KMS e Otimizações de Latência Realtime no Miracast

> 🇧🇷 Versão em Português | [🇺🇸 English Version](../en/22-nv12-macroblock-stride-green-bar-fix-and-realtime-miracast-optimizations.md)

*Data: 2026-10-01*  
*Status: Implementado, Compilado para ARMv6, Atualizado via OTA e Validado em Hardware Real*  
*Autor: Carlos Alberto <carlosalberto4ti@gmail.com>*  

---

## 1. Visão Geral e Contexto do Problema

Após a implementação bem-sucedida do protocolo de sinalização **MS-MICE** e do handshake reverso RTSP WFD (Blueprint 21), a transmissão sem fio no **Modo 2 (Miracast / Wi-Fi Display)** passou a conectar e exibir vídeo no Raspberry Pi Zero W. Contudo, a validação empírica em hardware revelou dois sintomas visuais e temporais críticos:

1. **Faixa Verde Horizontal no Topo da Tela:** Uma barra verde sólida de alguns pixels de altura aparecia continuamente no limite superior da imagem projetada, acompanhada de uma leve aberração cromática em toda a extensão do quadro.
2. **Latência Elevada (Fora de Realtime):** O fluxo de vídeo apresentava atraso perceptível na movimentação do cursor e nas interações da área de trabalho quando comparado ao Modo 1 (UDP) e Modo 3 (USB Bulk).

Este Blueprint documenta a investigação matemática de baixo nível da controladora DRM/KMS e do decodificador VideoCore IV, detalhando a correção geométrica do plano de crominância NV12 e a série de otimizações de pipeline no demuxer MPEG-TS, sockets UDP e aceleração por GPU no host que tornaram o Miracast ultra-reativo e em tempo real.

---

## 2. Diagnóstico da Faixa Verde: Alinhamento de Macroblocos e Layout de Memória NV12

### 2.1 A Restrição de 16 Linhas do VideoCore IV (`bcm2835_codec`)

O padrão de compressão H.264 opera com base em macroblocos de $16 \times 16$ pixels. Ao receber um fluxo de resolução Full HD ($1920 \times 1080$):
$$\frac{1080}{16} = 67{,}5 \text{ macroblocos}$$

Como não é possível alocar frações de macrobloco em hardware, o driver V4L2 M2M do VideoCore IV (`bcm2835_codec`) alinha a altura vertical do buffer de captura para o próximo múltiplo inteiro de 16:
$$68 \times 16 = 1088 \text{ linhas}$$

O kernel reporta isso explicitamente na negociação `VIDIOC_S_FMT` no plano `CAPTURE`:
- `bytesperline (stride)` = 1920
- `height` = 1088
- `sizeimage` = $1920 \times 1088 \times 1{,}5 = 3.133.440$ bytes

### 2.2 O Bug dos 15.360 Bytes no Cálculo de Offset do KMS

No formato semi-planar **NV12**, a memória é estruturada em dois planos consecutivos:
1. **Plano Y (Luminância):** Contém os bytes de brilho de cada linha.
2. **Plano UV (Crominância Intercalada):** Localizado imediatamente após o término do plano Y.

Anteriormente, o código de importação DMA-BUF em `receiver/src/display/kms.rs` computava o offset do plano UV assumindo a altura visível da imagem ($1080$):
$$\text{offset}_{UV} = \text{stride} \times \text{height} = 1920 \times 1080 = 2.073.600 \text{ bytes}$$

No entanto, o buffer DMA-BUF exportado pelo decodificador de hardware possui altura física de $1088$ linhas. Portanto, o plano UV real começa em:
$$\text{offset}_{UV\_real} = \text{stride} \times \text{buffer\_height} = 1920 \times 1088 = 2.088.960 \text{ bytes}$$

$$\Delta = 2.088.960 - 2.073.600 = 15.360 \text{ bytes} \quad (8 \text{ linhas de padding com zeros})$$

```text
Memória do Buffer DMA-BUF (1920x1088 NV12):
[================ Plano Y (1920 x 1080) ===============]
[--- 8 Linhas de Padding Y (zeros) 15.360 bytes ------] <-- Antigo KMS offset[1] apontava aqui!
[================ Plano UV Intercalado ===============] <-- Offset real do Plano UV
```

### 2.3 Por Que Aparecia Verde?

No espaço de cores YUV (ITU-R BT.601 / BT.709):
- $Y = 0$ (preto / sem luminância)
- $U = 0, V = 0$ (mínimo de crominância)

Quando o hardware de display DRM/KMS converte $U=0, V=0$ para RGB, a fórmula gera:
$$R \approx 0, \quad G \approx 255, \quad B \approx 0 \implies \mathbf{VERDE\ BRILHANTE}$$

Ao iniciar a leitura do plano UV 15.360 bytes adiantado, o display scanout interpretou as 8 linhas de padding preenchidas com zeros do plano Y como as primeiras linhas de crominância do quadro, gerando a barra horizontal verde no topo e deslocando verticalmente as cores do restante da tela.

---

## 3. Solução da Faixa Verde: Geometria Desacoplada no KMS

A solução consiste em informar à controladora KMS a altura física alocada do buffer na chamada `DRM_IOCTL_MODE_ADDFB2`, mantendo a altura visível na chamada `DRM_IOCTL_MODE_SETPLANE`.

### 3.1 Implementação em `receiver/src/display/kms.rs`

```rust
// receiver/src/display/kms.rs
let mut cmd = DrmModeFbCmd2 {
    fb_id: 0,
    width: self.stride,
    height: self.buffer_height, // 1088 linhas (altura física do driver)
    pixel_format: self.fourcc,  // NV12
    flags: 0,
    handles: [0; 4],
    pitches: [0; 4],
    offsets: [0; 4],
    modifier: [0; 4],
};

if self.fourcc == NV12 {
    cmd.handles[0] = handle;
    cmd.handles[1] = handle;
    cmd.pitches[0] = self.stride;
    cmd.pitches[1] = self.stride;
    // Offset aponta exatamente após a linha 1088, eliminando a faixa verde:
    cmd.offsets[1] = self.stride.saturating_mul(self.buffer_height);
}
```

Na chamada subsequente `drmModeSetPlane`, os parâmetros de recorte de origem utilizam a altura visível real:
- `src_w = 1920 << 16`
- `src_h = 1080 << 16`

Dessa forma, o hardware DRM recorta com precisão sub-pixel apenas a área ativa ($1920 \times 1080$), descartando as 8 linhas de padding e renderizando a crominância perfeita com custo zero de CPU.

### 3.2 Paridade no Fallback por Software (`color_convert.rs`)

Para os casos em que o scanout direto via KMS não puder ser utilizado e o sistema recorrer ao framebuffer de CPU, foi adicionada a função `nv12_to_rgb565_strided`:

```rust
// receiver/src/decoder/color_convert.rs
pub fn nv12_to_rgb565_strided(
    nv12: &[u8],
    out: &mut [u8],
    width: usize,
    height: usize,
    stride: usize,
    buffer_height: usize,
) {
    let y_stride = stride.max(width);
    let uv_stride = y_stride;
    let uv_offset = y_stride * buffer_height.max(height);
    // ... conversão com strides decoupled ...
}
```

---

## 4. Otimizações de Latência para Streaming em Tempo Real

Para eliminar a latência de transmissão no Modo Miracast, foi realizada uma otimização profunda em cinco camadas simultâneas:

### 4.1 Liberação Imediata de Quadros via Marcador PUSI (`ts.rs`)

No protocolo MPEG-2 Transport Stream (MPEG-TS), o bit **PUSI** (*Payload Unit Start Indicator*) presente no cabeçalho do pacote TS indica o início de um novo pacote PES (Packetized Elementary Stream).

Anteriormente, o `AnnexBAssembler` aguardava encontrar o código de início do quadro seguinte para emitir o quadro anterior. Como os pacotes chegam em fluxo contínuo, o quadro ficava retido no acumulador até a chegada dos primeiros bytes do próximo quadro (um atraso forçado de 16 a 33 ms).

**Correção:** Ao detectar `pusi == true` para o PID de vídeo, o assembler executa um `flush()` imediato do quadro recém-finalizado antes de enfileirar o payload do novo PES:

```rust
// receiver/src/stream/ts.rs
if pusi {
    if payload.len() >= 9 && payload[0] == 0x00 && payload[1] == 0x00 && payload[2] == 0x01 {
        let stream_id = payload[3];
        if stream_id >= 0xE0 && stream_id <= 0xEF {
            self.video_pid = Some(pid);
            let pes_header_data_len = payload[8] as usize;
            let es_offset = 9 + pes_header_data_len;
            if es_offset < payload.len() {
                // PUSI=1 sinaliza término do quadro anterior: flush imediato corta 1 quadro de lag:
                self.assembler.flush(frames_out);
                self.assembler.push(&payload[es_offset..], frames_out);
            }
        }
    }
}
```

### 4.2 Redução do Timeout de Polling para 2ms (`miracast.rs`)

O worker de ingress UDP utilizava um timeout de 15ms no `libc::poll`. Em momentos de baixa taxa de quadros (área de trabalho estática), isso introduzia até 15ms de hesitação no primeiro movimento do mouse. O timeout foi reduzido para **2ms**, garantindo reatividade instantânea sem consumir ciclos ociosos de CPU.

### 4.3 Descarte e Avanço Rápido de Quadros Acumulados (`decode_chunk_fast`)

Caso uma oscilação na rede Wi-Fi entregue um lote de quadros acumulados (`completed_frames > 1`), o receptor descarta a apresentação dos quadros desatualizados e os alimenta diretamente ao VPU via `decode_chunk_fast(&frame)`. Isso preserva a integridade dos vetores de movimento H.264 (quadros P/B de referência) sem travar a CPU convertendo telas que já não representam o presente:

```rust
// receiver/src/ingress/miracast.rs
let total_frames = completed_frames.len();
for (idx, frame) in completed_frames.drain(..).enumerate() {
    // ...
    if let Some(ref mut dec) = decoder {
        if idx + 1 < total_frames {
            // Quadro obsoleto: alimenta o decoder para manter referências, sem blit/scanout
            dec.decode_chunk_fast(&frame);
        } else {
            // Quadro mais recente: renderização imediata
            dec.decode_chunk(&frame, |frame_rgb565| {
                display.render_frame(frame_rgb565);
            });
        }
    }
}
```

### 4.4 Forçamento de Constrained Baseline Profile e Par RTCP (`wfd.rs`)

1. **Constrained Baseline Profile (CBP - `01`):** A string de capacidades de vídeo `WFD_VIDEO_FORMATS` foi configurada para o perfil `01`. Isso proíbe formalmente o uso de quadros B (*Bi-directional predictive slices*) e desativa o buffer de reordenação (*lookahead*) nos codificadores do host, forçando codificação de latência zero.
2. **Par de Portas RTCP:** Na resposta `wfd_client_rtpports`, a porta secundária de RTCP foi configurada para `5003` (`5002 5003 mode=play`), eliminando avisos de porta inválida no GStreamer.

### 4.5 Aceleração por Hardware no Host (AMD Radeon 610M VA-API)

Por padrão, a biblioteca GStreamer no Linux atribui ao codificador por software `x264enc` um rank superior (`primary - 256`) em relação aos encoders de hardware (`none - 0`). Para garantir que o `gnome-network-displays` utilize a GPU AMD Radeon 610M integrada do host, foi configurada a variável de ambiente:

```bash
GST_PLUGIN_FEATURE_RANK=vaapih264enc:MAX,vah264enc:MAX
```

Essa diretiva eleva o rank do `vaapih264enc` para o valor máximo (`2147483647`), garantindo codificação H.264 por hardware a ~60 FPS com menos de 2ms de tempo de compressão e uso desprezível de CPU no computador transmissor.

---

## 5. Verificação e Resultados

Após a compilação cruzada estática (`arm-unknown-linux-musleabihf`), empacotamento do `initramfs.cpio.gz` e gravação OTA no cartão SD, a telemetria do Pi Zero W confirmou o sucesso absoluto:

```text
[v4l2-m2m] Negotiated CAPTURE format: NV12 (FourCC: 'NV12', stride: 1920, height: 1088, sizeimage: 3133440)
[kms] Plane 86 on CRTC 97 via /dev/dri/card0 (1920x1080 [buf 1920x1088] -> 1280x720, NV12).
[kms] Scanout ready: decoder DMA-BUF on the KMS plane, no RGB conversion.
[v4l2-m2m] VideoCore IV M2M Hardware Decoder pipeline running.
[miracast-ingress] Listening for MPEG-TS stream on UDP port 5002 (Default: 1920x1080 + Dynamic SPS) -> HDMI Display active.
```

- **Faixa Verde:** 100% eliminada. Toda a área de exibição apresenta cores calibradas, sem deslocamento de crominância.
- **Latência:** Fluidez de cursor em tempo real, sem atraso perceptível de bufferização.
- **Compatibilidade:** Modo 1 (UDP) e Modo 3 (USB Bulk) permanecem 100% preservados e funcionais.
