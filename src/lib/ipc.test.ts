import { describe, it, expect, vi, beforeEach } from "vitest";

const invokeMock = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...a: unknown[]) => invokeMock(...a) }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

import { unlock, currentCodes } from "./ipc";

describe("ipc", () => {
  beforeEach(() => invokeMock.mockReset());

  it("unlock forwards password", async () => {
    invokeMock.mockResolvedValue(undefined);
    await unlock("pw");
    expect(invokeMock).toHaveBeenCalledWith("unlock", { password: "pw" });
  });

  it("currentCodes returns code views", async () => {
    invokeMock.mockResolvedValue([{ id: "1", issuer: "G", label: "a", code: "123456", remaining: 12 }]);
    const codes = await currentCodes();
    expect(codes[0].code).toBe("123456");
  });
});
