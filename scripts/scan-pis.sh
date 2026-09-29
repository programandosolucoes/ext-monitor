#!/bin/bash
# scripts/scan-pis.sh: Descobre Raspberry Pis na rede local (Wi-Fi, Ethernet ou USB Gadget)

echo -e "\x1b[1;36m=====================================================\x1b[0m"
echo -e "\x1b[1;36m       Varredura de Raspberry Pis na Rede Local      \x1b[0m"
echo -e "\x1b[1;36m=====================================================\x1b[0m"

echo -e "[*] Interfaces de rede ativas no notebook:"
ip -brief address show | grep -E 'UP|UNKNOWN'

echo -e "\n[*] Procurando Raspberry Pis ativos..."

# 1. Checa a USB Gadget padrão
if ping -c 1 -W 1 192.168.7.2 >/dev/null 2>&1; then
    echo -e "  \x1b[1;32m[+] Pi Zero USB Gadget encontrado:\x1b[0m 192.168.7.2 (Interface: usb0)"
fi

# 2. Varredura na tabela ARP
echo -e "\n[*] Varrendo tabela ARP (MAC Broadcom/Raspberry Pi Foundation):"
ip neigh show | grep -iE 'b8:27:eb|dc:a6:32|e4:5f:01|28:cd:c1|usb0' | while read -r line; do
    echo -e "  \x1b[1;32m[+] Dispositivo encontrado:\x1b[0m $line"
done

# 3. Resolução mDNS (.local)
echo -e "\n[*] Consultando mDNS (raspberrypi.local):"
if command -v avahi-resolve-host-name >/dev/null 2>&1; then
    avahi-resolve-host-name -4 raspberrypi.local 2>/dev/null || echo "  [-] Nenhum host raspberrypi.local respondendo via Avahi."
fi

echo -e "\n\x1b[1;33m[*] Como usar qualquer Pi encontrado:\x1b[0m"
echo -e "  ./scripts/start.sh extend auto 30 hud <IP_DO_PI>"
echo -e "  Exemplo: ./scripts/start.sh extend auto 30 hud 192.168.1.150\n"
