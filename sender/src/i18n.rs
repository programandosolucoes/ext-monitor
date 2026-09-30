//! Internationalization (i18n) and Multilingual Help System for ext-sender
//!
//! Provides CLI documentation, host system requirements, operational modes,
//! fast presets, and stopping procedures in four languages:
//! - English (EN)
//! - Portuguese (PT)
//! - Italian (IT)
//! - Chinese (ZH - 中文)
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

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
    println!("\x1b[1;32m  ext-sender: GPU Offload Virtual Second Monitor Sender v0.3.0          \x1b[0m");
    println!("\x1b[1;34m  100% Native Rust | Hardware Accelerated | Sub-15ms / Sub-1ms Latency  \x1b[0m");
    println!("\x1b[1;32m========================================================================\x1b[0m\n");

    println!("\x1b[1;33m🚀 QUICK START — RUN RIGHT NOW (RECOMMENDED / ZERO CONFIGURATION):\x1b[0m");
    println!("  \x1b[1;32mext-sender\x1b[0m");
    println!("      \x1b[1;37mRuns the BEST and FASTEST mode right away!\x1b[0m Zero arguments required.");
    println!("      - Extends display to Pi Zero at \x1b[1;32m60 FPS continuous CFR\x1b[0m (anti-freeze).");
    println!("      - GPU Hardware Encoder auto-detected (\x1b[1;36mAMD VA-API, NVIDIA NVENC, Intel QSV\x1b[0m).");
    println!("      - High-definition 6000 kbps VBR bitrate with sub-15ms latency.");
    println!("      - Synchronized stereo HDMI audio forwarded via PipeWire.");
    println!("      - Auto-inhibits screen sleep and auto-heals upon PC resume.\n");

    println!("  \x1b[1;32mext-sender --usb\x1b[0m (or \x1b[1;32mext-sender -u\x1b[0m)");
    println!("      \x1b[1;37mMode 3 (Direct USB Bulk):\x1b[0m Sub-millisecond latency (\x1b[1;32m< 1ms\x1b[0m),");
    println!("      bypassing TCP/IP network overhead completely.\n");

    println!("  \x1b[1;32mext-sender clone\x1b[0m");
    println!("      \x1b[1;37mMirrors primary laptop screen\x1b[0m directly to Pi Zero at 60 FPS.\n");

    println!("  \x1b[1;32mext-sender stop\x1b[0m");
    println!("      \x1b[1;31mImmediately terminates\x1b[0m any running streaming pipelines.\n");

    println!("  \x1b[1;32mext-sender status\x1b[0m");
    println!("      Queries live telemetry and status from Pi Zero (http://192.168.7.2:8080).\n");

    println!("\x1b[1;33m⚡ FAST PRESETS & EXAMPLES:\x1b[0m");
    println!("  Gaming / Ultra-Fluid Video (60 FPS, 6M) : \x1b[1;36mext-sender 60 6000\x1b[0m");
    println!("  Office / Reading / Low Bandwidth (30 FPS): \x1b[1;36mext-sender 30 2000 --color=256\x1b[0m");
    println!("  Kernel DRM/KMS Direct Hardware Scanout   : \x1b[1;36mext-sender --kms\x1b[0m");
    println!("  GNOME Mutter w/ Hardware Mouse Cursor    : \x1b[1;36mext-sender --mutter\x1b[0m");
    println!("  Silent Mode (Disable HDMI Audio)         : \x1b[1;36mext-sender --no-audio\x1b[0m\n");

    println!("\x1b[1;33mSYNOPSIS:\x1b[0m");
    println!("  ext-sender [SUBCOMMAND]");
    println!("  ext-sender [extend|clone] [FPS] [BITRATE] [OPTIONS]");
    println!("  ext-sender [TARGET_IP] [PORT] [BITRATE] [MODE] [ENCODER] [FPS] [OPTIONS]\n");

    println!("\x1b[1;33mSUBCOMMANDS:\x1b[0m");
    println!("  \x1b[1;32mstop\x1b[0m                        Stop active streaming session and kill pipelines");
    println!("  \x1b[1;32mstatus\x1b[0m                      Query Pi Zero connection and live telemetry");
    println!("  \x1b[1;32mextend\x1b[0m                      Extend desktop to Pi Zero (virtual HDMI-1) [DEFAULT]");
    println!("  \x1b[1;32mclone\x1b[0m                       Mirror primary laptop display (eDP-1)\n");

    println!("\x1b[1;33mOPTIONS & FLAGS:\x1b[0m");
    println!("  \x1b[1;32m--fps=<10-120>\x1b[0m              Target framerate (default: 60 FPS)");
    println!("  \x1b[1;32m--bitrate=<kbps>, -b\x1b[0m        Target bitrate in kbps (default: 6000 for 60 FPS)");
    println!("  \x1b[1;32m--capture=<kms|mutter>\x1b[0m      Capture Engine (kms: Kernel DRM Direct [DEFAULT], mutter: GNOME ScreenCast)");
    println!("  \x1b[1;32m--kms, --kernel\x1b[0m             Shortcut for Kernel DRM Direct Hardware Scanout (zero-copy)");
    println!("  \x1b[1;32m--mutter, --gnome\x1b[0m           Shortcut for GNOME Mutter ScreenCast (with embedded mouse cursor)");
    println!("  \x1b[1;32m--transport=<usb|network>\x1b[0m   Transport protocol (usb: USB Bulk [auto-fallback], network: UDP)");
    println!("  \x1b[1;32m--usb, -u, --usb-bulk\x1b[0m       Shortcut for Mode 3 (Direct USB Bulk via rusb)");
    println!("  \x1b[1;32m--network, --udp\x1b[0m            Force direct UDP network transport (port 5000)");
    println!("  \x1b[1;32m--encoder=<api>\x1b[0m             Encoder: 'vaapi' (AMD/Intel), 'nvenc' (NVIDIA), 'qsv', 'software', or 'auto'");
    println!("  \x1b[1;32m--color=<full|256|gray>\x1b[0m     Color profile (24-bit TrueColor, 256-color QP 30-44, Monochrome Grayscale)");
    println!("  \x1b[1;32m--no-audio\x1b[0m                  Disable PipeWire HDMI stereo audio forwarding");
    println!("  \x1b[1;32m--audio-port=<port>\x1b[0m         UDP port for audio streaming (default: 5002)");
    println!("  \x1b[1;32m--hud\x1b[0m                       Enable diagnostic on-screen telemetry overlay (auto-hides in 60s)");
    println!("  \x1b[1;32m--help, -h\x1b[0m                  Display this help message");
    println!("  \x1b[1;32m--lang=<en|pt|it|zh>\x1b[0m        Select help language (English, Portuguese, Italian, Chinese)\n");

    println!("\x1b[1;33mHOW TO STOP EXT-SENDER:\x1b[0m");
    println!("  - \x1b[1;37mInteractive:\x1b[0m Press \x1b[1;31mCtrl + C\x1b[0m in the terminal running ext-sender.");
    println!("  - \x1b[1;37mBackground/Terminal:\x1b[0m Run \x1b[1;31mext-sender stop\x1b[0m or \x1b[1;31mpkill -f ext-sender\x1b[0m.");
    println!("  - \x1b[1;37mWeb Control Panel:\x1b[0m Stop or tune stream in real-time at \x1b[1;34mhttp://192.168.7.2:8080\x1b[0m.\n");
}

fn print_help_pt() {
    println!("\x1b[1;32m========================================================================\x1b[0m");
    println!("\x1b[1;32m  ext-sender: Transmissor de Segundo Monitor Virtual via GPU v0.3.0    \x1b[0m");
    println!("\x1b[1;34m  100% Rust Nativo | Aceleração de Hardware | Latência Sub-15ms / Sub-1ms\x1b[0m");
    println!("\x1b[1;32m========================================================================\x1b[0m\n");

    println!("\x1b[1;33m🚀 EXECUÇÃO RÁPIDA — O MELHOR COMANDO NO ATO (RECOMENDADO / SEM CONFIGURAÇÃO):\x1b[0m");
    println!("  \x1b[1;32mext-sender\x1b[0m");
    println!("      \x1b[1;37mRoda o melhor e mais rápido comando no ato!\x1b[0m Sem precisar passar nenhum parâmetro.");
    println!("      - Estende a área de trabalho para o Pi Zero a \x1b[1;32m60 FPS contínuo\x1b[0m (CFR Anti-Freeze).");
    println!("      - Encoder GPU por hardware auto-detectado (\x1b[1;36mAMD VA-API, NVIDIA NVENC, Intel QSV\x1b[0m).");
    println!("      - Taxa de bits otimizada de 6000 kbps VBR com latência sub-15ms.");
    println!("      - Áudio estéreo sincronizado via PipeWire para a saída HDMI do Pi Zero.");
    println!("      - Inibe suspensão de tela automaticamente e auto-recupera ao acordar o PC.\n");

    println!("  \x1b[1;32mext-sender --usb\x1b[0m (ou \x1b[1;32mext-sender -u\x1b[0m)");
    println!("      \x1b[1;37mModo 3 (USB Bulk Direto):\x1b[0m Latência física submilisegundo (\x1b[1;32m< 1ms\x1b[0m),");
    println!("      contornando completamente a pilha de rede TCP/IP.\n");

    println!("  \x1b[1;32mext-sender clone\x1b[0m");
    println!("      \x1b[1;37mEspelha a tela principal do notebook\x1b[0m para o Pi Zero a 60 FPS.\n");

    println!("  \x1b[1;32mext-sender stop\x1b[0m");
    println!("      \x1b[1;31mEncerra instantaneamente\x1b[0m todas as transmissões ativas e fecha os pipelines.\n");

    println!("  \x1b[1;32mext-sender status\x1b[0m");
    println!("      Consulta telemetria ao vivo e conectividade do Pi Zero (http://192.168.7.2:8080).\n");

    println!("\x1b[1;33m⚡ PRESETS RÁPIDOS E CASOS DE USO:\x1b[0m");
    println!("  Jogos / Vídeos (Fluidez Máxima 60 FPS, 6M)  : \x1b[1;36mext-sender 60 6000\x1b[0m");
    println!("  Escritório / Leitura (Economia 30 FPS, 2M)  : \x1b[1;36mext-sender 30 2000 --color=256\x1b[0m");
    println!("  Captura Direta de Hardware Kernel DRM/KMS   : \x1b[1;36mext-sender --kms\x1b[0m");
    println!("  Captura GNOME Mutter com Cursor do Mouse    : \x1b[1;36mext-sender --mutter\x1b[0m");
    println!("  Modo Silencioso (Sem Envio de Áudio)        : \x1b[1;36mext-sender --no-audio\x1b[0m\n");

    println!("\x1b[1;33mSINOPSE E SINTAXE FLEXÍVEL:\x1b[0m");
    println!("  ext-sender [SUBCOMANDO]");
    println!("  ext-sender [extend|clone] [FPS] [BITRATE] [OPÇÕES]");
    println!("  ext-sender [IP_DESTINO] [PORTA] [BITRATE] [MODO] [ENCODER] [FPS] [OPÇÕES]\n");

    println!("\x1b[1;33mSUBCOMANDOS:\x1b[0m");
    println!("  \x1b[1;32mstop\x1b[0m                        Interrompe o stream ativo e encerra os processos");
    println!("  \x1b[1;32mstatus\x1b[0m                      Verifica a conexão e telemetria da API do Pi Zero");
    println!("  \x1b[1;32mextend\x1b[0m                      Estende a área de trabalho (HDMI-1 virtual) [PADRÃO]");
    println!("  \x1b[1;32mclone\x1b[0m                       Espelha a tela principal do notebook (eDP-1)\n");

    println!("\x1b[1;33mOPÇÕES E PARÂMETROS (FLAGS):\x1b[0m");
    println!("  \x1b[1;32m--fps=<10-120>\x1b[0m              Taxa de quadros por segundo (padrão: 60 FPS)");
    println!("  \x1b[1;32m--bitrate=<kbps>, -b\x1b[0m        Taxa de bits em kbps (padrão: 6000 para 60 FPS)");
    println!("  \x1b[1;32m--capture=<kms|mutter>\x1b[0m      Motor de captura (kms: Kernel DRM [PADRÃO], mutter: GNOME Mutter)");
    println!("  \x1b[1;32m--kms, --kernel\x1b[0m             Atalho para motor Kernel DRM/KMS direto (zero-copy)");
    println!("  \x1b[1;32m--mutter, --gnome\x1b[0m           Atalho para GNOME Mutter ScreenCast (com cursor do mouse embutido)");
    println!("  \x1b[1;32m--transport=<usb|network>\x1b[0m   Protocolo de envio (usb: USB Bulk [auto-fallback], network: UDP)");
    println!("  \x1b[1;32m--usb, -u, --usb-bulk\x1b[0m       Atalho para Modo 3 (USB Bulk Direto via rusb)");
    println!("  \x1b[1;32m--network, --udp\x1b[0m            Forçar envio direto via rede UDP (porta 5000)");
    println!("  \x1b[1;32m--encoder=<api>\x1b[0m             Encoder: 'vaapi' (AMD/Intel), 'nvenc' (NVIDIA), 'qsv', 'software' ou 'auto'");
    println!("  \x1b[1;32m--color=<full|256|gray>\x1b[0m     Perfil de cor (TrueColor 24-bit, 256 cores QP 30-44, Monocromático)");
    println!("  \x1b[1;32m--no-audio\x1b[0m                  Desativa o encaminhamento de áudio estéreo HDMI");
    println!("  \x1b[1;32m--audio-port=<porta>\x1b[0m        Porta UDP para transmissão de áudio (padrão: 5002)");
    println!("  \x1b[1;32m--hud\x1b[0m                       Ativa o painel de telemetria na tela (auto-oculta em 60s)");
    println!("  \x1b[1;32m--help, -h\x1b[0m                  Exibe esta mensagem de ajuda");
    println!("  \x1b[1;32m--lang=<en|pt|it|zh>\x1b[0m        Seleciona o idioma (Inglês, Português, Italiano, Chinês)\n");

    println!("\x1b[1;33mCOMO PARAR O EXT-SENDER:\x1b[0m");
    println!("  - \x1b[1;37mInterativo:\x1b[0m Pressione \x1b[1;31mCtrl + C\x1b[0m no terminal onde o ext-sender está rodando.");
    println!("  - \x1b[1;37mTerminal/Segundo Plano:\x1b[0m Execute \x1b[1;31mext-sender stop\x1b[0m ou \x1b[1;31mpkill -f ext-sender\x1b[0m.");
    println!("  - \x1b[1;37mPainel Web Remoto:\x1b[0m Ajuste o fluxo ou encerre em tempo real em \x1b[1;34mhttp://192.168.7.2:8080\x1b[0m.\n");
}

fn print_help_it() {
    println!("\x1b[1;32m========================================================================\x1b[0m");
    println!("\x1b[1;32m  ext-sender: Trasmettitore Secondo Monitor Virtuale via GPU v0.3.0     \x1b[0m");
    println!("\x1b[1;34m  100% Rust Nativo | Accelerazione Hardware | Latenza Sub-15ms / Sub-1ms \x1b[0m");
    println!("\x1b[1;32m========================================================================\x1b[0m\n");

    println!("\x1b[1;33m🚀 AVVIO RAPIDO — IL COMANDO MIGLIORE SUBITO (CONSIGLIATO / ZERO CONFIG):\x1b[0m");
    println!("  \x1b[1;32mext-sender\x1b[0m");
    println!("      \x1b[1;37mEsegue la modalità MIGLIORE e più veloce all'istante!\x1b[0m Nessun parametro richiesto.");
    println!("      - Estende il desktop al Pi Zero a \x1b[1;32m60 FPS continui\x1b[0m (CFR Anti-Freeze).");
    println!("      - Encoder hardware GPU auto-rilevato (\x1b[1;36mAMD VA-API, NVIDIA NVENC, Intel QSV\x1b[0m).");
    println!("      - Bitrate ottimizzato di 6000 kbps VBR con latenza sub-15ms.");
    println!("      - Audio stereo HDMI sincronizzato via PipeWire.");
    println!("      - Inibisce la sospensione schermo e recupera automaticamente al risveglio.\n");

    println!("  \x1b[1;32mext-sender --usb\x1b[0m (o \x1b[1;32mext-sender -u\x1b[0m)");
    println!("      \x1b[1;37mModalità 3 (USB Bulk Diretto):\x1b[0m Latenza sub-millisecondo (\x1b[1;32m< 1ms\x1b[0m),");
    println!("      bypassando interamente lo stack di rete TCP/IP.\n");

    println!("  \x1b[1;32mext-sender clone\x1b[0m");
    println!("      \x1b[1;37mClona lo schermo principale\x1b[0m direttamente sul Pi Zero a 60 FPS.\n");

    println!("  \x1b[1;32mext-sender stop\x1b[0m");
    println!("      \x1b[1;31mArresta immediatamente\x1b[0m tutte le trasmissioni e le pipeline attive.\n");

    println!("  \x1b[1;32mext-sender status\x1b[0m");
    println!("      Interroga telemetria e stato del Pi Zero (http://192.168.7.2:8080).\n");

    println!("\x1b[1;33m⚡ PROFILI RAPIDI ED ESEMPI:\x1b[0m");
    println!("  Gaming / Video Fluido (60 FPS, 6M)         : \x1b[1;36mext-sender 60 6000\x1b[0m");
    println!("  Ufficio / Lettura / Risparmio (30 FPS, 2M) : \x1b[1;36mext-sender 30 2000 --color=256\x1b[0m");
    println!("  Cattura Diretta Hardware Kernel DRM/KMS    : \x1b[1;36mext-sender --kms\x1b[0m");
    println!("  GNOME Mutter con Cursore del Mouse         : \x1b[1;36mext-sender --mutter\x1b[0m");
    println!("  Modalità Silenziosa (Disattiva Audio)      : \x1b[1;36mext-sender --no-audio\x1b[0m\n");

    println!("\x1b[1;33mSINOSSI:\x1b[0m");
    println!("  ext-sender [SOTTOCOMANDO]");
    println!("  ext-sender [extend|clone] [FPS] [BITRATE] [OPZIONI]");
    println!("  ext-sender [IP_DESTINAZIONE] [PORTA] [BITRATE] [MODALITA] [ENCODER] [FPS] [OPZIONI]\n");

    println!("\x1b[1;33mSOTTOCOMANDI:\x1b[0m");
    println!("  \x1b[1;32mstop\x1b[0m                        Arresta lo streaming attivo");
    println!("  \x1b[1;32mstatus\x1b[0m                      Controlla connessione e telemetria del Pi Zero");
    println!("  \x1b[1;32mextend\x1b[0m                      Estende il desktop (HDMI-1 virtuale) [PREDEFINITO]");
    println!("  \x1b[1;32mclone\x1b[0m                       Clona lo schermo principale (eDP-1)\n");

    println!("\x1b[1;33mOPZIONI E PARAMETRI:\x1b[0m");
    println!("  \x1b[1;32m--fps=<10-120>\x1b[0m              Frequenza fotogrammi (default: 60 FPS)");
    println!("  \x1b[1;32m--bitrate=<kbps>, -b\x1b[0m        Bitrate in kbps (default: 6000 per 60 FPS)");
    println!("  \x1b[1;32m--capture=<kms|mutter>\x1b[0m      Motore di cattura (kms: DRM Kernel [PREDEFINITO], mutter: GNOME)");
    println!("  \x1b[1;32m--kms, --kernel\x1b[0m             Scorciatoia per cattura Kernel DRM diretta");
    println!("  \x1b[1;32m--mutter, --gnome\x1b[0m           Scorciatoia per GNOME Mutter con cursore del mouse");
    println!("  \x1b[1;32m--transport=<usb|network>\x1b[0m   Protocollo (usb: USB Bulk [auto-fallback], network: UDP)");
    println!("  \x1b[1;32m--usb, -u, --usb-bulk\x1b[0m       Scorciatoia per Modalità 3 (USB Bulk Diretto)");
    println!("  \x1b[1;32m--network, --udp\x1b[0m            Forza invio via rete UDP (porta 5000)");
    println!("  \x1b[1;32m--encoder=<api>\x1b[0m             Encoder: 'vaapi', 'nvenc', 'qsv', 'software', 'auto'");
    println!("  \x1b[1;32m--color=<full|256|gray>\x1b[0m     Profilo colore (TrueColor 24-bit, 256 colori QP 30-44, Monocromatico)");
    println!("  \x1b[1;32m--no-audio\x1b[0m                  Disattiva inoltro audio HDMI");
    println!("  \x1b[1;32m--audio-port=<porta>\x1b[0m        Porta UDP audio (default: 5002)");
    println!("  \x1b[1;32m--hud\x1b[0m                       Attiva telemetria OSD (auto-scomparsa in 60s)");
    println!("  \x1b[1;32m--help, -h\x1b[0m                  Mostra questo messaggio di aiuto");
    println!("  \x1b[1;32m--lang=<en|pt|it|zh>\x1b[0m        Seleziona lingua (Inglese, Portoghese, Italiano, Cinese)\n");

    println!("\x1b[1;33mCOME ARRESTARE EXT-SENDER:\x1b[0m");
    println!("  - \x1b[1;37mInterattivo:\x1b[0m Premi \x1b[1;31mCtrl + C\x1b[0m nel terminale di ext-sender.");
    println!("  - \x1b[1;37mDa Terminale:\x1b[0m Esegui \x1b[1;31mext-sender stop\x1b[0m o \x1b[1;31mpkill -f ext-sender\x1b[0m.");
    println!("  - \x1b[1;37mPannello Web:\x1b[0m Gestisci lo streaming su \x1b[1;34mhttp://192.168.7.2:8080\x1b[0m.\n");
}

fn print_help_zh() {
    println!("\x1b[1;32m========================================================================\x1b[0m");
    println!("\x1b[1;32m  ext-sender: 基于 GPU 硬件加速的虚拟第二显示器发送端 v0.3.0           \x1b[0m");
    println!("\x1b[1;34m  100% 纯 Rust 原生开发 | 硬件加速 | 亚 15ms / 亚 1ms 物理超低延迟     \x1b[0m");
    println!("\x1b[1;32m========================================================================\x1b[0m\n");

    println!("\x1b[1;33m🚀 极速启动 — 推荐最佳命令（即刻运行，零参数配置）：\x1b[0m");
    println!("  \x1b[1;32mext-sender\x1b[0m");
    println!("      \x1b[1;37m即刻以最优、最高速配置运行！\x1b[0m 无需传入任何额外参数。");
    println!("      - 以 \x1b[1;32m60 FPS 连续帧 (CFR 防冻结)\x1b[0m 极速扩展桌面至树莓派 Zero。");
    println!("      - 自动检测并启用 GPU 硬件加速 (\x1b[1;36mAMD VA-API, NVIDIA NVENC, Intel QSV\x1b[0m)。");
    println!("      - 6000 kbps 高清动态码率，端到端延迟低于 15ms。");
    println!("      - 自动通过 PipeWire 同步传输 HDMI 立体声音频。");
    println!("      - 自动阻止屏幕休眠并在主机唤醒时自动重连自愈。\n");

    println!("  \x1b[1;32mext-sender --usb\x1b[0m (或 \x1b[1;32mext-sender -u\x1b[0m)");
    println!("      \x1b[1;37m模式 3 (直接 USB Bulk 直通):\x1b[0m 亚毫秒级物理超低延迟 (\x1b[1;32m< 1ms\x1b[0m),");
    println!("      完全绕过 TCP/IP 网络协议栈开销。\n");

    println!("  \x1b[1;32mext-sender clone\x1b[0m");
    println!("      以 60 FPS 极速\x1b[1;37m镜像笔记本主屏幕\x1b[0m到树莓派。\n");

    println!("  \x1b[1;32mext-sender stop\x1b[0m");
    println!("      \x1b[1;31m立即终止\x1b[0m当前运行的推流进程与音视频管线。\n");

    println!("  \x1b[1;32mext-sender status\x1b[0m");
    println!("      查询树莓派 Zero 的连接与实时遥测状态 (http://192.168.7.2:8080)。\n");

    println!("\x1b[1;33m⚡ 常用预设与快捷用法:\x1b[0m");
    println!("  游戏 / 影视高帧率模式 (60 FPS, 6M)   : \x1b[1;36mext-sender 60 6000\x1b[0m");
    println!("  办公 / 文本阅读省流模式 (30 FPS, 2M) : \x1b[1;36mext-sender 30 2000 --color=256\x1b[0m");
    println!("  内核 DRM/KMS 硬件直通扫描模式        : \x1b[1;36mext-sender --kms\x1b[0m");
    println!("  GNOME Mutter 混入鼠标指针模式        : \x1b[1;36mext-sender --mutter\x1b[0m");
    println!("  静音模式 (禁用 HDMI 音频转发)        : \x1b[1;36mext-sender --no-audio\x1b[0m\n");

    println!("\x1b[1;33m命令格式:\x1b[0m");
    println!("  ext-sender [子命令]");
    println!("  ext-sender [extend|clone] [帧率] [码率] [选项]");
    println!("  ext-sender [目标IP] [端口] [码率] [显示模式] [编码引擎] [帧率] [选项]\n");

    println!("\x1b[1;33m子命令:\x1b[0m");
    println!("  \x1b[1;32mstop\x1b[0m                        终止推流进程并清理环境");
    println!("  \x1b[1;32mstatus\x1b[0m                      查询树莓派连接状态与遥测数据");
    println!("  \x1b[1;32mextend\x1b[0m                      扩展桌面至第二显示器 (虚拟 HDMI-1) [默认]");
    println!("  \x1b[1;32mclone\x1b[0m                       镜像笔记本主屏幕 (eDP-1)\n");

    println!("\x1b[1;33m选项与参数:\x1b[0m");
    println!("  \x1b[1;32m--fps=<10-120>\x1b[0m              目标帧率 (默认: 60 FPS)");
    println!("  \x1b[1;32m--bitrate=<kbps>, -b\x1b[0m        传输码率 kbps (默认: 60 FPS 下为 6000 kbps)");
    println!("  \x1b[1;32m--capture=<kms|mutter>\x1b[0m      画面捕获引擎 (kms: 内核 DRM 硬件直通 [默认], mutter: GNOME)");
    println!("  \x1b[1;32m--kms, --kernel\x1b[0m             内核 DRM/KMS 捕获快捷方式");
    println!("  \x1b[1;32m--mutter, --gnome\x1b[0m           GNOME Mutter 捕获快捷方式 (包含鼠标指针)");
    println!("  \x1b[1;32m--transport=<usb|network>\x1b[0m   传输协议 (usb: USB Bulk [自动降级], network: UDP)");
    println!("  \x1b[1;32m--usb, -u, --usb-bulk\x1b[0m       模式 3 (直接 USB Bulk 直通) 快捷方式");
    println!("  \x1b[1;32m--network, --udp\x1b[0m            直接强制使用 UDP 网络推流 (端口 5000)");
    println!("  \x1b[1;32m--encoder=<api>\x1b[0m             编码器: 'vaapi', 'nvenc', 'qsv', 'software', 'auto'");
    println!("  \x1b[1;32m--color=<full|256|gray>\x1b[0m     色彩配置文件 (24位全彩, 256色粗量化 QP 30-44, 黑白单色)");
    println!("  \x1b[1;32m--no-audio\x1b[0m                  禁用 HDMI 立体声音频转发");
    println!("  \x1b[1;32m--audio-port=<端口>\x1b[0m         音频 UDP 端口 (默认: 5002)");
    println!("  \x1b[1;32m--hud\x1b[0m                       开启屏幕半透明遥测诊断浮层 (60秒后自动隐藏)");
    println!("  \x1b[1;32m--help, -h\x1b[0m                  显示此帮助信息");
    println!("  \x1b[1;32m--lang=<en|pt|it|zh>\x1b[0m        选择帮助信息语言 (英语, 葡萄牙语, 意大利语, 中文)\n");

    println!("\x1b[1;33m如何停止 EXT-SENDER 推流:\x1b[0m");
    println!("  - \x1b[1;37m交互式终止:\x1b[0m 在运行 ext-sender 的终端中按下 \x1b[1;31mCtrl + C\x1b[0m。");
    println!("  - \x1b[1;37m命令行后台终止:\x1b[0m 执行 \x1b[1;31mext-sender stop\x1b[0m 或 \x1b[1;31mpkill -f ext-sender\x1b[0m。");
    println!("  - \x1b[1;37mWeb 控制面板:\x1b[0m 访问 \x1b[1;34mhttp://192.168.7.2:8080\x1b[0m 实时调控或暂停画面。\n");
}
