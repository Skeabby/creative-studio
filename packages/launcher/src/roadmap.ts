/**
 * Roadmap entries: the "planned" apps shown by the launcher. Kept separate
 * from discovered apps so a roadmap entry can turn into a real app without
 * being listed twice (the app hides the roadmap card).
 */

export interface RoadmapEntry {
  id: string;
  name: string;
  description?: string;
  category?: string;
  inspiredBy?: string;
  emoji?: string;
  color?: string;
}

/** Drops any roadmap entry that a discovered app already provides. */
export function mergeRoadmap(
  discoveredIds: ReadonlySet<string>,
  entries: readonly RoadmapEntry[],
): RoadmapEntry[] {
  return entries.filter((entry) => !discoveredIds.has(entry.id));
}