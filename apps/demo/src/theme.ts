export type ThemePreference = "system" | "light" | "dark";
export type EffectiveTheme = "light" | "dark";
export const THEME_STORAGE_KEY = "rscpp-theme";

export function resolveEffectiveTheme(
  preference: ThemePreference,
  prefersDark: boolean,
): EffectiveTheme {
  if (preference === "light") return "light";
  if (preference === "dark") return "dark";
  return prefersDark ? "dark" : "light";
}

export function cyclePreference(preference: ThemePreference): ThemePreference {
  if (preference === "system") return "light";
  if (preference === "light") return "dark";
  return "system";
}

function isPreference(v: string | null): v is ThemePreference {
  return v === "system" || v === "light" || v === "dark";
}

export function loadPreference(storage?: Storage | null): ThemePreference {
  try {
    const raw =
      (storage ?? globalThis.localStorage)?.getItem(THEME_STORAGE_KEY) ?? null;
    return isPreference(raw) ? raw : "system";
  } catch {
    return "system";
  }
}

export function savePreference(
  preference: ThemePreference,
  storage?: Storage | null,
): void {
  try {
    (storage ?? globalThis.localStorage)?.setItem(
      THEME_STORAGE_KEY,
      preference,
    );
  } catch {
    /* private mode / SSR */
  }
}

export function prefersDarkScheme(mql?: MediaQueryList | null): boolean {
  if (mql) return mql.matches;
  if (typeof matchMedia === "function") {
    return matchMedia("(prefers-color-scheme: dark)").matches;
  }
  return false;
}

export function applyTheme(
  preference: ThemePreference,
  opts?: {
    root?: HTMLElement;
    storage?: Storage | null;
    mql?: MediaQueryList | null;
  },
): EffectiveTheme {
  savePreference(preference, opts?.storage);
  const effective = resolveEffectiveTheme(
    preference,
    prefersDarkScheme(opts?.mql),
  );
  const root = opts?.root ?? globalThis.document?.documentElement;
  if (root) root.dataset.theme = effective;
  return effective;
}

export function watchEffectiveTheme(
  cb: (t: EffectiveTheme) => void,
): () => void {
  const root = document.documentElement;
  const obs = new MutationObserver(() => {
    const t = root.dataset.theme;
    if (t === "light" || t === "dark") cb(t);
  });
  obs.observe(root, { attributes: true, attributeFilter: ["data-theme"] });
  return () => obs.disconnect();
}

export function initTheme(opts?: {
  root?: HTMLElement;
  storage?: Storage | null;
  mql?: MediaQueryList | null;
}): {
  getPreference(): ThemePreference;
  setPreference(p: ThemePreference): EffectiveTheme;
  cycle(): EffectiveTheme;
  destroy(): void;
} {
  let preference = loadPreference(opts?.storage);
  const root = opts?.root ?? document.documentElement;
  const mql =
    opts?.mql ??
    (typeof matchMedia === "function"
      ? matchMedia("(prefers-color-scheme: dark)")
      : null);

  const apply = () =>
    applyTheme(preference, { root, storage: opts?.storage, mql });

  apply();

  const onScheme = () => {
    if (preference === "system") apply();
  };
  mql?.addEventListener?.("change", onScheme);

  return {
    getPreference: () => preference,
    setPreference: (p) => {
      preference = p;
      return apply();
    },
    cycle: () => {
      preference = cyclePreference(preference);
      return apply();
    },
    destroy: () => mql?.removeEventListener?.("change", onScheme),
  };
}
