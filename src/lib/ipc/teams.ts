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
