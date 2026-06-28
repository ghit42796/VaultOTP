import { describe, it, expect } from "vitest";
import { initial, badgeColor, groupCode, ringCircumference, ringDashoffset, passwordStrength, BADGE_PALETTE } from "./display";

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
  it("ringDashoffset: full at remaining=period, empty at 0", () => {
    const r = 17;
    const c = ringCircumference(r);
    expect(ringDashoffset(30, 30, r)).toBeCloseTo(0, 5);
    expect(ringDashoffset(0, 30, r)).toBeCloseTo(c, 5);
    expect(ringDashoffset(15, 30, r)).toBeCloseTo(c / 2, 5);
    expect(ringDashoffset(5, 0, r)).toBeCloseTo(c, 5); // period 0 → treat as empty
    expect(ringDashoffset(40, 30, r)).toBeCloseTo(0, 5); // remaining > period → clamp to full
  });
  it("passwordStrength grows with length and variety (0..4)", () => {
    expect(passwordStrength("")).toBe(0);
    expect(passwordStrength("short")).toBe(0);
    expect(passwordStrength("abcdefgh")).toBe(1);
    expect(passwordStrength("Abcdefghijkl")).toBe(3);
    expect(passwordStrength("Abcdef1!ghijkl")).toBe(4);
  });
});
