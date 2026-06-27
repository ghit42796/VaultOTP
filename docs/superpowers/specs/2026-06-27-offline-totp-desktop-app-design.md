# 離線 TOTP 桌面應用 — 設計文件

- **日期：** 2026-06-27
- **狀態：** 已核准設計，待實作規劃
- **平台：** Windows / macOS / Linux（跨平台桌面）

## 1. 目標與範圍

打造一個 **完全離線** 的 TOTP（Time-based One-Time Password，RFC 6238）桌面應用，
作為 Google Authenticator 的桌面替代品。所有資料以主密碼加密儲存在本機，
不連網、不同步雲端。

### 必做功能
- 手動新增帳號（貼上 Base32 secret + issuer/label）。
- 掃描 QR 新增：**讀圖片檔（png/jpg）** 與 **截取螢幕區域**。
- 匯入 Google Authenticator 匯出的 `otpauth-migration://` QR（批次）。
- 加密匯出/備份與匯入。
- 顯示即時 6/8 位數驗證碼與倒數，點擊複製。

### 非目標（YAGNI）
- HOTP（計數型 OTP）：先不做，GA 與絕大多數服務皆用 TOTP。
- 雲端同步 / 多裝置即時同步。
- 相機即時掃描（桌面無一致相機支援）。
- 完整 KDBX 檔案格式互通（僅採用相同密碼學原理，見 §4）。

## 2. 技術選型

- **框架：** Tauri（Rust 核心 + WebView 前端）。理由：體積小（~5–10MB）、
  記憶體佔用低、安全邊界清楚，敏感邏輯可全部留在 Rust。
- **前端：** Svelte + TypeScript（輕量、reactive、適合小型 UI）。
- **後端：** Rust。

## 3. 架構

**核心原則：secret 永不離開 Rust 後端。** 前端只負責顯示與互動；
master key、解密後 secret、TOTP 計算全在 Rust 記憶體中處理。
前端透過 Tauri command/event 取得的只有「當前 code + 剩餘秒數」。

```
┌─────────────────────────────────────────────┐
│  前端 (WebView): Svelte + TypeScript          │
│  · 解鎖畫面、帳號列表、新增/匯入 UI            │
│  · 只顯示 code 與倒數，不碰 secret             │
└───────────────┬─────────────────────────────┘
                │ Tauri IPC (commands/events)
┌───────────────┴─────────────────────────────┐
│  後端 (Rust core)                             │
│  · vault：Argon2id + AES-256-GCM 加解密        │
│  · totp：RFC 6238 計算（背景 tick 推送）        │
│  · qr：圖片 / 螢幕區域解碼                      │
│  · migration：Google Authenticator protobuf    │
│  · storage：加密 vault 檔案原子讀寫             │
└──────────────────────────────────────────────┘
```

各模組職責單一、可獨立測試，透過明確介面溝通。

### 模組職責
| 模組 | 做什麼 | 依賴 |
|------|--------|------|
| `vault` | 金鑰推導、AEAD 加解密、記憶體中帳號集合管理、自動上鎖 | `storage` |
| `totp` | RFC 6238 code 計算、背景每秒 tick 與事件推送 | `vault`（讀 secret） |
| `qr` | 圖片檔 / 螢幕區域截圖 → QR 解碼 → otpauth URI | — |
| `migration` | `otpauth-migration://` Base64 → protobuf → 帳號清單 | — |
| `storage` | vault 檔案原子讀寫（temp + rename） | — |

## 4. 資料模型與加密格式（KeePass 風格密碼學）

採用與 KeePass 相同的密碼學原理（composite key → Argon2id KDF → AEAD + 完整性驗證），
但使用自訂、較簡單的資料格式。**不**追求 KDBX 檔案互通。

### Vault 檔案結構
單一檔案，預設位置為使用者設定目錄（例：`~/.config/auth-totp-app/vault.bin`，
各平台依 OS 慣例）。

```
[ Header (明文，但納入 AEAD 認證作為 AAD) ]
  magic           "ATOTP1\0"
  version         u16
  kdf_algo        Argon2id
  kdf_salt        16 bytes (隨機)
  kdf_params      memory / iterations / parallelism
  cipher          AES-256-GCM
  nonce           12 bytes (隨機，每次存檔重新產生)
[ Ciphertext + GCM Tag ]
  → 解密後為 JSON：帳號陣列
```

### 金鑰推導（composite key）
```
master_password ─┐
                 ├─► SHA-256 composite ─► Argon2id(salt, params) ─► 32-byte key
(未來可擴充 keyfile)┘
```
- Argon2id 預設參數：memory 64 MiB、iterations 3、parallelism 4（可調）。
- AES-256-GCM 同時提供機密性與完整性；header 作為 AAD。
  密碼錯誤或檔案被竄改皆會解密失敗。
- 每次存檔重新產生 nonce，避免 nonce 重用。

### 單筆帳號模型（解密後 JSON）
```jsonc
{
  "id": "uuid",
  "issuer": "GitHub",
  "label": "alice@example.com",
  "secret": "BASE32...",     // 僅存在於解密後記憶體 / 加密 vault 內
  "algorithm": "SHA1",       // SHA1 | SHA256 | SHA512
  "digits": 6,               // 6 | 8
  "period": 30,              // 秒
  "type": "totp"
}
```
涵蓋 Google Authenticator 匯入所需全部欄位。

## 5. 主要流程

### 首次啟動 / 解鎖
- 無 vault → 引導建立主密碼（輸入兩次 + 強度提示）→ 建立空 vault。
- 有 vault → 解鎖畫面輸入主密碼 → 解密失敗統一回「密碼錯誤或檔案損毀」
  （不洩漏是哪一種）。
- 解鎖後 master key 僅留在 Rust 記憶體；支援**閒置自動上鎖**（預設 5 分鐘）與手動上鎖。

### 新增帳號（四種路徑）
1. **手動**：輸入 issuer/label/secret，驗證 Base32 合法性。
2. **讀圖片檔**：選 png/jpg → 解 QR → 解析 `otpauth://totp/...`。
3. **截取螢幕區域**：框選 → 截圖 → 解 QR → 同上。
4. **匯入 GA**：解 `otpauth-migration://offline?data=...` 的 Base64 → protobuf
   → 批次匯入（預覽 + 可勾選）。

### 顯示與更新
Rust 背景每秒 tick，透過 Tauri event 推送各帳號當前 code 與剩餘秒數；
點擊複製到剪貼簿（可設定數秒後自動清空剪貼簿）。

### 加密匯出 / 備份
匯出為獨立加密檔（同一主密碼或另設密碼）；匯入時驗證並合併。

## 6. 錯誤處理原則
- 解密 / 驗證失敗一律安全失敗、不外洩細節。
- QR 解不出、Base32 非法、protobuf 格式錯誤皆有明確使用者訊息。
- 所有 vault 檔案寫入採「寫暫存檔 → 原子改名」，避免存檔中斷損毀。

## 7. 測試策略（TDD：先測試後實作）
- **TOTP**：RFC 6238 官方測試向量（SHA1/256/512、6/8 位）。
- **加密**：Argon2/AES round-trip；竄改偵測（改一個 byte 應解密失敗）。
- **Migration**：`otpauth-migration` protobuf 解析（含多帳號）。
- **Base32**：合法 / 非法 / 邊界輸入。
- **Storage**：原子寫入、損毀檔處理。
- **前端**：關鍵互動以元件測試覆蓋。

## 8. 未來可擴充（不在本期範圍）
- Keyfile 作為第二因子（composite key 已預留）。
- HOTP 支援。
- 完整 KDBX 互通匯入/匯出。
- 標籤 / 分類 / 搜尋強化。
