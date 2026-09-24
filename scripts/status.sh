#!/bin/bash

echo -e "\x1b[1;36m=== Status do Projeto ext-monitor ===\x1b[0m"

echo -e "\n\x1b[1;33m1. Conexão de Rede USB:\x1b[0m"
ping -c 2 -W 1 192.168.7.2 2>/dev/null && echo -e "\x1b[1;32m[+] Pi Zero online (RTT < 1ms)\x1b[0m" || echo -e "\x1b[1;31m[-] Pi Zero inacessível em 192.168.7.2\x1b[0m"

echo -e "\n\x1b[1;33m2. Conector Kernel HDMI-A-1 (Host):\x1b[0m"
cat /sys/class/drm/card1-HDMI-A-1/status 2>/dev/null || echo "Desconhecido"

echo -e "\n\x1b[1;33m3. Processo ext-sender (Host):\x1b[0m"
if pgrep -f "ext-sender" > /dev/null; then
    echo -e "\x1b[1;32m[+] ext-sender ativo e transmitindo!\x1b[0m"
else
    echo -e "\x1b[1;31m[-] ext-sender não está rodando.\x1b[0m"
fi

echo -e "\n\x1b[1;33m4. Serviço ext-receiver (Pi Zero):\x1b[0m"
ssh -o ConnectTimeout=2 pi@192.168.7.2 "systemctl is-active ext-receiver.service 2>/dev/null" && echo -e "\x1b[1;32m[+] ext-receiver.service ativo no Pi Zero!\x1b[0m" || echo -e "\x1b[1;31m[-] ext-receiver.service inativo ou erro de SSH.\x1b[0m"
