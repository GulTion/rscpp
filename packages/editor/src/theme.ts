import { EditorView } from "@codemirror/view";
import {
  HighlightStyle,
  syntaxHighlighting,
} from "@codemirror/language";
import { tags as t } from "@lezer/highlight";
import type { Extension } from "@codemirror/state";

export type EditorColorMode = "light" | "dark";

export function readDocumentTheme(): EditorColorMode {
  return document.documentElement.dataset.theme === "dark" ? "dark" : "light";
}

const lightHighlight = HighlightStyle.define(
  [
    { tag: t.comment, color: "#6b7280", fontStyle: "italic" },
    { tag: t.lineComment, color: "#6b7280", fontStyle: "italic" },
    { tag: t.blockComment, color: "#6b7280", fontStyle: "italic" },
    { tag: t.keyword, color: "#7c3aed" },
    { tag: [t.controlKeyword, t.moduleKeyword], color: "#7c3aed" },
    { tag: t.operatorKeyword, color: "#a855f7" },
    { tag: t.typeName, color: "#0e7490" },
    { tag: t.className, color: "#0e7490" },
    { tag: t.namespace, color: "#0e7490" },
    { tag: t.string, color: "#b45309" },
    { tag: t.character, color: "#b45309" },
    { tag: t.number, color: "#c2410c" },
    { tag: t.bool, color: "#c2410c" },
    { tag: t.null, color: "#c2410c" },
    { tag: t.operator, color: "#475569" },
    { tag: t.punctuation, color: "#64748b" },
    { tag: t.bracket, color: "#64748b" },
    { tag: t.paren, color: "#64748b" },
    { tag: t.meta, color: "#9333ea" },
    { tag: t.processingInstruction, color: "#9333ea" },
    { tag: t.definition(t.variableName), color: "#1d4ed8" },
    { tag: t.variableName, color: "#0f172a" },
    { tag: t.function(t.variableName), color: "#1d4ed8" },
    { tag: t.function(t.propertyName), color: "#1d4ed8" },
    { tag: t.propertyName, color: "#0369a1" },
    { tag: t.invalid, color: "#dc2626" },
  ],
  { themeType: "light" },
);

const darkHighlight = HighlightStyle.define(
  [
    { tag: t.comment, color: "#94a3b8", fontStyle: "italic" },
    { tag: t.lineComment, color: "#94a3b8", fontStyle: "italic" },
    { tag: t.blockComment, color: "#94a3b8", fontStyle: "italic" },
    { tag: t.keyword, color: "#c4b5fd" },
    { tag: [t.controlKeyword, t.moduleKeyword], color: "#c4b5fd" },
    { tag: t.operatorKeyword, color: "#d8b4fe" },
    { tag: t.typeName, color: "#67e8f9" },
    { tag: t.className, color: "#67e8f9" },
    { tag: t.namespace, color: "#67e8f9" },
    { tag: t.string, color: "#fcd34d" },
    { tag: t.character, color: "#fcd34d" },
    { tag: t.number, color: "#fdba74" },
    { tag: t.bool, color: "#fdba74" },
    { tag: t.null, color: "#fdba74" },
    { tag: t.operator, color: "#cbd5e1" },
    { tag: t.punctuation, color: "#94a3b8" },
    { tag: t.bracket, color: "#94a3b8" },
    { tag: t.paren, color: "#94a3b8" },
    { tag: t.meta, color: "#e9d5ff" },
    { tag: t.processingInstruction, color: "#e9d5ff" },
    { tag: t.definition(t.variableName), color: "#93c5fd" },
    { tag: t.variableName, color: "#e2e8f0" },
    { tag: t.function(t.variableName), color: "#93c5fd" },
    { tag: t.function(t.propertyName), color: "#93c5fd" },
    { tag: t.propertyName, color: "#7dd3fc" },
    { tag: t.invalid, color: "#f87171" },
  ],
  { themeType: "dark" },
);

/** C++ token colors; active style follows EditorView theme light/dark. */
export const syntaxColors: Extension = [
  syntaxHighlighting(lightHighlight),
  syntaxHighlighting(darkHighlight),
];

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
