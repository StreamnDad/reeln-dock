export interface HookExecutionResult {
  success: boolean;
  hook: string;
  shared: Record<string, unknown>;
  logs: string[];
  errors: string[];
}
