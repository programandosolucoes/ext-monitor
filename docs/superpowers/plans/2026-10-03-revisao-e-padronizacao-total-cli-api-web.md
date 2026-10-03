# Plano de Revisão, Padronização e Revalidação Total (CLI, REST API, Swagger, Web UI e Automação Operacional)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Revalidar, padronizar e blindar os 4 pilares do ext-monitor (CLI do Host, Daemon do Pi Zero, Swagger/OpenAPI e Painel Web), isolando o popup interativo exclusivamente para o Google Cast LAUNCH (com persistência, urgência e botão Cancelar), garantindo sincronismo bidirecional sem sobrescrever o Painel Web, e redigindo o Manual Técnico Operacional definitivo para operação estrita via CLI/HTTP (Zero Edições de Código).

**Architecture:** 
1. **Isolamento de Origem de Sinal:** Restringir o diálogo interativo D-Bus exclusivamente a eventos de `cast_server` (Chrome Cast LAUNCH). Comandos do Painel Web e CLI possuem intenção explícita de transporte e modo, devendo aplicar o estado diretamente sem disparar popups intrusivos.
2. **Robustez do Diálogo D-Bus:** Adicionar `urgency: critical (2)`, `resident: true` e ação explícita `"cancel", "❌ Cancelar Compartilhamento"` no FreeDesktop Notifications para impedir desaparecimento precoce.
3. **Paridade Swagger/OpenAPI & REST:** Auditar e documentar 100% dos endpoints no `swagger.rs` com schemas JSON válidos e parâmetros documentados (`/api/transport/active`, `/api/mode`, `/api/host/control`, `/api/audio/*`, `/api/stream/*`).
4. **Sincronismo do Painel Web:** Garantir que o `web_ui.rs` sincronize o estado ativo real a cada heartbeat e reflita exatamente o que o Host e o Pi estão executando.
5. **Manual Técnico Operacional:** Criar o manual de referência com tabela de comandos CLI e chamadas HTTP REST para que qualquer mudança de modo seja feita estritamente pelas interfaces públicas.

**Tech Stack:** Rust (100% puro), `zbus 5` (D-Bus), `std::net::TcpStream` / `UdpSocket`, OpenAPI 3.0 (Swagger UI), HTML5/CSS3/Vanilla JS (Web UI).

---

## Global Constraints

- **100% Rust Puro:** Proibido uso de `bash -c`, `zenity`, `curl`, `ip` ou qualquer subprocesso externo auxiliar.
- **Zero Edições para Uso Operacional:** Mudanças de modo e transporte devem ser possíveis via `ext-sender switch <mode>` ou `POST http://192.168.7.2:8080/api/...`.
- **Isolamento do Diálogo:** Diálogo interativo só dispara quando a fonte da requisição for `chrome_cast_launch`. Comandos do Painel Web ou CLI nunca devem invocar diálogo.
- **Documentação Integral no Swagger:** Todos os endpoints e seus JSON payloads devem estar descritos em `receiver/src/swagger.rs`.
- **Testes Unitários:** Todas as rotas, decodificadores de argumentos CLI e avaliadores de modo devem ter testes com cobertura total.

---

### Task 1: Isolamento do Diálogo D-Bus Exclusivo para Chrome Cast e Adição de Urgência/Cancelamento

**Files:**
- Modify: `sender/src/dialog.rs`
- Modify: `sender/src/main.rs`
- Modify: `sender/src/cast_server.rs`
- Test: `sender/tests/dialog_test.rs`

**Interfaces:**
- Produces: `dialog::prompt_user_mode_selection_native() -> Option<String>` com flags de urgência crítica, resident=true e ação explícita `cancel`.
- Consumes: `cast_server.rs` dispara `prompt_user_mode_selection()` ao receber `LAUNCH` do Chrome.

- [ ] **Step 1: Atualizar `sender/src/dialog.rs` com urgência crítica e botão Cancelar**
Adicionar hint `urgency: 2u8`, hint `resident: true`, ação `cancel: "❌ Cancelar / Manter Atual"`, e ignorar timeout se o usuário interagir.

- [ ] **Step 2: Remover chamadas de diálogo do laço genérico do painel web/UDP em `sender/src/main.rs`**
Garantir que `ControlAction::StartStreaming` e comandos do painel web apliquem o modo configurado (`extend` ou `clone`) sem abrir diálogo. O diálogo só é invocado quando a ação for explicitamente `ControlAction::PromptCastMode` ou disparada pelo `cast_server`.

- [ ] **Step 3: Testar compilação e unit tests do sender**
Run: `cargo test -p ext-sender`
Expected: 100% PASS

---

### Task 2: Auditoria e Sincronização da API REST e Swagger (`web.rs` e `swagger.rs`)

**Files:**
- Modify: `receiver/src/web.rs`
- Modify: `receiver/src/swagger.rs`
- Test: `receiver/src/web.rs` (módulo `tests`)

**Interfaces:**
- Endpoints auditados:
  * `POST /api/transport/active`: Troca atômica de transporte (Modo 1 UDP, Modo 2 Miracast, Modo 3 USB Bulk, Standby).
  * `POST /api/host/control`: Controle do daemon host (`start`, `stop`, `extend`, `clone`).
  * `POST /api/audio/rate`: Configuração da taxa de amostragem ALSA (44.1k, 48k, 88.2k, 96k, 192k).
  * `POST /api/stream/stop`: Parada atômica de vídeo e retorno imediato à Splash Screen.
  * `GET /api/status`: Telemetria completa (VPU, CPU, temp, active_transport, displays, audio).

- [ ] **Step 1: Mapear todas as rotas no `swagger.rs`**
Assegurar que cada rota existente em `web.rs` esteja rigorosamente descrita no JSON do Swagger em `receiver/src/swagger.rs`, com exemplos de requisição e resposta.

- [ ] **Step 2: Implementar testes unitários para o analisador de rotas e Swagger**
Adicionar testes validando que todo endpoint documentado possui handler correspondente em `web.rs`.

- [ ] **Step 3: Testar compilação do receiver**
Run: `cargo check -p ext-receiver`
Expected: PASS

---

### Task 3: Validação e Robustez do Painel Web (`web_ui.rs`)

**Files:**
- Modify: `receiver/src/web_ui.rs`
- Test: Verificação no navegador e inspeção de código JS

- [ ] **Step 1: Revisar seletores e botões de transporte**
Garantir que ao clicar em "🖥️ Estender (USB)", "🖥️ Estender (UDP)" ou "Windows Miracast", a requisição enviada a `/api/transport/active` contenha o payload canônico e atualize o estado local sem conflitar com o host.

- [ ] **Step 2: Blindar o seletor de modo interativo**
O seletor interativo na Aba 1 deve deixar claro que "Perguntar ao Iniciar" aplica-se exclusivamente quando a transmissão for iniciada pelo Chrome Cast (Google Cast LAUNCH).

---

### Task 4: Padronização da CLI do Host (`sender/src/config.rs`)

**Files:**
- Modify: `sender/src/config.rs`
- Test: `sender/src/config.rs` (módulo `tests`)

**Interfaces:**
- Subcomandos suportados:
  * `ext-sender switch usb-bulk` (ou `ext-sender switch 3`)
  * `ext-sender switch network` (ou `ext-sender switch 1`)
  * `ext-sender switch miracast` (ou `ext-sender switch 2`)
  * `ext-sender switch standby` (ou `ext-sender stop`)
  * `ext-sender mode extend` / `ext-sender mode clone`
  * `ext-sender status`

- [ ] **Step 1: Padronizar o parser de subcomandos**
Garantir que flags como `--mode=extend` e `--mode=clone` possam ser combinadas livremente com `switch` ou passadas diretamente na chamada do executável.

- [ ] **Step 2: Escrever testes unitários para todos os casos de uso de linha de comando**
Adicionar testes cobrindo `switch usb-bulk`, `switch network`, `switch miracast`, `mode extend`, `mode clone`.

---

### Task 5: Manual Técnico Operacional Definitivo (`docs/MANUAL-TECNICO-OPERACIONAL.md`)

**Files:**
- Create: `docs/MANUAL-TECNICO-OPERACIONAL.md`
- Modify: `README.md` e `README.pt-BR.md`

- [ ] **Step 1: Redigir o Manual com tabelas completas de operação**
Documentar:
1. Comandos CLI exatos para o agente e usuário (`ext-sender ...`).
2. Chamadas HTTP REST com exemplos `curl` e payloads JSON para controle remoto.
3. Comportamento do Chrome Cast e seletor gráfico de 4 modos.
4. Tabela de portas e protocolos de rede.
5. Regra de Ouro: Proibição de edição de código para operações de rotina.

- [ ] **Step 2: Gravar nas memórias do projeto e registrar evidências**
Garantir que a regra seja persistida para evitar repetição do erro em sessões futuras.
