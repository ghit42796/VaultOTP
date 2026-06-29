# VaultOTP — Release 缺少各平台安裝檔（root cause + 修復）設計文件

- **日期：** 2026-06-29
- **狀態：** 已定位 root cause，待核准修復
- **影響範圍：** `.github/workflows/release.yml`、`RELEASING.md`、發佈流程
- **嚴重度：** 低（非建置失敗；安裝檔已成功產生，只是停在草稿未發佈）

## 1. 症狀

`v0.1.0` 的 GitHub Release 頁面只看得到 **Source code (zip)** 與 **Source code (tar.gz)**
兩個附件，看不到 `.exe` / `.msi` / `.dmg` / `.AppImage` / `.deb` 等各平台安裝檔。

## 2. Root Cause（已用證據證實）

**安裝檔其實已經成功建置並上傳——只是上傳到一個「草稿（draft）」Release，
草稿對外（未登入者）完全隱藏，所以公開頁面看不到。使用者看到的那一頁
其實不是 Release，而是 GitHub 為「附註標籤（annotated tag）」自動產生的標籤頁。**

### 2.1 證據鏈

| # | 證據 | 來源 | 結論 |
|---|------|------|------|
| 1 | `GET /repos/.../releases` 回傳 `[]`；`GET /releases/tags/v0.1.0` 回傳 404 | GitHub REST API（未驗證） | 公開可見的 Release 物件 **不存在** |
| 2 | 截圖文案是「**tagged** this 8 hours ago」（不是 "released this"），內文＝附註標籤訊息 | 截圖 vs `git tag -n99 v0.1.0` | 該頁是 **標籤頁**，非 Release |
| 3 | Release workflow 在 ref `v0.1.0` 上 **執行且成功**（3 個平台 job 全綠） | `GET /actions/runs` | workflow 有跑、沒失敗 |
| 4 | 三個 job 各跑 6–9 分鐘真實建置：macOS 7m25s、Windows 9m34s、Linux 6m20s | `GET /actions/runs/<id>/jobs` | 是真的編譯＋打包，不是 no-op |
| 5 | 每個 job 的 step 8「**Build and attach installers to a draft release**」皆 `success` | 同上 | 安裝檔已建好並上傳 |
| 6 | `releaseDraft: true` | [release.yml:97](../../../.github/workflows/release.yml#L97) | 上傳目標是 **草稿** Release |
| 7 | `bundle.active: true`、`bundle.targets: "all"` | [tauri.conf.json](../../../src-tauri/tauri.conf.json) | 確實會輸出 msi/exe/dmg/deb/AppImage |
| 8 | RELEASING.md 第 55–61 步把「Publish the draft」列為**人工**最後步驟 | [RELEASING.md:55-61](../../../RELEASING.md#L55-L61) | 設計上就需人工發佈，而此步未完成 |

### 2.2 為什麼公開頁面只剩 Source code

- 推送 annotated tag 後，GitHub 會在 Releases UI 用 **標籤訊息** 當內文、並自動掛上
  `Source code (zip/tar.gz)` 兩個原始碼壓縮檔——這是**每個 tag 都有**的，與 workflow 無關。
- `tauri-action` 把安裝檔上傳到的是 `draft: true` 的 Release；草稿**只有具 push 權限、
  且登入**的維護者看得到，未驗證 API 與一般訪客都看不到（證據 #1）。
- 因此「安裝檔在草稿裡、公開頁面只剩原始碼」是**設計行為**，不是建置壞掉。

### 2.3 結論

> 沒有任何建置失敗。`v0.1.0` 的各平台安裝檔已成功產生並上傳到**草稿 Release**，
> 但**從未被人工 Publish**，所以對外不可見。這正是 [release.yml:96-97](../../../.github/workflows/release.yml#L96-L97)
> 與 [RELEASING.md:55-61](../../../RELEASING.md#L55-L61) 所描述的「留草稿待人工審核」流程，
> 只是最後一步沒做。

## 3. 次要風險（需登入後驗證，尚未證實）

`tauri-action` + `releaseDraft: true` + 多平台 matrix 有已知競態：GitHub 的
「依 tag 找 release」API 不會回傳草稿，並行的三個 job 可能各自 **建立一個草稿**，
導致出現**多個重複草稿**、每個只掛單一平台的附件。登入後若在 Releases 看到多個
`VaultOTP v0.1.0` 草稿即屬此情況。§5 的重構會一併消除此風險。

## 4. 立即修復（讓 v0.1.0 對外可見，維護者手動）

> 需具 repo push 權限並登入；本機無 `gh` 與認證，無法代為執行。

1. 登入 GitHub →`ghit42796/VaultOTP`→ **Releases**，找到 **Draft** 的 `VaultOTP v0.1.0`。
2. 確認附件齊全：Windows `.msi`/`.exe`、macOS `.dmg`、Linux `.AppImage`/`.deb`。
3. 若出現**多個**重複草稿（見 §3）：把附件最齊的留下、刪掉其餘，必要時把缺的平台附件補上。
4. 點 **Publish release**。發佈後公開頁面文案會從「tagged this」變「released this」，並列出安裝檔。

   或用 CLI：
   ```bash
   gh release edit v0.1.0 --repo ghit42796/VaultOTP --draft=false
   gh release view v0.1.0 --repo ghit42796/VaultOTP   # 核對附件
   ```

## 5. 結構性修復（重構 release.yml，根除競態並讓發佈可控）

改為官方建議的三段式：**create-release → build-tauri（matrix）→ publish-release**。
單一 Release 只建立一次，所有 matrix job 用同一個 `releaseId` 上傳，最後一個 job 統一翻成已發佈。

```yaml
name: Release
on:
  push:
    tags: ["v*"]
permissions:
  contents: write
jobs:
  create-release:
    runs-on: ubuntu-latest
    outputs:
      release_id: ${{ steps.create.outputs.result }}
    steps:
      - uses: actions/checkout@v4
      - id: create
        uses: actions/github-script@v7
        with:
          script: |
            const tag = context.ref.replace('refs/tags/', '');
            const { data } = await github.rest.repos.createRelease({
              owner: context.repo.owner, repo: context.repo.repo,
              tag_name: tag, name: `VaultOTP ${tag}`,
              body: 'See assets below for per-platform installers (unsigned).',
              draft: true, prerelease: false,
            });
            return data.id;

  build-tauri:
    needs: create-release
    strategy:
      fail-fast: false
      matrix:
        include:
          - platform: macos-latest
            args: --target universal-apple-darwin
            rust-targets: aarch64-apple-darwin,x86_64-apple-darwin
          - platform: ubuntu-latest
            args: ""
            rust-targets: x86_64-unknown-linux-gnu
          - platform: windows-latest
            args: ""
            rust-targets: x86_64-pc-windows-msvc
    runs-on: ${{ matrix.platform }}
    steps:
      # ...（沿用現有：checkout / Linux deps / rust / cache / node / npm ci）...
      - uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          releaseId: ${{ needs.create-release.outputs.release_id }}   # 全平台上傳同一個 release
          args: ${{ matrix.args }}

  publish-release:
    needs: [create-release, build-tauri]
    runs-on: ubuntu-latest
    steps:
      - uses: actions/github-script@v7
        with:
          script: |
            await github.rest.repos.updateRelease({
              owner: context.repo.owner, repo: context.repo.repo,
              release_id: ${{ needs.create-release.outputs.release_id }},
              draft: false,        # 全平台成功才自動發佈
            });
```

**決策點：自動發佈 vs 保留人工審核。**
- **建議（自動發佈）：** 三平台 job 全成功才執行 `publish-release`，把 `draft:false`。
  好處：推 tag 即得到對外可下載的 Release，不會再卡在草稿。
- **若仍要人工審核：** 移除 `publish-release` job，保留草稿，依 §4 手動 Publish；
  或在 `publish-release` 掛上 GitHub Environment 的人工核准 gate。

> 此重構亦解決 §3 的重複草稿競態：`releaseId` 由 `create-release` 唯一產生。

## 6. 文件修正（RELEASING.md）

- 加一節「常見誤解」：標籤頁的 `Source code (zip/tar.gz)` 是 GitHub 對 **每個 tag** 自動產生的，
  **不是** workflow 產物；安裝檔在 **Releases → Draft** 內，發佈後才會出現在公開頁。
- 教如何分辨：頁面文案「tagged this」＝尚未發佈（僅標籤）；「released this」＝已發佈。
- 移除已過時的「LICENSE 尚未建立」敘述（repo 根目錄已有 `LICENSE`）。

## 7. 驗收條件（Acceptance Criteria）

- [ ] `v0.1.0`（或後續版本）公開 Release 列出 Windows/macOS/Linux 安裝檔，文案為「released this」。
- [ ] `GET /repos/ghit42796/VaultOTP/releases` 未驗證即可看到該 Release 與其附件。
- [ ] 重構後：推一個測試 tag → 只產生**一個** Release；三平台附件齊全；全綠後自動（或經核准）發佈。
- [ ] `RELEASING.md` 含「草稿 vs 已發佈」與「Source code 附件來源」說明。

## 8. 不在本次範圍（YAGNI / 既有 backlog）

- 程式碼簽章 / 公證（SmartScreen、Gatekeeper）—— 見 [RELEASING.md:76-84](../../../RELEASING.md#L76-L84)。
- 自動更新器（updater artifacts + `latest.json`）。
