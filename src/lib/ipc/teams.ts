import { invoke } from "@tauri-apps/api/core";
import type { TeamProfile } from "$lib/types/team";

export async function listTeamLevels(): Promise<string[]> {
  return invoke<string[]>("list_team_levels");
}

export async function listTeamProfiles(
  level: string,
): Promise<TeamProfile[]> {
  return invoke<TeamProfile[]>("list_team_profiles", { level });
}

export async function saveTeamProfile(
  profile: TeamProfile,
): Promise<TeamProfile> {
  return invoke<TeamProfile>("save_team_profile", { profile });
}

export async function deleteTeamProfile(
  level: string,
  teamName: string,
): Promise<void> {
  return invoke<void>("delete_team_profile", { level, teamName });
}

export async function cloneTeamProfile(
  sourceLevel: string,
  sourceName: string,
  newName: string,
  newLevel?: string,
): Promise<TeamProfile> {
  return invoke<TeamProfile>("clone_team_profile", {
    sourceLevel,
    sourceName,
    newName,
    newLevel: newLevel ?? null,
  });
}

export async function renameTeamLevel(
  oldName: string,
  newName: string,
): Promise<void> {
  return invoke<void>("rename_team_level", { oldName, newName });
}

export async function deleteTeamLevel(
  level: string,
): Promise<void> {
  return invoke<void>("delete_team_level", { level });
}

export interface RosterEntry {
  number: string;
  name: string;
}

export async function loadRoster(
  rosterPath: string,
): Promise<RosterEntry[]> {
  return invoke<RosterEntry[]>("load_roster", { rosterPath });
}

export async function loadTeamRoster(
  teamName: string,
  level: string,
): Promise<RosterEntry[]> {
  return invoke<RosterEntry[]>("load_team_roster", { teamName, level });
}
