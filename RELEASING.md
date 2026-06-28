# Releasing VaultOTP

VaultOTP ships downloadable installers via **GitHub Releases**. Pushing a `v*` tag triggers
[`.github/workflows/release.yml`](.github/workflows/release.yml), which builds Windows,
macOS, and Linux bundles and attaches them to a **draft** Release for you to review and
publish.

## One-time setup

1. Create the GitHub repository and add it as a remote:
   ```bash
   git remote add origin https://github.com/<owner>/vaultotp.git
   git push -u origin main
   ```
2. Ensure **Actions** are enabled (Settings → Actions → General) and that workflows have
   **Read and write permissions** (Settings → Actions → General → Workflow permissions) —
   the release workflow needs this to create the Release. No extra secrets are required;
   it uses the built-in `GITHUB_TOKEN`.

## ⚠️ The version comes from the manifests, not the tag

The installer filenames and the app's reported version are read from
`tauri.conf.json` / `package.json` — **not** from the git tag. If you tag `v0.2.0` but
forget to bump the manifests, the Release will be named `v0.2.0` while the `.msi` inside
still says `0.1.0`. **Always bump the version first.**

The version must be kept in sync across three files:

| File | Field |
|------|-------|
| `package.json` | `"version"` |
| `src-tauri/tauri.conf.json` | `"version"` |
| `src-tauri/Cargo.toml` | `version` under `[package]` |

## Cutting a release

1. **Bump the version** (e.g. `0.1.0` → `0.2.0`) in all three files above.
2. **Sync the lockfile** so `Cargo.lock` records the new version:
   ```bash
   cd src-tauri && cargo build && cd ..
   ```
3. **Commit** the version bump:
   ```bash
   git add package.json package-lock.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock
   git commit -m "chore: release v0.2.0"
   git push origin main
   ```
4. **Tag and push** (the tag must be `v` + the manifest version):
   ```bash
   git tag v0.2.0
   git push origin v0.2.0
   ```
5. **Watch the build** under the repo's **Actions** tab. The three platform jobs build in
   parallel (~10–20 min including the Rust compile).
6. **Publish the draft.** When the workflow finishes, open **Releases** → the new
   `VaultOTP v0.2.0` **draft** → confirm all expected assets are attached:
   - Windows: `.msi` and `.exe`
   - macOS: `.dmg` (universal)
   - Linux: `.AppImage` and `.deb`

   Edit the release notes if you like, then click **Publish release**.

## If something goes wrong

- **Re-run after a fix:** delete the draft Release and the tag, then re-tag:
  ```bash
  git push --delete origin v0.2.0   # delete remote tag
  git tag -d v0.2.0                 # delete local tag
  # ...fix, commit, push, then re-tag and push again
  ```
  (Also delete the draft Release in the GitHub UI if it was created.)
- **A single platform failed:** `fail-fast: false` means the other platforms still produce
  assets. Fix the failing job and re-run just that job from the Actions UI, or re-tag.

## Not yet wired up (future work)

- **Code signing & notarization.** Builds are currently **unsigned**, so users hit
  SmartScreen (Windows) / Gatekeeper (macOS) warnings. Adding signing requires paid
  certificates and additional repository secrets:
  - Windows: a code-signing certificate.
  - macOS: an Apple Developer ID certificate + notarization (`APPLE_CERTIFICATE`,
    `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID` secrets consumed by `tauri-action`).
- **Auto-updater.** `tauri-action` can also produce updater artifacts + a `latest.json`
  if the Tauri updater plugin is enabled. Not configured today.
- **LICENSE.** There is no license file yet; publishing binaries publicly without one is
  legally ambiguous. Add a `LICENSE` before a public release.
