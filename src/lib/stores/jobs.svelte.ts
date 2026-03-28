import type { ProgressEvent } from "$lib/types/render";

export interface Job {
  job_id: string;
  phase: string;
  progress: number;
  message: string;
  started_at: number;
  completed: boolean;
}

let jobs = $state<Job[]>([]);
let listenerInitialized = false;

export function getJobs(): Job[] {
  return jobs;
}

export function getActiveJobs(): Job[] {
  return jobs.filter((j) => !j.completed);
}

export function getJob(jobId: string): Job | undefined {
  return jobs.find((j) => j.job_id === jobId);
}

export function clearCompletedJobs(): void {
  jobs = jobs.filter((j) => !j.completed);
}

export function handleProgressEvent(event: ProgressEvent): void {
  const existing = jobs.find((j) => j.job_id === event.job_id);
  if (existing) {
    jobs = jobs.map((j) =>
      j.job_id === event.job_id
        ? {
            ...j,
            phase: event.phase,
            progress: event.progress,
            message: event.message,
            completed: event.phase === "done",
          }
        : j,
    );
  } else {
    jobs = [
      ...jobs,
      {
        job_id: event.job_id,
        phase: event.phase,
        progress: event.progress,
        message: event.message,
        started_at: Date.now(),
        completed: event.phase === "done",
      },
    ];
  }
}

export async function initJobListener(): Promise<void> {
  if (listenerInitialized) return;
  listenerInitialized = true;

  const { listen } = await import("@tauri-apps/api/event");
  await listen<ProgressEvent>("job:progress", (event) => {
    handleProgressEvent(event.payload);
  });
}
