# Blueprint 25: Parada Unificada de Serviços (Standby), Teardown RTSP Miracast e Encerramento de Transmissores Host

## 1. Visão Geral e Contexto do Problema
Durante a operação no Modo 2 (Miracast) ou outros modos de transmissão, o acionamento do botão **"⏹ Disable Extension / Standby"** no Web Dashboard não interrompia a transmissão de vídeo por completo.

### Sintomas Identificados:
1. **Transmissor Host Ativo:** O `gnome-network-displays` (ou cliente de streaming local) permanecia em execução no laptop, mantendo a codificação VAAPI/GStreamer em loop contínuo e disparando pacotes UDP para a porta 5002.
2. **Sessão RTSP Órfã no Pi Zero:** O receptor (`ext-receiver`) pausava o pipeline interno via `pipeline_mgr.pause()`, porém não mantinha controle sobre as conexões TCP/RTSP ativas criadas pelo `wfd.rs`, de modo que a sessão Wi-Fi Display (WFD) continuava aberta e aguardando pacotes.
3. **Inconsistência de Splash na Saída do Worker:** Ao encerrar o worker de decodificação Miracast, o código exibia a tela de instruções do Miracast (`SplashEngine::show_miracast()`), em vez da tela padrão de Standby/Pronto (`SplashEngine::show_ready()`), confundindo o estado da appliance.

---

## 2. Requisitos Obrigatórios
1. **Encerramento de Todos os Transmissores no Host:**
   - O daemon `ext-sender` no laptop deve encerrar instâncias ativas de `gnome-network-displays` via sinais POSIX (`SIGTERM` e `SIGKILL`).
   - Pipelines filhos internos (Modo 1 UDP e Modo 3 USB) e pipelines GStreamer externos direcionados ao Pi Zero devem ser finalizados imediatamente.
2. **Teardown Imediato das Sessões RTSP no Receiver:**
   - O módulo `wfd.rs` deve rastrear sockets TCP de clientes Miracast/MS-MICE conectados e forçar o fechamento imediato via `shutdown(Shutdown::Both)`.
   - O encerramento do socket dispara EOF instantâneo para o transmissor (Windows ou GNOME Displays), resultando na desmontagem limpa da sessão.
3. **Manutenção do Sinal Físico HDMI (Sem Desconectar Tela):**
   - A saída HDMI `/dev/fb0` e o conector KMS DRM (`ensure_drm_hdmi_connected`) devem permanecer **100% ativos**. A TV/Monitor não entra em tela preta e não perde o sinal digital HDMI.
   - O framebuffer deve exibir de forma imediata e nítida a tela de Standby/Ready com as informações de rede e QR Code.
4. **Parada Abrangente dos 3 Serviços de Vídeo:**
   - Modo 1 (Rede UDP porta 5000): Decodificador V4L2 M2M pausado.
   - Modo 2 (Miracast RTSP 7236 / UDP 5002): Sessões RTSP derrubadas, decodificador pausado.
   - Modo 3 (USB Bulk Direct): Endpoint FunctionFS desacoplado e decodificador pausado.
   - Supervisor loop no Pi Zero permanece em estado passivo (`is_paused == true`), evitando reinicialização indesejada de fluxos.

---

## 3. Arquitetura e Implementação

```
[ Usuário clica "⏹ Disable Extension / Standby" ]
                        │
                        ▼
       ┌─────────────────────────────────┐
       │ Web UI: setExtensionAction('stop)│
       └────────────────┬────────────────┘
                        │
       ┌────────────────┴────────────────────────┐
       ▼                                         ▼
POST /api/stream/stop                     POST /api/host/control
(Ext-Receiver no Pi Zero)                  (Ext-Sender no Host Laptop UDP 5001)
       │                                         │
       ├─► CONFIG.mode1/2/3 = false              ├─► stop_gnome_network_displays() (pkill)
       ├─► pipeline_mgr.pause()                  ├─► child.kill() & child.wait()
       ├─► wfd::terminate_active_sessions()      ├─► pkill gst-launch-1.0.*192.168.7.2
       ├─► Frame cache limpo                     ├─► close_usb_transport()
       ├─► SplashEngine::show_ready()            └─► is_paused = true
       └─► Sinal HDMI ativo no /dev/fb0
```

### 3.1. Rastreamento e Término de Sessões RTSP (`receiver/src/wfd.rs`)
```rust
static ACTIVE_RTSP_STREAMS: Mutex<Vec<TcpStream>> = Mutex::new(Vec::new());

pub fn terminate_active_sessions() {
    println!("\x1b[1;33m[wfd-rust]\x1b[0m Forcing termination of all active Miracast RTSP/MICE sessions...");
    if let Ok(mut list) = ACTIVE_RTSP_STREAMS.lock() {
        for s in list.drain(..) {
            let _ = s.shutdown(Shutdown::Both);
        }
    }
}
```

### 3.2. Finalização de Transmissores no Host (`sender/src/miracast_launcher.rs` & `sender/src/main.rs`)
```rust
pub fn stop_gnome_network_displays() {
    println!("\x1b[1;33m[miracast-launcher]\x1b[0m Finalizando instâncias de gnome-network-displays...");
    let _ = Command::new("pkill").arg("-15").arg("-f").arg("gnome-network-displays").output();
    let _ = Command::new("pkill").arg("-9").arg("-f").arg("gnome-network-displays").output();
}
```

---

## 4. Validação e Testes em Tempo Real
1. **Execução de Testes Unitários:** 34 testes unitários automatizados passaram com 100% de sucesso (`cargo test` em sender e receiver).
2. **Atualização OTA da Appliance:** Imagem `initramfs.cpio.gz` gravada no SD Card e validada via SHA256 (`952e5b460a0fa7b5adf1deae9a96b4afefd31b0af45a98b07e18dee35249c7a0`).
3. **Transição de Standby Testada:**
   - `curl -s -X POST http://192.168.7.2:8080/api/stream/stop` retornou `{"status":"paused"}`.
   - Logs do host confirmaram: `[miracast-launcher] Finalizando instâncias de gnome-network-displays...` e `[*] Todos os transmissores de vídeo do Host foram finalizados. Modo Standby ativo.`
   - Screenshot do `/dev/fb0` confirmou exibição límpida e instantânea da tela de Standby sem desconectar o HDMI da TV.
4. **Retomada de Fluxo:** Acionamento de retorno para Modo 1 (Estender HDMI-1) restabeleceu a transmissão em < 15ms a 60 FPS com aceleração VAAPI.
