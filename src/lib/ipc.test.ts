import { describe, it, expect, vi, beforeEach } from "vitest";

const invokeMock = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...a: unknown[]) => invokeMock(...a) }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

import { unlock, currentCodes, createVault, setCurrentVault, listRecentVaults, saveVaultAs, exportSecrets, reorderAccounts } from "./ipc";

describe("ipc", () => {
  beforeEach(() => invokeMock.mockReset());

  it("unlock forwards optional password + keyfile path", async () => {
    invokeMock.mockResolvedValue(undefined);
    await unlock("pw", "/k.vaultkey");
    expect(invokeMock).toHaveBeenCalledWith("unlock", { password: "pw", keyfilePath: "/k.vaultkey" });
  });

  it("createVault forwards mode and credentials", async () => {
    invokeMock.mockResolvedValue(undefined);
    await createVault("composite", "pw", "/k.vaultkey");
    expect(invokeMock).toHaveBeenCalledWith("create_vault", {
      mode: "composite",
      password: "pw",
      keyfilePath: "/k.vaultkey",
    });
  });

  it("currentCodes returns code views", async () => {
    invokeMock.mockResolvedValue([{ id: "1", issuer: "G", label: "a", code: "123456", remaining: 12 }]);
    const codes = await currentCodes();
    expect(codes[0].code).toBe("123456");
  });

  it("setCurrentVault forwards path", async () => {
    invokeMock.mockResolvedValue(undefined);
    await setCurrentVault("/vaults/a.bin");
    expect(invokeMock).toHaveBeenCalledWith("set_current_vault", { path: "/vaults/a.bin" });
  });

  it("listRecentVaults returns entries", async () => {
    invokeMock.mockResolvedValue([{ path: "/a.bin", exists: true }]);
    const r = await listRecentVaults();
    expect(r[0].path).toBe("/a.bin");
    expect(r[0].exists).toBe(true);
  });

  it("saveVaultAs forwards path", async () => {
    invokeMock.mockResolvedValue(undefined);
    await saveVaultAs("/copy.bin");
    expect(invokeMock).toHaveBeenCalledWith("save_vault_as", { path: "/copy.bin" });
  });

  it("exportSecrets forwards ids, path, format", async () => {
    invokeMock.mockResolvedValue(2);
    const n = await exportSecrets(["id1", "id2"], "/out.txt", "otpauth_text");
    expect(invokeMock).toHaveBeenCalledWith("export_secrets", { ids: ["id1", "id2"], path: "/out.txt", format: "otpauth_text" });
    expect(n).toBe(2);
  });

  it("reorderAccounts forwards ids", async () => {
    invokeMock.mockResolvedValue(undefined);
    await reorderAccounts(["a", "b", "c"]);
    expect(invokeMock).toHaveBeenCalledWith("reorder_accounts", { ids: ["a", "b", "c"] });
  });
});
