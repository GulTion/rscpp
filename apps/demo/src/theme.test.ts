import { describe, it, expect, beforeEach } from "vitest";
import {
  resolveEffectiveTheme,
  cyclePreference,
  loadPreference,
  savePreference,
  THEME_STORAGE_KEY,
} from "./theme";

describe("resolveEffectiveTheme", () => {
  it("honors light/dark overrides", () => {
    expect(resolveEffectiveTheme("light", true)).toBe("light");
    expect(resolveEffectiveTheme("dark", false)).toBe("dark");
  });
  it("follows system when preference is system", () => {
    expect(resolveEffectiveTheme("system", true)).toBe("dark");
    expect(resolveEffectiveTheme("system", false)).toBe("light");
  });
});

describe("cyclePreference", () => {
  it("cycles system → light → dark → system", () => {
    expect(cyclePreference("system")).toBe("light");
    expect(cyclePreference("light")).toBe("dark");
    expect(cyclePreference("dark")).toBe("system");
  });
});

describe("load/savePreference", () => {
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

  it("defaults to system for missing/invalid", () => {
    expect(loadPreference(storage)).toBe("system");
    storage.setItem(THEME_STORAGE_KEY, "nope");
    expect(loadPreference(storage)).toBe("system");
  });

  it("round-trips", () => {
    savePreference("dark", storage);
    expect(loadPreference(storage)).toBe("dark");
  });
});
