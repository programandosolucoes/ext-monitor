#!/bin/bash
# scripts/hud-pi.sh: Painel de Telemetria e Diagnostico em Tempo Real do Pi Zero

PI_IP="${1:-192.168.7.2}"

echo -e "\x1b[1;36m=====================================================\x1b[0m"
echo -e "\x1b[1;36m   EXT-MONITOR: Telemetria em Tempo Real (Pi Zero)   \x1b[0m"
echo -e "\x1b[1;36m=====================================================\x1b[0m"
echo -e "Conectando em $PI_IP via SSH..."

ssh -t pi@"$PI_IP" 'bash -s' << 'EOF'
while true; do
    clear
    TEMP=$(vcgencmd measure_temp | cut -d'=' -f2)
    ARM_CLOCK=$(vcgencmd measure_clock arm | awk -F'=' '{printf "%.0f MHz", $2/1000000}')
    CORE_CLOCK=$(vcgencmd measure_clock core | awk -F'=' '{printf "%.0f MHz", $2/1000000}')
    RECEIVER_PID=$(pgrep -f "ext-receiver" | head -n1)
    
    if [ -n "$RECEIVER_PID" ]; then
        CPU_USE=$(ps -p "$RECEIVER_PID" -o %cpu= 2>/dev/null | tr -d ' ')
        MEM_USE=$(ps -p "$RECEIVER_PID" -o %mem= 2>/dev/null | tr -d ' ')
        STATUS="\e[1;32mATIVO (Hardware V4L2 M2M -> KMS)\e[0m"
    else
        CPU_USE="0.0"
        MEM_USE="0.0"
        STATUS="\e[1;31mPARADO\e[0m"
    fi

    echo -e "\e[1;32m=====================================================\e[0m"
    echo -e "\e[1;32m  EXT-MONITOR: Painel de Diagnóstico do Pi Zero W    \e[0m"
    echo -e "\e[1;32m=====================================================\e[0m"
    echo -e "  \e[1mStatus Receptor:\e[0m       $STATUS"
    echo -e "  \e[1mPID / CPU / RAM:\e[0m       PID $RECEIVER_PID | CPU: ${CPU_USE:-0.1}% | RAM: ${MEM_USE:-0.4}%"
    echo -e "  \e[1mTemperatura SoC:\e[0m       \e[1;33m$TEMP\e[0m (Limite seguro: 75°C)"
    echo -e "  \e[1mFrequencia CPU ARM:\e[0m    $ARM_CLOCK"
    echo -e "  \e[1mFrequencia GPU Core:\e[0m   \e[1;36m$CORE_CLOCK\e[0m (Decodificador VideoCore IV)"
    echo -e "  \e[1mRede USB Gadget:\e[0m       192.168.7.2:5000 (UDP)"
    echo -e "\e[1;32m-----------------------------------------------------\e[0m"
    echo -e "Pressione [Ctrl+C] para sair."
    sleep 1
done
EOF
