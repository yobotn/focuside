<script lang="ts">
  import { ArrowUp, Stop } from "phosphor-svelte";
  import { t } from "../i18n";
  import { keyLabel, mod } from "../platform";
  import { app } from "../stores/app.svelte";
  import type { Session } from "../stores/session.svelte";
  import Picker, { type PickerItem } from "./Picker.svelte";

  let { session, onfocuscard }: { session: Session; onfocuscard: () => void } = $props();

  let box: HTMLTextAreaElement | undefined = $state();
  let historyIndex = -1;
  let images = $state<{ mimeType: string; data: string; url: string }[]>([]);
  let mentions = $state<string[]>([]);
  let picker = $state<null | { kind: "files" | "commands"; start: number; query: string }>(null);
  let pickerRef: Picker | undefined = $state();

  const plan = $derived(session.waitingPlan);
  const placeholder = $derived(plan ? t("box.placeholderPlan", { key: keyLabel("Mod+Enter") }) : t("box.placeholder"));

  export function focus() {
    box?.focus();
  }

  $effect(() => {
    void session.key;
    historyIndex = -1;
    images = [];
    mentions = [];
    picker = null;
    queueMicrotask(() => box?.focus());
  });

  function autosize() {
    if (!box) return;
    box.style.height = "auto";
    box.style.height = Math.min(box.scrollHeight, 240) + "px";
  }
  $effect(() => {
    void session.draft;
    autosize();
  });

  async function send() {
    const text = session.draft;
    const extra: unknown[] = [];
    for (const img of images) extra.push({ type: "image", mimeType: img.mimeType, data: img.data });
    for (const m of mentions) {
      if (!text.includes("@" + m)) continue;
      const abs = session.root.replace(/\\/g, "/").replace(/\/$/, "") + "/" + m;
      const uri = "file:///" + abs.replace(/^\//, "");
      extra.push({ type: "resource_link", uri, name: m });
    }
    if (!text.trim() && !images.length) return;
    session.draft = "";
    historyIndex = -1;
    const files = mentions.filter((m) => text.includes("@" + m));
    images = [];
    mentions = [];
    await app.send(text, extra, files);
  }

  function onkeydown(e: KeyboardEvent) {
    if (picker && pickerRef?.key(e)) return;
    if (e.key === "Enter" && mod(e)) {
      e.preventDefault();
      if (!session.draft.trim() && plan) app.runPlan(session, plan);
      else send();
      return;
    }
    if (e.key === "Enter" && !e.shiftKey && !e.altKey && !e.isComposing) {
      e.preventDefault();
      send();
      return;
    }
    if (e.key === "ArrowUp" && e.altKey) {
      e.preventDefault();
      onfocuscard();
      return;
    }
    // Up recalls earlier prompts when the caret is on the first line.
    if (e.key === "ArrowUp" && !e.shiftKey && box && !box.value.slice(0, box.selectionStart).includes("\n")) {
      if (!app.history.length || (session.draft && historyIndex === -1)) return;
      e.preventDefault();
      historyIndex = historyIndex === -1 ? app.history.length - 1 : Math.max(0, historyIndex - 1);
      session.draft = app.history[historyIndex];
      return;
    }
    if (e.key === "ArrowDown" && historyIndex !== -1 && box && !box.value.slice(box.selectionEnd).includes("\n")) {
      e.preventDefault();
      historyIndex++;
      if (historyIndex >= app.history.length) {
        historyIndex = -1;
        session.draft = "";
      } else session.draft = app.history[historyIndex];
    }
  }

  /** Opens the @ file picker or the / command list when the caret follows the trigger. */
  function oninput() {
    autosize();
    if (!box) return;
    const pos = box.selectionStart;
    const before = box.value.slice(0, pos);
    const at = before.match(/(?:^|\s)@([^\s@]*)$/);
    const slash = before.match(/^\/([\w:-]*)$/);
    if (at) picker = { kind: "files", start: pos - at[1].length - 1, query: at[1] };
    else if (slash) picker = { kind: "commands", start: 0, query: slash[1] };
    else picker = null;
  }

  function pick(item: PickerItem) {
    if (!picker || !box) return;
    const end = box.selectionStart;
    const value = session.draft;
    const insert = picker.kind === "files" ? "@" + item.value + " " : "/" + item.value + " ";
    session.draft = value.slice(0, picker.start) + insert + value.slice(end);
    if (picker.kind === "files" && !mentions.includes(item.value)) mentions.push(item.value);
    const caret = picker.start + insert.length;
    picker = null;
    queueMicrotask(() => {
      box?.focus();
      box?.setSelectionRange(caret, caret);
      autosize();
    });
  }

  async function addImage(file: File) {
    if (!session.images) {
      app.flash(t("box.imageNotSupported"));
      return;
    }
    const buf = await file.arrayBuffer();
    let bin = "";
    const bytes = new Uint8Array(buf);
    for (let i = 0; i < bytes.length; i += 0x8000) bin += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
    images.push({ mimeType: file.type, data: btoa(bin), url: URL.createObjectURL(file) });
  }

  function onpaste(e: ClipboardEvent) {
    const files = Array.from(e.clipboardData?.files ?? []).filter((f) => f.type.startsWith("image/"));
    if (!files.length) return;
    e.preventDefault();
    files.forEach(addImage);
  }

  function ondrop(e: DragEvent) {
    const files = Array.from(e.dataTransfer?.files ?? []).filter((f) => f.type.startsWith("image/"));
    if (!files.length) return;
    e.preventDefault();
    files.forEach(addImage);
  }
</script>

<div class="wrap">
  <div class="box" class:plan={!!plan}>
    {#if picker}
      <Picker bind:this={pickerRef} kind={picker.kind} query={picker.query} {session} onpick={pick} onclose={() => (picker = null)} />
    {/if}
    {#if images.length}
      <div class="flex gap-2 px-3 pt-3">
        {#each images as img, i (img.url)}
          <button class="thumb" onclick={() => images.splice(i, 1)} title={t("common.remove")}>
            <img src={img.url} alt="" />
          </button>
        {/each}
      </div>
    {/if}
    <textarea
      bind:this={box}
      bind:value={session.draft}
      rows="1"
      {placeholder}
      spellcheck="true"
      {onkeydown}
      {oninput}
      {onpaste}
      {ondrop}
      onclick={oninput}
      aria-label={t("box.placeholder")}
    ></textarea>
    <div class="bar">
      <label class="toggle" title={t("box.planFirstHint")}>
        <input type="checkbox" checked={app.planFirst} onchange={(e) => app.setPlanFirst(e.currentTarget.checked)} />
        <span class="switch" aria-hidden="true"></span>
        {t("box.planFirst")}
      </label>
      <span class="hint">{t("box.files")}</span>
      <span class="hint">{t("box.commands")}</span>
      <span class="ms-auto"></span>
      {#if session.running}
        <button class="btn" onclick={() => app.stop(session)} disabled={session.stopping} title={keyLabel("Mod+.")}>
          <Stop size={14} weight="fill" />
          {session.stopping ? t("conv.stopping") : t("conv.stop")}
        </button>
      {:else}
        <button
          class="send"
          onclick={send}
          disabled={!session.draft.trim() && !images.length}
          aria-label={t("box.send")}
          title={t("box.send")}
        >
          <ArrowUp size={16} weight="bold" />
        </button>
      {/if}
    </div>
  </div>
</div>

<style>
  .wrap {
    max-width: 760px;
    width: 100%;
    margin: 0 auto;
    padding: 0 24px 16px;
  }
  .box {
    position: relative;
    border: 1px solid var(--border-strong);
    border-radius: 10px;
    background: var(--bg);
    transition: border-color var(--fast) var(--ease);
  }
  .box:focus-within {
    border-color: color-mix(in srgb, var(--fg) 35%, var(--border));
  }
  .box.plan {
    border-color: color-mix(in srgb, var(--warn-line) 55%, var(--border));
  }
  textarea {
    display: block;
    width: 100%;
    resize: none;
    border: 0;
    outline: 0;
    background: transparent;
    padding: 12px 14px 4px;
    font-family: var(--font-read);
    font-size: var(--read-size);
    line-height: 1.5;
    color: var(--fg);
    max-height: 240px;
    user-select: text;
  }
  textarea::placeholder {
    color: var(--fg-faint);
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 6px 8px 8px 12px;
  }
  .hint {
    color: var(--fg-faint);
  }
  .toggle {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--fg-muted);
  }
  .toggle input {
    position: absolute;
    opacity: 0;
    width: 1px;
    height: 1px;
  }
  .switch {
    width: 26px;
    height: 15px;
    border-radius: 99px;
    background: var(--bg-active);
    position: relative;
    transition: background var(--fast) var(--ease);
  }
  .switch::after {
    content: "";
    position: absolute;
    top: 2px;
    inset-inline-start: 2px;
    width: 11px;
    height: 11px;
    border-radius: 50%;
    background: var(--bg);
    box-shadow: 0 1px 2px rgb(0 0 0 / 0.2);
    transition: transform var(--fast) var(--ease);
  }
  .toggle input:checked + .switch {
    background: var(--accent);
  }
  .toggle input:checked + .switch::after {
    transform: translateX(11px);
  }
  .toggle input:focus-visible + .switch {
    outline: 2px solid color-mix(in srgb, var(--fg) 45%, transparent);
    outline-offset: 2px;
  }
  .send {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border-radius: 7px;
    border: 0;
    background: var(--fg);
    color: var(--bg);
  }
  .send:disabled {
    background: var(--bg-active);
    color: var(--fg-faint);
  }
  .thumb {
    width: 48px;
    height: 48px;
    border-radius: 6px;
    overflow: hidden;
    border: 1px solid var(--border);
    padding: 0;
  }
  .thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
</style>
