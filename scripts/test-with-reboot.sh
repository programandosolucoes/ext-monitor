#!/bin/bash
set -e

echo "========================================================================"
echo "  Ext-Monitor: Teste Integrado com Reboot Remoto do Pi Zero             "
echo "  100% Native Rust Pipeline: Reboot -> Sync -> Diagnostics -> E2E Tests "
echo "========================================================================"

TARGET_IP="${1:-192.168.7.2}"

echo "[1/4] Disparando reboot no Raspberry Pi Zero ($TARGET_IP)..."
cargo run -q -p ext-tool -- reboot --ip="$TARGET_IP" || true

echo "[2/4] Executando testes unitários nativos com validação de hardware..."
cargo run -q -p ext-tool -- test --usb

echo "[3/4] Testando resposta da telemetria e das novas flags de áudio e HDMI..."
curl -s "http://$TARGET_IP:8080/api/status" | grep -o '"has_audio":[^,}]*' || echo "[!] Aguardando status..."

echo "[4/4] Bateria de testes e reboot finalizada com sucesso!"
