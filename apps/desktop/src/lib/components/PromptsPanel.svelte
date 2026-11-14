<script lang="ts">
  // The Prompts panel: the prompts of the active session. A placeholder until the real panel
  // lands. It has no required props, so the dock can place it anywhere.
  import { t } from "../i18n";
</script>

<div class="panel">
  <p class="text-fg-muted">{t("prompts.empty")}</p>
</div>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    overflow-y: auto;
    padding: 12px 14px;
    font-size: 13px;
  }
</style>
