#!/bin/bash
set -e

echo "========================================================================"
echo "  Ext-Monitor: OTA Flash to SD Card & Instant Appliance Reboot          "
echo "========================================================================"

INITRAMFS="build-appliance/boot/initramfs.cpio.gz"
if [ ! -f "$INITRAMFS" ]; then
    echo "[-] Erro: $INITRAMFS não encontrado!"
    exit 1
fi

echo "[*] Aguardando Raspberry Pi Zero conectar em 192.168.7.2..."
while ! ping -c 1 -W 1 192.168.7.2 >/dev/null 2>&1; do
    sleep 1
done

echo "[+] Pi Zero online! Testando Web API..."
for i in {1..10}; do
    if curl -s http://192.168.7.2:8080/api/status >/dev/null 2>&1; then
        break
    fi
    sleep 0.5
done

echo "[*] Subindo servidor HTTP temporário na máquina host (0.0.0.0:8899)..."
python3 -m http.server 8899 --bind 0.0.0.0 --directory build-appliance/boot &
HTTP_PID=$!
trap "kill $HTTP_PID 2>/dev/null || true" EXIT

for i in {1..25}; do
    if curl -s -I http://127.0.0.1:8899/initramfs.cpio.gz >/dev/null 2>&1; then
        echo "[+] Servidor HTTP local ativo e respondendo na porta 8899"
        break
    fi
    sleep 0.2
done

LOCAL_SHA=$(sha256sum "$INITRAMFS" | cut -d' ' -f1)
echo "[*] Local SHA256: $LOCAL_SHA"

echo "[*] Montando partição de boot (/dev/mmcblk0p1) no Pi Zero..."
curl -s -X POST http://192.168.7.2:8080/api/exec \
     -H "Content-Type: text/plain" \
     -d "mkdir -p /mnt/boot && (mount -t vfat /dev/mmcblk0p1 /mnt/boot 2>/dev/null || mount -t vfat /dev/mmcblk0 /mnt/boot 2>/dev/null) && ls -la /mnt/boot"
echo ""

echo "[*] Baixando novo initramfs.cpio.gz para a memória RAM (/tmp) do Pi Zero..."
curl -s -X POST http://192.168.7.2:8080/api/exec \
     -H "Content-Type: text/plain" \
     -d "rm -f /mnt/boot/initramfs.cpio.gz.new 2>/dev/null || true; wget -O /tmp/initramfs.cpio.gz http://192.168.7.1:8899/initramfs.cpio.gz"
echo ""

echo "[*] Gravando no Cartão SD (/mnt/boot)..."
curl -s -X POST http://192.168.7.2:8080/api/exec \
     -H "Content-Type: text/plain" \
     -d "rm -f /mnt/boot/initramfs.cpio.gz && cp /tmp/initramfs.cpio.gz /mnt/boot/initramfs.cpio.gz && rm -f /tmp/initramfs.cpio.gz && sync"
echo ""

echo "[*] Verificando SHA256 no Cartão SD..."
REMOTE_OUT=$(curl -s -X POST http://192.168.7.2:8080/api/exec \
     -H "Content-Type: text/plain" \
     -d "sha256sum /mnt/boot/initramfs.cpio.gz && sync && umount /mnt/boot")
echo "$REMOTE_OUT"
REMOTE_SHA=$(echo "$REMOTE_OUT" | tr -d '\r' | awk '{print $1}' | head -n1)
echo "[*] Remote SHA256: $REMOTE_SHA"

if [ "$LOCAL_SHA" != "$REMOTE_SHA" ]; then
    echo "[-] ERRO: Checksums não conferem! Abortando reboot."
    exit 1
fi
echo "[+] Checksum validado com sucesso!"

echo "[*] Reiniciando Raspberry Pi Zero com o novo sistema..."
curl -s --max-time 2 -X POST http://192.168.7.2:8080/api/exec \
     -H "Content-Type: text/plain" \
     -d "reboot -f" || true

echo "[✔] Cartão SD atualizado com sucesso! O Pi Zero está reiniciando com o novo painel."
