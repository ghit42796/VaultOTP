import { describe, it, expect } from "vitest";
import { initial, badgeColor, groupCode, passwordStrength, BADGE_PALETTE, barFraction } from "./display";

describe("display helpers", () => {
  it("initial returns first uppercase letter, fallback for empty", () => {
    expect(initial("GitHub")).toBe("G");
    expect(initial("  google ")).toBe("G");
    expect(initial("")).toBe("•");
  });
  it("badgeColor is deterministic and within the palette", () => {
    expect(badgeColor("GitHub")).toBe(badgeColor("GitHub"));
    expect(BADGE_PALETTE).toContain(badgeColor("GitHub"));
    expect(BADGE_PALETTE).toContain(badgeColor("")); // empty → "•" branch
  });
  it("groupCode splits 6 and 8 digit codes, passes others through", () => {
    expect(groupCode("482913")).toBe("482 913");
    expect(groupCode("01522445")).toBe("0152 2445");
    expect(groupCode("12345")).toBe("12345");
  });
  it("barFraction: 1 when full, 0 when empty, clamps out-of-range", () => {
    expect(barFraction(30, 30)).toBe(1);
    expect(barFraction(0, 30)).toBe(0);
    expect(barFraction(15, 30)).toBe(0.5);
    expect(barFraction(40, 30)).toBe(1); // remaining > period → clamp to 1
    expect(barFraction(5, 0)).toBe(0);   // period 0 → empty
  });
  it("passwordStrength grows with length and variety (0..4)", () => {
    expect(passwordStrength("")).toBe(0);
    expect(passwordStrength("short")).toBe(0);
    expect(passwordStrength("abcdefgh")).toBe(1);
    expect(passwordStrength("Abcdefghijkl")).toBe(3);
    expect(passwordStrength("Abcdef1!ghijkl")).toBe(4);
  });
});
