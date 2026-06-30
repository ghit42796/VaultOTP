export const BADGE_PALETTE = [
  "#e05d44", "#d6883b", "#2f9e44", "#2563eb",
  "#7048e8", "#c2255c", "#0c8599", "#5c7cfa",
];

/** First letter of issuer, uppercased; "•" when empty. */
export function initial(issuer: string): string {
  const t = issuer.trim();
  return t ? t[0].toUpperCase() : "•";
}

/** Deterministic badge color from the issuer string. */
export function badgeColor(issuer: string): string {
  const s = issuer.trim() || "•";
  let h = 0;
  for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) >>> 0;
  return BADGE_PALETTE[h % BADGE_PALETTE.length];
}

/** Group a TOTP code: 6 -> "NNN NNN", 8 -> "NNNN NNNN", else unchanged. */
export function groupCode(code: string): string {
  if (code.length === 6) return code.slice(0, 3) + " " + code.slice(3);
  if (code.length === 8) return code.slice(0, 4) + " " + code.slice(4);
  return code;
}

/** Fraction of the period remaining, clamped to 0..1. period <= 0 -> 0. */
export function barFraction(remaining: number, period: number): number {
  if (period <= 0) return 0;
  return Math.max(0, Math.min(1, remaining / period));
}

/** Heuristic password strength, 0..4 (length + character variety). */
export function passwordStrength(pw: string): number {
  let s = 0;
  if (pw.length >= 8) s++;
  if (pw.length >= 12) s++;
  if (/[a-z]/.test(pw) && /[A-Z]/.test(pw)) s++;
  if (/\d/.test(pw) && /[^A-Za-z0-9]/.test(pw)) s++;
  return Math.min(4, s);
}
