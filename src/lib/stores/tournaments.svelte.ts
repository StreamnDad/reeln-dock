import type { TournamentMeta } from "$lib/types/tournament";
import { listTournamentMetadata, setTournamentArchived } from "$lib/ipc/tournaments";

let metadata = $state<TournamentMeta[]>([]);
let loading = $state(false);

export function getTournamentMetadata(): TournamentMeta[] {
  return metadata;
}

export function isTournamentsLoading(): boolean {
  return loading;
}

/** Check if a tournament is archived. Tournaments with no metadata are active. */
export function isArchived(name: string): boolean {
  return metadata.find((m) => m.name === name)?.archived ?? false;
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

export async function toggleArchived(name: string): Promise<void> {
  const current = isArchived(name);
  const updated = await setTournamentArchived(name, !current);
  metadata = metadata.some((m) => m.name === name)
    ? metadata.map((m) => (m.name === name ? updated : m))
    : [...metadata, updated];
}
