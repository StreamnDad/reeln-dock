import type { TeamProfile } from "$lib/types/team";
import { listTeamLevels, listTeamProfiles } from "$lib/ipc/teams";

/** Map of team_name (lowercase) → TeamProfile for quick logo lookups. */
let teamMap = $state<Map<string, TeamProfile>>(new Map());
let loaded = $state(false);

export function getTeamMap(): Map<string, TeamProfile> {
  return teamMap;
}

export function isTeamsLoaded(): boolean {
  return loaded;
}

/** Look up a team profile by name (case-insensitive). */
export function lookupTeam(name: string): TeamProfile | undefined {
  return teamMap.get(name.toLowerCase());
}

/** Update a single team in the lookup map (call after save). */
export function updateTeamInMap(profile: TeamProfile): void {
  const newMap = new Map(teamMap);
  newMap.set(profile.team_name.toLowerCase(), profile);
  teamMap = newMap;
}

/** Remove a team from the lookup map (call after delete). */
export function removeTeamFromMap(teamName: string): void {
  const newMap = new Map(teamMap);
  newMap.delete(teamName.toLowerCase());
  teamMap = newMap;
}

/** Load all team profiles from all levels into the lookup map. */
export async function loadAllTeams(): Promise<void> {
  try {
    const levels = await listTeamLevels();
    const newMap = new Map<string, TeamProfile>();
    for (const level of levels) {
      const profiles = await listTeamProfiles(level);
      for (const p of profiles) {
        newMap.set(p.team_name.toLowerCase(), p);
      }
    }
    teamMap = newMap;
    loaded = true;
  } catch {
    teamMap = new Map();
  }
}
