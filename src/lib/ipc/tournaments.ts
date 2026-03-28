import { invoke } from "@tauri-apps/api/core";
import type { TournamentMeta } from "$lib/types/tournament";

export async function listTournamentMetadata(): Promise<TournamentMeta[]> {
  return invoke<TournamentMeta[]>("list_tournament_metadata");
}

export async function setTournamentArchived(
  name: string,
  archived: boolean,
): Promise<TournamentMeta> {
  return invoke<TournamentMeta>("set_tournament_archived", { name, archived });
}

export async function updateTournamentMetadata(
  meta: TournamentMeta,
): Promise<TournamentMeta> {
  return invoke<TournamentMeta>("update_tournament_metadata", { meta });
}
