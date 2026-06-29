const KEY = "vaultotp.settings";
export interface Settings { idleLockMs: number; clipboardClearMs: number; }
const DEFAULTS: Settings = { idleLockMs: 5 * 60_000, clipboardClearMs: 20_000 };

export function loadSettings(): Settings {
  try { return { ...DEFAULTS, ...JSON.parse(localStorage.getItem(KEY) || "{}") }; }
  catch { return { ...DEFAULTS }; }
}
export function saveSettings(s: Settings) { localStorage.setItem(KEY, JSON.stringify(s)); }
