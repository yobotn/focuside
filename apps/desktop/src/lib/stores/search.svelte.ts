// Search in files (the Search panel, SearchPanel.svelte). The query and the results live here,
// so they stay when the panel closes or moves. Visibility is not saved between runs.
import type { SearchHit } from "../api";

class SearchStore {
  visible = $state(false);
  query = $state("");
  replace = $state("");
  caseSensitive = $state(false);
  wholeWord = $state(false);
  regex = $state(false);
  include = $state("");
  exclude = $state("");
  showReplace = $state(false);
  showFilters = $state(false);
  hits = $state.raw<SearchHit[]>([]);
  truncated = $state(false);
  error = $state<string | null>(null);
  busy = $state(false);
  /** The project the results belong to. */
  projectId = $state<string | null>(null);
  /** Folded file groups in the results. */
  folded = $state<Record<string, boolean>>({});
  /** The last Replace all, for Undo. */
  lastReplace = $state<{ projectId: string; id: string; matches: number; files: number } | null>(null);
  /** Bumped to run the search again (after Replace all or Undo). */
  rerun = $state(0);
}

export const search = new SearchStore();
