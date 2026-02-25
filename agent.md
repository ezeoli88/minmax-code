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
