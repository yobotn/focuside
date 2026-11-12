// File and folder icons from the generated Symbols set (see scripts/build-icons.mjs).
import map from "./icons/generated/map.json";

const urls = import.meta.glob("./icons/generated/*.svg", { eager: true, query: "?url", import: "default" }) as Record<
  string,
  string
>;

const byId = new Map<string, string>();
for (const [path, url] of Object.entries(urls)) {
  byId.set(path.slice(path.lastIndexOf("/") + 1, -4), url);
}

type IconMap = {
  file: string;
  folder: string;
  folderOpen: string;
  extensions: Record<string, string>;
  names: Record<string, string>;
  folders: Record<string, string>;
  foldersOpen: Record<string, string>;
};
const m = map as IconMap;

function url(id: string | undefined, fallback: string): string {
  return (id && byId.get(id)) || byId.get(fallback) || "";
}

/** Icon URL for a path. Matches the full file name first, then the longest extension. */
export function iconFor(path: string, folder = false, open = false): string {
  const name = path.replace(/\\/g, "/").split("/").pop()!.toLowerCase();
  if (folder) {
    if (open && m.foldersOpen[name]) return url(m.foldersOpen[name], m.folderOpen);
    return url(m.folders[name], open ? m.folderOpen : m.folder);
  }
  if (m.names[name]) return url(m.names[name], m.file);
  const parts = name.split(".");
  for (let i = 1; i < parts.length; i++) {
    const ext = parts.slice(i).join(".");
    if (m.extensions[ext]) return url(m.extensions[ext], m.file);
  }
  return url(undefined, m.file);
}
