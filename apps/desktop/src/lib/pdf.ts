// PDF.js, loaded only when a PDF opens (PdfView imports this module dynamically). The legacy
// build runs on older WebKit too. Nothing is fetched from the web: the worker, font data,
// CMaps, ICC profiles and image decoders ship with the app under /pdfjs/ (see vite.config.ts).
// No document JavaScript runs: scripting is part of the PDF.js viewer, which is not used, and
// PDF.js 6 has no eval code path (the old isEvalSupported option is gone). The CSP has no
// 'unsafe-eval' either.
import * as pdfjs from "pdfjs-dist/legacy/build/pdf.mjs";
import workerUrl from "pdfjs-dist/legacy/build/pdf.worker.min.mjs?url";

pdfjs.GlobalWorkerOptions.workerSrc = workerUrl;

export type PdfDoc = pdfjs.PDFDocumentProxy;
export type PdfPage = pdfjs.PDFPageProxy;
export type RenderTask = pdfjs.RenderTask;

export function openPdf(data: Uint8Array): pdfjs.PDFDocumentLoadingTask {
  const base = new URL("/pdfjs/", location.href).href;
  return pdfjs.getDocument({
    data,
    cMapUrl: base + "cmaps/",
    cMapPacked: true,
    standardFontDataUrl: base + "standard_fonts/",
    wasmUrl: base + "wasm/",
    iccUrl: base + "iccs/",
    enableXfa: false,
    disableAutoFetch: true,
    disableStream: true,
    disableRange: true,
    disableWorkerRendering: true,
  });
}
