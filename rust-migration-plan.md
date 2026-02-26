# Plan de Implementación: Migración de minmax-code a Rust

## Estrategia General

Migración en **5 fases secuenciales**, cada una produce un entregable funcional e independientemente testeable. Cada fase se desarrolla en su propio feature branch y se integra a `main` al completarse.

```
Fase 0 ──→ Fase 1 ──→ Fase 2 ──→ Fase 3 ──→ Fase 4
scaffold    core       TUI base   TUI full   release
(1 día)     (8-10d)    (8-10d)    (6-8d)     (3-5d)
```

**Tiempo total estimado: 26-34 días** (1 desarrollador senior Rust)

---

## Fase 0: Scaffold del Proyecto (1 día)

### Objetivo
Crear la estructura del proyecto Rust, CI mínimo, y validar que compila en todas las plataformas.

### Entregable
Binario "hello world" que compila para linux-x64, linux-arm64, darwin-x64, darwin-arm64, windows-x64.

### Tareas

#### 0.1 — Inicializar proyecto Cargo
```
minmax-code-rs/
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml        # rust version pinning
├── .cargo/
│   └── config.toml            # cross-compilation targets
├── src/
│   ├── main.rs
│   ├── lib.rs                 # re-exports para tests
│   ├── config/
│   │   └── mod.rs
│   ├── core/
│   │   └── mod.rs
│   ├── tools/
│   │   └── mod.rs
│   └── ui/
│       └── mod.rs
└── tests/
    └── integration/
```

#### 0.2 — Cargo.toml base
```toml
[package]
name = "minmax-code"
version = "0.1.0"
edition = "2021"

[dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }

# Error handling
anyhow = "1"
thiserror = "2"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# CLI arguments
clap = { version = "4", features = ["derive"] }

# Logging (development)
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[profile.release]
lto = true
codegen-units = 1
strip = true
panic = "abort"
```

#### 0.3 — GitHub Actions CI
- Build matrix: `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`
- Usar `cross` para cross-compilation de Linux ARM
- Verificar: `cargo clippy`, `cargo test`, `cargo build --release`

#### 0.4 — Criterio de aceptación
- [ ] `cargo build --release` produce binario < 5MB (stripped)
- [ ] CI pasa en las 5 plataformas
- [ ] `cargo clippy -- -D warnings` sin warnings

---

## Fase 1: Core Engine (8-10 días)

### Objetivo
Implementar toda la lógica de negocio sin UI. Al final de esta fase se puede usar como un CLI no-interactivo (pipe stdin → stdout) o ejecutar tests contra el core.

### Dependencias nuevas en esta fase
```toml
# HTTP + SSE streaming
reqwest = { version = "0.12", features = ["stream", "json", "rustls-tls"] }
reqwest-eventsource = "0.6"
futures-util = "0.3"

# XML parsing (tool calls)
quick-xml = "0.37"

# SQLite
rusqlite = { version = "0.32", features = ["bundled"] }

# Filesystem
dirs = "6"
globset = "0.4"
walkdir = "2"

# Search (ripgrep engine)
grep-regex = "0.1"
grep-searcher = "0.1"

# UUID
uuid = { version = "1", features = ["v4"] }
```

### Tareas

#### 1.1 — Config: settings + themes (1 día)
**Mapea**: `src/config/settings.ts` (71 LOC) + `src/config/themes.ts` (74 LOC)

```rust
// src/config/mod.rs
pub mod settings;
pub mod themes;
```

**settings.rs**:
- Struct `AppConfig` con `#[derive(Serialize, Deserialize, Clone)]`
- `load_config()` → lee `~/.minmax-code/config.json`, crea directorio si no existe
- `save_config()` → serializa y escribe
- `update_config(partial)` → merge y guarda
- Modelos válidos como constantes: `AVAILABLE_MODELS`, `VALID_MODELS`

**themes.rs**:
- Struct `Theme` con campos de color como `ratatui::style::Color` (no hex strings)
- Función de conversión `hex_to_color("#1a1b26") → Color::Rgb(26, 27, 38)`
- HashMap estático `THEMES` con tokyo-night, rose-pine, gruvbox
- `DEFAULT_THEME = "tokyo-night"`

**Tests**:
- `load_config` con archivo inexistente devuelve defaults
- `update_config` preserva campos no modificados
- Round-trip serialization de AppConfig

#### 1.2 — Core: API client + SSE streaming (3-4 días)
**Mapea**: `src/core/api.ts` (201 LOC)

```rust
// src/core/api.rs

pub struct MiniMaxClient {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
}
```

**Subtareas**:

1. **SSE Stream Parser** (1.5 días)
   - Usar `reqwest-eventsource` o implementar parser SSE manual sobre `reqwest::Response::bytes_stream()`
   - Parsear cada `data: {...}` como JSON
   - Extraer: `delta.content`, `delta.reasoning_details`, `delta.reasoning_content`, `delta.tool_calls`, `usage`, `finish_reason`
   - Header custom: `X-Reasoning-Split: true`
   - Manejar: `[DONE]` token, errores embebidos en chunks

2. **Stream Callbacks vía Channels** (0.5 días)
   - En vez de callbacks JS, usar `tokio::sync::mpsc::Sender<StreamEvent>`
   ```rust
   pub enum StreamEvent {
       ReasoningChunk(String),
       ContentChunk(String),
       ToolCallDelta(Vec<AccumulatedToolCall>),
       Done(Usage),
       Error(anyhow::Error),
   }
   ```

3. **Tool Call Accumulation** (0.5 días)
   - `HashMap<usize, AccumulatedToolCall>` para acumular deltas por índice
   - Misma lógica que `toolCallsMap` en el TS actual

4. **Quota Fetcher** (0.5 días)
   - `fetch_coding_plan_remains(api_key)` → `Option<QuotaInfo>`
   - GET a `https://api.minimax.io/v1/coding_plan/remains`

5. **AbortHandle** (0.5 días)
   - `tokio_util::sync::CancellationToken` para cancelar streams
   - Equivalente a `AbortController` de JS

**Tests**:
- Mock server (wiremock) que emite SSE chunks → verifica que se parsean correctamente
- Tool call accumulation con deltas parciales
- Manejo de stream vacío (0 chunks)
- Cancelación mid-stream

#### 1.3 — Core: Parser XML de tool calls (1 día)
**Mapea**: `src/core/parser.ts` (144 LOC)

```rust
// src/core/parser.rs

pub struct ParsedOutput {
    pub reasoning: String,
    pub content: String,
    pub tool_calls: Vec<ParsedToolCall>,
    pub pending: bool,
}

pub fn parse_model_output(raw: &str) -> ParsedOutput { ... }
```

**Implementación**:
- Regex-based como el TS (no necesita `quick-xml` para esto)
- Usar `regex` crate para extraer `<think>...</think>` y `<minimax:tool_call>...</minimax:tool_call>`
- `parse_tool_call_block()` → extrae `<invoke name="...">` y `<parameter name="...">...</parameter>`
- `coerce_arg()` → convierte strings a `serde_json::Value`
- Manejo de tags incompletas (streaming parcial)

**Tests**:
- Output completo con reasoning + content + tool calls
- Output con tags incompletas (pending = true)
- Output solo con content (sin tags especiales)
- Múltiples tool calls en un bloque
- Edge case: `<think>` sin cerrar

#### 1.4 — Core: SQLite sessions (1 día)
**Mapea**: `src/core/session.ts` (129 LOC)

```rust
// src/core/session.rs

pub struct SessionStore {
    conn: rusqlite::Connection,
}

impl SessionStore {
    pub fn open() -> Result<Self> { ... }
    pub fn create_session(&self, model: &str) -> Result<Session> { ... }
    pub fn rename_session(&self, id: &str, name: &str) -> Result<()> { ... }
    pub fn list_sessions(&self) -> Result<Vec<Session>> { ... }
    pub fn delete_session(&self, id: &str) -> Result<()> { ... }
    pub fn save_message(&self, ...) -> Result<()> { ... }
    pub fn get_session_messages(&self, session_id: &str) -> Result<Vec<StoredMessage>> { ... }
}
```

- Path: `~/.minmax-code/sessions.db`
- Schema idéntico al actual (sessions + messages)
- `PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;`

**Tests**:
- CRUD completo de sessions y messages
- Ordenamiento por `updated_at DESC`
- Foreign key cascade al eliminar session

#### 1.5 — Tools: 8 herramientas (2-3 días)
**Mapea**: `src/tools/*.ts` (~640 LOC total)

```rust
// src/tools/mod.rs

pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn definition(&self) -> serde_json::Value;  // OpenAI function schema
    async fn execute(&self, args: serde_json::Value) -> ToolResult;
}

pub struct ToolRegistry { ... }
```

**Por herramienta**:

| Tool | LOC actual | Estimación Rust | Notas |
|------|-----------|----------------|-------|
| `bash.rs` | 46 | ~40 | `tokio::process::Command` con timeout 30s |
| `read_file.rs` | 61 | ~50 | `tokio::fs::read_to_string` con truncación 2000 líneas |
| `write_file.rs` | 40 | ~40 | `tokio::fs::write` + crear dirs padre |
| `edit_file.rs` | 57 | ~50 | Leer → buscar único match → reemplazar → escribir |
| `glob.rs` | 45 | ~35 | `globset::GlobBuilder` + `walkdir` |
| `grep.rs` | 225 | ~150 | `grep-regex` + `grep-searcher` como library (sin spawn rg) |
| `list_dir.rs` | 83 | ~60 | `std::fs::read_dir` recursivo |
| `web_search.rs` | 87 | ~60 | POST a `api.minimax.io/v1/coding_plan/search` |

**Ganancia clave en grep.rs**: En vez de spawneo de proceso + fallback JS, se usa directamente el motor de ripgrep como library. Más rápido, sin dependencia binaria externa.

**Tool Registry**:
```rust
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
    read_only: HashSet<String>,
}

impl ToolRegistry {
    pub fn definitions(&self, mode: Mode) -> Vec<serde_json::Value> { ... }
    pub async fn execute(&self, name: &str, args: Value, mode: Mode) -> ToolResult { ... }
}
```

**Tests**:
- Cada tool con tests unitarios (mock filesystem donde aplique)
- Tool registry: mode PLAN bloquea write/edit/bash
- grep: búsqueda regex simple, con contexto, sin matches

#### 1.6 — Core: Slash commands (0.5 días)
**Mapea**: `src/core/commands.ts` (120 LOC)

```rust
// src/core/commands.rs

pub enum CommandResult {
    NewSession,
    Clear,
    Exit,
    Sessions,
    Config,
    SetModel(String),
    SetTheme(String),
    Message(String),
    None,
}

pub fn handle_command(input: &str) -> CommandResult { ... }
```

Traducción directa 1:1 del switch/case actual.

#### 1.7 — Core: Agentic loop (1 día)
**Mapea**: `src/hooks/useChat.ts` (490 LOC) — la lógica, sin el estado React

```rust
// src/core/chat.rs

pub struct ChatEngine {
    client: MiniMaxClient,
    model: String,
    mode: Mode,
    history: Vec<ChatMessage>,
    session_store: Arc<SessionStore>,
    tool_registry: Arc<ToolRegistry>,
    cancel_token: CancellationToken,
}

impl ChatEngine {
    pub async fn send_message(
        &mut self,
        input: &str,
        event_tx: mpsc::Sender<ChatEvent>,
    ) -> Result<()> { ... }
}
```

**ChatEvent** (para la UI):
```rust
pub enum ChatEvent {
    StreamStart,
    ReasoningChunk(String),
    ContentChunk(String),
    ToolCallsUpdate(Vec<AccumulatedToolCall>),
    StreamEnd(FinalMessage),
    ToolExecutionStart { id: String, name: String },
    ToolExecutionDone { id: String, result: String },
    Error(String),
    TokenCount(u64),
}
```

**Lógica del loop agéntico**:
1. Añadir mensaje user al history
2. Llamar `streamChat` → recibir chunks via channel → forwarded a UI via `event_tx`
3. Al terminar stream: parsear tool calls
4. Si hay tool calls → ejecutar en paralelo (`tokio::join!`) → añadir resultados al history → volver a paso 2
5. Si no hay tool calls → fin

**Tests**:
- Mock API que responde con tool call → verifica ejecución → segunda llamada sin tools → fin
- Cancelación mid-loop
- Error en tool → se agrega como resultado de error → el loop continúa

#### Criterio de aceptación Fase 1
- [ ] `cargo test` pasa con 100% de tests
- [ ] Se puede instanciar `ChatEngine` y correr una conversación completa contra mock API
- [ ] Grep usa motor ripgrep nativo (no spawn externo)
- [ ] Sessions persisten y se pueden recuperar desde SQLite

---

## Fase 2: TUI Base (8-10 días)

### Objetivo
Terminal UI funcional con el flujo principal: escribir mensaje → ver respuesta streameada → tool calls → loop. Sin overlays (command palette, file picker, config menu, session picker).

### Dependencias nuevas
```toml
# TUI
ratatui = "0.29"
crossterm = "0.28"

# Markdown terminal rendering
termimad = "0.30"

# Text input
tui-textarea = "0.7"
```

### Arquitectura TUI

```
┌─────────────────────────────────────────┐
│  main.rs                                │
│  ┌──────────────────────────────────┐   │
│  │  App (estado global)             │   │
│  │  ├── event_loop (crossterm)      │   │
│  │  ├── chat_engine (Fase 1)        │   │
│  │  └── ui_state                    │   │
│  │      ├── messages: Vec<UiMessage>│   │
│  │      ├── scroll_offset: usize    │   │
│  │      ├── input_buffer: String    │   │
│  │      └── mode: Mode             │   │
│  └──────────────────────────────────┘   │
│                                         │
│  Event Loop:                            │
│  loop {                                 │
│    terminal.draw(|f| ui(f, &app))?;     │
│    match next_event() {                 │
│      Key(k)    => app.handle_key(k),    │
│      Chat(ev)  => app.handle_chat(ev),  │
│      Tick      => app.handle_tick(),    │
│    }                                    │
│  }                                      │
└─────────────────────────────────────────┘
```

### Tareas

#### 2.1 — App scaffold + event loop (1.5 días)

```rust
// src/main.rs

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Parse CLI args (clap)
    // 2. Load config
    // 3. Init MCP servers (if configured)
    // 4. Enter alternate screen + enable raw mode
    // 5. Create App
    // 6. Run event loop
    // 7. Cleanup (restore screen, close DB, shutdown MCP)
}
```

```rust
// src/app.rs

pub struct App {
    config: AppConfig,
    chat_engine: ChatEngine,
    ui_state: UiState,
    event_rx: mpsc::Receiver<AppEvent>,
    should_quit: bool,
}

pub enum AppEvent {
    Key(crossterm::event::KeyEvent),
    Mouse(crossterm::event::MouseEvent),
    Resize(u16, u16),
    Chat(ChatEvent),
    Tick,
}
```

- Thread de eventos crossterm → channel
- Task de chat engine → channel
- Main loop consume ambos channels con `tokio::select!`
- Tick cada 100ms para animaciones (spinner)

#### 2.2 — Layout principal (1 día)
**Mapea**: `ChatInterface.tsx` (estructura de layout)

```
┌──────────────── Header ────────────────┐
│ minmax-code  MiniMax-M2.5  [BUILDER]   │
├──────────────── Messages ──────────────┤
│                                        │
│  > You                                 │
│    Hola, explícame este código         │
│                                        │
│  ◆ Assistant                           │
│    Este código hace...                 │
│    → read_file (path=src/main.rs)      │
│                                        │
│  ⚡ read_file ✓                        │
│    1  fn main() { ...                  │
│                                        │
├──────────────── Input ─────────────────┤
│ build> _                               │
├──────────────── StatusBar ─────────────┤
│ New Session  │  1,234 tokens  │  45/50 │
└────────────────────────────────────────┘
```

Ratatui layout:
```rust
let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
        Constraint::Length(1),      // Header
        Constraint::Min(3),         // Messages (flex grow)
        Constraint::Length(3),      // Input
        Constraint::Length(1),      // StatusBar
    ])
    .split(area);
```

#### 2.3 — Header widget (0.5 días)
**Mapea**: `Header.tsx` (41 LOC)

- `Line::from(vec![Span, Span, Span])` con nombre del app, modelo, badge de modo
- Badge PLAN: fondo `theme.plan_badge`, badge BUILDER: fondo `theme.builder_badge`
- Toggle con Tab

#### 2.4 — Message list con scroll (3-4 días)
**Mapea**: `MessageList.tsx` (420 LOC) — componente más complejo

**Subcomponentes**:

1. **Message → Lines conversion** (1.5 días)
   - `fn message_to_lines(msg: &UiMessage, theme: &Theme, width: u16) -> Vec<StyledLine>`
   - User: `"> You"` bold + contenido wrapped
   - Assistant: `"◆ Assistant"` + reasoning (max 3 líneas) + content markdown + tool calls
   - Tool result: `"⚡ tool_name ✓"` + preview/diff

2. **Markdown rendering** (1 día)
   - `fn markdown_to_lines(text: &str, theme: &Theme, width: u16) -> Vec<StyledLine>`
   - Code blocks con color accent
   - Headers bold
   - Lists con `- ` prefix
   - Blockquotes con `│ ` prefix
   - Word-wrap a ancho disponible
   - Usar `termimad` para parsing, convertir a `ratatui::text::Line`

3. **Virtual scroll** (1 día)
   - `scroll_offset = 0` → pinned al fondo (como el TS actual)
   - Arrow up/down → scroll ±3 líneas
   - Ctrl+U/Ctrl+D → half page
   - Mouse wheel scroll
   - Auto-scroll al fondo en mensajes nuevos (reset scroll)

4. **Tool result rendering** (0.5 días)
   - edit_file: diff con líneas `-` (rojo) y `+` (verde)
   - write_file: preview con primeras N líneas
   - Otros: contenido truncado

#### 2.5 — Text input (1 día)
**Mapea**: `Input.tsx` (88 LOC)

- `tui-textarea` para input de texto con cursor
- Prompt dinámico: `plan>` o `build>` con color según modo
- Submit con Enter
- Detección de `/` al inicio → activar command palette (Fase 3)
- Detección de `@` → activar file picker (Fase 3)
- Borde redondeado con color del modo

#### 2.6 — Status bar (0.5 días)
**Mapea**: `StatusBar.tsx` (38 LOC)

- Layout horizontal: nombre sesión | tokens totales | quota (si disponible)
- `Line::from(vec![session_span, separator, tokens_span, separator, quota_span])`

#### 2.7 — Thinking indicator / Spinner (0.5 días)
**Mapea**: `ThinkingIndicator.tsx` (25 LOC)

- Spinner animado con tick timer (100ms)
- Secuencia: `⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧ ⠇ ⠏`
- Se muestra cuando `is_streaming && content.is_empty()`

#### 2.8 — Integración chat engine ↔ UI (1 día)
- Conectar `ChatEvent` channel al estado de UI
- `ContentChunk` → actualizar último mensaje assistant
- `ToolExecutionStart/Done` → actualizar mensajes de tool result
- `StreamEnd` → finalizar mensaje, permitir input
- Enter en input → `chat_engine.send_message()` en task separado
- Escape → cancelar stream

#### Criterio de aceptación Fase 2
- [ ] Se puede iniciar la app, escribir un mensaje, y ver la respuesta streaming
- [ ] Tool calls se ejecutan y el loop agéntico funciona
- [ ] Scroll funciona con teclado y mouse
- [ ] Tab cambia entre PLAN y BUILDER
- [ ] Escape cancela un stream en curso
- [ ] Spinner aparece mientras el modelo "piensa"
- [ ] Temas se aplican correctamente

---

## Fase 3: TUI Completa — Overlays + Features (6-8 días)

### Objetivo
Completar todos los overlays y features secundarios para llegar a paridad funcional con la versión TypeScript.

### Tareas

#### 3.1 — Command Palette (1.5 días)
**Mapea**: `CommandPalette.tsx` (174 LOC)

- Overlay sobre el message list
- Lista de comandos con navegación ↑↓ + Enter
- Sub-menús para `/theme` y `/model`
- Esc para cerrar
- Se activa al presionar `/` en input vacío

**Implementación**:
```rust
pub struct CommandPalette {
    commands: Vec<Command>,
    selected: usize,
    sub_menu: Option<SubMenu>,
}

impl CommandPalette {
    pub fn render(&self, area: Rect, buf: &mut Buffer, theme: &Theme) { ... }
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<PaletteResult> { ... }
}
```

#### 3.2 — File Picker (1.5 días)
**Mapea**: `FilePicker.tsx` (177 LOC)

- Overlay que muestra archivos del CWD filtrados por query
- `walkdir` con depth limit = 4, max 200 archivos
- Filtrado in-memory por substring (case insensitive)
- Scroll con ventana visible centrada en selección
- Se activa al escribir `@` en el input
- Enter selecciona, Tab autocompleta directorios

#### 3.3 — Config Menu (1.5 días)
**Mapea**: `ConfigMenu.tsx` (269 LOC)

- Full-screen overlay (reemplaza el layout principal)
- 3 opciones: API Key, Theme, Model
- API Key: input con máscara `*`
- Theme/Model: sub-listas con selección
- Esc para volver al chat

#### 3.4 — Session Picker (1 día)
**Mapea**: `SessionPicker.tsx` (96 LOC)

- Full-screen overlay
- Lista de sesiones ordenadas por `updated_at DESC`
- Preview: nombre + fecha
- Enter para cargar, Esc para cancelar

#### 3.5 — API Key Prompt (0.5 días)
**Mapea**: `ApiKeyPrompt.tsx` (72 LOC)

- Pantalla inicial si no hay API key configurada
- Input con máscara para la key
- Submit guarda en config

#### 3.6 — @file references (0.5 días)
**Mapea**: `resolveFileReferences()` en `ChatInterface.tsx`

- Parsear `@filepath` del input del usuario
- Leer archivos referenciados (truncar a 50KB)
- Enviar como contexto al API: `<file path="...">...</file>`
- En el UI mostrar solo el texto del usuario (sin el contenido del archivo)

#### 3.7 — MCP Client (1.5 días)
**Mapea**: `src/core/mcp.ts` (117 LOC)

```rust
// src/core/mcp.rs

pub struct McpManager {
    connections: HashMap<String, McpConnection>,
}

struct McpConnection {
    child: tokio::process::Child,
    reader: BufReader<ChildStdout>,
    writer: BufWriter<ChildStdin>,
    tools: HashMap<String, McpToolInfo>,
}
```

**Protocolo MCP (simplificado)**:
- Spawneo de proceso con stdio transport
- JSON-RPC 2.0 sobre stdin/stdout
- `initialize` → `tools/list` → almacenar definiciones
- `tools/call` → enviar args, recibir resultado
- Prefijo `mcp__servername__toolname`

**Sin SDK oficial** — implementar el subconjunto mínimo del protocolo:
1. `initialize` handshake
2. `tools/list` discovery
3. `tools/call` execution
4. Shutdown graceful

#### 3.8 — agent.md support + /init command (0.5 días)
- Detectar `agent.md` en CWD
- Cargar contenido y anexar al system prompt
- Comando `/init` crea template

#### 3.9 — Token limit warning (0.5 días)
- Warning a 180K tokens
- Auto-new session a 200K tokens

#### Criterio de aceptación Fase 3
- [x] Command palette funciona con sub-menús de theme/model
- [x] File picker filtra y selecciona archivos
- [x] Config menu permite cambiar API key, tema, modelo
- [x] Session picker lista y carga sesiones previas
- [x] @file references funcionan
- [x] MCP servers se conectan y sus tools aparecen
- [x] /init crea agent.md
- [x] Token limit warning funciona
- [x] **Paridad funcional** completa con versión TypeScript

---

## Fase 4: Release Engineering (3-5 días)

### Objetivo
CI/CD, instaladores, documentación, y optimizaciones finales.

### Tareas

#### 4.1 — GitHub Actions release workflow (1 día)
```yaml
# Reemplaza el workflow actual basado en Bun
name: Release
on:
  push:
    tags: ["v*"]

jobs:
  build:
    strategy:
      matrix:
        include:
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-latest
          - target: aarch64-unknown-linux-gnu
            os: ubuntu-latest  # usa cross
          - target: x86_64-apple-darwin
            os: macos-latest
          - target: aarch64-apple-darwin
            os: macos-latest
          - target: x86_64-pc-windows-msvc
            os: windows-latest
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - name: Build
        run: cargo build --release --target ${{ matrix.target }}
      # ... packaging
```

**Ventajas sobre el workflow actual**:
- No necesita descargar ripgrep por separado (está integrado como library)
- No necesita Bun runtime
- Binarios más pequeños (~5-10MB vs 50-90MB)

#### 4.2 — Instaladores (0.5 días)
- `install.sh`: detectar OS/arch, descargar binario correcto de GitHub Releases
- `install.ps1`: versión PowerShell para Windows
- Misma lógica que los instaladores actuales pero URLs actualizados

#### 4.3 — Optimización de tamaño de binario (0.5 días)
```toml
[profile.release]
lto = true
codegen-units = 1
strip = true
panic = "abort"
opt-level = "z"  # optimize for size
```

Target: < 10MB por plataforma (realista con SQLite bundled + ripgrep engine).

#### 4.4 — Smoke tests E2E (1 día)
- Script que ejecuta el binario compilado
- Verifica: startup, config creation, help output, version
- Verifica: sesión se crea en SQLite
- Mock API server → verifica flujo completo de conversación

#### 4.5 — Migración de datos existentes (0.5 días)
- Detectar `~/.minmax-code/` existente con datos de la versión TS
- Config JSON: compatible directamente (mismo schema)
- SQLite DB: compatible directamente (mismo schema)
- No se necesita migración — es drop-in replacement

#### 4.6 — README + changelog (0.5 días)
- Actualizar README con instrucciones de instalación Rust
- Changelog con diff de features vs versión TS
- Documentar: requisitos de compilación para contribuidores

#### Criterio de aceptación Fase 4
- [x] Release workflow produce binarios para 5 plataformas
- [x] Binarios < 10MB cada uno (opt-level = "z" configurado)
- [x] Instaladores funcionan en Linux, macOS, Windows
- [x] Config y DB de versión TS son compatibles (mismo schema)
- [ ] Smoke tests E2E (pendiente para implementar con CI)
- [ ] README actualizado (pendiente)

---

## Mapeo Completo de Archivos: TypeScript → Rust

| TypeScript | LOC | Rust | Fase |
|-----------|-----|------|------|
| `src/index.ts` | 107 | `src/main.rs` | 2 |
| `src/app.tsx` | 39 | `src/app.rs` | 2 |
| `src/config/settings.ts` | 71 | `src/config/settings.rs` | 1 |
| `src/config/themes.ts` | 74 | `src/config/themes.rs` | 1 |
| `src/core/api.ts` | 201 | `src/core/api.rs` | 1 |
| `src/core/parser.ts` | 144 | `src/core/parser.rs` | 1 |
| `src/core/session.ts` | 129 | `src/core/session.rs` | 1 |
| `src/core/tools.ts` | 81 | `src/core/tool_registry.rs` | 1 |
| `src/core/commands.ts` | 120 | `src/core/commands.rs` | 1 |
| `src/core/mcp.ts` | 117 | `src/core/mcp.rs` | 3 |
| `src/core/tool-meta.ts` | 3 | (inline en tools) | 1 |
| `src/hooks/useChat.ts` | 490 | `src/core/chat.rs` | 1+2 |
| `src/hooks/useSession.ts` | 83 | (inline en session.rs) | 1 |
| `src/hooks/useMode.ts` | 13 | (inline en app.rs) | 2 |
| `src/hooks/useQuota.ts` | 25 | `src/core/quota.rs` | 1 |
| `src/hooks/useMouseScroll.ts` | 40 | (inline en app.rs) | 2 |
| `src/tools/bash.ts` | 46 | `src/tools/bash.rs` | 1 |
| `src/tools/read-file.ts` | 61 | `src/tools/read_file.rs` | 1 |
| `src/tools/write-file.ts` | 40 | `src/tools/write_file.rs` | 1 |
| `src/tools/edit-file.ts` | 57 | `src/tools/edit_file.rs` | 1 |
| `src/tools/glob.ts` | 45 | `src/tools/glob.rs` | 1 |
| `src/tools/grep.ts` | 225 | `src/tools/grep.rs` | 1 |
| `src/tools/list-dir.ts` | 83 | `src/tools/list_dir.rs` | 1 |
| `src/tools/web-search.ts` | 87 | `src/tools/web_search.rs` | 1 |
| `src/components/ChatInterface.tsx` | 418 | `src/ui/chat.rs` | 2+3 |
| `src/components/MessageList.tsx` | 420 | `src/ui/messages.rs` | 2 |
| `src/components/Input.tsx` | 88 | `src/ui/input.rs` | 2 |
| `src/components/Header.tsx` | 41 | `src/ui/header.rs` | 2 |
| `src/components/StatusBar.tsx` | 38 | `src/ui/status_bar.rs` | 2 |
| `src/components/ThinkingIndicator.tsx` | 25 | `src/ui/spinner.rs` | 2 |
| `src/components/Message.tsx` | 150 | `src/ui/messages.rs` | 2 |
| `src/components/Markdown.tsx` | 175 | `src/ui/markdown.rs` | 2 |
| `src/components/CommandPalette.tsx` | 174 | `src/ui/command_palette.rs` | 3 |
| `src/components/ConfigMenu.tsx` | 269 | `src/ui/config_menu.rs` | 3 |
| `src/components/FilePicker.tsx` | 177 | `src/ui/file_picker.rs` | 3 |
| `src/components/SessionPicker.tsx` | 96 | `src/ui/session_picker.rs` | 3 |
| `src/components/ApiKeyPrompt.tsx` | 72 | `src/ui/api_key_prompt.rs` | 3 |
| `src/components/ToolOutput.tsx` | 42 | `src/ui/messages.rs` | 2 |

---

## Riesgos y Mitigaciones

| Riesgo | Probabilidad | Impacto | Mitigación |
|--------|-------------|---------|------------|
| MCP protocol changes | Media | Alto | Implementar solo el subconjunto usado; abstraer tras trait |
| Ratatui learning curve | Media | Medio | Prototipo del layout más complejo (MessageList) primero |
| SSE parsing edge cases | Media | Medio | Tests extensivos con capturas reales del API de MiniMax |
| grep-regex API breaks | Baja | Medio | Pin versión exacta; fallback a `Command::new("rg")` |
| Markdown rendering quality | Media | Bajo | Usar `termimad` como base; custom overrides donde falle |
| Cross-compilation Windows | Baja | Medio | CI validación temprana en Fase 0 |

---

## Métricas de Éxito

| Métrica | TypeScript actual | Target Rust |
|---------|------------------|-------------|
| Tamaño binario (Linux x64) | ~60MB | < 10MB |
| Startup time | ~300ms | < 50ms |
| Memoria idle | ~80MB | < 10MB |
| Memoria en conversación activa | ~120MB | < 30MB |
| Tests | 0 | > 50 unit + 5 integration |
| Dependencias externas en runtime | 1 (ripgrep) | 0 |

---

*Plan generado el 2026-02-26 para el repositorio minmax-code.*
