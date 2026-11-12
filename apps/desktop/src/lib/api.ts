// Typed wrappers around the Tauri commands.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { FileVersions, ProjectInfo, UndoOutcome, UndoPreview } from "./types";

export interface EnvCheck {
  claude: string | null;
  git: string | null;
  adapter: string | null;
  os: string;
  version: string;
}

export const api = {
  envCheck: () => invoke<EnvCheck>("env_check"),
  startupPaths: () => invoke<string[]>("startup_paths"),
  uiStateLoad: () => invoke<unknown>("ui_state_load"),
  uiStateSave: (value: unknown) => invoke<void>("ui_state_save", { value }),

  projectOpen: (path: string) => invoke<ProjectInfo>("project_open", { path }),
  projectClose: (id: string) => invoke<void>("project_close", { id }),

  sessionStart: (projectId: string, resume: string | null) =>
    invoke<string>("session_start", { projectId, resume }),
  sessionPrompt: (key: string, blocks: unknown[], planFirst: boolean, continueTurn: string | null = null) =>
    invoke<string>("session_prompt", { key, blocks, planFirst, continueTurn }),
  sessionAnswer: (key: string, requestId: string, optionId: string) =>
    invoke<void>("session_answer", { key, requestId, optionId }),
  sessionRunPlan: (key: string, requestId: string) => invoke<string>("session_run_plan", { key, requestId }),
  sessionDiscardPlan: (key: string, requestId: string) => invoke<void>("session_discard_plan", { key, requestId }),
  sessionRevisePlan: (key: string, requestId: string, text: string) =>
    invoke<void>("session_revise_plan", { key, requestId, text }),
  sessionCancel: (key: string) => invoke<void>("session_cancel", { key }),
  sessionResume: (key: string) => invoke<void>("session_resume", { key }),
  sessionClose: (key: string) => invoke<void>("session_close", { key }),
  sessionSetConfig: (key: string, configId: string, value: unknown) =>
    invoke<unknown>("session_set_config", { key, configId, value }),
  sessionList: (key: string) => invoke<unknown[]>("session_list", { key }),

  undoPreview: (projectId: string, turnId: string) => invoke<UndoPreview>("turn_undo_preview", { projectId, turnId }),
  undo: (projectId: string, turnId: string, revertAnyway: string[]) =>
    invoke<UndoOutcome>("turn_undo", { projectId, turnId, revertAnyway }),
  redo: (projectId: string, turnId: string) => invoke<UndoOutcome>("turn_redo", { projectId, turnId }),
  revertFile: (projectId: string, turnId: string, path: string) =>
    invoke<UndoOutcome>("turn_revert_file", { projectId, turnId, path }),
  fileVersions: (projectId: string, turnId: string, path: string) =>
    invoke<FileVersions>("turn_file_versions", { projectId, turnId, path }),
};

export type SessionEvent = { key: string; projectId: string; kind: string } & Record<string, any>;

export function onSessionEvent(cb: (e: SessionEvent) => void): Promise<UnlistenFn> {
  return listen<SessionEvent>("session", (e) => cb(e.payload));
}

export function onOpenPaths(cb: (paths: string[]) => void): Promise<UnlistenFn> {
  return listen<string[]>("open-paths", (e) => cb(e.payload));
}

/** Error text from a rejected command. */
export function errText(e: unknown): string {
  return typeof e === "string" ? e : e instanceof Error ? e.message : JSON.stringify(e);
}
