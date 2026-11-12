<script lang="ts">
  import { iconFor } from "../icons";

  let { path, folder = false, open = false, size = 16 }: { path: string; folder?: boolean; open?: boolean; size?: number } = $props();

  const src = $derived(iconFor(path, folder, open));
</script>

<img class="shrink-0 select-none" src={src} width={size} height={size} alt="" draggable="false" />
