# Blueprint 29: Diagnóstico do Congelamento Periódico de 1s (Loop mDNS), Damage Pacer 100% Rust Puro e Scanout Direto Kernel DRM/KMS

> 🇧🇷 Versão em Português | [🇺🇸 English Version](../en/29-one-second-freeze-mdns-rust-pacer-and-kms-scanout.md)

*Data: 2026-10-02*  
*Status: Aprovado em Produção e Validado a 60 FPS Contínuos no Modo 1 (UDP) e Modo 3 (USB)*  
*Autor: Carlos Alberto <carlosalberto4ti@gmail.com>*  

---

## 1. Visão Geral e Demandas do Usuário

Durante a validação prática do **Ext-Monitor** em transmissão de vídeo contínua em rede (Modo 1: UDP RTP) e USB Bulk (Modo 3), três diretrizes essenciais foram estabelecidas:
1. **Regra de Ouro da Linguagem:** Eliminação total de interpretadores externos (Python). Todo o sistema deve ser **100% Rust puro em processo**.
2. **Eliminação do Congelamento Cíclico de 1 Segundo:** Um sintoma sutil onde a tela operava com alta fluidez, porém sofria um congelamento periódico de exatamente ~1 segundo em intervalos regulares, retomando a fluidez logo em seguida.
3. **Independência de Compositor (GNOME Mutter) e Scanout Direto via Kernel DRM/KMS:** Análise aprofundada da infraestrutura de captura de tela para garantir operação direta no kernel Linux, sem amarração obrigatória ao Mutter/PipeWire.

Este blueprint detalha as causas raízes de nível de rede, driver de kernel e gráficos, acompanhado das soluções definitivas implementadas em Rust.

---

## 2. Anatomia e Resolução do Congelamento de 1 Segundo

### 2.1 O Fenômeno de Tempestade de Eco Multicast mDNS (Multicast Loop Storm)
* **Sintoma Observado:** Em transmissão UDP (Modo 1), o vídeo reproduzia fluidamente por alguns momentos, mas a cada período fixo congelava por 1 segundo e retornava.
* **Inspeção de Telemetria no Hardware (Pi Zero):**
  A consulta das tabelas SNMP do kernel Linux do Raspberry Pi Zero via `/proc/net/snmp` revelou um cenário crítico de saturação:
  ```
  Udp: InDatagrams NoPorts InErrors OutDatagrams RcvbufErrors SndbufErrors InCsumErrors
  Udp: 13360937    365137  11875296 23759925     11875296     0            0
  ```
  * O contador `RcvbufErrors` acumulava **mais de 9,7 milhões de pacotes descartados**.
  * A taxa de descarte contínua excedia **3.300 pacotes UDP por segundo**.
  * O processo `ext-receiver` apresentava consumo de CPU anormalmente elevado (entre 83% e 100% de uso contínuo de CPU no único núcleo ARM1176JZF-S), elevando a temperatura do SoC para mais de 56 °C.

### 2.2 Causa Raiz: Loop Infinito no Responder mDNS
Ao auditar o módulo [`receiver/src/mdns.rs`](file:///home/carlos/ide/ext-monitor/receiver/src/mdns.rs), identificou-se que a thread de escuta mDNS (`224.0.0.251:5353`):
1. **Não desabilitava `IP_MULTICAST_LOOP`:** Fazendo com que o próprio socket recebesse seus próprios pacotes multicast de saída.
2. **Não validava o cabeçalho DNS (Bit QR):** Qualquer pacote recebido no socket com palavras-chave como `display`, `miracast` ou `pi-zero` era respondido emitindo outro pacote de resposta para o grupo multicast (`224.0.0.251:5353`).
3. **A Retroalimentação de Alta Frequência:** O responder recebia a sua própria resposta, interpretava como nova requisição, e emitia outra resposta multicast, criando um **loop infinito de eco em velocidade de barramento**.

### 2.3 Por que o Congelamento Durava Exatamente 1 Segundo?
A codificação H.264 do transmissor opera com:
$$\text{key-int-max} = 60 \quad \text{a} \quad 60\text{ FPS} \implies \text{GOP} = 1.0\text{ segundo}$$
* A tempestade de eco mDNS esgotava as filas do buffer de recepção UDP do kernel (`so_rcvbuf`).
* Quando os pacotes RTP de vídeo sofriam descarte pelo kernel (`RcvbufErrors`), ocorria a perda de fatias P-frame.
* O decodificador de hardware VideoCore IV (`bcm2835-codec`) descarta fatias que perderam o quadro de referência preditivo.
* **O hardware ficava travado esperando o próximo quadro IDR completo**, que é emitido a cada 1 segundo (60 frames).

### 2.4 Solução Aplicada no Código Rust
No arquivo [`receiver/src/mdns.rs`](file:///home/carlos/ide/ext-monitor/receiver/src/mdns.rs):
1. Desativação explícita de loopback multicast:
   ```rust
   let loop_opt: libc::c_int = 0;
   libc::setsockopt(
       fd,
       libc::IPPROTO_IP,
       libc::IP_MULTICAST_LOOP,
       &loop_opt as *const _ as *const libc::c_void,
       std::mem::size_of::<libc::c_int>() as libc::socklen_t,
   );
   ```
2. Validação do cabeçalho DNS para ignorar pacotes de resposta:
   ```rust
   let flags = u16::from_be_bytes([buf[2], buf[3]]);
   let is_query = (flags & 0x8000) == 0;
   if !is_query {
       continue; // Ignora respostas DNS, interrompendo o ciclo de eco
   }
   ```
3. Resposta enviada por unicast diretamente ao endereço e porta do solicitante (`socket.send_to(&response_packet, src)`), sem retransmitir para o multicast global.

### 2.5 Resultados Verificados Após o Fix
* `RcvbufErrors` no kernel do Pi Zero: **Zero descartes adicionais (delta = 0)**.
* Uso de CPU do Pi Zero: **Queda de ~83% para 0,28% a 0,74%**.
* Temperatura do hardware: **Redução de 56,2 °C para 48,2 °C**.
* Estabilidade do streaming de vídeo: **Zero congelamentos de 1 segundo**.

---

## 3. Damage Pacer 100% Rust Puro In-Process (Zero Python)

### 3.1 O Problema da Dependência de Scripts Externos
O uso de scripts Python em background (`wayland-damage-pacer.py`) contrariava a especificação do projeto de uma solução autocontida, de alta performance e estritamente compilada em Rust.

### 3.2 Arquitetura Nativa em Rust ([`sender/src/damage_pacer.rs`](file:///home/carlos/ide/ext-monitor/sender/src/damage_pacer.rs))
A implementação foi realizada via FFI direta com `libX11.so.6` e `libXext.so.6`, carregadas dinamicamente:
* **Janela Invisível Desanexada:** Janela de 1x1 pixel criada com `CW_OVERRIDE_REDIRECT = 1` (`override_redirect = 1`), impedindo que o gerenciador de janelas do GNOME tente decorar, reposicionar ou atribuir foco à janela.
* **Transparência de Entrada Total (100% Click-Through):** Configuração de `XShapeCombineRectangles(SHAPE_INPUT, 0, 0, NULL, 0, SHAPE_SET, 0)`, eliminando qualquer área de colisão de entrada. Cliques, toques e movimentações de mouse passam 100% direto para as janelas subjacentes.
* **Ciclo de Dano Ativo a 60 Hz:** A cada 16,6 ms, a thread Rust desenha um pixel alternado usando um Graphics Context dedicado (`XCreateGC`):
  ```rust
  let color = if tick % 2 == 0 { 0x00000000 } else { 0x00010101 };
  set_fg(display, gc, color);
  fill_rect(display, window, gc, 0, 0, 1, 1);
  (x11.clear_area)(display, window, 0, 0, 1, 1, 1);
  (x11.flush)(display);
  ```
* Essa alternância de valor de pixel força o Xwayland a emitir comandos `wl_surface.attach` e `wl_surface.commit` contínuos ao compositor Mutter, mantendo o clock do PipeWire a 60 FPS ininterruptos mesmo sem movimento de mouse.

---

## 4. Scanout Direto via Kernel DRM/KMS vs GNOME Mutter

### 4.1 O Conflito do DRM Master
O projeto visa a capacidade de capturar a tela diretamente do kernel Linux via Direct Rendering Manager (DRM/KMS), sem intermediação obrigatória do GNOME Mutter.

Durante a investigação com o probe [`src/bin/ext_kms_probe.rs`](file:///home/carlos/ide/ext-monitor/sender/src/bin/ext_kms_probe.rs), identificou-se:
1. Em sessões ativas do GNOME Wayland, o compositor Mutter assume o papel de **DRM Master** no dispositivo `/dev/dri/cardX`.
2. Quando um processo não-privilegiado tenta invocar `DRM_IOCTL_MODE_GETFB2` para extrair os handles de memória do scanout da tela, o kernel Linux retorna erro de permissão ou `None`.

### 4.2 Prova de Conceito: Exportação de PRIME DMA-BUF com `CAP_SYS_ADMIN`
Ao executar o probe com permissão administrativa (`sudo` ou atribuindo a capability `cap_sys_admin+ep` ao binário compilado), o probe obteve sucesso imediato:
```
[INFO] Device /dev/dri/card1: Driver 'amdgpu'
[INFO] Found CRTC 368 with Framebuffer ID 416
[INFO] FB 416: 1280x720, format=AR24, pitches=[5120], offsets=[0]
[SUCCESS] buffer_to_prime_fd exported: OwnedFd { fd: 4 } directly from CRTC scanout!
```
* O descritor de arquivo DMA-BUF PRIME é exportado diretamente da memória de vídeo (VRAM) do hardware sem copiar um único byte via CPU.
* Para habilitar o sender nativo a operar com scanout direto do kernel em estações Linux:
  ```bash
  sudo setcap cap_sys_admin+ep /usr/local/bin/ext-sender
  ```

---

## 5. Topologia Final de Alta Eficiência

```
+------------------------------------------------------------------+
|                            HOST LINUX                            |
|                                                                  |
|  [Damage Pacer (Rust In-Process)] ---> Mantém Clock Mutter 60Hz  |
|                                                                  |
|  [KMS Scanout / PRIME DMA-BUF]   ou   [GNOME Mutter / PipeWire]  |
|            (CAP_SYS_ADMIN)                      |                |
|                    \                           /                 |
|                     +-------------------------+                  |
|                                  |                               |
|                                  v                               |
|                         [VA-API H.264 Encoder]                   |
|                        (6000 kbps, Low-Latency)                  |
|                                  |                               |
|                                  v                               |
|                         [Lossless UDP Queue]                     |
|                                  |                               |
|                   [UDP Socket 512KB Buffer]                      |
+----------------------------------+-------------------------------+
                                   | (Rede Local / USB Ethernet)
                                   v
+----------------------------------+-------------------------------+
|                      RASPBERRY PI ZERO W                         |
|                                                                  |
|  [mDNS Responder (Sem Loop Multicast / Sem Bufferbloat)]         |
|                                                                  |
|  [V4L2 M2M VideoCore IV Decoder]                                 |
|  (16 OUTPUT Buffers / 8 CAPTURE Buffers @ 500 MHz VPU)           |
|                                  |                               |
|                                  v                               |
|               [DRM/KMS Hardware Plane Scanout]                   |
|                                  |                               |
|                                  v                               |
|                   [Monitor HDMI: 1280x720 @ 60Hz]                |
+------------------------------------------------------------------+
```

---

## 6. Sumário de Diretrizes de Engenharia

1. **Protocolos Multicast (mDNS/SSDP) em SoCs Embarcados:** Sempre desative `IP_MULTICAST_LOOP` e valide rigorosamente o bit QR antes de responder. Loops de eco geram descarte de pacotes no kernel e derrubam o stream de vídeo.
2. **Damage Pacing Sob Wayland:** O pacer deve residir estritamente em processo Rust, desenhando via X11 GC sem gerenciar janelas (`override_redirect = 1`) e com 100% de transparência a cliques (`SHAPE_INPUT` vazio).
3. **Acesso Direto DRM/KMS:** O binário transmissor com `cap_sys_admin+ep` é capaz de realizar scanout direto de buffers PRIME DMA-BUF da GPU, proporcionando autonomia em relação ao compositor gráfico do desktop.
