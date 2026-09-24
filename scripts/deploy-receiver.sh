#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"

echo -e "\x1b[1;33m[*] Compilando ext-receiver para ARMv6 (Pi Zero)...\x1b[0m"
cd "$REPO_DIR"
cargo build -p ext-receiver --target arm-unknown-linux-gnueabihf --release

echo -e "\x1b[1;33m[*] Reduzindo tamanho do binário (strip)...\x1b[0m"
arm-linux-gnueabihf-strip "$REPO_DIR/target/arm-unknown-linux-gnueabihf/release/ext-receiver"

echo -e "\x1b[1;33m[*] Enviando binário para o Pi Zero...\x1b[0m"
scp "$REPO_DIR/target/arm-unknown-linux-gnueabihf/release/ext-receiver" pi@192.168.7.2:/tmp/ext-receiver

echo -e "\x1b[1;33m[*] Instalando e reiniciando o serviço no Pi Zero...\x1b[0m"
ssh pi@192.168.7.2 "sudo mv /tmp/ext-receiver /usr/local/bin/ext-receiver && sudo chmod +x /usr/local/bin/ext-receiver && sudo systemctl restart ext-receiver.service"

echo -e "\x1b[1;32m[+] ext-receiver instalado e ativo no Pi Zero com sucesso!\x1b[0m"
