import type { TournamentMeta } from "$lib/types/tournament";
import { listTournamentMetadata } from "$lib/ipc/tournaments";

let metadata = $state<TournamentMeta[]>([]);
let loading = $state(false);

export function getTournamentMetadata(): TournamentMeta[] {
  return metadata;
}

export function isTournamentsLoading(): boolean {
  return loading;
}

/** A tournament is archived if its end_date is in the past. */
export function isArchived(name: string): boolean {
  const m = metadata.find((m) => m.name === name);
  if (!m?.end_date) return false;
  return m.end_date < new Date().toISOString().slice(0, 10);
}

export async function loadTournamentMetadata(): Promise<void> {
  loading = true;
  try {
    metadata = await listTournamentMetadata();
  } catch {
    metadata = [];
  } finally {
    loading = false;
  }
}

