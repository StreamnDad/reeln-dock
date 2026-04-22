/** Application-level logging store with level filtering. */

import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type LogLevel = "debug" | "info" | "warn" | "error";

export interface LogEntry {
  timestamp: number;
  level: LogLevel;
  source: string;
  message: string;
}

interface DockLogEvent {
  level: string;
  source: string;
  message: string;
}

const LEVEL_ORDER: Record<LogLevel, number> = { debug: 0, info: 1, warn: 2, error: 3 };
const MAX_ENTRIES = 1000;

let entries = $state<LogEntry[]>([]);
let minLevel = $state<LogLevel>("debug");
let unlisten: UnlistenFn | null = null;

/** Start listening for backend log events. Call once during app init. */
export async function initLogListener(): Promise<void> {
  if (unlisten) return;
  unlisten = await listen<DockLogEvent>("dock:log", (event) => {
    const { level, source, message } = event.payload;
    const validLevel = level in LEVEL_ORDER ? (level as LogLevel) : "info";
    push(validLevel, source, message);
  });
}

export function getLogEntries(): LogEntry[] {
  const min = LEVEL_ORDER[minLevel];
  return entries.filter((e) => LEVEL_ORDER[e.level] >= min);
}

export function getAllLogEntries(): LogEntry[] {
  return entries;
}

export function getLogLevel(): LogLevel {
  return minLevel;
}

export function setLogLevel(level: LogLevel): void {
  minLevel = level;
}

export function clearLogs(): void {
  entries = [];
}

function push(level: LogLevel, source: string, message: string): void {
  const entry: LogEntry = { timestamp: Date.now(), level, source, message };
  entries = [...entries.slice(-(MAX_ENTRIES - 1)), entry];
}

export const log = {
  debug: (source: string, message: string) => push("debug", source, message),
  info: (source: string, message: string) => push("info", source, message),
  warn: (source: string, message: string) => push("warn", source, message),
  error: (source: string, message: string) => push("error", source, message),
};
