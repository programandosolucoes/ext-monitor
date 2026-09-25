//! Internationalization (i18n) and Multilingual Help System
//!
//! Provides CLI documentation, system requirements, operational modes,
//! and stop/restart procedures in four languages:
//! - English (EN)
//! - Portuguese (PT)
//! - Italian (IT)
//! - Chinese (ZH - 中文)
//!
//! License: MIT
//! Author: Carlos Alberto <psncarlosalberto4ti@gmail.com>

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    English,
    Portuguese,
    Italian,
    Chinese,
}

impl Language {
    /// Detect language from environment variables (LANG, LC_ALL) with English fallback
    pub fn detect() -> Self {
        let env_lang = std::env::var("LC_ALL")
            .or_else(|_| std::env::var("LANG"))
            .unwrap_or_default()
            .to_lowercase();

        if env_lang.starts_with("pt") {
            Language::Portuguese
        } else if env_lang.starts_with("it") {
            Language::Italian
        } else if env_lang.starts_with("zh") {
            Language::Chinese
        } else {
            Language::English
        }
    }

    pub fn from_str(code: &str) -> Self {
        match code.to_lowercase().as_str() {
            "pt" | "pt-br" | "pt_br" | "portuguese" | "portugues" => Language::Portuguese,
            "it" | "it-it" | "it_it" | "italian" | "italiano" => Language::Italian,
            "zh" | "zh-cn" | "zh_cn" | "chinese" | "zhongwen" => Language::Chinese,
            _ => Language::English,
        }
    }
}

/// Print comprehensive CLI help in the specified language
pub fn print_help(lang: Language) {
    match lang {
        Language::English => print_help_en(),
        Language::Portuguese => print_help_pt(),
        Language::Italian => print_help_it(),
        Language::Chinese => print_help_zh(),
    }
}

fn print_help_en() {
    println!("\x1b[1;32m========================================================================\x1b[0m");
    println!("\x1b[1;32m  ext-receiver: Raspberry Pi Zero GPU Hardware Display Receiver v0.2.0  \x1b[0m");
    println!("\x1b[1;34m  100% Native Rust | Broadcom VideoCore IV V4L2 M2M | KMS DRM Output    \x1b[0m");
    println!("\x1b[1;32m========================================================================\x1b[0m\n");

    println!("\x1b[1;33mSYNOPSIS:\x1b[0m");
    println!("  ext-receiver [OPTIONS] [UDP_PORT]\n");

    println!("\x1b[1;33mDESCRIPTION:\x1b[0m");
    println!("  High-performance hardware-accelerated video display receiver designed for");
    println!("  Raspberry Pi Zero W and Pi Zero 2. Renders H.264 video streams directly to");
    println!("  the HDMI output using Broadcom VideoCore IV hardware decoder (0% CPU usage).\n");

    println!("\x1b[1;33mWHAT IS NEEDED (SYSTEM PREREQUISITES):\x1b[0m");
    println!("  1. \x1b[1;37mHardware:\x1b[0m Raspberry Pi Zero W or Pi Zero 2, Micro-USB data cable");
    println!("     (connected to the USB OTG port, not power-only), and an HDMI monitor or dummy EDID.");
    println!("  2. \x1b[1;37mVideoCore IV GPU Driver:\x1b[0m Ensure 'dtoverlay=vc4-kms-v3d' is in /boot/firmware/config.txt.");
    println!("     The V4L2 M2M decoder (/dev/video10) must be available.");
    println!("  3. \x1b[1;37mGStreamer 1.0 Plugins:\x1b[0m gst-plugins-base, gst-plugins-good, gst-plugins-bad,");
    println!("     gst-plugins-ugly, and gstreamer1.0-plugins-v4l2.");
    println!("  4. \x1b[1;37mHost PC:\x1b[0m Linux (GNOME Wayland Mutter with PipeWire & VA-API) or");
    println!("     Windows 10/11 with Miracast wireless display support.\n");

    println!("\x1b[1;33mOPERATIONAL MODES:\x1b[0m");
    println!("  \x1b[1;36m1. Mode 1: USB Network + Miracast Hybrid (Default)\x1b[0m");
    println!("     - Receives raw RTP H.264 from Linux Wayland (ext-sender) on UDP port 5000.");
    println!("     - Serves native Wi-Fi Display (Miracast / MS-MICE) RTSP server on TCP port 7236.");
    println!("       Supports Windows 10/11 out-of-the-box casting via \x1b[1;37mWin + K\x1b[0m (Zero drivers needed).");
    println!("     - Embedded Web Control Dashboard served on HTTP port 8080.");
    println!("     - Seamless auto-switching between Linux desktop and Windows cast.\n");

    println!("  \x1b[1;36m2. Mode 2: USB Bulk Direct via FunctionFS (--mode=usb-bulk)\x1b[0m");
    println!("     - Completely eliminates kernel TCP/IP, UDP, and network stack overhead.");
    println!("     - Streams raw H.264 NAL units directly across USB endpoints at 480 Mbps.");
    println!("     - Sub-millisecond transfer latency for ultra-low latency setups.\n");

    println!("\x1b[1;33mOPTIONS:\x1b[0m");
    println!("  \x1b[1;32m--mode=<network|usb-bulk>\x1b[0m   Select operational mode (default: network)");
    println!("  \x1b[1;32m--backend=<v4l2|gst|ffmpeg>\x1b[0m Select decoding engine (v4l2: Pure Rust zero-dep, gst: GStreamer, ffmpeg: FFmpeg)");
    println!("  \x1b[1;32m-b, --usb-bulk\x1b[0m              Shortcut for Mode 2 (USB Bulk Direct)");
    println!("  \x1b[1;32m--help, -h\x1b[0m                  Display this help message");
    println!("  \x1b[1;32m--lang=<en|pt|it|zh>\x1b[0m        Select help language (English, Portuguese, Italian, Chinese)\n");

    println!("\x1b[1;33mWEB CONTROL DASHBOARD:\x1b[0m");
    println!("  - \x1b[1;37mURL:\x1b[0m \x1b[1;34mhttp://192.168.7.2:8080\x1b[0m");
    println!("  - Real-time multilingual control for FPS, bitrate, color profiles, and diagnostic HUD.");
    println!("  - Live hardware telemetry (SoC temperature, CPU load, free RAM).\n");

    println!("\x1b[1;33mHOW TO STOP THE WEB PANEL & SERVICE:\x1b[0m");
    println!("  - \x1b[1;37mVia Systemd (Recommended):\x1b[0m");
    println!("      \x1b[1;31msudo systemctl stop ext-receiver\x1b[0m       (Stops the service & web panel immediately)");
    println!("      \x1b[1;33msudo systemctl disable ext-receiver\x1b[0m    (Prevents automatic startup on boot)");
    println!("      \x1b[1;32msudo systemctl restart ext-receiver\x1b[0m    (Restarts the service & web panel)");
    println!("  - \x1b[1;37mVia Web Dashboard:\x1b[0m");
    println!("      Click the \x1b[1;31m'⏹ Stop Panel & Service'\x1b[0m button on the web page, or");
    println!("      Click \x1b[1;33m'⏸ Pause Stream'\x1b[0m to pause display output while keeping the panel online.");
    println!("  - \x1b[1;37mVia Terminal Signals:\x1b[0m Press \x1b[1;31mCtrl + C\x1b[0m or run \x1b[1;31mpkill -f ext-receiver\x1b[0m.\n");
}

fn print_help_pt() {
    println!("\x1b[1;32m========================================================================\x1b[0m");
    println!("\x1b[1;32m  ext-receiver: Receptor de Display por Hardware GPU Pi Zero v0.2.0    \x1b[0m");
    println!("\x1b[1;34m  100% Rust Nativo | Broadcom VideoCore IV V4L2 M2M | Saída KMS DRM    \x1b[0m");
    println!("\x1b[1;32m========================================================================\x1b[0m\n");

    println!("\x1b[1;33mSINOPSE:\x1b[0m");
    println!("  ext-receiver [OPÇÕES] [PORTA_UDP]\n");

    println!("\x1b[1;33mDESCRIÇÃO:\x1b[0m");
    println!("  Receptor de vídeo de alta performance com aceleração por hardware para");
    println!("  Raspberry Pi Zero W e Pi Zero 2. Decodifica fluxos H.264 diretamente na");
    println!("  saída HDMI usando o chip Broadcom VideoCore IV (~0% de uso de CPU).\n");

    println!("\x1b[1;33mO QUE É NECESSÁRIO (PRÉ-REQUISITOS DO SISTEMA):\x1b[0m");
    println!("  1. \x1b[1;37mHardware:\x1b[0m Raspberry Pi Zero W ou Pi Zero 2, cabo Micro-USB de dados");
    println!("     (conectado na porta OTG USB, não na de energia pura), tela HDMI ou EDID virtual.");
    println!("  2. \x1b[1;37mDriver GPU VideoCore IV:\x1b[0m Certifique-se de ter 'dtoverlay=vc4-kms-v3d' em /boot/firmware/config.txt.");
    println!("     O decodificador V4L2 M2M (/dev/video10) deve estar disponível.");
    println!("  3. \x1b[1;37mPlugins GStreamer 1.0:\x1b[0m gst-plugins-base, gst-plugins-good, gst-plugins-bad,");
    println!("     gst-plugins-ugly e gstreamer1.0-plugins-v4l2 instalados.");
    println!("  4. \x1b[1;37mComputador Host:\x1b[0m Linux (GNOME Wayland Mutter com PipeWire e VA-API) ou");
    println!("     Windows 10/11 com suporte a transmissão sem fio Miracast.\n");

    println!("\x1b[1;33mMODOS DE OPERAÇÃO:\x1b[0m");
    println!("  \x1b[1;36m1. Modo 1: Rede USB + Miracast Híbrido (Padrão)\x1b[0m");
    println!("     - Recebe RTP H.264 bruto do Linux Wayland (ext-sender) na porta UDP 5000.");
    println!("     - Servidor nativo Wi-Fi Display (Miracast / MS-MICE) na porta TCP 7236.");
    println!("       Suporta transmissão nativa do Windows 10/11 via \x1b[1;37mWin + K\x1b[0m (Zero drivers).");
    println!("     - Painel de controle Web embarcado na porta HTTP 8080.");
    println!("     - Troca automática e suave entre tela Linux e projeção Windows.\n");

    println!("  \x1b[1;36m2. Modo 2: USB Bulk Direto via FunctionFS (--mode=usb-bulk)\x1b[0m");
    println!("     - Elimina completamente o overhead da pilha de rede (TCP/IP e UDP).");
    println!("     - Latência de transferência sub-milissegundo para máxima fluidez.\n");

    println!("\x1b[1;33mOPÇÕES:\x1b[0m");
    println!("  \x1b[1;32m--mode=<network|usb-bulk>\x1b[0m   Seleciona o modo de operação (padrão: network)");
    println!("  \x1b[1;32m--backend=<v4l2|gst|ffmpeg>\x1b[0m Motor de decodificação (v4l2: Rust puro zero-dep, gst: GStreamer, ffmpeg: FFmpeg)");
    println!("  \x1b[1;32m-b, --usb-bulk\x1b[0m              Atalho para o Modo 2 (USB Bulk Direto)");
    println!("  \x1b[1;32m--help, -h\x1b[0m                  Exibe esta mensagem de ajuda");
    println!("  \x1b[1;32m--lang=<en|pt|it|zh>\x1b[0m        Seleciona o idioma (Inglês, Português, Italiano, Chinês)\n");

    println!("\x1b[1;33mPAINEL DE CONTROLE WEB:\x1b[0m");
    println!("  - \x1b[1;37mEndereço:\x1b[0m \x1b[1;34mhttp://192.168.7.2:8080\x1b[0m");
    println!("  - Controle em 4 idiomas para FPS, bitrate, perfis de cor e telemetria na tela (HUD).");
    println!("  - Monitoramento de temperatura SoC, carga de CPU e memória RAM em tempo real.\n");

    println!("\x1b[1;33mCOMO PARAR O PAINEL WEB E O SERVIÇO:\x1b[0m");
    println!("  - \x1b[1;37mVia Systemd (Recomendado):\x1b[0m");
    println!("      \x1b[1;31msudo systemctl stop ext-receiver\x1b[0m       (Para o serviço e o painel web imediatamente)");
    println!("      \x1b[1;33msudo systemctl disable ext-receiver\x1b[0m    (Desativa a inicialização no boot)");
    println!("      \x1b[1;32msudo systemctl restart ext-receiver\x1b[0m    (Reinicia o serviço e o painel)");
    println!("  - \x1b[1;37mVia Painel Web:\x1b[0m");
    println!("      Clique no botão \x1b[1;31m'⏹ Parar Painel e Serviço'\x1b[0m na interface web, ou");
    println!("      Clique em \x1b[1;33m'⏸ Pausar Exibição'\x1b[0m para suspender a tela mantendo o painel online.");
    println!("  - \x1b[1;37mVia Terminal:\x1b[0m Pressione \x1b[1;31mCtrl + C\x1b[0m ou execute \x1b[1;31mpkill -f ext-receiver\x1b[0m.\n");
}

fn print_help_it() {
    println!("\x1b[1;32m========================================================================\x1b[0m");
    println!("\x1b[1;32m  ext-receiver: Ricevitore Display Hardware GPU Pi Zero v0.2.0         \x1b[0m");
    println!("\x1b[1;34m  100% Rust Nativo | Broadcom VideoCore IV V4L2 M2M | Uscita KMS DRM    \x1b[0m");
    println!("\x1b[1;32m========================================================================\x1b[0m\n");

    println!("\x1b[1;33mSINOSSI:\x1b[0m");
    println!("  ext-receiver [OPZIONI] [PORTA_UDP]\n");

    println!("\x1b[1;33mDESCRIZIONE:\x1b[0m");
    println!("  Ricevitore video ad alte prestazioni con accelerazione hardware progettato");
    println!("  per Raspberry Pi Zero W e Pi Zero 2. Decodifica flussi H.264 direttamente");
    println!("  sull'uscita HDMI utilizzando il chip Broadcom VideoCore IV (~0% uso CPU).\n");

    println!("\x1b[1;33mREQUISITI NECESSARI (PREREQUISITI DI SISTEMA):\x1b[0m");
    println!("  1. \x1b[1;37mHardware:\x1b[0m Raspberry Pi Zero W o Pi Zero 2, cavo dati Micro-USB");
    println!("     (connesso alla porta USB OTG), schermo HDMI o EDID virtuale.");
    println!("  2. \x1b[1;37mDriver GPU VideoCore IV:\x1b[0m 'dtoverlay=vc4-kms-v3d' attivo in /boot/firmware/config.txt.");
    println!("     Decoder hardware V4L2 M2M (/dev/video10) abilitato.");
    println!("  3. \x1b[1;37mPlugin GStreamer 1.0:\x1b[0m gst-plugins-base, good, bad, ugly e v4l2 installati.");
    println!("  4. \x1b[1;37mComputer Host:\x1b[0m Linux (GNOME Wayland con PipeWire e VA-API) o");
    println!("     Windows 10/11 con supporto alla trasmissione wireless Miracast.\n");

    println!("\x1b[1;33mMODALITÀ DI FUNZIONAMENTO:\x1b[0m");
    println!("  \x1b[1;36m1. Modalità 1: Rete USB + Miracast Ibrido (Predefinito)\x1b[0m");
    println!("     - Riceve RTP H.264 da Linux Wayland (ext-sender) sulla porta UDP 5000.");
    println!("     - Server nativo Wi-Fi Display (Miracast / MS-MICE) su porta TCP 7236.");
    println!("       Supporta la trasmissione nativa da Windows 10/11 con \x1b[1;37mWin + K\x1b[0m (Senza driver).");
    println!("     - Pannello di controllo Web integrato su porta HTTP 8080.");
    println!("     - Commutazione automatica e trasparente tra Linux e Windows.\n");

    println!("  \x1b[1;36m2. Modalità 2: USB Bulk Diretto via FunctionFS (--mode=usb-bulk)\x1b[0m");
    println!("     - Elimina completamente il carico dello stack di rete (TCP/IP e UDP).");
    println!("     - Trasferisce pacchetti H.264 direttamente sugli endpoint USB a 480 Mbps.");
    println!("     - Latenza di trasferimento inferiore al millisecondo per massima fluidità.\n");

    println!("\x1b[1;33mOPZIONI:\x1b[0m");
    println!("  \x1b[1;32m--mode=<network|usb-bulk>\x1b[0m   Seleziona modalità operativa (default: network)");
    println!("  \x1b[1;32m--backend=<v4l2|gst|ffmpeg>\x1b[0m Motore di decodifica (v4l2: Rust puro zero-dep, gst: GStreamer, ffmpeg: FFmpeg)");
    println!("  \x1b[1;32m-b, --usb-bulk\x1b[0m              Scorciatoia per Modalità 2 (USB Bulk Diretto)");
    println!("  \x1b[1;32m--help, -h\x1b[0m                  Mostra questo messaggio di aiuto");
    println!("  \x1b[1;32m--lang=<en|pt|it|zh>\x1b[0m        Seleziona lingua (Inglese, Portoghese, Italiano, Cinese)\n");

    println!("\x1b[1;33mPANNELLO DI CONTROLLO WEB:\x1b[0m");
    println!("  - \x1b[1;37mIndirizzo:\x1b[0m \x1b[1;34mhttp://192.168.7.2:8080\x1b[0m");
    println!("  - Controllo multilingue per FPS, bitrate, profili colore e OSD di diagnostica.");
    println!("  - Telemetria in tempo reale di temperatura, carico CPU e memoria libera.\n");

    println!("\x1b[1;33mCOME ARRESTARE IL PANNELLO WEB E IL SERVIZIO:\x1b[0m");
    println!("  - \x1b[1;37mTramite Systemd (Consigliato):\x1b[0m");
    println!("      \x1b[1;31msudo systemctl stop ext-receiver\x1b[0m       (Arresta immediatamente demone e pannello)");
    println!("      \x1b[1;33msudo systemctl disable ext-receiver\x1b[0m    (Disabilita l'avvio automatico al boot)");
    println!("      \x1b[1;32msudo systemctl restart ext-receiver\x1b[0m    (Riavvia servizio e pannello)");
    println!("  - \x1b[1;37mTramite Pannello Web:\x1b[0m");
    println!("      Premi il pulsante \x1b[1;31m'⏹ Arresta Pannello e Servizio'\x1b[0m nel browser, oppure");
    println!("      Premi \x1b[1;33m'⏸ Sospendi Display'\x1b[0m per fermare il flusso video lasciando il pannello attivo.");
    println!("  - \x1b[1;37mTramite Terminale:\x1b[0m Premi \x1b[1;31mCtrl + C\x1b[0m oppure esegui \x1b[1;31mpkill -f ext-receiver\x1b[0m.\n");
}

fn print_help_zh() {
    println!("\x1b[1;32m========================================================================\x1b[0m");
    println!("\x1b[1;32m  ext-receiver: 树莓派 Zero GPU 硬件显示接收器 v0.2.0                  \x1b[0m");
    println!("\x1b[1;34m  100% 纯 Rust 原生开发 | 博通 VideoCore IV V4L2 M2M | KMS DRM 输出    \x1b[0m");
    println!("\x1b[1;32m========================================================================\x1b[0m\n");

    println!("\x1b[1;33m命令格式:\x1b[0m");
    println!("  ext-receiver [选项] [UDP端口]\n");

    println!("\x1b[1;33m功能简介:\x1b[0m");
    println!("  专为 Raspberry Pi Zero W 和 Pi Zero 2 设计的高性能硬件加速视频显示接收端。");
    println!("  使用 Broadcom VideoCore IV 硬件解码器直接将 H.264 视频渲染至 HDMI 输出（约 0% CPU 占用）。\n");

    println!("\x1b[1;33m运行所需环境与条件:\x1b[0m");
    println!("  1. \x1b[1;37m硬件准备:\x1b[0m 树莓派 Zero W 或 Zero 2，Micro-USB 数据线（必须连接在 OTG 数据口），");
    println!("     HDMI 显示器或虚拟 EDID 假负载。");
    println!("  2. \x1b[1;37mGPU VideoCore IV 驱动:\x1b[0m 确保 /boot/firmware/config.txt 包含 'dtoverlay=vc4-kms-v3d'，");
    println!("     且 V4L2 M2M 解码节点 (/dev/video10) 正常加载。");
    println!("  3. \x1b[1;37mGStreamer 1.0 组件包:\x1b[0m 已安装 gst-plugins-base, good, bad, ugly 及 v4l2 插件。");
    println!("  4. \x1b[1;37m电脑主机端:\x1b[0m 支持 VA-API GPU 加速的 Linux Wayland (GNOME)，或支持无线投屏的 Windows 10/11。\n");

    println!("\x1b[1;33m工作模式:\x1b[0m");
    println!("  \x1b[1;36m1. 模式 1: USB 网络 + Miracast 混合模式 (默认模式)\x1b[0m");
    println!("     - 在 UDP 端口 5000 接收来自 Linux Wayland (ext-sender) 的原生 RTP H.264 数据流。");
    println!("     - 在 TCP 端口 7236 运行原生 Wi-Fi Display (Miracast / MS-MICE) RTSP 服务。");
    println!("       完美支持 Windows 10/11 原生投屏快捷键 \x1b[1;37mWin + K\x1b[0m（无需安装任何驱动或软件）。");
    println!("     - 在 HTTP 端口 8080 提供内嵌式 Web 控制仪表盘。");
    println!("     - 支持 Linux 桌面与 Windows 投屏之间的无缝自动切换。\n");

    println!("  \x1b[1;36m2. 模式 2: 基于 FunctionFS 的 USB Bulk 直通模式 (--mode=usb-bulk)\x1b[0m");
    println!("     - 完全绕过内核 TCP/IP、UDP 和网络协议栈开销。");
    println!("     - 以 480 Mbps 速率直接在 USB 端点上高速传输原始 H.264 NAL 单元。");
    println!("     - 亚毫秒级超低传输延迟，提供丝滑显示体验。\n");

    println!("\x1b[1;33m可用选项:\x1b[0m");
    println!("  \x1b[1;32m--mode=<network|usb-bulk>\x1b[0m   选择运行模式 (默认: network)");
    println!("  \x1b[1;32m--backend=<v4l2|gst|ffmpeg>\x1b[0m 选择解码引擎 (v4l2: 纯 Rust 零依赖内核解码, gst: GStreamer, ffmpeg: FFmpeg)");
    println!("  \x1b[1;32m-b, --usb-bulk\x1b[0m              模式 2 (USB Bulk 直通) 的快捷参数");
    println!("  \x1b[1;32m--help, -h\x1b[0m                  显示此帮助信息");
    println!("  \x1b[1;32m--lang=<en|pt|it|zh>\x1b[0m        选择帮助信息语言 (英语, 葡萄牙语, 意大利语, 中文)\n");

    println!("\x1b[1;33mWEB 控制面板:\x1b[0m");
    println!("  - \x1b[1;37m访问地址:\x1b[0m \x1b[1;34mhttp://192.168.7.2:8080\x1b[0m");
    println!("  - 提供四种语言界面，支持在线热调帧率、码率、色彩配置文件以及屏幕 HUD 诊断。");
    println!("  - 实时监控核心温度、CPU 使用率及可用内存。\n");

    println!("\x1b[1;33m如何停止控制面板及服务:\x1b[0m");
    println!("  - \x1b[1;37m通过 Systemd 服务管理 (推荐):\x1b[0m");
    println!("      \x1b[1;31msudo systemctl stop ext-receiver\x1b[0m       (立即停止服务并关闭 Web 面板)");
    println!("      \x1b[1;33msudo systemctl disable ext-receiver\x1b[0m    (取消开机自启)");
    println!("      \x1b[1;32msudo systemctl restart ext-receiver\x1b[0m    (重启接收端服务及面板)");
    println!("  - \x1b[1;37m通过 Web 控制面板:\x1b[0m");
    println!("      点击网页上的 \x1b[1;31m'⏹ 停止控制面板及服务'\x1b[0m 按钮；或点击 \x1b[1;33m'⏸ 暂停画面推流'\x1b[0m");
    println!("  - \x1b[1;37m通过终端信号:\x1b[0m 按下 \x1b[1;31mCtrl + C\x1b[0m 或在终端执行 \x1b[1;31mpkill -f ext-receiver\x1b[0m。\n");
}
