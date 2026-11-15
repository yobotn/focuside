// The project's own git repository, for the Git panel (GitPanel.svelte), the branch in the
// panel header, the letters in the file tree and the change markers in the editor. The status
// of the active project is read again on file changes, on changes in its .git folder and when
// the window gets the focus, after a short pause. Visibility is not saved between runs.
//
// Paths from git are relative to the repository root. The project can be a folder inside the
// repository (`status.prefix`); `projectPath` and `repoPath` convert between the two.
import { listen } from "@tauri-apps/api/event";
import { untrack } from "svelte";
import { api, errText, type GitCommit, type GitCommitFile, type GitEntry, type GitStatus } from "../api";
import { t } from "../i18n";
import { app, type GitDiff } from "./app.svelte";
import { dock } from "./dock.svelte";
import { editor } from "./editor.svelte";

export type GitGroup = "conflicts" | "staged" | "unstaged" | "untracked";

/** A question before a git action: discard changes, or commit all when nothing is staged. */
export interface GitAsk {
  kind: "discard" | "commitAll";
  paths: string[];
  /** Untracked files that discard deletes. */
  deletes: number;
  answer: (choice: "yes" | "always" | "cancel") => void;
}

/** The letter a file shows: the work tree letter when it has one, else the index letter. */
export function letterOf(e: GitEntry, group: GitGroup): string {
  if (group === "conflicts") return "!";
  if (group === "untracked") return "U";
  const c = group === "staged" ? e.x : e.y;
  return c === "?" ? "U" : c;
}

/** The label of a git diff in the viewer bar. */
export function gitDiffName(d: GitDiff): string {
  if (d.source.kind === "staged") return t("git.diffStaged");
  if (d.source.kind === "unstaged") return t("git.diffChanges");
  return d.label ? t("git.diffCommitOf", { subject: d.label }) : t("git.diffCommit");
}

const DEBOUNCE = 400;
export const LOG_LIMIT = 50;

class GitStore {
  visible = $state(false);
  /** The status of `projectId`. Null until read. */
  status = $state.raw<GitStatus | null>(null);
  projectId = $state<string | null>(null);
  /** The status could not be read. */
  error = $state<string | null>(null);
  log = $state.raw<GitCommit[]>([]);
  /** Files of the commits opened in the history, by hash. */
  commitFiles = $state<Record<string, GitCommitFile[]>>({});
  /** The action that runs now ("commit", "push" and so on), or null. */
  busy = $state<string | null>(null);
  /** The last action failed with this text (git's own message). */
  actionError = $state<string | null>(null);
  /** The commit message per project id. */
  messages = $state<Record<string, string>>({});
  ask = $state<GitAsk | null>(null);
  /** Bumped when the status or a file on disk changed. The viewer reloads a git diff. */
  version = $state(0);
  /** Bumped when HEAD moved (commit, switch, pull). The editor reloads its change markers. */
  headVersion = $state(0);
  /** Set to focus the commit message box (palette "Commit" with no message). */
  focusMessage = $state(0);

  private timer: ReturnType<typeof setTimeout> | null = null;
  private running = false;
  private again = false;
  private inited = false;
  private lastJson = "";
  private lastHead = "";
  /** Files changed on disk since the last read: a git diff in the viewer reloads too. */
  private diskChanged = false;
  /** The history loads when the panel shows in a repository. */
  private wantLog = $derived(this.visible && !!this.status?.repo);

  init() {
    if (this.inited) return;
    this.inited = true;
    $effect.root(() => {
      $effect(() => {
        const id = app.activeId;
        untrack(() => {
          if (id !== this.projectId) {
            this.status = null;
            this.log = [];
            this.commitFiles = {};
            this.error = null;
            this.actionError = null;
            this.lastJson = "";
          }
          this.projectId = id;
          if (id) this.refresh();
        });
      });
      $effect(() => {
        if (this.wantLog) untrack(() => this.loadLog());
      });
    });
    listen<{ projectId: string }>("fs-changed", (e) => {
      if (e.payload.projectId !== this.projectId) return;
      this.diskChanged = true;
      this.schedule();
    }).catch(() => {});
    listen<{ projectId: string }>("git-changed", (e) => {
      if (e.payload.projectId === this.projectId) this.schedule();
    }).catch(() => {});
    window.addEventListener("focus", () => this.schedule());
  }

  /** Reads the status again after a short pause. Calls in the pause share one read. */
  schedule(delay = DEBOUNCE) {
    if (this.timer) clearTimeout(this.timer);
    this.timer = setTimeout(() => {
      this.timer = null;
      this.refresh();
    }, delay);
  }

  /** Reads the status now. One read at a time; a call during a read runs once more after it. */
  async refresh() {
    const id = this.projectId;
    if (!id) return;
    if (this.running) {
      this.again = true;
      return;
    }
    this.running = true;
    const disk = this.diskChanged;
    this.diskChanged = false;
    try {
      const st = await api.gitStatus(id);
      if (id !== this.projectId) return;
      this.error = null;
      const json = JSON.stringify(st);
      if (json === this.lastJson) {
        // Same status, but a changed file still changes its diff.
        if (disk) this.version++;
      } else {
        this.lastJson = json;
        this.status = st;
        this.version++;
        const head = `${st.branch}:${st.head}`;
        if (head !== this.lastHead) {
          this.lastHead = head;
          this.headVersion++;
          if (this.visible) this.loadLog();
        }
      }
    } catch (e) {
      if (id === this.projectId) this.error = errText(e);
    } finally {
      this.running = false;
      if (this.again) {
        this.again = false;
        this.refresh();
      }
    }
  }

  async loadLog() {
    const id = this.projectId;
    if (!id || !this.status?.repo) return;
    try {
      const log = await api.gitLog(id, LOG_LIMIT);
      if (id === this.projectId) this.log = log;
    } catch {
      if (id === this.projectId) this.log = [];
    }
  }

  async loadCommitFiles(hash: string) {
    const id = this.projectId;
    if (!id || this.commitFiles[hash]) return;
    try {
      const files = await api.gitCommitFiles(id, hash);
      if (id === this.projectId) this.commitFiles[hash] = files;
    } catch (e) {
      this.actionError = errText(e);
    }
  }

  // Groups

  get entries(): GitEntry[] {
    return this.status?.entries ?? [];
  }
  get conflicts(): GitEntry[] {
    return this.entries.filter((e) => e.kind === "conflict");
  }
  get staged(): GitEntry[] {
    return this.entries.filter((e) => e.kind !== "conflict" && e.kind !== "untracked" && e.x !== ".");
  }
  get unstaged(): GitEntry[] {
    return this.entries.filter((e) => e.kind !== "conflict" && e.kind !== "untracked" && e.y !== ".");
  }
  get untracked(): GitEntry[] {
    return this.entries.filter((e) => e.kind === "untracked");
  }
  /** Changed files, for the badge on the rail. */
  get count(): number {
    return this.entries.length;
  }

  // Paths

  /** A repository path as a project path, or null when it is outside the project folder. */
  projectPath(path: string): string | null {
    const prefix = this.status?.prefix ?? "";
    if (!prefix) return path;
    return path.startsWith(prefix) ? path.slice(prefix.length) : null;
  }

  repoPath(path: string): string {
    return (this.status?.prefix ?? "") + path;
  }

  /**
   * Letters for the file tree, by project path: "M" modified, "A" added, "D" deleted,
   * "U" untracked, "R" renamed, "!" conflict. Folders get the letter of a changed file in them
   * (as "dot").
   */
  get marks(): Record<string, string> {
    const out: Record<string, string> = {};
    for (const e of this.entries) {
      const p = this.projectPath(e.path);
      if (p === null) continue;
      const letter = e.kind === "conflict" ? "!" : e.kind === "untracked" ? "U" : e.y !== "." ? e.y : e.x;
      out[p] = letter;
      const parts = p.split("/");
      for (let i = 1; i < parts.length; i++) {
        const dir = parts.slice(0, i).join("/");
        if (!out[dir] || letter === "!") out[dir] = letter === "!" ? "!" : "dot";
      }
    }
    return out;
  }

  // Actions

  /** Runs an action, shows git's error text when it fails, and reads the status after it. */
  private async act(name: string, fn: (id: string) => Promise<unknown>): Promise<boolean> {
    const id = this.projectId;
    if (!id) return false;
    this.busy = name;
    this.actionError = null;
    try {
      await fn(id);
      return true;
    } catch (e) {
      this.actionError = errText(e);
      // From the palette or the branch pick with the panel hidden: show it in a toast too.
      if (!dock.shown("git")) app.flash(this.actionError, "error");
      return false;
    } finally {
      this.busy = null;
      await this.refresh();
    }
  }

  initRepo() {
    return this.act("init", (id) => api.gitInit(id));
  }
  stage(paths: string[]) {
    return this.act("stage", (id) => api.gitStage(id, paths));
  }
  stageAll() {
    return this.act("stage", (id) => api.gitStageAll(id));
  }
  unstage(paths: string[]) {
    return this.act("stage", (id) => api.gitUnstage(id, paths));
  }
  unstageAll() {
    return this.act("stage", (id) => api.gitUnstageAll(id));
  }

  /** Discards changes after the user says yes. Untracked files are deleted. */
  async discard(tracked: GitEntry[], untracked: GitEntry[]) {
    if (!tracked.length && !untracked.length) return;
    const paths = [...tracked, ...untracked].map((e) => e.path);
    const choice = await this.askUser("discard", paths, untracked.length);
    if (choice !== "yes") return;
    await this.act("discard", (id) =>
      api.gitDiscard(
        id,
        tracked.map((e) => e.path),
        untracked.map((e) => e.path),
      ),
    );
  }

  /** Commits the staged changes. With nothing staged it asks once to commit all changes. */
  async commit() {
    const id = this.projectId;
    const st = this.status;
    if (!id || !st?.repo) return;
    const message = (this.messages[id] ?? "").trim();
    if (!message) {
      this.show();
      this.focusMessage++;
      return;
    }
    let all = false;
    if (!this.staged.length) {
      // Staging all would mark conflicts as resolved, markers and all.
      if (this.conflicts.length) {
        this.actionError = t("git.resolveFirst");
        return;
      }
      if (!this.entries.length) {
        this.actionError = t("git.nothingToCommit");
        return;
      }
      if (!app.settings.gitCommitAll) {
        const choice = await this.askUser("commitAll", [], 0);
        if (choice === "cancel") return;
        if (choice === "always") {
          app.settings.gitCommitAll = true;
          app.save();
        }
      }
      all = true;
    }
    const ok = await this.act("commit", (pid) => api.gitCommit(pid, message, all));
    if (ok) {
      this.messages[id] = "";
      app.flash(t("git.committed"));
    }
  }

  fetch() {
    return this.act("fetch", (id) => api.gitFetch(id));
  }
  async pull() {
    if (await this.act("pull", (id) => api.gitPull(id))) app.flash(t("git.pulled"));
  }
  async push() {
    const up = !!this.status?.upstream;
    if (await this.act("push", (id) => api.gitPush(id, up))) app.flash(t("git.pushed"));
  }
  switchTo(name: string, remote: boolean) {
    return this.act("switch", (id) => api.gitSwitch(id, name, remote));
  }
  createBranch(name: string) {
    return this.act("switch", (id) => api.gitCreateBranch(id, name.trim()));
  }

  private askUser(kind: GitAsk["kind"], paths: string[], deletes: number): Promise<"yes" | "always" | "cancel"> {
    return new Promise((resolve) => {
      this.ask = {
        kind,
        paths,
        deletes,
        answer: (c) => {
          this.ask = null;
          resolve(c);
        },
      };
    });
  }

  // Opening

  show() {
    dock.show("git");
  }

  /** Opens a group's diff in the viewer at one file. Untracked files open as they are. */
  openDiff(group: GitGroup, list: GitEntry[], entry: GitEntry) {
    const id = this.projectId;
    if (!id) return;
    if (group === "untracked") {
      const p = this.projectPath(entry.path);
      if (p !== null) editor.open(id, p);
      return;
    }
    const source = group === "staged" ? ({ kind: "staged" } as const) : ({ kind: "unstaged" } as const);
    editor.openDiff(id, {
      turnId: "",
      sessionKey: "",
      paths: list.map((e) => e.path),
      index: Math.max(0, list.indexOf(entry)),
      git: { source, origs: list.map((e) => (group === "staged" ? (e.orig ?? null) : null)) },
    });
  }

  openCommitDiff(commit: GitCommit, files: GitCommitFile[], file: GitCommitFile) {
    const id = this.projectId;
    if (!id) return;
    editor.openDiff(id, {
      turnId: "",
      sessionKey: "",
      paths: files.map((f) => f.path),
      index: Math.max(0, files.indexOf(file)),
      git: { source: { kind: "commit", hash: commit.hash }, origs: files.map((f) => f.orig ?? null), label: commit.subject },
    });
  }

  /** Opens the file of a change in the editor. */
  openFile(entry: GitEntry) {
    const id = this.projectId;
    const p = this.projectPath(entry.path);
    if (id && p !== null) editor.open(id, p, { pin: true });
  }
}

export const git = new GitStore();
