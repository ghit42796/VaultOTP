import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AccountView, CodeView } from "./types";

export const vaultExists = () => invoke<boolean>("vault_exists");
export const createVault = (password: string) => invoke<void>("create_vault", { password });
export const unlock = (password: string) => invoke<void>("unlock", { password });
export const lock = () => invoke<void>("lock");
export const isUnlocked = () => invoke<boolean>("is_unlocked");
export const listAccounts = () => invoke<AccountView[]>("list_accounts");
export const currentCodes = () => invoke<CodeView[]>("current_codes");
export const addManual = (issuer: string, label: string, secret: string) =>
  invoke<void>("add_manual", { issuer, label, secret });
export const addFromUri = (uri: string) => invoke<void>("add_from_uri", { uri });
export const decodeQrFile = (path: string) => invoke<string[]>("decode_qr_file", { path });
export const previewMigration = (uri: string) => invoke<AccountView[]>("preview_migration", { uri });
export const importMigration = (uri: string, selectedIndices: number[]) =>
  invoke<number>("import_migration", { uri, selectedIndices });
export const removeAccount = (id: string) => invoke<void>("remove_account", { id });
export const exportBackup = (path: string, password: string) => invoke<void>("export_backup", { path, password });
export const importBackup = (path: string, password: string) => invoke<number>("import_backup", { path, password });

export const onTick = (cb: () => void): Promise<UnlistenFn> => listen("tick", () => cb());
export const onLocked = (cb: () => void): Promise<UnlistenFn> => listen("locked", () => cb());
