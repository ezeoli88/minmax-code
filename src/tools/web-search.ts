import { loadConfig } from "../config/settings.js";

export const definition = {
  type: "function" as const,
  function: {
    name: "web_search",
    description:
      "Search the web for current information. Use when you need up-to-date data, documentation, or answers not available in local files. Returns top results with titles, URLs, and snippets.",
    parameters: {
      type: "object",
      properties: {
        query: {
          type: "string",
          description: "The search query",
        },
      },
      required: ["query"],
    },
  },
};

interface SearchResult {
  title?: string;
  url?: string;
  snippet?: string;
  content?: string;
}

interface SearchResponse {
  organic_results?: SearchResult[];
  results?: SearchResult[];
  related_searches?: string[];
  [key: string]: any;
}

export async function execute(args: { query: string }): Promise<string> {
  const config = loadConfig();
  if (!config.apiKey) {
    return "Error: No API key configured. Run /config to set it.";
  }

  try {
    const res = await fetch("https://api.minimax.io/v1/coding_plan/search", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${config.apiKey}`,
      },
      body: JSON.stringify({ q: args.query }),
    });

    if (!res.ok) {
      const text = await res.text().catch(() => "");
      return `Error: Search API returned ${res.status}${text ? ` — ${text.slice(0, 200)}` : ""}`;
    }

    const data: SearchResponse = await res.json();

    // Extract results — the API may use different field names
    const results = data.organic_results || data.results || [];
    const items = results.slice(0, 8);

    if (items.length === 0) {
      return `No results found for "${args.query}".`;
    }

    const formatted = items.map((r, i) => {
      const title = r.title || "Untitled";
      const url = r.url || "";
      const snippet = r.snippet || r.content || "";
      return `${i + 1}. **${title}**\n   ${url}\n   ${snippet}`;
    });

    let output = formatted.join("\n\n");

    if (data.related_searches && data.related_searches.length > 0) {
      output += `\n\nRelated searches: ${data.related_searches.slice(0, 5).join(", ")}`;
    }

    return output;
  } catch (err: any) {
    if (err.code === "ENOTFOUND" || err.cause?.code === "ENOTFOUND") {
      return "Error: No internet connection.";
    }
    return `Error: ${err.message}`;
  }
}
