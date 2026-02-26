# Análisis de Viabilidad: Migración de minmax-code a Rust

## Estado Actual del Proyecto

| Aspecto | Detalle |
|---------|---------|
| Lenguaje | TypeScript (strict mode, ESNext) |
| Runtime | Bun |
| UI Framework | React 18 + Ink 5 (React para terminal) |
| Líneas de código | ~4,400 LOC en 38 archivos |
| Base de datos | SQLite (via `bun:sqlite`) |
| Distribución | Binario standalone (`bun build --compile`) |
| Tests | No hay suite de tests formal |

### Módulos principales

| Módulo | LOC | Complejidad | Descripción |
|--------|-----|-------------|-------------|
| `components/` (14 archivos) | ~2,100 | Alta | UI terminal con React/Ink |
| `hooks/` (5 archivos) | ~650 | Alta | Lógica de chat, sesiones, estado |
| `core/` (7 archivos) | ~800 | Media-Alta | API, parser XML, MCP, sesiones SQLite |
| `tools/` (8 archivos) | ~640 | Media | Herramientas de filesystem, bash, grep |
| `config/` (2 archivos) | ~100 | Baja | Settings y temas |

---

## Equivalencias en el Ecosistema Rust

### 1. UI de Terminal (React/Ink → Ratatui)

**Actual**: React + Ink (modelo declarativo con componentes, hooks, estado reactivo)

**En Rust**: [Ratatui](https://github.com/ratatui/ratatui) + [crossterm](https://github.com/crossterm-rs/crossterm)

| Característica | TypeScript (Ink) | Rust (Ratatui) |
|----------------|-----------------|----------------|
| Paradigma | Declarativo (React) | Inmediato (immediate mode) |
| Estado | Hooks (`useState`, `useEffect`) | Struct + enum manual |
| Componentes | JSX/TSX | Funciones que pintan en `Frame` |
| Scroll virtual | Builtin con Ink | Manual o con `tui-scrollview` |
| Text input | `ink-text-input` | `tui-textarea` o manual |
| Spinner | `ink-spinner` | `throbber-widgets-tui` |
| Markdown rendering | Custom (`Markdown.tsx`) | `termimad` o `mdcat` |

**Impacto**: Este es el **cambio más grande**. El modelo mental cambia completamente: de React declarativo a un loop `draw → handle_event → update_state`. Los 2,100 LOC de componentes necesitan reescritura total, no traducción.

### 2. HTTP Client + Streaming (OpenAI SDK → reqwest)

**Actual**: `openai` SDK de npm con streaming SSE

**En Rust**:
- [`reqwest`](https://docs.rs/reqwest) para HTTP
- [`async-openai`](https://github.com/64bit/async-openai) o cliente SSE manual
- [`eventsource-stream`](https://docs.rs/eventsource-stream) para parsear Server-Sent Events

```rust
// Ejemplo conceptual de streaming SSE en Rust
let response = client.post(url)
    .header("X-Reasoning-Split", "true")
    .json(&body)
    .send()
    .await?;

let mut stream = response.bytes_stream().eventsource();
while let Some(event) = stream.next().await {
    // procesar tokens incrementales
}
```

**Impacto**: Medio. `async-openai` soporta la API compatible de OpenAI, pero los headers custom de MiniMax (`X-Reasoning-Split`) y el formato de respuesta extendido requieren un cliente personalizado.

### 3. Parser XML (Custom → quick-xml / roxmltree)

**Actual**: Parser custom en `parser.ts` (144 LOC) que extrae `<think>` y `<minimax:tool_call>` de streams

**En Rust**:
- [`quick-xml`](https://docs.rs/quick-xml) para parsing incremental
- [`roxmltree`](https://docs.rs/roxmltree) para parsing de documentos completos

**Impacto**: Bajo. El parser actual es relativamente simple y se traduce bien a Rust. El parsing incremental (streaming) es incluso más natural en Rust con iteradores.

### 4. SQLite (bun:sqlite → rusqlite)

**Actual**: `bun:sqlite` nativo

**En Rust**:
- [`rusqlite`](https://docs.rs/rusqlite) (binding directo a SQLite3)
- [`sqlx`](https://docs.rs/sqlx) (async, compile-time checked queries)

**Impacto**: Bajo. Migración directa. `rusqlite` es maduro y la API es similar. El esquema es simple (2 tablas).

### 5. MCP - Model Context Protocol (SDK JS → SDK Rust)

**Actual**: `@modelcontextprotocol/sdk` (stdio transport)

**En Rust**:
- [`mcp-rust-sdk`](https://github.com/Derek-X-Wang/mcp-rust-sdk) (comunidad)
- O implementación manual del protocolo JSON-RPC sobre stdio

**Impacto**: Medio-Alto. El SDK oficial de MCP es para TypeScript/Python. El soporte en Rust es comunitario y menos maduro. Los 117 LOC de `mcp.ts` necesitarían más código en Rust.

### 6. File System & Process Spawning

**Actual**: `Bun.spawn()`, `Bun.file()`, `node:fs`, `node:path`

**En Rust**:
- `std::fs` y `std::path` (builtin)
- `std::process::Command` o [`tokio::process`](https://docs.rs/tokio/latest/tokio/process/index.html)
- [`glob`](https://docs.rs/glob) para pattern matching
- [`grep-regex`](https://docs.rs/grep-regex) (el motor de ripgrep directamente como library)

**Impacto**: Bajo. Rust tiene soporte nativo excelente. Bonus: podrías usar el motor de ripgrep como library en vez de spawnearlo como proceso.

### 7. Async Runtime

**Actual**: Event loop de Bun (implícito)

**En Rust**:
- [`tokio`](https://tokio.rs/) (estándar de facto)
- Necesario para HTTP streaming, process spawning async, y timers

**Impacto**: Bajo. Tokio es maduro y bien documentado.

---

## Análisis FODA

### Fortalezas de migrar a Rust

1. **Binario nativo real**: Sin runtime embebido. El binario de Bun compile incluye el runtime completo (~50-90MB). Un binario Rust sería ~5-15MB.
2. **Rendimiento de arranque**: Startup instantáneo vs ~200-500ms del runtime de Bun.
3. **Uso de memoria**: Significativamente menor. Un TUI en Ratatui usa ~5-10MB vs ~50-100MB con React/Ink/Bun.
4. **Ripgrep como library**: En vez de spawn un proceso externo, se puede usar `grep-regex`/`grep-searcher` directamente como dependencia.
5. **Cross-compilation**: `cross` hace trivial compilar para Linux/macOS/Windows sin CI matrix compleja.
6. **Sin dependencias de runtime**: No necesitas Bun instalado ni bundlear ripgrep por separado.
7. **Seguridad de tipos**: El sistema de tipos de Rust es más robusto que TypeScript, especialmente para manejo de errores.

### Debilidades / Desafíos

1. **Reescritura de UI completa**: ~2,100 LOC de componentes React no se "migran", se reescriben desde cero con un paradigma completamente diferente.
2. **Curva de aprendizaje**: Si el equipo no tiene experiencia en Rust, el borrow checker y lifetimes añaden fricción significativa.
3. **Velocidad de iteración**: Prototipar UI en Ratatui es más lento que en React/Ink. Los cambios de layout requieren más código boilerplate.
4. **Ecosistema MCP inmaduro**: El SDK de MCP para Rust no es oficial. Podría requerir mantenimiento propio.
5. **Markdown rendering**: No hay equivalente directo de la calidad de rendering de terminal markdown que se tiene actualmente.
6. **Tiempo de compilación**: Builds incrementales de ~5-15s vs instant en Bun.

### Oportunidades

1. **Distribución más simple**: Un solo binario sin dependencias externas.
2. **Integración directa con ripgrep**: Usar las mismas crates que ripgrep usa internamente.
3. **Mejor manejo de señales y procesos**: Control fino sobre procesos hijos (bash tool).
4. **Plugin system nativo**: Posibilidad de cargar plugins como shared libraries (.so/.dylib/.dll).

### Amenazas

1. **Velocidad de desarrollo reducida**: Features nuevas tomarán más tiempo de implementar.
2. **Contribuciones externas**: TypeScript tiene una base de contribuidores mucho mayor que Rust.
3. **MCP evoluciona rápido**: Si el protocolo cambia frecuentemente, mantener un cliente Rust propio es costoso.

---

## Estimación de Esfuerzo

| Módulo | Esfuerzo | Notas |
|--------|----------|-------|
| Setup proyecto (Cargo, CI, estructura) | 1-2 días | Straightforward |
| Core: API client + streaming SSE | 3-5 días | Custom SSE parser para MiniMax |
| Core: Parser XML (tool calls) | 1-2 días | Traducción directa |
| Core: SQLite sessions | 1 día | `rusqlite` es directo |
| Core: MCP client | 3-5 días | Sin SDK oficial, implementación manual |
| Tools: file/bash/grep/glob | 2-3 días | Buen soporte nativo en Rust |
| UI: Layout principal + navigation | 5-7 días | Ratatui desde cero |
| UI: Chat/message list con scroll | 3-5 días | Virtualización manual |
| UI: Input con edición de texto | 2-3 días | Multiline, shortcuts |
| UI: Markdown rendering | 3-5 días | `termimad` + customización |
| UI: Config menu, file picker, etc. | 3-5 días | Componentes secundarios |
| UI: Temas y colores | 1 día | Mapeo directo |
| Config: Settings, CLI args | 1-2 días | `clap` + `serde` |
| Integración y debugging | 5-7 días | Integrar todo, edge cases |
| **TOTAL ESTIMADO** | **~35-55 días** | **1 desarrollador con experiencia en Rust** |

---

## Estructura Propuesta en Rust

```
minmax-code-rs/
├── Cargo.toml
├── src/
│   ├── main.rs                 # Entry point, CLI args (clap)
│   ├── app.rs                  # Estado global de la app, event loop
│   ├── ui/
│   │   ├── mod.rs
│   │   ├── chat.rs             # Vista principal de chat
│   │   ├── messages.rs         # Lista de mensajes con scroll
│   │   ├── input.rs            # Input de texto multiline
│   │   ├── markdown.rs         # Rendering de markdown
│   │   ├── config_menu.rs      # Menú de configuración
│   │   ├── file_picker.rs      # Selector de archivos
│   │   ├── session_picker.rs   # Browser de sesiones
│   │   ├── command_palette.rs  # Comandos /slash
│   │   ├── status_bar.rs       # Barra inferior
│   │   └── theme.rs            # Temas de color
│   ├── core/
│   │   ├── mod.rs
│   │   ├── api.rs              # Cliente MiniMax (reqwest + SSE)
│   │   ├── parser.rs           # Parser de tool calls XML
│   │   ├── mcp.rs              # Cliente MCP (JSON-RPC/stdio)
│   │   ├── session.rs          # Persistencia SQLite
│   │   └── commands.rs         # Slash commands
│   ├── tools/
│   │   ├── mod.rs
│   │   ├── bash.rs             # Ejecución de shell
│   │   ├── file_ops.rs         # read/write/edit file
│   │   ├── search.rs           # grep (ripgrep engine) + glob
│   │   └── web_search.rs       # Búsqueda web
│   └── config.rs               # Settings (serde + dirs)
```

### Dependencias clave (Cargo.toml)

```toml
[dependencies]
# TUI
ratatui = "0.29"
crossterm = "0.28"
tui-textarea = "0.7"

# Async
tokio = { version = "1", features = ["full"] }

# HTTP + Streaming
reqwest = { version = "0.12", features = ["stream", "json"] }
eventsource-stream = "0.2"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# XML parsing
quick-xml = "0.37"

# SQLite
rusqlite = { version = "0.32", features = ["bundled"] }

# CLI
clap = { version = "4", features = ["derive"] }

# Search (ripgrep internals)
grep-regex = "0.1"
grep-searcher = "0.1"
globset = "0.4"

# Markdown
termimad = "0.30"

# Utilities
dirs = "5"           # ~/.minmax-code/
anyhow = "1"         # Error handling
tracing = "0.1"      # Logging
```

---

## Recomendación

### Veredicto: Viable, pero con matices

**La migración es técnicamente viable.** El ecosistema de Rust tiene equivalentes para cada componente. Sin embargo, la decisión depende de las prioridades:

#### Migrar a Rust SI:
- El tamaño del binario y el consumo de memoria son problemas reales para los usuarios
- Se quiere distribuir el CLI sin dependencias externas (sin bundlear ripgrep)
- El equipo tiene experiencia en Rust o quiere invertir en aprenderlo
- Se busca máxima performance en operaciones de filesystem y búsqueda
- La estabilidad es más importante que la velocidad de iteración

#### Mantener TypeScript/Bun SI:
- La velocidad de desarrollo de features nuevas es la prioridad
- El equipo es primariamente TypeScript
- El ecosistema MCP sigue evolucionando rápido (SDK oficial es TS/Python)
- La UI del chat necesita iteración frecuente
- El tamaño del binario (~50-90MB) es aceptable

#### Alternativa intermedia: Migración gradual
1. **Fase 1**: Reescribir el core (API client, parser, tools) como un CLI "headless" en Rust
2. **Fase 2**: Añadir la TUI con Ratatui una vez que el core esté estable
3. **Fase 3**: Migrar MCP client cuando haya un SDK Rust más maduro

Esta aproximación permite validar la viabilidad sin comprometerse a una reescritura completa de una sola vez.

---

*Análisis generado el 2026-02-26 para el repositorio minmax-code.*
