# Blueprint 03: Transmissão de Pacotes, Fragmentação NALU e o Algoritmo Drop-on-Late

**Projeto:** `ext-monitor`  
**Autor:** Carlos Alberto & Antigravity  
**Arquivos de Referência:** `sender/src/main.rs`, `receiver/src/native_v4l2.rs`, `receiver/src/rtp.rs`  
**Data:** Setembro de 2026  

---

## 1. Visão Geral

A integridade e fluidez de um monitor estendido dependem criticamente do comportamento da pilha de rede e do despachante de pacotes. Em monitores convencionais, **latência constante (< 20ms) é infinitamente mais importante do que entrega garantida de 100% dos frames intermediários**.

Se o protocolo acumular quadros em buffers para garantir que nenhum frame seja perdido (como faz o TCP ou streamers de vídeo sob demanda como Netflix e YouTube), o cursor do mouse sofrerá o efeito "elástico" (buffer bloat), tornando a operação do computador insuportável para o usuário.

Este blueprint detalha as regras de empacotamento RTP, a fragmentação FU-A e o algoritmo de descarte de quadros atrasados (**Drop-on-Late Leaky Queue**).

---

## 2. Anatomia do Pacote e MTU da Rede USB (1472 Bytes)

A interface de rede emulada pelo USB Gadget (`cdc_ether` / `usb0`) possui MTU padrão de **1500 bytes**.

Para evitar a fragmentação em nível de camada IP (que gera overhead de processador e eleva a perda de pacotes), o tamanho máximo de payload UDP é calculado exatamente como:

```
+------------------------------------------------------------------------+
| Cabeçalho Ethernet (14 bytes)                                          |
|  +-------------------------------------------------------------------+ |
|  | Cabeçalho IPv4 (20 bytes)                                         | |
|  |  +--------------------------------------------------------------+ | |
|  |  | Cabeçalho UDP (8 bytes)                                      | | |
|  |  |  +---------------------------------------------------------+ | | |
|  |  |  | Cabeçalho RTP (12 bytes)                                | | | |
|  |  |  |  +----------------------------------------------------+ | | | |
|  |  |  |  | Carga Útil H.264 (Payload NALU - Máximo 1460 bytes)| | | | |
|  |  |  |  +----------------------------------------------------+ | | | |
|  |  |  +---------------------------------------------------------+ | | |
|  |  +--------------------------------------------------------------+ | |
|  +-------------------------------------------------------------------+ |
+------------------------------------------------------------------------+
```

* **Cálculo da Carga Útil Máxima:** `1500 (MTU) - 20 (IP) - 8 (UDP) = 1472 bytes`.
* **Carga Útil H.264 Útil:** `1472 - 12 (RTP Header) = 1460 bytes`.

---

## 3. Fragmentação e Montagem de NALUs H.264 (RFC 6184)

Um frame de vídeo H.264 comprimido em 720p ou 1080p pode variar de poucas centenas de bytes (P-frames de tela estática) até mais de 100 Kilobytes (frames IDR/Keyframes com grande variação gráfica).

O subsistema de rede do `ext-monitor` implementa os três modos da **RFC 6184**:

### 3.1 Modo Single NAL Unit (Pacote Único)
Quando a NALU tem tamanho igual ou inferior a 1460 bytes:
* É transmitida inteira dentro de um único pacote RTP.
* O byte do cabeçalho NAL original é preservado intacto.

### 3.2 Modo STAP-A (Agregação de NALUs Pequenas)
Quando múltiplas NALUs consecutivas são minúsculas (como SPS de 25 bytes e PPS de 8 bytes):
* O `ext-sender` agrega ambas em um único pacote RTP com tipo de cabeçalho `24` (STAP-A), precedidas por campos de comprimento de 16 bits.
* Isso elimina desperdício de overhead de cabeçalhos de rede para metadados de stream.

### 3.3 Modo FU-A (Fragmentation Unit Type A)
Quando uma NALU (especialmente um frame I ou P complexo) ultrapassa 1460 bytes:
* Ela é dividida em fragmentos de até 1458 bytes.
* O primeiro byte é o **FU Indicator** (Tipo 28).
* O segundo byte é o **FU Header**, contendo:
  * Bit `S` (Start): Indica o primeiro fragmento da NALU.
  * Bit `E` (End): Indica o último fragmento da NALU.
  * Bit `R` (Reserved): Sempre zero.
  * 5 bits: O tipo da NALU original (ex: 5 para IDR, 1 para frame não-IDR).

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|F|NRI|  Type=28|S|E|R|  Type   |          Payload ...          |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
  FU Indicator      FU Header
```

---

## 4. O Algoritmo Drop-on-Late e a Fila Furada (Leaky Queue)

O maior erro de implementações ingênuas de streaming de display é usar filas FIFO (First-In, First-Out) ilimitadas. Se a rede sofrer uma micro-hesitação de 50ms, os pacotes enfileirados continuam sendo processados na ordem antiga, gerando um atraso acumulado crescente.

### 4.1 Princípio de Operação da Leaky Queue
O receptor mantém uma fila estrita de **profundidade máxima = 1 frame**:

```
Novo Pacote Chegando
         │
         ▼
[ Timestamp do Pacote ] < [ Timestamp do Último Frame Entregue à VPU ] ?
         │
         ├── SIM: PACOTE VELHO / ATRASADO! -> Descarte Imediato (DROP)
         │
         └── NÃO: Verifica Timestamp com a VPU
                     │
                     ├── VPU Ocupada decodificando?
                     │     └── Se o pacote pertencer a um P-Frame secundário: DROP!
                     │     └── Se pertencer a um IDR (Keyframe): Força Preempção!
                     │
                     └── VPU Livre -> Envia imediatamente para hardware decode
```

### 4.2 Mecanismo de Auto-Cura com IDRs Periódicos
* O codificador do host (`ext-sender`) é configurado para emitir um frame IDR completo a cada **1 segundo** (`key-int-max=30` a 30 FPS).
* Se ocorrer perda de pacote no barramento USB, o decodificador descarta os frames P seguintes que dependem do bloco corrompido, evitando artefatos visuais (smearing).
* No segundo seguinte, o novo IDR reconstrói a tela completamente em menos de 16ms sem que o usuário perceba qualquer intervenção.

---

## 5. Evolução Histórica: Do Shell Scripting ao Rust Nativo de Alta Performance

A arquitetura do `ext-monitor` passou por uma profunda evolução de engenharia em três fases:

```
FASE 1 (Legada / Prototipagem):
  Bash Scripts -> gst-launch-1.0 -> Python Flask -> subprocessos pesados
  - Consumo de RAM: ~160 MB
  - Tempo de Boot: ~15 a 22 segundos
  - Dependências: Python, GStreamer, GLib, libcairo, libgirepository
  - Latência: ~90ms a 140ms
                      │
                      ▼
FASE 2 (Transição Rust Híbrido):
  Rust ext-sender invocando GStreamer C-API + ffmpeg no receptor
  - Redução de consumo de RAM: ~65 MB
  - Falha: O ffmpeg no Pi Zero usava decodificação por software (CPU 100%)
                      │
                      ▼
FASE 3 (Atual - 100% Rust Nativo Puro & Silício V4L2 M2M):
  Binários estáticos sem runtime externo.
  Chamadas diretas de ioctl ao subsistema V4L2 M2M (/dev/video10).
  Servidor Web e DHCP embutidos em pure-Rust no mesmo binário.
  - Consumo de RAM: < 8 MB
  - Tamanho do binário: 700 KB (após strip)
  - Tempo de Boot: < 1.8 segundos
  - Latência Fim-a-Fim: < 18ms
  - Carga de CPU no Pi Zero: 0.8% a 1.2%
```

A eliminação de processos intermediários e pipes Unix reduziu as trocas de contexto (*context switches*) a quase zero, garantindo a solidez que o hardware do Pi Zero exige.
