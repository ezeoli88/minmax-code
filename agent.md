# MinMax Terminal

An AI-powered terminal interface built with Ink (React for terminals), allowing users to interact with MiniMax LLMs through a rich command-line chat experience with integrated file system tools and MCP support.

## Project Overview

This is a terminal-based chat application that combines:
- **Interactive CLI Chat**: Chat with MiniMax AI models directly in your terminal
- **File System Tools**: Read, write, edit, search and navigate files
- **MCP Integration**: Connect to Model Context Protocol servers for extended capabilities
- **Session Management**: Persistent chat sessions stored in SQLite
- **Themed UI**: Multiple terminal color themes

## Tech Stack

- **Runtime**: Bun
- **UI Framework**: Ink (React for terminal)
- **LLM Provider**: MiniMax API
- **Database**: SQLite (via better-sqlite3)
- **MCP**: @modelcontextprotocol/sdk
- **Language**: TypeScript

## Configuration

Configuration is stored in `~/.minmax-terminal/config.json`:

```json
{
  "apiKey": "your-api-key",
  "model": "MiniMax-M2.5",
  "theme": "tokyo-night",
  "mcpServers": {
    "server-name": {
      "command": "npx",
      "args": ["-y", "@some/mcp-server"],
      "env": {}
    }
  }
}
```

### Available Models

- `MiniMax-M2.5` - Full reasoning (~60 tps)
- `MiniMax-M2.5-highspeed` - Faster response (~100 tps)

### Available Themes

- `tokyo-night` (default)
- `rose-pine`
- `gruvbox`

## Built-in Tools

The terminal provides these file system tools:

| Tool | Description | Parameters |
|------|-------------|------------|
| `bash` | Execute shell commands | `command: string` |
| `read-file` | Read file contents | `path: string`, `start_line?: number`, `end_line?: number` |
| `write-file` | Write content to a file | `path: string`, `content: string` |
| `edit-file` | Edit a file by string replacement | `path: string`, `old_str: string`, `new_str: string` |
| `glob` | Find files matching a pattern | `cwd?: string`, `pattern: string` |
| `grep` | Search file contents | `path: string`, `pattern: string`, `include?: string`, `context_lines?: number` |
| `list-dir` | List directory contents | `path?: string`, `max_depth?: number` |

## Commands

| Command | Description |
|---------|-------------|
| `/new` | Start a new chat session |
| `/sessions` | Browse previous sessions |
| `/config` | Open configuration menu |
| `/model [name]` | Change or list available models |
| `/theme [name]` | Change or list themes |
| `/init` | Create an agent.md template in current directory |
| `/clear` | Clear current chat |
| `/exit` | Exit the terminal |
| `/help` | Show help |

## MCP Integration

The application supports the Model Context Protocol (MCP) for connecting to external tools and services. Configure MCP servers in the config file:

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/path/to/directory"]
    }
  }
}
```

MCP tools are prefixed with `mcp__` when called (e.g., `mcp__servername__toolname`).

## Key Source Files

- `src/index.ts` - Application entry point
- `src/app.tsx` - Main React component
- `src/core/api.ts` - MiniMax API client and streaming
- `src/core/parser.ts` - Parses AI output (reasoning, content, tool calls)
- `src/core/tools.ts` - Tool registry and execution
- `src/core/mcp.ts` - MCP server initialization and tool handling
- `src/core/commands.ts` - Command handling
- `src/hooks/useChat.ts` - Chat state management
- `src/components/` - UI components

## Development

```bash
# Install dependencies
bun install

# Run in development mode
bun dev

# Build
bun build

# Start
bun start
```

## Output Parsing

The AI output is parsed to extract:
- **Reasoning** (`<think>...</think>`): Model's thinking process
- **Content**: Regular text response
- **Tool Calls** (`<minimax:tool_call>...</minimax:tool_call>`): Function calls to execute

This follows MiniMax's specific XML-style tool call format.
