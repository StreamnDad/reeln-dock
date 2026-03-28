/** Application-level logging store with level filtering. */

export type LogLevel = "debug" | "info" | "warn" | "error";

export interface LogEntry {
  timestamp: number;
  level: LogLevel;
  source: string;
  message: string;
}

const LEVEL_ORDER: Record<LogLevel, number> = { debug: 0, info: 1, warn: 2, error: 3 };
const MAX_ENTRIES = 500;

let entries = $state<LogEntry[]>([]);
let minLevel = $state<LogLevel>("debug");

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
