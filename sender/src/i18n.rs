//! Internationalization (i18n) and Multilingual Help System for ext-sender
//!
//! Provides CLI documentation, host system requirements, operational modes,
//! and stopping procedures in four languages:
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
    println!("\x1b[1;32m  ext-sender: GPU Offload Virtual Second Monitor Sender v0.2.0          \x1b[0m");
    println!("\x1b[1;34m  100% Native Rust | PipeWire Zero-Copy | VA-API / NVENC / QSV Hardware  \x1b[0m");
    println!("\x1b[1;32m========================================================================\x1b[0m\n");

    println!("\x1b[1;33mSYNOPSIS:\x1b[0m");
    println!("  ext-sender [TARGET_IP] [PORT] [BITRATE] [MODE] [ENCODER] [FPS] [OPTIONS]\n");

    println!("\x1b[1;33mDESCRIPTION:\x1b[0m");
    println!("  Captures virtual monitor framebuffers from GNOME Mutter via PipeWire D-Bus,");
    println!("  encodes them using GPU hardware acceleration (AMD VA-API, NVIDIA NVENC, Intel QSV),");
    println!("  and transmits the ultra-low-latency H.264 stream to Raspberry Pi Zero.\n");

    println!("\x1b[1;33mWHAT IS NEEDED (SYSTEM PREREQUISITES):\x1b[0m");
    println!("  1. \x1b[1;37mWayland Session:\x1b[0m GNOME 44+ on Wayland with org.gnome.Mutter.ScreenCast.");
    println!("  2. \x1b[1;37mPipeWire:\x1b[0m pipewire, wireplumber, and pipewire-pulse running.");
    println!("  3. \x1b[1;37mGPU Hardware Acceleration:\x1b[0m AMD (VA-API), Intel (VA-API/QSV), or NVIDIA (NVENC).");
    println!("  4. \x1b[1;37mGStreamer 1.0:\x1b[0m gst-plugins-base, good, bad, and vaapi/nvenc/qsv.");
    println!("  5. \x1b[1;37mPi Zero Connection:\x1b[0m Connected via USB OTG cable (IP 192.168.7.2 or USB Direct).\n");

    println!("\x1b[1;33mARGUMENTS & DEFAULTS:\x1b[0m");
    println!("  1. TARGET_IP     Target IP address (default: 192.168.7.2)");
    println!("  2. PORT          Target UDP port (default: 5000)");
    println!("  3. BITRATE       Stream bitrate in kbps (default: auto, 150 - 15000 kbps)");
    println!("  4. MODE          Display mode: 'extend' (virtual HDMI-1) or 'clone' (eDP-1)");
    println!("  5. ENCODER       Encoding engine: 'vaapi', 'nvenc', 'qsv', 'software', or 'auto'");
    println!("  6. FPS           Target framerate: 10 to 60 FPS (default: 30)\n");

    println!("\x1b[1;33mOPTIONS & FLAGS:\x1b[0m");
    println!("  \x1b[1;32m--transport=<network|usb>\x1b[0m Select transport protocol (default: network)");
    println!("  \x1b[1;32m--usb, --usb-bulk\x1b[0m         Shortcut for Mode 2 (Direct USB Bulk via rusb)");
    println!("  \x1b[1;32m--hud\x1b[0m                     Enable diagnostic on-screen telemetry overlay (auto-hides in 60s)");
    println!("  \x1b[1;32m--color=<full|256|gray>\x1b[0m   Set color profile (24-bit TrueColor, 256-color QP, Monochrome)");
    println!("  \x1b[1;32m--help, -h\x1b[0m                Display this help message");
    println!("  \x1b[1;32m--lang=<en|pt|it|zh>\x1b[0m      Select help language (English, Portuguese, Italian, Chinese)\n");

    println!("\x1b[1;33mHOW TO STOP EXT-SENDER:\x1b[0m");
    println!("  - \x1b[1;37mInteractive:\x1b[0m Press \x1b[1;31mCtrl + C\x1b[0m in the terminal running ext-sender.");
    println!("  - \x1b[1;37mBackground/Terminal:\x1b[0m Run \x1b[1;31mpkill -f ext-sender\x1b[0m to terminate immediately.");
    println!("  - \x1b[1;37mWeb Control Panel:\x1b[0m Adjust stream or stop receiver at \x1b[1;34mhttp://192.168.7.2:8080\x1b[0m.\n");
}

fn print_help_pt() {
    println!("\x1b[1;32m========================================================================\x1b[0m");
    println!("\x1b[1;32m  ext-sender: Transmissor de Segundo Monitor Virtual via GPU v0.2.0    \x1b[0m");
    println!("\x1b[1;34m  100% Rust Nativo | PipeWire Zero-Copy | Aceleração VA-API/NVENC/QSV   \x1b[0m");
    println!("\x1b[1;32m========================================================================\x1b[0m\n");

    println!("\x1b[1;33mSINOPSE:\x1b[0m");
    println!("  ext-sender [IP_DESTINO] [PORTA] [BITRATE] [MODO] [ENCODER] [FPS] [OPÇÕES]\n");

    println!("\x1b[1;33mDESCRIÇÃO:\x1b[0m");
    println!("  Captura o monitor virtual do GNOME Mutter via PipeWire D-Bus, codifica em");
    println!("  hardware na GPU (AMD VA-API, NVIDIA NVENC, Intel QSV) e transmite o fluxo");
    println!("  H.264 de ultra-baixa latência para o Raspberry Pi Zero.\n");

    println!("\x1b[1;33mO QUE É NECESSÁRIO (PRÉ-REQUISITOS DO SISTEMA):\x1b[0m");
    println!("  1. \x1b[1;37mSessão Wayland:\x1b[0m GNOME 44+ em Wayland com interface org.gnome.Mutter.ScreenCast.");
    println!("  2. \x1b[1;37mPipeWire:\x1b[0m Servidores pipewire, wireplumber e pipewire-pulse em execução.");
    println!("  3. \x1b[1;37mAceleração de GPU:\x1b[0m Placa AMD (VA-API), Intel (VA-API/QSV) ou NVIDIA (NVENC).");
    println!("  4. \x1b[1;37mGStreamer 1.0:\x1b[0m gst-plugins-base, good, bad e plugins de hardware de vídeo.");
    println!("  5. \x1b[1;37mConexão com Pi Zero:\x1b[0m Cabo Micro-USB OTG conectado (IP 192.168.7.2 ou USB Direto).\n");

    println!("\x1b[1;33mARGUMENTOS E VALORES PADRÃO:\x1b[0m");
    println!("  1. IP_DESTINO    Endereço IP do Pi Zero (padrão: 192.168.7.2)");
    println!("  2. PORTA         Porta UDP de destino (padrão: 5000)");
    println!("  3. BITRATE       Taxa de bits em kbps (padrão: auto, 150 - 15000 kbps)");
    println!("  4. MODO          Modo de exibição: 'extend' (HDMI-1) ou 'clone' (eDP-1)");
    println!("  5. ENCODER       Motor de codificação: 'vaapi', 'nvenc', 'qsv', 'software' ou 'auto'");
    println!("  6. FPS           Taxa de quadros: 10 a 60 FPS (padrão: 30)\n");

    println!("\x1b[1;33mOPÇÕES E PARÂMETROS:\x1b[0m");
    println!("  \x1b[1;32m--transport=<network|usb>\x1b[0m Seleciona o protocolo de transporte (padrão: network)");
    println!("  \x1b[1;32m--usb, --usb-bulk\x1b[0m         Atalho para o Modo 2 (USB Bulk Direto via rusb)");
    println!("  \x1b[1;32m--hud\x1b[0m                     Ativa o painel de telemetria na tela (auto-oculta em 60s)");
    println!("  \x1b[1;32m--color=<full|256|gray>\x1b[0m   Perfil de cor (TrueColor 24-bit, 256 cores QP, Monocromático)");
    println!("  \x1b[1;32m--help, -h\x1b[0m                Exibe esta mensagem de ajuda");
    println!("  \x1b[1;32m--lang=<en|pt|it|zh>\x1b[0m      Seleciona o idioma (Inglês, Português, Italiano, Chinês)\n");

    println!("\x1b[1;33mCOMO PARAR O EXT-SENDER:\x1b[0m");
    println!("  - \x1b[1;37mInterativo:\x1b[0m Pressione \x1b[1;31mCtrl + C\x1b[0m no terminal onde o ext-sender está rodando.");
    println!("  - \x1b[1;37mTerminal/Segundo Plano:\x1b[0m Execute \x1b[1;31mpkill -f ext-sender\x1b[0m para encerrar o processo.");
    println!("  - \x1b[1;37mPainel Web Remoto:\x1b[0m Ajuste o fluxo ou encerre em \x1b[1;34mhttp://192.168.7.2:8080\x1b[0m.\n");
}

fn print_help_it() {
    println!("\x1b[1;32m========================================================================\x1b[0m");
    println!("\x1b[1;32m  ext-sender: Trasmettitore Secondo Monitor Virtuale via GPU v0.2.0     \x1b[0m");
    println!("\x1b[1;34m  100% Rust Nativo | PipeWire Zero-Copy | Accelerazione VA-API/NVENC/QSV\x1b[0m");
    println!("\x1b[1;32m========================================================================\x1b[0m\n");

    println!("\x1b[1;33mSINOSSI:\x1b[0m");
    println!("  ext-sender [IP_DESTINAZIONE] [PORTA] [BITRATE] [MODALITA] [ENCODER] [FPS] [OPZIONI]\n");

    println!("\x1b[1;33mDESCRIZIONE:\x1b[0m");
    println!("  Cattura il monitor virtuale da GNOME Mutter tramite PipeWire D-Bus, codifica");
    println!("  in hardware su GPU (AMD VA-API, NVIDIA NVENC, Intel QSV) e trasmette il flusso");
    println!("  H.264 ad ultra-bassa latenza al Raspberry Pi Zero.\n");

    println!("\x1b[1;33mREQUISITI NECESSARI (PREREQUISITI DI SISTEMA):\x1b[0m");
    println!("  1. \x1b[1;37mSessione Wayland:\x1b[0m GNOME 44+ su Wayland con org.gnome.Mutter.ScreenCast.");
    println!("  2. \x1b[1;37mPipeWire:\x1b[0m Servizi pipewire, wireplumber e pipewire-pulse attivi.");
    println!("  3. \x1b[1;37mAccelerazione GPU:\x1b[0m Scheda AMD (VA-API), Intel (VA-API/QSV) o NVIDIA (NVENC).");
    println!("  4. \x1b[1;37mGStreamer 1.0:\x1b[0m Plugin gst-plugins-base, good, bad e plugin di codifica hardware.");
    println!("  5. \x1b[1;37mConnessione Pi Zero:\x1b[0m Cavo Micro-USB OTG connesso (IP 192.168.7.2 o USB Diretto).\n");

    println!("\x1b[1;33mARGOMENTI E PREDEFINITI:\x1b[0m");
    println!("  1. IP_DESTINAZIONE Indirizzo IP del Pi Zero (default: 192.168.7.2)");
    println!("  2. PORTA           Porta UDP di destinazione (default: 5000)");
    println!("  3. BITRATE         Bitrate del flusso in kbps (default: auto, 150 - 15000 kbps)");
    println!("  4. MODALITA        Modalità: 'extend' (HDMI-1 virtuale) o 'clone' (eDP-1)");
    println!("  5. ENCODER         Motore di codifica: 'vaapi', 'nvenc', 'qsv', 'software', 'auto'");
    println!("  6. FPS             Frequenza fotogrammi: da 10 a 60 FPS (default: 30)\n");

    println!("\x1b[1;33mOPZIONI:\x1b[0m");
    println!("  \x1b[1;32m--transport=<network|usb>\x1b[0m Seleziona trasporto dati (default: network)");
    println!("  \x1b[1;32m--usb, --usb-bulk\x1b[0m         Scorciatoia per Modalità 2 (USB Bulk Diretto via rusb)");
    println!("  \x1b[1;32m--hud\x1b[0m                     Attiva telemetria OSD a schermo (scomparsa in 60s)");
    println!("  \x1b[1;32m--color=<full|256|gray>\x1b[0m   Profilo colore (TrueColor 24-bit, 256 colori QP, Bianco/Nero)");
    println!("  \x1b[1;32m--help, -h\x1b[0m                Mostra questo messaggio di aiuto");
    println!("  \x1b[1;32m--lang=<en|pt|it|zh>\x1b[0m      Seleziona lingua (Inglese, Portoghese, Italiano, Cinese)\n");

    println!("\x1b[1;33mCOME ARRESTARE EXT-SENDER:\x1b[0m");
    println!("  - \x1b[1;37mInterattivo:\x1b[0m Premi \x1b[1;31mCtrl + C\x1b[0m nel terminale di ext-sender.");
    println!("  - \x1b[1;37mDa Terminale:\x1b[0m Esegui \x1b[1;31mpkill -f ext-sender\x1b[0m per terminare il processo.");
    println!("  - \x1b[1;37mPannello Web:\x1b[0m Gestisci lo streaming su \x1b[1;34mhttp://192.168.7.2:8080\x1b[0m.\n");
}

fn print_help_zh() {
    println!("\x1b[1;32m========================================================================\x1b[0m");
    println!("\x1b[1;32m  ext-sender: 基于 GPU 硬件加速的虚拟第二显示器发送端 v0.2.0           \x1b[0m");
    println!("\x1b[1;34m  100% 纯 Rust 原生开发 | PipeWire 零拷贝 | VA-API / NVENC / QSV 硬件加速\x1b[0m");
    println!("\x1b[1;32m========================================================================\x1b[0m\n");

    println!("\x1b[1;33m命令格式:\x1b[0m");
    println!("  ext-sender [目标IP] [端口] [码率] [显示模式] [编码引擎] [帧率] [选项]\n");

    println!("\x1b[1;33m功能简介:\x1b[0m");
    println!("  通过 PipeWire D-Bus 捕获 GNOME Mutter 虚拟显示器画面，使用 GPU 硬件");
    println!("  编码引擎 (AMD VA-API, NVIDIA NVENC, Intel QSV) 将超低延迟 H.264 流实时");
    println!("  推送到树莓派 Zero。\n");

    println!("\x1b[1;33m运行所需环境与条件:\x1b[0m");
    println!("  1. \x1b[1;37mWayland 桌面会话:\x1b[0m GNOME 44+ 并开启 org.gnome.Mutter.ScreenCast 接口。");
    println!("  2. \x1b[1;37mPipeWire 音视频服务:\x1b[0m pipewire, wireplumber 及 pipewire-pulse 正常运行。");
    println!("  3. \x1b[1;37mGPU 硬件加速环境:\x1b[0m AMD 显卡 (VA-API), Intel 核显 (VA-API/QSV) 或 NVIDIA 显卡 (NVENC)。");
    println!("  4. \x1b[1;37mGStreamer 1.0 组件:\x1b[0m 已安装 gst-plugins-base, good, bad 及硬件编码插件。");
    println!("  5. \x1b[1;37m树莓派连接状态:\x1b[0m 已通过 Micro-USB OTG 数据线连接 (IP 192.168.7.2 或 USB 直通模式)。\n");

    println!("\x1b[1;33m位置参数与默认值:\x1b[0m");
    println!("  1. 目标IP        树莓派 Zero IP 地址 (默认: 192.168.7.2)");
    println!("  2. 端口          目标 UDP 端口 (默认: 5000)");
    println!("  3. 码率          传输码率 kbps (默认: auto, 150 - 15000 kbps)");
    println!("  4. 显示模式      显示器模式: 'extend' (扩展屏 HDMI-1) 或 'clone' (镜像 eDP-1)");
    println!("  5. 编码引擎      编码器类型: 'vaapi', 'nvenc', 'qsv', 'software', 或 'auto'");
    println!("  6. 帧率          目标帧率: 10 到 60 FPS (默认: 30)\n");

    println!("\x1b[1;33m可用选项:\x1b[0m");
    println!("  \x1b[1;32m--transport=<network|usb>\x1b[0m 选择数据传输模式 (默认: network)");
    println!("  \x1b[1;32m--usb, --usb-bulk\x1b[0m         模式 2 快捷参数 (基于 rusb 的 USB Bulk 直通模式)");
    println!("  \x1b[1;32m--hud\x1b[0m                     开启屏幕半透明遥测诊断浮层 (60秒后自动隐藏)");
    println!("  \x1b[1;32m--color=<full|256|gray>\x1b[0m   色彩配置文件 (24位全彩, 256色粗量化, 黑白单色)");
    println!("  \x1b[1;32m--help, -h\x1b[0m                显示此帮助信息");
    println!("  \x1b[1;32m--lang=<en|pt|it|zh>\x1b[0m      选择帮助信息语言 (英语, 葡萄牙语, 意大利语, 中文)\n");

    println!("\x1b[1;33m如何停止 EXT-SENDER 推流:\x1b[0m");
    println!("  - \x1b[1;37m交互式终止:\x1b[0m 在运行 ext-sender 的终端中按下 \x1b[1;31mCtrl + C\x1b[0m。");
    println!("  - \x1b[1;37m命令行后台终止:\x1b[0m 在终端执行 \x1b[1;31mpkill -f ext-sender\x1b[0m 即可立刻关闭推流。");
    println!("  - \x1b[1;37mWeb 控制面板:\x1b[0m 访问 \x1b[1;34mhttp://192.168.7.2:8080\x1b[0m 可实时调控或暂停画面。\n");
}
