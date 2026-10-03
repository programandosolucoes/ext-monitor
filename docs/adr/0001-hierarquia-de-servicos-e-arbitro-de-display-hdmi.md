# 0001. Hierarquia de Serviços, Arbitragem do Display HDMI e Arquitetura em Micro-Blocos

Data: 2026-10-03  
Status: Aprovado  

## Contexto

O receptor Ext-Monitor opera em um Raspberry Pi Zero com uma única porta física HDMI (controlada pelo driver do kernel Linux `vc4-drm` e BCM2835).
Diferentes funcionalidades precisam utilizar a tela e os alto-falantes da TV:
1. Modos de tela de computador (Modo 3 USB Bulk, Modo 1 Rede UDP, Modo 2 Miracast).
2. Transmissão de mídia avulsa (DLNA, Google Cast).
3. Transmissão de áudio independente (Notebook usando a TV como caixa de som Bluetooth/UDP com equalizador opcional).
4. Tela estática de boas-vindas / pronto para conectar (Splash Screen).

Anteriormente, sem um árbitro centralizado com hierarquia rígida, o visualizador de espectro de áudio (`VisualizerEngine`) e o decodificador de vídeo (`V4L2 M2M` / `KMS`) competiam pela posse do hardware HDMI. Isso gerava duas falhas graves:
- `Plane update failed (Permission denied (os error 13))`: O driver DRM recusava a criação de plano de sobreposição porque múltiplos descritores tentavam agir como DRM Master.
- Efeito "Looping" na TV: Ao receber áudio com vídeo pausado, o visualizador pintava o equalizador durante o som e restaurava a tela de aviso de serviço durante silêncios (> 800ms).

## Decisão

Adotamos uma **Hierarquia de 4 Níveis de Serviços** com um **Árbitro Central de Hardware** e separação do código em **Micro-Blocos de Fluxo**:

1. **Nível 0: Modos de Tela Principal (Prioridade Máxima e Exclusiva sobre Vídeo)**
   - Modo 3 (USB Bulk), Modo 1 (Rede UDP) e Modo 2 (Miracast).
   - O decoder tem posse exclusiva do KMS Plane.
   - Qualquer visualizador gráfico, equalizador ou splash é **estritamente inibido** enquanto o Nível 0 estiver ativo.
   - O áudio é cooperativo: toca no ALSA sem tocar no display.

2. **Nível 1: Streaming de Mídia / Cast (Sem tela desktop)**
   - Ativo apenas se o Nível 0 for nulo. O player dedicado assume o display e o áudio.

3. **Nível 2: Áudio Isolado do Notebook (Modo Caixa de Som)**
   - Áudio ativo sem transmissão de vídeo. O display exibe uma tela estática suave, ou o equalizador gráfico se expressamente ativado pelo usuário na API (`/api/media/visualizer`).
   - O equalizador não oscila para tela de aviso em intervalos de silêncio.

4. **Nível 3: Standby / Repouso**
   - Exibição de tela estática de espera ("Pronto para Conectar") com zero consumo de processamento.

5. **Arquitetura de Micro-Blocos de Fluxo**:
   - Cada etapa do pipeline (USB ingress, Rede ingress, Miracast ingress, V4L2 decoder, KMS display, Framebuffer display, ALSA audio, FFT spectrum) é isolada em um arquivo próprio dentro de `receiver/src/flow/` e `sender/src/flow/`.
   - Nomes de arquivos refletem com precisão o fluxo de dados.
   - Todo o estado de transição é verificado por testes unitários determinísticos.

## Consequências

- **Positivas**:
  - Eliminação definitiva de colisões de `DRM Master` e `os error 13`.
  - Fim do flickering e do looping visual na TV Philips.
  - Facilidade de manutenção: cada micro-bloco pode ser testado, alterado ou estendido de forma isolada sem risco de quebrar os outros modos.
  - Regra de "Zero Code Edits for Routine Operations" integralmente respeitada.
- **Negativas**:
  - Exige que qualquer novo modo ou funcionalidade de tela registre-se e respeite o Árbitro de Display antes de desenhar no framebuffer ou KMS.
