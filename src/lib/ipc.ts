import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AccountView, CodeView } from "./types";

export const vaultExists = () => invoke<boolean>("vault_exists");
export const vaultMode = () => invoke<"password" | "keyfile" | "composite">("vault_mode");
export const createVault = (mode: string, password?: string, keyfilePath?: string) =>
  invoke<void>("create_vault", { mode, password, keyfilePath });
export const unlock = (password?: string, keyfilePath?: string) =>
  invoke<void>("unlock", { password, keyfilePath });
export const generateKeyfile = (outPath: string) => invoke<void>("generate_keyfile", { outPath });
export const addKeyfile = (password: string, keyfilePath: string) =>
  invoke<void>("add_keyfile", { password, keyfilePath });
export const removeKeyfile = (password: string, keyfilePath: string) =>
  invoke<void>("remove_keyfile", { password, keyfilePath });
export const changePassword = (
  currentPassword: string,
  currentKeyfilePath: string | undefined,
  newPassword: string,
  newKeyfilePath: string | undefined,
) =>
  invoke<void>("change_password", { currentPassword, currentKeyfilePath, newPassword, newKeyfilePath });
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
export const reorderAccounts = (ids: string[]) => invoke<void>("reorder_accounts", { ids });
export const exportBackup = (path: string, password: string) => invoke<void>("export_backup", { path, password });
export const importBackup = (path: string, password: string) => invoke<number>("import_backup", { path, password });

export const onTick = (cb: () => void): Promise<UnlistenFn> => listen("tick", () => cb());
export const onLocked = (cb: () => void): Promise<UnlistenFn> => listen("locked", () => cb());

export type ExportFormat = "otpauth_qr" | "otpauth_text" | "google_migration";
export const exportSecrets = (ids: string[], path: string, format: ExportFormat): Promise<number> =>
  invoke<number>("export_secrets", { ids, path, format });

export interface RecentVaultView { path: string; exists: boolean }
export const setCurrentVault = (path: string) => invoke<void>("set_current_vault", { path });
export const listRecentVaults = () => invoke<RecentVaultView[]>("list_recent_vaults");
export const currentVaultPath = () => invoke<string>("current_vault_path");
export const saveVaultAs = (path: string) => invoke<void>("save_vault_as", { path });
