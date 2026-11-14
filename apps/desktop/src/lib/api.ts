// Typed wrappers around the Tauri commands.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { FileVersions, ProjectInfo, UndoOutcome, UndoPreview } from "./types";

export interface AgentInfo {
  id: string;
  name: string;
  path: string | null;
  install: string;
  /** The agent's own TUI is installed, so it can open in a pane. */
  tui: boolean;
}

export interface ClaudeRouteInfo {
  access: "allowed" | "subscription" | "signedOut";
  route: "chat" | "pane";
}

/** The hook helper and the Codex and Gemini CLI snippets for this OS. Null fields: the helper is missing. */
export interface HooksInfo {
  helper: string | null;
  codex: string | null;
  gemini: string | null;
}

export interface EnvCheck {
  claude: string | null;
  git: string | null;
  adapter: string | null;
  /** The upstream release FocusIDE downloads when the adapter is missing. Null on a system without a build. */
  adapterDownload: { repo: string; version: string; sizeMb: number } | null;
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

  agentList: () => invoke<AgentInfo[]>("agent_list"),
  adapterDownload: () => invoke<string>("adapter_download"),
  /** `hooks`: turn tracking in Claude panes (the focuside plugin). `planFirst`: a Claude pane starts in plan mode.
   * `brief`: the plugin copy with the brief replies output style (needs `hooks`). `planFolder`: Settings > Plan
   * folder, a folder in the project for Claude's plan files (null: Claude's default folder). */
  sessionStart: (
    projectId: string,
    resume: string | null,
    agentId: string,
    customCommand: string,
    mode: "chat" | "pane" = "chat",
    hooks = false,
    planFirst = false,
    brief = false,
    planFolder: string | null = null,
  ) => invoke<string>("session_start", { projectId, resume, agentId, customCommand, mode, hooks, planFirst, brief, planFolder }),
  /** Settings > Plan folder. Snapshots skip the folder. Rejects with "outside" for a folder outside the project. */
  planFolderSet: (folder: string | null) => invoke<string | null>("plan_folder_set", { folder }),
  /** Whether the project's git ignores the plan folder. `ignored` is null when the project is not a git repo. */
  planFolderStatus: (projectId: string, folder: string) =>
    invoke<{ ignored: boolean | null; line?: string }>("plan_folder_status", { projectId, folder }),
  /** "Add to .gitignore": only on the user's click. */
  planFolderGitignore: (projectId: string, folder: string) => invoke<void>("plan_folder_gitignore", { projectId, folder }),
  /** Plan versions and prompts of one session, kept in app data. */
  sessionDataLoad: (projectId: string, id: string) => invoke<unknown>("session_data_load", { projectId, id }),
  sessionDataSave: (projectId: string, id: string, value: unknown) => invoke<void>("session_data_save", { projectId, id, value }),
  hooksInfo: () => invoke<HooksInfo>("hooks_info"),
  /** Sends one Shift+Tab key sequence to a Claude pane (its mode button). Rejects with "busy" while a prompt runs. */
  paneModeKey: (key: string) => invoke<void>("pane_mode_key", { key }),
  claudeRoute: () => invoke<ClaudeRouteInfo | null>("claude_route"),
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

export function onAdapterDownload(cb: (p: { received: number; total: number }) => void): Promise<UnlistenFn> {
  return listen<{ received: number; total: number }>("adapter-download", (e) => cb(e.payload));
}

/** Error text from a rejected command. */
export function errText(e: unknown): string {
  return typeof e === "string" ? e : e instanceof Error ? e.message : JSON.stringify(e);
}
