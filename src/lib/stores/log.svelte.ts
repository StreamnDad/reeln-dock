/** Application-level logging store with level filtering.
 *
 * Implemented as a class instance instead of module-scope ``$state``
 * because Svelte 5 cross-module reactivity tracks reliably through
 * property reads on a class but is fragile when a module-scope ``$state``
 * variable is REASSIGNED (vs. mutated) and read from another module via
 * a wrapper function. The previous implementation did
 * ``entries = [...entries.slice(...), entry]`` which silently broke
 * the LogViewer's ``$derived`` subscriptions — clicking Settings → Logs
 * mounted the component with stale dependency tracking and nothing
 * appeared.
 */

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

class LogStore {
  /** Reactive entry buffer. Mutated in-place via splice() so cross-module
   * consumers see updates without depending on variable reassignment. */
  entries = $state<LogEntry[]>([]);

  /** Minimum level shown in ``visible``. ``debug`` shows everything. */
  minLevel = $state<LogLevel>("debug");

  /** Entries above the current min level, in insertion order. */
  get visible(): LogEntry[] {
    const min = LEVEL_ORDER[this.minLevel];
    return this.entries.filter((e) => LEVEL_ORDER[e.level] >= min);
  }

  /** Append an entry, trimming oldest if over MAX_ENTRIES. */
  push(level: LogLevel, source: string, message: string): void {
    this.entries.push({ timestamp: Date.now(), level, source, message });
    // splice() mutates in place — the $state proxy tracks the mutation
    // without us needing to reassign ``this.entries``.
    if (this.entries.length > MAX_ENTRIES) {
      this.entries.splice(0, this.entries.length - MAX_ENTRIES);
    }
  }

  clear(): void {
    this.entries.length = 0;
  }
}

export const logStore = new LogStore();

let unlisten: UnlistenFn | null = null;

/** Start listening for backend log events. Call once during app init. */
export async function initLogListener(): Promise<void> {
  if (unlisten) return;
  unlisten = await listen<DockLogEvent>("dock:log", (event) => {
    const { level, source, message } = event.payload;
    const validLevel = level in LEVEL_ORDER ? (level as LogLevel) : "info";
    logStore.push(validLevel, source, message);
  });
}

// ---------------------------------------------------------------------------
// Function-style wrappers kept for backwards compatibility with the
// existing call sites. New code should use ``logStore`` directly.
// ---------------------------------------------------------------------------

export function getLogEntries(): LogEntry[] {
  return logStore.visible;
}

export function getAllLogEntries(): LogEntry[] {
  return logStore.entries;
}

export function getLogLevel(): LogLevel {
  return logStore.minLevel;
}

export function setLogLevel(level: LogLevel): void {
  logStore.minLevel = level;
}

export function clearLogs(): void {
  logStore.clear();
}

export const log = {
  debug: (source: string, message: string) => logStore.push("debug", source, message),
  info: (source: string, message: string) => logStore.push("info", source, message),
  warn: (source: string, message: string) => logStore.push("warn", source, message),
  error: (source: string, message: string) => logStore.push("error", source, message),
};
