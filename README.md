<p align="center">
  <h1 align="center">minmax-code</h1>
  <p align="center">AI coding assistant in your terminal — think Cursor, but in your terminal.</p>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/runtime-Bun-f472b6?style=flat-square&logo=bun" />
  <img src="https://img.shields.io/badge/UI-Ink%20(React%2018)-61dafb?style=flat-square&logo=react" />
  <img src="https://img.shields.io/badge/LLM-MiniMax%20M2.5-7c3aed?style=flat-square" />
  <img src="https://img.shields.io/badge/license-MIT-22c55e?style=flat-square" />
</p>

<p align="center">
  <a href="https://mm-code.pro">Website</a> · <a href="#install">Install</a> · <a href="#usage">Usage</a> · <a href="https://platform.minimaxi.com">Get API Key</a>
</p>

---

### PLAN mode — read-only analysis

<img src="docs/cli-plan-mode.gif" alt="Plan mode demo" width="100%" />

### BUILDER mode — full agentic execution

<img src="docs/cli-builder-mode.gif" alt="Builder mode demo" width="100%" />

---

## Install

**macOS / Linux:**

```bash
curl -fsSL https://raw.githubusercontent.com/ezeoli88/minmax-code/main/install.sh | bash
```

**Windows (PowerShell):**

```powershell
irm https://raw.githubusercontent.com/ezeoli88/minmax-code/main/install.ps1 | iex
```

Standalone binary — no dependencies, no runtime needed. Includes bundled ripgrep.

On first launch you'll be prompted for a [MiniMax API key](https://platform.minimaxi.com) (free tier available).

---

## Usage

```bash
minmax-code
```

### Modes

Toggle with **Tab**:

| Mode | Description |
|------|-------------|
| **PLAN** | Read-only — AI can analyze and suggest but not modify anything |
| **BUILDER** | Full access — AI can read, write, edit files and run commands |

### Commands

Type `/` to open the command palette:

| Command | Description |
|---------|-------------|
| `/new` | New chat session |
| `/sessions` | Browse & resume previous sessions |
| `/model` | Switch model |
| `/theme` | Change color theme |
| `/config` | Open settings |
| `/init` | Create `agent.md` template |
| `/clear` | Clear chat |
| `/exit` | Quit |

### Keyboard shortcuts

| Key | Action |
|-----|--------|
| `Tab` | Toggle PLAN / BUILDER |
| `Esc` | Cancel AI response |
| `Up/Down` | Scroll |
| `Ctrl+U/D` | Half-page scroll |
| `@` | Attach files |

---

## Tools

The AI has 7 built-in tools:

| Tool | Description | PLAN | BUILDER |
|------|-------------|:----:|:-------:|
| `read_file` | Read file contents | x | x |
| `glob` | Find files by pattern | x | x |
| `grep` | Search with regex (ripgrep) | x | x |
| `list_directory` | Directory tree | x | x |
| `write_file` | Create/overwrite files | | x |
| `edit_file` | Find-and-replace in files | | x |
| `bash` | Run shell commands | | x |

Extend with [MCP servers](#mcp) for unlimited capabilities.

---

## Models

| Model | Speed | Best for |
|-------|-------|----------|
| `MiniMax-M2.5` | ~60 tok/s | Complex reasoning |
| `MiniMax-M2.5-highspeed` | ~100 tok/s | Quick iterations |

Switch with `/model`.

---

## Themes

Three built-in themes — switch with `/theme`:

- **tokyo-night** (default) — cool blues and purples
- **rose-pine** — soft pinks and muted tones
- **gruvbox** — warm retro palette

---

## agent.md

Drop an `agent.md` in your project root to give the AI persistent context. Auto-loaded into the system prompt.

```bash
/init   # generates a template
```

---

## MCP

Extend the AI by connecting [Model Context Protocol](https://modelcontextprotocol.io) servers in `~/.minmax-code/config.json`:

```json
{
  "mcpServers": {
    "github": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-github"],
      "env": { "GITHUB_TOKEN": "ghp_..." }
    }
  }
}
```

MCP tools appear as `mcp__servername__toolname`.

---

## Configuration

All config lives at `~/.minmax-code/config.json`. Sessions persist in `~/.minmax-code/sessions.db` (SQLite).

---

## Development

```bash
bun install       # install deps
bun dev           # run with hot reload
bun start         # run normally
bun run build     # compile standalone binary
```

---

## License

MIT
