#!/bin/bash

echo -e "\x1b[1;33m[*] Parando ext-sender...\x1b[0m"
pkill -f "ext-sender" 2>/dev/null || true
pkill -f "ximagesrc" 2>/dev/null || true
echo -e "\x1b[1;32m[+] ext-sender parado com sucesso.\x1b[0m"
