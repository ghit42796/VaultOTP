export type ThemePref = "system" | "light" | "dark";

const KEY = "vaultotp.theme";

/** Pure: coerce any stored/raw value to a valid ThemePref (default "system"). */
export function normalizePref(raw: string | null): ThemePref {
  return raw === "light" || raw === "dark" || raw === "system" ? raw : "system";
}

/** Pure: the `data-theme` attribute value for a pref; null means "remove it" (system). */
export function themeAttr(pref: ThemePref): "light" | "dark" | null {
  return pref === "system" ? null : pref;
}

export function loadThemePref(): ThemePref {
  try { return normalizePref(localStorage.getItem(KEY)); } catch { return "system"; }
}

export function saveThemePref(pref: ThemePref): void {
  try { localStorage.setItem(KEY, pref); } catch { /* storage may be unavailable */ }
}

/** Apply the preference to <html data-theme>. `system` removes the attribute. */
export function applyTheme(pref: ThemePref): void {
  const attr = themeAttr(pref);
  const root = document.documentElement;
  if (attr !== null) root.setAttribute("data-theme", attr);
  else root.removeAttribute("data-theme");
}
