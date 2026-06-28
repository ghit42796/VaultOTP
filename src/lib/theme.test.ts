import { describe, it, expect } from "vitest";
import { normalizePref, themeAttr } from "./theme";

describe("theme helpers", () => {
  it("normalizePref accepts valid values, defaults to system", () => {
    expect(normalizePref("light")).toBe("light");
    expect(normalizePref("dark")).toBe("dark");
    expect(normalizePref("system")).toBe("system");
    expect(normalizePref(null)).toBe("system");
    expect(normalizePref("bogus")).toBe("system");
  });
  it("themeAttr maps pref to the data-theme value (null for system)", () => {
    expect(themeAttr("system")).toBeNull();
    expect(themeAttr("light")).toBe("light");
    expect(themeAttr("dark")).toBe("dark");
  });
});
