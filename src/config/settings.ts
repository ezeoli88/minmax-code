import { existsSync, mkdirSync, readFileSync, writeFileSync } from "fs";
import { homedir } from "os";
import { join } from "path";
import { defaultTheme } from "./themes.js";

export interface AppConfig {
  apiKey: string;
  model: string;
  theme: string;
  mcpServers: Record<
    string,
    { command: string; args?: string[]; env?: Record<string, string> }
  >;
}

const CONFIG_DIR = join(homedir(), ".minmax-terminal");
const CONFIG_FILE = join(CONFIG_DIR, "config.json");

const DEFAULT_MODEL = "MiniMax-M2.5";
const VALID_MODELS = ["MiniMax-M2.5", "MiniMax-M2.5-highspeed"];

const defaultConfig: AppConfig = {
  apiKey: "",
  model: DEFAULT_MODEL,
  theme: defaultTheme,
  mcpServers: {},
};

export function loadConfig(): AppConfig {
  try {
    if (!existsSync(CONFIG_DIR)) {
      mkdirSync(CONFIG_DIR, { recursive: true });
    }
    if (!existsSync(CONFIG_FILE)) {
      saveConfig(defaultConfig);
      return { ...defaultConfig };
    }
    const raw = readFileSync(CONFIG_FILE, "utf-8");
    const parsed = JSON.parse(raw);
    const config = { ...defaultConfig, ...parsed };

    // Migrate: if saved model is not valid, reset to default
    if (!VALID_MODELS.includes(config.model)) {
      config.model = DEFAULT_MODEL;
      saveConfig(config);
    }

    return config;
  } catch {
    return { ...defaultConfig };
  }
}

export function saveConfig(config: AppConfig): void {
  try {
    if (!existsSync(CONFIG_DIR)) {
      mkdirSync(CONFIG_DIR, { recursive: true });
    }
    writeFileSync(CONFIG_FILE, JSON.stringify(config, null, 2), "utf-8");
  } catch (err) {
    console.error("Failed to save config:", err);
  }
}

export function updateConfig(partial: Partial<AppConfig>): AppConfig {
  const config = loadConfig();
  const updated = { ...config, ...partial };
  saveConfig(updated);
  return updated;
}
