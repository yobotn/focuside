// Images and PDFs in the viewer. The bytes come from the `fs_media` and `turn_file_bytes`
// commands as raw IPC responses (no base64, no asset protocol). Keep the limits in sync with
// crates/core/src/files.rs.

const MIME: Record<string, string> = {
  png: "image/png",
  jpg: "image/jpeg",
  jpeg: "image/jpeg",
  gif: "image/gif",
  webp: "image/webp",
  svg: "image/svg+xml",
  avif: "image/avif",
  bmp: "image/bmp",
  ico: "image/x-icon",
  pdf: "application/pdf",
};

export const MAX_IMAGE_BYTES = 50 * 1024 * 1024;
export const MAX_PDF_BYTES = 100 * 1024 * 1024;

export function mediaMime(path: string): string | null {
  const m = /\.([a-z0-9]+)$/i.exec(path);
  return (m && MIME[m[1].toLowerCase()]) || null;
}

export function mediaKind(path: string): "image" | "pdf" | null {
  const mime = mediaMime(path);
  if (!mime) return null;
  return mime === "application/pdf" ? "pdf" : "image";
}

export function mediaLimit(kind: "image" | "pdf"): number {
  return kind === "pdf" ? MAX_PDF_BYTES : MAX_IMAGE_BYTES;
}

/** An image blob with the right type. SVG needs it to render in an img element. */
export function imageBlob(path: string, data: ArrayBuffer | string): Blob {
  return new Blob([data], { type: mediaMime(path) ?? "application/octet-stream" });
}
