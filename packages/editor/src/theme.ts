import { EditorView } from "@codemirror/view";
import type { Extension } from "@codemirror/state";

export type EditorColorMode = "light" | "dark";

export function readDocumentTheme(): EditorColorMode {
  return document.documentElement.dataset.theme === "dark" ? "dark" : "light";
}

/** CodeMirror chrome; surfaces use CSS vars so demo tokens drive colors. */
export function editorTheme(mode: EditorColorMode): Extension {
  const dark = mode === "dark";
  return EditorView.theme(
    {
      "&": {
        height: "100%",
        fontSize: "13px",
        backgroundColor: "var(--code-bg)",
        color: "var(--code-fg)",
      },
      ".cm-scroller": { overflow: "auto" },
      ".cm-content": { caretColor: "var(--text)" },
      ".cm-gutters": {
        backgroundColor: "var(--gutter-bg)",
        color: "var(--gutter-fg)",
        borderRight: "1px solid var(--border)",
      },
      ".cm-activeLineGutter": {
        backgroundColor: dark ? "#1e293b88" : "#e2e8f088",
      },
      ".cm-activeLine": {
        backgroundColor: dark ? "#1e293b55" : "#f1f5f988",
      },
      ".cm-selectionBackground, &.cm-focused .cm-selectionBackground": {
        backgroundColor: dark ? "#334155aa" : "#bfdbfe88",
      },
    },
    { dark },
  );
}

export function watchDocumentTheme(
  cb: (mode: EditorColorMode) => void,
): () => void {
  const root = document.documentElement;
  const obs = new MutationObserver(() => cb(readDocumentTheme()));
  obs.observe(root, { attributes: true, attributeFilter: ["data-theme"] });
  return () => obs.disconnect();
}
