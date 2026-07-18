import katex from "katex";
import "katex/dist/katex.min.css";

/** Convert a plain DS value/label to TeX. */
export function toTex(plain: string): string {
  const s = plain.trim();
  if (s === "") return "";
  if (/^-?\d+(\.\d+)?([eE][+-]?\d+)?$/.test(s)) return s;
  if (/^#-?\d+$/.test(s)) return `\\#${s.slice(1)}`;
  if (s === "nullptr") return "\\mathrm{nullptr}";
  if (s === "void") return "\\mathrm{void}";
  if (s === "true" || s === "false") return `\\mathtt{${s}}`;
  if (s.length === 1) return `\\texttt{${escapeText(s)}}`;
  return `\\text{${escapeText(s)}}`;
}

function escapeText(s: string): string {
  return s
    .replace(/\\/g, "\\textbackslash{}")
    .replace(/[{}]/g, (c) => `\\${c}`)
    .replace(/#/g, "\\#")
    .replace(/%/g, "\\%")
    .replace(/_/g, "\\_")
    .replace(/&/g, "\\&");
}

/**
 * Render plain text as KaTeX math into `el` (MathJax-like Computer Modern look).
 */
export function setMathContent(el: HTMLElement, plain: string): void {
  el.classList.add("ds-math");
  const tex = toTex(plain);
  if (!tex) {
    el.textContent = "";
    return;
  }
  try {
    katex.render(tex, el, {
      throwOnError: false,
      displayMode: false,
      output: "html",
    });
  } catch {
    el.textContent = plain;
  }
}

/** Font stack matching KaTeX / MathJax for SVG `<text>`. */
export const MATH_FONT =
  'KaTeX_Main, "Latin Modern Math", "STIX Two Math", "Cambria Math", "Times New Roman", serif';

export function applyMathFont(el: SVGTextElement | HTMLElement): void {
  el.style.fontFamily = MATH_FONT;
}
