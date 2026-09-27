export type EditorDraft = {
  source: string;
  method: string;
  argsJson: string;
  profile: "leetcode" | "codeforces";
};

export const EDITOR_DRAFT_KEY = "rscpp-editor-draft";

export function loadEditorDraft(
  storage?: Storage | null,
): EditorDraft | null {
  try {
    const raw = (storage ?? globalThis.localStorage)?.getItem(EDITOR_DRAFT_KEY);
    if (!raw) return null;
    const v = JSON.parse(raw) as Partial<EditorDraft>;
    if (typeof v.source !== "string") return null;
    return {
      source: v.source,
      method: typeof v.method === "string" ? v.method : "Solution::twoSum",
      argsJson:
        typeof v.argsJson === "string" ? v.argsJson : "[[2,7,11,15],9]",
      profile: v.profile === "codeforces" ? "codeforces" : "leetcode",
    };
  } catch {
    return null;
  }
}

export function saveEditorDraft(
  draft: EditorDraft,
  storage?: Storage | null,
): void {
  try {
    (storage ?? globalThis.localStorage)?.setItem(
      EDITOR_DRAFT_KEY,
      JSON.stringify(draft),
    );
  } catch {
    /* private mode */
  }
}
