# Arquitectura de MinMax Terminal

## Resumen

MinMax Terminal es un cliente de chat con IA para la terminal, construido con **Ink** (React para terminales) y conectado a la API de **MiniMax**. Permite interactuar con modelos LLM desde la CLI con herramientas integradas de sistema de archivos, soporte MCP y persistencia de sesiones en SQLite.

---

## Stack Tecnologico

| Capa | Tecnologia |
|------|------------|
| Runtime | Bun |
| UI Framework | Ink 5 (React 18 para terminal) |
| Lenguaje | TypeScript (ESNext, strict) |
| LLM API | MiniMax (compatible con OpenAI SDK) |
| Base de datos | SQLite via `bun:sqlite` |
| MCP | `@modelcontextprotocol/sdk` |
| Validacion | Zod |

---

## Estructura de Directorios

```
src/
  index.ts                    # Entry point - bootstrap y lifecycle
  app.tsx                     # Componente raiz - routing de API key vs chat
  config/
    settings.ts               # Carga/guardado de config (~/.minmax-terminal/config.json)
    themes.ts                 # Definicion de temas (tokyo-night, rose-pine, gruvbox)
  core/
    api.ts                    # Cliente OpenAI apuntando a MiniMax, streaming
    parser.ts                 # Parser de output XML del modelo (<think>, <minimax:tool_call>)
    tools.ts                  # Registro de herramientas y dispatch
    mcp.ts                    # Inicializacion y manejo de servidores MCP
    commands.ts               # Handler de slash commands (/new, /config, etc.)
    session.ts                # Persistencia de sesiones en SQLite
  hooks/
    useChat.ts                # Estado del chat, loop agentico, ejecucion de tools
    useSession.ts             # CRUD de sesiones, auto-naming
    useMode.ts                # Toggle PLAN/BUILDER
    useQuota.ts               # Polling de cuota de la API MiniMax
    useMouseScroll.ts         # Soporte de scroll con mouse (SGR protocol)
  components/
    ChatInterface.tsx          # Componente principal - orquesta todo el UI
    Header.tsx                 # Barra superior (modelo, modo, directorio)
    MessageList.tsx            # Lista de mensajes virtualizada con scroll
    Message.tsx                # Render individual de mensaje (no usado actualmente)
    Markdown.tsx               # Renderer de markdown para Ink
    Input.tsx                  # Campo de input con prompt contextual
    StatusBar.tsx              # Barra inferior (sesion, tokens, cuota, hints)
    ToolOutput.tsx             # Visualizacion de resultado de tool
    ApiKeyPrompt.tsx           # Pantalla de ingreso de API key
    SessionPicker.tsx          # Selector de sesiones anteriores
    CommandPalette.tsx         # Paleta de comandos (activada con `/`)
    ConfigMenu.tsx             # Menu de configuracion (API key, tema, modelo)
  tools/
    bash.ts                    # Ejecucion de comandos shell
    read-file.ts               # Lectura de archivos con soporte de rangos
    write-file.ts              # Escritura de archivos
    edit-file.ts               # Edicion por reemplazo exacto de strings
    glob.ts                    # Busqueda de archivos por patron glob
    grep.ts                    # Busqueda de contenido con regex
    list-dir.ts                # Listado de directorios con profundidad
```

---

## Flujo de Ejecucion

### 1. Bootstrap (`index.ts`)

```
main()
  -> loadConfig()                  # Lee ~/.minmax-terminal/config.json
  -> initMCPServers()              # Conecta servidores MCP via stdio
  -> render(<App onExit={...} />)  # Monta la UI con Ink
  -> registra SIGINT/SIGTERM       # Shutdown graceful
```

### 2. Routing inicial (`app.tsx`)

```
App
  |-- No API key? -> <ApiKeyPrompt />     # Pide la key y la persiste
  |-- Tiene key?  -> createClient()       # Crea cliente OpenAI con baseURL de MiniMax
                  -> <ChatInterface />     # Monta la interfaz principal
```

### 3. Loop principal de chat (`useChat.ts`)

Este es el nucleo de la aplicacion. Implementa un **loop agentico** que permite al modelo ejecutar herramientas iterativamente:

```
sendMessage(userInput)
  -> Agrega mensaje de usuario al estado y al historial
  -> while (continueLoop):
      1. Construye historial con system prompt (incluye agent.md si existe)
      2. Llama streamChat() con streaming
         - onReasoningChunk: actualiza razonamiento en tiempo real
         - onContentChunk: actualiza contenido, parsea XML incrementalmente
         - onToolCallDelta: acumula tool calls
      3. Parsea output final con parseModelOutput()
         - Extrae <think>...</think> (razonamiento)
         - Extrae <minimax:tool_call>...</minimax:tool_call> (tool calls XML)
         - Prioriza tool calls estructuradas de la API sobre XML parseadas
      4. Si hay tool calls:
         - Ejecuta cada tool secuencialmente
         - Agrega resultados al historial
         - continueLoop = true (el modelo ve los resultados y puede seguir)
      5. Si no hay tool calls -> termina el loop
```

### 4. Modos de operacion (`useMode.ts`)

La app tiene dos modos, alternados con **Tab**:

| Modo | Herramientas disponibles | Proposito |
|------|-------------------------|-----------|
| **BUILDER** | Todas (bash, read, write, edit, glob, grep, list_dir + MCP) | Ejecutar cambios |
| **PLAN** | Solo lectura (read_file, glob, grep, list_directory) | Analizar sin modificar |

El system prompt cambia segun el modo, instruyendo al modelo sobre que puede y no puede hacer.

---

## Capa de API (`core/api.ts`)

- Usa el SDK de **OpenAI** con `baseURL: "https://api.minimax.io/v1"` (API compatible)
- Modelos: `MiniMax-M2.5` (~60 tps) y `MiniMax-M2.5-highspeed` (~100 tps)
- `streamChat()` maneja el streaming SSE con callbacks:
  - Soporta `reasoning_details` (array de `{text}`) y `reasoning_content` de MiniMax
  - Acumula tool calls incrementalmente por indice
  - Captura usage (tokens) de cualquier chunk
  - Soporta cancelacion via `AbortController`
- `fetchCodingPlanRemains()` consulta la cuota del plan de MiniMax

---

## Parser de Output (`core/parser.ts`)

MiniMax M2.5 puede retornar output en formato XML custom:

```xml
<think>razonamiento interno</think>
contenido normal
<minimax:tool_call>
  <invoke name="tool_name">
    <parameter name="param1">valor</parameter>
  </invoke>
</minimax:tool_call>
```

El parser:
1. Extrae bloques `<think>` (completos y parciales durante streaming)
2. Extrae bloques `<minimax:tool_call>` con invocaciones anidadas
3. Maneja tags incompletos (streaming en progreso) con flag `pending`
4. `coerceArg()` convierte strings a tipos JS (bool, int, float, JSON)

Los tool calls estructurados de la API tienen prioridad; el parser XML es un fallback.

---

## Sistema de Herramientas (`core/tools.ts`)

### Registro

```typescript
TOOL_REGISTRY = Map<nombre, ejecutor>
  - bash, read_file, write_file, edit_file, glob, grep, list_directory
```

### Dispatch

```
executeTool(name, args)
  -> Es builtin?   -> TOOL_REGISTRY.get(name)(args)
  -> Es mcp__*?    -> callMCPTool(name, args)
  -> Else          -> "Error: Unknown tool"
```

### Herramientas Built-in

| Tool | Archivo | Limites |
|------|---------|---------|
| `bash` | `tools/bash.ts` | Timeout 30s, output truncado a 10KB |
| `read_file` | `tools/read-file.ts` | Max 2000 lineas, soporte de rangos |
| `write_file` | `tools/write-file.ts` | Crea directorios padre automaticamente |
| `edit_file` | `tools/edit-file.ts` | Requiere match unico del string |
| `glob` | `tools/glob.ts` | Max 500 resultados, excluye dotfiles |
| `grep` | `tools/grep.ts` | Max 200 matches, soporte de contexto, excluye node_modules |
| `list_directory` | `tools/list-dir.ts` | Profundidad configurable, muestra tamanos |

---

## Integracion MCP (`core/mcp.ts`)

- Conecta a servidores MCP externos via **stdio transport**
- Los tools se registran con prefijo `mcp__{serverName}__{toolName}`
- Ciclo de vida:
  1. `initMCPServers()` en bootstrap: conecta y lista tools
  2. `getMCPToolDefinitions()` los expone al modelo
  3. `callMCPTool()` invoca herramientas en el servidor correcto
  4. `shutdownMCPServers()` cierra conexiones al salir

---

## Persistencia (`core/session.ts`)

Base de datos SQLite en `~/.minmax-terminal/sessions.db`:

### Tablas

```sql
sessions (
  id TEXT PRIMARY KEY,        -- UUID
  name TEXT,                  -- Auto-generado del primer mensaje
  model TEXT,
  created_at TEXT,
  updated_at TEXT
)

messages (
  id INTEGER PRIMARY KEY,
  session_id TEXT REFERENCES sessions(id),
  role TEXT,                  -- user | assistant | tool
  content TEXT,
  tool_calls TEXT,            -- JSON serializado
  tool_call_id TEXT,
  name TEXT,
  created_at TEXT
)
```

- WAL mode habilitado para rendimiento
- Foreign keys con CASCADE delete
- Auto-naming: la sesion toma los primeros 50 chars del primer mensaje del usuario

---

## Configuracion (`config/settings.ts`)

Archivo: `~/.minmax-terminal/config.json`

```typescript
interface AppConfig {
  apiKey: string;
  model: string;           // "MiniMax-M2.5" | "MiniMax-M2.5-highspeed"
  theme: string;           // "tokyo-night" | "rose-pine" | "gruvbox"
  mcpServers: Record<string, {
    command: string;
    args?: string[];
    env?: Record<string, string>;
  }>;
}
```

Migracion automatica: si el modelo guardado no es valido, se resetea al default.

---

## Sistema de Temas (`config/themes.ts`)

Cada tema define 14 colores hex:

| Propiedad | Uso |
|-----------|-----|
| `bg`, `surface` | Fondos |
| `border` | Bordes de boxes |
| `text`, `dimText` | Texto principal y secundario |
| `accent` | Highlights, headers, codigo |
| `success`, `warning`, `error` | Feedback visual |
| `purple` | Label del asistente |
| `planBadge`, `builderBadge` | Color del modo activo |
| `userBubble`, `assistantBubble` | Fondo de mensajes |

Temas disponibles: **Tokyo Night** (default), **Rose Pine**, **Gruvbox**.

---

## Componentes UI

### `ChatInterface.tsx` (Orquestador)

Componente central que:
- Compone hooks: `useMode`, `useChat`, `useSession`, `useQuota`, `useMouseScroll`
- Maneja input de teclado: `Tab` (modo), `Esc` (cancelar), `Up/Down` (scroll), `Ctrl+U/D` (paginar)
- Procesa slash commands y los delega
- Controla vistas modales (SessionPicker, ConfigMenu, CommandPalette)
- Monitorea limites de tokens (warning a 180K, auto-nueva sesion a 200K)

### `MessageList.tsx` (Renderizado virtualizado)

- Convierte todos los mensajes a lineas virtuales (`VLine[]`) con `messageToLines()`
- Implementa word-wrapping manual y un markdown parser simplificado
- Scroll por offset: `scrollOffset=0` = fijado abajo, incrementa hacia arriba
- El viewport es una slice del array total de lineas

### `Input.tsx`

- Filtra secuencias de escape de mouse del stdin
- Typing `/` solo abre la CommandPalette
- Prompt cambia segun modo: `plan>` o `build>`

### `CommandPalette.tsx`

- Paleta con 8 comandos navegables con flechas
- Sub-menus para temas y modelos
- Se activa con `/` y se cierra con `Esc`

---

## Diagrama de Dependencias

```
index.ts
  └─ app.tsx
       └─ ChatInterface.tsx
            ├─ Header.tsx
            ├─ MessageList.tsx
            │    └─ (markdownToLines, wrapText)
            ├─ Input.tsx
            ├─ StatusBar.tsx
            ├─ SessionPicker.tsx
            ├─ ConfigMenu.tsx
            ├─ CommandPalette.tsx
            ├─ useChat.ts
            │    ├─ api.ts (streamChat)
            │    ├─ parser.ts (parseModelOutput)
            │    └─ tools.ts (executeTool)
            │         ├─ tools/bash.ts
            │         ├─ tools/read-file.ts
            │         ├─ tools/write-file.ts
            │         ├─ tools/edit-file.ts
            │         ├─ tools/glob.ts
            │         ├─ tools/grep.ts
            │         ├─ tools/list-dir.ts
            │         └─ mcp.ts (callMCPTool)
            ├─ useSession.ts
            │    └─ session.ts (SQLite)
            ├─ useMode.ts
            ├─ useQuota.ts
            │    └─ api.ts (fetchCodingPlanRemains)
            └─ useMouseScroll.ts
```

---

## Build y Ejecucion

```bash
bun install          # Instalar dependencias
bun dev              # Desarrollo con hot reload (--watch)
bun start            # Ejecutar directamente
bun build            # Compilar a binario standalone (minmax.exe)
```

El build usa `bun build --compile` que produce un ejecutable nativo que incluye el runtime de Bun.

---

## Decisiones de Diseno

1. **OpenAI SDK como cliente**: MiniMax expone una API compatible con OpenAI, lo que permite reutilizar el SDK oficial sin adaptadores custom.

2. **Dual parsing (API + XML)**: El modelo puede retornar tool calls como respuesta estructurada de la API o como XML embebido en el contenido. Se priorizan los estructurados y se usa el parser XML como fallback.

3. **Loop agentico en el hook**: `useChat` implementa un `while` loop que permite al modelo ejecutar multiples rondas de herramientas automaticamente hasta que responde sin tools.

4. **MessageList virtualizada**: En lugar de renderizar componentes React por mensaje, se pre-computan todas las lineas como texto plano (`VLine[]`) y se renderizan solo las visibles. Esto evita problemas de rendimiento con conversaciones largas.

5. **Modos PLAN/BUILDER**: Restriccion a nivel de system prompt Y a nivel de tools disponibles. En PLAN solo se envian definiciones de tools read-only al modelo.

6. **SQLite nativo**: Usa `bun:sqlite` (integrado en el runtime) en lugar de dependencias externas como better-sqlite3.

7. **Scroll manual**: No hay auto-scroll durante el loop agentico para no interrumpir al usuario que esta leyendo. Solo se resetea cuando el usuario envia un nuevo mensaje.
