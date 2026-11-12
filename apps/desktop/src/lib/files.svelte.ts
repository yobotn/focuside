// File lists per project, for the @ picker and Open file. Refreshed when the watcher reports
// changes.
import { invoke } from "@tauri-apps/api/core";

const cache = new Map<string, { files: string[]; at: number }>();
const pending = new Map<string, Promise<string[]>>();

export async function allFiles(projectId: string, fresh = false): Promise<string[]> {
  const c = cache.get(projectId);
  if (c && !fresh && Date.now() - c.at < 30_000) return c.files;
  let p = pending.get(projectId);
  if (!p) {
    p = invoke<string[]>("fs_all_files", { projectId })
      .then((files) => {
        cache.set(projectId, { files, at: Date.now() });
        return files;
      })
      .finally(() => pending.delete(projectId));
    pending.set(projectId, p);
  }
  return p;
}

export function invalidate(projectId: string) {
  cache.delete(projectId);
}
