import { describe, it, expect, beforeEach } from "vitest";
import {
  loadEditorDraft,
  saveEditorDraft,
  EDITOR_DRAFT_KEY,
} from "./editorDraft";

describe("editorDraft", () => {
  const mem = new Map<string, string>();
  const storage = {
    getItem: (k: string) => mem.get(k) ?? null,
    setItem: (k: string, v: string) => {
      mem.set(k, v);
    },
    removeItem: (k: string) => {
      mem.delete(k);
    },
  } as Storage;

  beforeEach(() => mem.clear());

  it("returns null when missing/invalid", () => {
    expect(loadEditorDraft(storage)).toBeNull();
    storage.setItem(EDITOR_DRAFT_KEY, "{");
    expect(loadEditorDraft(storage)).toBeNull();
  });

  it("round-trips", () => {
    const draft = {
      source: "int main() {}",
      method: "Solution::foo",
      argsJson: "[1]",
      profile: "leetcode" as const,
    };
    saveEditorDraft(draft, storage);
    expect(loadEditorDraft(storage)).toEqual(draft);
  });
});
