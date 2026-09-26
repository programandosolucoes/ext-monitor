# Blueprint 07: Cartão Micro-SD em RAM, Proteção Elétrica e Upgrade de Firmware via USB

**Projeto:** `ext-monitor`  
**Autor:** Carlos Alberto & Antigravity  
**Arquivos de Referência:** `build-appliance/initramfs/init`, `receiver/src/web.rs`, `receiver/src/web_ui.rs`  
**Data:** Setembro de 2026  

---

## 1. Visão Geral

Um dos maiores problemas crônicos que afetam usuários de Raspberry Pi no mundo inteiro é a **corrupção do sistema de arquivos do cartão micro-SD**:
* Quando um sistema operacional Linux convencional (Raspberry Pi OS / Ubuntu) está em execução, o kernel realiza escritas em segundo plano contínuas (logs do systemd, journal do ext4, swap, caches do navegador).
* Se o usuário desligar o computador ou puxar o cabo USB repentinamente, uma operação de escrita de bloco em andamento corrompe a árvore de inodes, impedindo que o cartão inicialize novamente.

O `ext-monitor` resolve esse problema de forma definitiva: **o sistema operacional executa 100% em memória RAM**, e o cartão micro-SD permanece completamente **desmontado, travado contra escritas e livre para ser atualizado pela porta USB**.

---

## 2. A Mecânica da Execução 100% em Memória RAM

Durante a inicialização do aparelho:
1. O Bootloader de silício Broadcom lê o arquivo compactado `initramfs.cpio.gz` da partição FAT16 e o descompacta diretamente na memória RAM (`/dev/ram0`).
2. O kernel assume o controle e executa o script `/init`.
3. O script `/init` monta sistemas de arquivos virtuais (`tmpfs` para `/tmp`, `/var` e `/run`, `procfs` para `/proc`, `sysfs` para `/sys`).
4. **Nenhum sistema de arquivos físico é mantido montado.**
5. O driver do cartão de memória (`/dev/mmcblk0`) entra em modo de repouso (idle).

### Vantagens Desta Abordagem:
* **Zero Risco de Corrupção Elétrica:** O usuário pode puxar o cabo USB ou desligar a tomada a qualquer momento sem nenhum risco de danificar os arquivos de boot.
* **Velocidade Extrema:** A leitura e escrita de binários e configurações operam na velocidade da memória RAM (LPDDR2 a centenas de Megabytes por segundo), sem a lentidão de leitura de cartões SD baratos.
* **Vida Útil Infinita da Mídia Flash:** Sem escritas contínuas em células NAND, o cartão micro-SD dura indefinidamente.

---

## 3. O Cartão Micro-SD Exposto Direto na USB (USB Mass Storage Gadget)

Como o sistema operacional em RAM não mantém a partição `/dev/mmcblk0p1` montada, podemos conectá-la diretamente à função **USB Mass Storage** do Gadget ConfigFS:

```sh
mkdir -p functions/mass_storage.0
echo 1 > functions/mass_storage.0/stall
echo 0 > functions/mass_storage.0/lun.0/cdrom
echo 0 > functions/mass_storage.0/lun.0/ro
echo 0 > functions/mass_storage.0/lun.0/nofua
echo "/dev/mmcblk0p1" > functions/mass_storage.0/lun.0/file
ln -sf functions/mass_storage.0 configs/c.1/
```

### O Que Acontece no Computador Host (Windows / Linux / Mac):
1. No instante em que o cabo USB é conectado, o sistema operacional do PC detecta uma **unidade de disco removível (pendrive)** com o rótulo **`EXTMONITOR`**.
2. Ao abrir o Explorador de Arquivos no Windows ou o Gerenciador de Arquivos no Linux, todos os arquivos de firmware são exibidos:
   * `config.txt` (Configurações de resolução e HDMI)
   * `cmdline.txt` (Linha de comando do kernel)
   * `initramfs.cpio.gz` (Sistema de arquivos do appliance)
   * `kernel.img` (Kernel Linux do Pi Zero 1 ARMv6)
   * `kernel7.img` (Kernel Linux do Pi Zero 2 W ARMv7)
   * Arquivos `.dtb` e overlays.

---

## 4. Como Fazer Upgrade de Firmware Sem Retirar o Cartão do Pi Zero

O usuário dispõe de três métodos práticos para atualizar o sistema:

### Método 1: Arrastar e Soltar no Explorador de Arquivos do Host
1. Conecte o Pi Zero ao computador via cabo USB central.
2. Abra a unidade de disco **`EXTMONITOR`**.
3. Substitua o arquivo `initramfs.cpio.gz` ou o binário `kernel.img` pelo novo arquivo baixado das releases.
4. Ejete com segurança a unidade no Windows ou Linux.
5. Reinicie o Pi Zero (via Web Dashboard ou reconectando o cabo). A nova versão inicializará imediatamente!

---

### Método 2: Upgrade Através do Web Dashboard Integrado (`http://192.168.7.2:8080`)
Na **Aba 4 (Cartão SD & Upgrade em RAM)** do painel web:
1. Clique no botão **"Montar Cartão SD"**. O appliance executa internamente:
   ```bash
   mkdir -p /mnt/boot && mount -t vfat /dev/mmcblk0p1 /mnt/boot
   ```
2. O painel exibe a lista completa de arquivos no cartão, espaço livre e integridade.
3. Arraste e solte o novo arquivo de firmware na área de upload da página.
4. Clique em **"Desmontar com Segurança"** e em seguida em **"Reiniciar Appliance"**.

---

### Método 3: Upgrade via Linha de Comando (`curl` / `wget`)
Para administradores que desejam automação via script no computador host:
```bash
# 1. Monta o cartão SD no Pi Zero
curl -s -X POST http://192.168.7.2:8080/api/sdcard/mount

# 2. Transfere a nova imagem do initramfs diretamente
curl -T build-appliance/boot/initramfs.cpio.gz http://192.168.7.2:8080/api/sdcard/upload?filename=initramfs.cpio.gz

# 3. Desmonta o cartão com segurança
curl -s -X POST http://192.168.7.2:8080/api/sdcard/unmount

# 4. Reinicia o Raspberry Pi Zero
curl -s -X POST http://192.168.7.2:8080/api/system/reboot
```

---

## 5. Salvaguardas de Segurança Implementadas

* **Proteção contra Concorrência de Montagem:** Para evitar que o host e o Pi Zero gravem no mesmo volume FAT16 simultaneamente (o que geraria corrupção de FAT), o endpoint `/api/sdcard/mount` verifica se a unidade está sendo acessada ativamente pelo host antes de conceder escrita.
* **Sincronização de Buffers:** Toda operação de gravação no cartão executa a chamada de sistema `sync()` obrigatória e remonta em modo somente-leitura antes da confirmação do usuário.
