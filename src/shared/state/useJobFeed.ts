import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { jobsCancel, jobsList, type JobSnapshot } from "@/api_tauri";

export function useJobFeed() {
  const [jobs, setJobs] = useState<JobSnapshot[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let active = true;

    const refresh = async () => {
      try {
        const nextJobs = await jobsList();
        if (active) {
          setJobs(nextJobs);
        }
      } finally {
        if (active) {
          setLoading(false);
        }
      }
    };

    refresh();

    const unlistenPromise = listen<JobSnapshot>("job://updated", (event) => {
      setJobs((current) => {
        const without = current.filter((job) => job.id !== event.payload.id);
        return [event.payload, ...without].sort((a, b) => b.createdAt - a.createdAt);
      });
    });

    return () => {
      active = false;
      unlistenPromise.then((unlisten) => unlisten()).catch(() => undefined);
    };
  }, []);

  return {
    jobs,
    loading,
    cancelJob: async (jobId: string) => {
      const nextJob = await jobsCancel({ jobId });
      setJobs((current) => [nextJob, ...current.filter((job) => job.id !== nextJob.id)]);
    },
  };
}
