# VaultOTP — Dependency Provenance, CVE, Malicious-History & Sans-IO Audit

**Date:** 2026-06-28
**Target:** VaultOTP — offline TOTP authenticator (Tauri 2 desktop app; Rust backend + Svelte/TypeScript frontend)
**Lockfiles audited:** `src-tauri/Cargo.lock` (Rust, 555 resolved crates), `package-lock.json` (npm, 167 resolved packages)

---

## 1. Scope & method

### What was examined
- **Lockfiles (ground truth for resolved versions):**
  - Rust: `src-tauri/Cargo.lock` — **555 resolved crates** (18 direct deps in `[dependencies]` + `tauri-build` build-dep + a `windows` target-dep = 20 direct).
  - npm: `package-lock.json` — **167 resolved packages** (10 direct).
- **Web advisory databases (per-package):** OSV (`api.osv.dev` / `osv.dev/list`), RustSec advisory DB, GitHub Security Advisory Database, NVD, plus vendor/security blogs (Snyk, Socket.dev, ReversingLabs, SentinelOne, Wiz, CISA) for supply-chain incident corroboration.
- **Source code (sans-IO design audit):** all backend Rust modules under `src-tauri/src/` and the frontend I/O-touching modules under `src/`.

### Explicit limits
- **Per-package web deep-dive** (provenance + CVE + malicious-history, web-cited) was performed for the **20 direct Rust deps** (covered collectively/spot-checked) and individually for the high-value set captured in the structured data: Rust — `tauri`, `tauri-build`, `tauri-plugin-dialog`, `serde`, `serde_json`, `thiserror`, `argon2`, `aes-gcm`, `hmac`, `sha1`, `sha2`, `base32`, `rqrr`, `image`, `prost`, `zeroize`, `uuid`, `xcap`, `rand`, `windows`; npm — all 10 direct deps.
- **Catalog classification (no individual web deep-dive)** was applied to the transitive tails of both trees. Notable transitive crates/packages are listed in §2 "Non-official packages" but did not each receive a dedicated CVE/malicious-history query.
- **Full-tree CVE scanning is covered separately** by `cargo-audit` and `npm-audit`, which the controller runs against the complete 555-crate / 167-package graphs. This report's CVE table (§3) reflects the per-package deep-dive set, not the exhaustive transitive scan.
- **Provenance integrity:** all 555 Rust crates resolve from the crates.io registry — **zero git sources, zero alternate registries** — so there is no source-override supply-chain risk in the Rust tree.
- **Note on input paths:** the orchestrator-supplied `undefined/rust-deps.txt` / `undefined/js-deps.txt` paths were unresolved template variables and did not exist; resolved versions were taken authoritatively from the lockfiles (and a mirrored scratchpad copy where present).

### Confidence
All per-package findings below were returned with `web_access_ok=true` and `confidence=high`. Two OSV calls hit the GET-only `WebFetch` HTTP 405 limitation on the POST endpoint (`aes-gcm`, `tauri-build`, `windows`); in each case OSV's web list, RustSec, and GitHub Advisories were reachable and corroborated the result, so confidence remained high. **No `web_access_ok=false` / low-confidence gaps were reported for any deep-dived package.**

---

## 2. Dependency inventory & provenance

### Direct dependencies — Rust (20)

| Crate | Version | Maintainer / Org | Official? |
|---|---|---|---|
| tauri | 2.11.3 | Tauri org (tauri-apps) | yes |
| tauri-build *(build-dep)* | 2.6.3 | Tauri org (tauri-apps) | yes |
| tauri-plugin-dialog | 2.7.1 | Tauri org (plugins-workspace) | yes |
| serde | 1.0.228 | David Tolnay (dtolnay) / serde-rs | yes |
| serde_json | 1.0.150 | David Tolnay (dtolnay) / serde-rs | yes |
| thiserror | 1.0.69 | David Tolnay (dtolnay) | yes |
| argon2 | 0.5.3 | RustCrypto org | yes |
| aes-gcm | 0.10.3 | RustCrypto org | yes |
| hmac | 0.12.1 | RustCrypto org | yes |
| sha1 | 0.10.6 | RustCrypto org | yes |
| sha2 | 0.10.9 | RustCrypto org | yes |
| image | 0.25.10 | image-rs org | yes |
| prost | 0.13.5 | tokio-rs org | yes |
| zeroize | 1.9.0 | RustCrypto org | yes |
| uuid | 1.23.4 | uuid-rs org | yes |
| rand | 0.8.6 | rust-random org | yes |
| windows | 0.58.0 *(target-dep; 0.57.0/0.61.3/0.62.2 also present transitively)* | Microsoft (windows-rs) | yes |
| **base32** | **0.5.1** | **Andreas Ots (andreasots), individual** | **no** |
| **rqrr** | **0.8.0** | **Moritz Wanzenböck (@WanzenBug), individual** | **no** |
| **xcap** | **0.0.15** | **nashaofu, individual** | **no** |

**Split:** 16 official direct, **4 non-official direct** (one of which, `windows`, is counted official; `uuid` counted official as a well-known community-org steward). 15 of 20 trace to major first-party stewards (Tauri, RustCrypto, dtolnay/serde-rs, tokio-rs, image-rs, rust-random, uuid-rs, Microsoft).

### Direct dependencies — npm (10)

| Package | Version | Maintainer / Org | Official? |
|---|---|---|---|
| @tauri-apps/api | 2.11.1 | Tauri Working Group | yes |
| @tauri-apps/plugin-dialog | 2.7.1 | Tauri Working Group | yes |
| @tauri-apps/cli | 2.11.3 | Tauri Working Group | yes |
| svelte | 4.2.20 | sveltejs (Svelte core team) | yes |
| @sveltejs/vite-plugin-svelte | 3.1.2 | sveltejs (Svelte core team) | yes |
| svelte-check | 3.8.6 | sveltejs (Svelte core team) | yes |
| @tsconfig/svelte | 5.0.8 | @tsconfig org (config-only) | yes |
| typescript | 5.9.3 | Microsoft (TypeScript team) | yes |
| vite | 5.4.21 | vitejs (Vite team) | yes |
| vitest | 2.1.9 | vitest-dev (Vitest team) | yes |

**Split: 10/10 direct npm deps official, 0 non-official direct.** Risk in the npm tree concentrates entirely in the transitive tail (mostly platform-specific `@esbuild` / `@rollup` / `@tauri-apps` binary subpackages plus tiny single-maintainer utilities).

### Non-official packages

> Listed explicitly per the user requirement. "Non-official" = maintained by an individual or small third party rather than a major first-party steward.

**Non-official DIRECT Rust deps (4):**
- **base32 0.5.1** — individual-maintained (andreasots). Trivial scope (Base32 decode of otpauth secrets). ~25.5M downloads; latest version; no advisories.
- **rqrr 0.8.0** — individual-maintained (Moritz Wanzenböck). QR-code reader; bundles `g2p`/`g2gen`/`g2poly` 1.2.2 from the same author for Galois-field math. ~3.9M downloads; no advisories.
- **xcap 0.0.15** — individual-maintained (nashaofu). Pre-1.0 cross-platform **screen capture** (security-relevant: screen access). Pulls `dbus`, `dlopen2`, `xcb`, `core-graphics`, `image 0.24`, `windows 0.61`. Used in `qr.rs` for region capture. No advisories on any version.

*(Note: `windows` and `uuid` are individual/community-org maintained but classified official as recognized stewards — Microsoft and uuid-rs respectively.)*

**Notable non-official TRANSITIVE Rust crates** (flagged for a reviewer; not individually CVE-scanned here):
- **xcap platform FFI stack:** `dbus 0.9.11` (diwic), `dlopen2 0.8.2` + `dlopen2_derive 0.4.3` (OpenByteDev — **runtime dynamic library loading via dlopen**, elevated review interest), `xcb 1.7.0` (rust-x-bindings), `libdbus-sys 0.2.7`.
- **g2p / g2gen / g2poly 1.2.2** — single author (same as rqrr); niche Galois-field/crypto-adjacent math, very low popularity.
- **AVIF / image codec stack (large unsafe/SIMD surface):** `rav1e 0.8.1` (xiph, reputable), `ravif 0.13.0` + `avif-serialize` (kornelski, individual), `av-scenechange`, `av1-grain`, `moxcms 0.8.1` + `pxfm 0.1.29` (awxkee, individual), `zmij 1.0.21` (obscure name — worth confirming), `fax 0.2.7` (s3bk; CCITT/G4 fax decoder pulled via TIFF — unexpected in a TOTP app).
- **swift-rs 1.0.7** (Brendonovich/Spacedrive) — Rust↔Swift FFI via the Tauri macOS stack (macOS-only; FFI = elevated review interest).
- **vswhom / vswhom-sys** (nabijaczleweli) — locates MSVC/VS at build time on Windows; tiny FFI, build-time only.
- **dom_query 0.27.0** (niklak), **softbuffer 0.4.8** (rust-windowing — recognized community org), **bs58 0.5.1** (Nemo157 — Base58 codec, crypto-adjacent).

Roughly 30–40 distinct crates in the 555-crate graph are individual/small-third-party maintained; the rest trace to recognized stewards. The non-official transitive tail is driven primarily by (a) the AVIF/image codec stack and (b) xcap's platform FFI.

**Notable non-official TRANSITIVE npm packages** (single-maintainer micro-utilities historically targeted in npm supply-chain attacks — all observed at current/patched versions; classification only):
- **debug 4.4.3** + **ms 2.1.3** (qix-/debug-js) — qix- was the victim of the **Sept 2025 npm account-takeover that poisoned chalk/debug/ansi**. Versions here are **post-incident clean**, but this is the canonical high-fan-out target to verify.
- **minimist 1.2.8** (ljharb) — prior prototype-pollution CVEs; pinned at patched 1.2.8 (via rimraf/mkdirp).
- **nanoid 3.3.15** (ai) — prior infinite-loop/predictability CVEs; current/patched (via postcss).
- **lukeed micro-portfolio:** `kleur 4.1.5`, `sade 1.8.1`, `mri 1.2.0` — single-author concentration (via svelte-check/sade).
- **picocolors 1.1.1** (alexeyraspopov) — colors/chalk niche (via postcss/vite).
- **Stale isaacs cluster:** `rimraf 2.7.1` → **deprecated `glob 7.2.3`** → **deprecated `inflight 1.0.6`** (memory leak); plus **es6-promise 3.3.1** (2016-era polyfill). Reliability/prune candidates, not active compromises.

---

## 3. CVE / known vulnerabilities

All advisory IDs below were verified against the resolved versions. **Advisories apply to three packages: `vite` and `vitest` (DEV/BUILD-only — do not ship), and `svelte` (RUNTIME, but the affected code paths are not reachable in this app — see analysis).** Every other runtime-shipped dependency is fully patched.

### Advisories that APPLY to a resolved version

| Package | Advisory | Severity | Affected range | Applies? | Dev-only? | URL |
|---|---|---|---|---|---|---|
| vite | GHSA-fx2h-pf6j-xcff (CVE-2026-53571) | High (CVSS v4 8.2) | ≤ 6.4.2 (no 5.x fix; 5.x EOL) | **yes** | **yes (dev-server only)** | https://github.com/advisories/GHSA-fx2h-pf6j-xcff |
| vite | GHSA-4w7w-66w2-5vf9 (CVE-2026-39365) | Medium | ≤ 6.4.1 (no 5.x fix; 5.x EOL) | **yes** | **yes (dev-server only)** | https://github.com/advisories/GHSA-4w7w-66w2-5vf9 |
| vitest | GHSA-5xrq-8626-4rwp (CVE-2026-47429) | Critical | < 3.2.6 | **yes** | **yes (test runner)** | https://github.com/advisories/GHSA-5xrq-8626-4rwp |
| svelte | GHSA-rcqx-6q8c-2c42 (DOM clobbering of internal state) | Moderate | introduced `0`, fixed `5.55.7` | **yes (metadata)** | no (runtime) | https://github.com/advisories/GHSA-rcqx-6q8c-2c42 |
| svelte | GHSA-pr6f-5x2q-rwfp + GHSA-f7gr-6p89-r883 + GHSA-crpf-4hrx-3jrp + GHSA-m56q-vw4c-c2cp + GHSA-phwv-c562-gvmh (SSR XSS family) | Moderate | introduced `0`, fixed `5.55.7` | **yes (metadata)** | no (runtime) | https://github.com/advisories/GHSA-pr6f-5x2q-rwfp |

**Context on the `svelte` runtime advisories (correction to the workflow's "5.x-only" claim):**
- Authoritative OSV/GHSA structured data for these advisories declares the npm `svelte` affected range as **`introduced: "0"`, `fixed: "5.55.7"`** — i.e. **svelte 4.2.20 IS within the affected range** (the only declared fix is the breaking upgrade to svelte 5.55.7). `npm audit` correctly flags svelte 4.2.20; the earlier "other svelte CVEs are 5.x-only / 0 runtime CVEs" statement was inaccurate.
- **Real-world exploitability in THIS app is effectively nil**, verified against source: there is **no SSR** (this is a client-only Tauri SPA — every SSR-family advisory is N/A), **no `@html`/`innerHTML`/`svelte:element`** (`grep` over `src/` returns none), and **no attribute spreading `{...}`** anywhere (the DOM-clobbering vector requires `{...spread}` on a form with attacker-controlled dynamic `name`s). Untrusted strings (`issuer`/`label` from QR/migration) are rendered only via auto-escaped text interpolation — `AccountCard.svelte:28-29` `{item.issuer}` / `{item.label}`, `ImportGoogle.svelte:51`.
- **Net:** the advisory metadata applies to the version, but no advisory's attack path is reachable. Remediation (svelte 5) is a major breaking upgrade; acceptable to defer given zero reachable surface, but it is a tracked runtime item, not "0 CVEs."

**Context on the applicable DEV/BUILD-only advisories (do NOT ship in the Tauri runtime bundle):**
- **vite 5.4.21** — two dev-server `server.fs.deny`/path-traversal bypasses. Both require the dev server to be exposed to the network (`server.host` / `--host`); not exploitable on plain localhost dev and absent from the shipped artifact (Tauri serves the prebuilt `dist/`). **vite 5.x is EOL with no backported fix** — patches go only to 6.4.3 / 7.3.5 / 8.0.16. *(Important nuance: an early WebFetch summary mis-stated "5.4.21 not affected"; authoritative OSV data — `introduced:0`, `fixed:6.4.x` — confirms the ≤6.4.x catch-all range includes 5.4.21.)*
- **vitest 2.1.9** — `/__vitest_attachment__` path traversal + save/rerun APIs enable arbitrary file read / RCE **only when the Vitest UI/API server is listening** (esp. network-exposed). This repo runs headless `vitest run` (no UI/API server, no browser mode), so real-world exposure is low. Fixed in 3.2.6.

### Historical advisories that do NOT apply (resolved version is patched / out of range)

| Package | Advisory | Severity | Affected range | Runtime? | Note |
|---|---|---|---|---|---|
| tauri | GHSA-7gmj-67g7-phm9 (CVE-2026-42184) | Moderate 6.1 | ≥2.0.0, ≤2.11.0 (fix 2.11.1) | runtime | Most recent; 2.11.3 patched. Plus 7 older 1.x/2.x-alpha advisories — all fixed long before 2.11.3. |
| aes-gcm | RUSTSEC-2023-0096 / CVE-2023-42811 / GHSA-423w-p2w9-r7vq | Medium 4.7 | 0.10.0–0.10.2 (fix 0.10.3) | runtime | Resolved 0.10.3 is the fix. |
| image | RUSTSEC-2019-0014 / CVE-2019-16138 | Critical 9.8 | 0.10.2–<0.21.3 | runtime | Both image advisories fixed far below 0.25.10 (and transitive 0.24.9). |
| image | RUSTSEC-2020-0073 / CVE-2020-35916 | Moderate 5.5 | <0.23.12 | runtime | |
| sha2 | RUSTSEC-2021-0100 / CVE-2021-45696 | High 9.8 | 0.9.7 only (fix 0.9.8) | runtime | 0.10.9 unaffected. |
| prost | RUSTSEC-2020-0002 / CVE-2020-35858 | Critical 9.8 | <0.6.1 | runtime | 0.13.5 unaffected. (RUSTSEC-2021-0073 is `prost-types`, not `prost`.) |
| rand | RUSTSEC-2026-0097 | Low (unsound) | <0.8.6; 0.9.0–<0.9.3; 0.10.0–<0.10.1 | runtime | 0.8.6 is the fix boundary; transitive 0.9.4 also patched. |
| windows | RUSTSEC-2022-0008 | Unsound (GitHub auto-scores 9.8) | <0.32.0 | runtime | All resolved versions (0.57/0.58/0.61.3) far above fix. |
| zeroize | RUSTSEC-2021-0115 / CVE-2021-45706 | Critical 9.8 | `zeroize_derive` <1.1.1 (not zeroize) | runtime | Repo uses zeroize_derive 1.5.0; zeroize crate itself has zero advisories. |
| serde | — | — | precompiled-binary controversy (serde_derive 1.0.172–1.0.183) | runtime | Not a CVE; reverted in 1.0.184; 1.0.228 builds from source. |
| @tauri-apps/cli | GHSA-2rcp-jvr4-r259 (CVE-2023-46115) | High 8.4 | <1.5.6 / <2.0.0-alpha.16 | **dev-only** | 2.11.3 far above fix; project `vite.config.ts` has no insecure `envPrefix`. |
| svelte | CVE-2024-45047 | Medium 6.1 | <4.2.19 (fix 4.2.19) | runtime | 4.2.20 patched for THIS one. ⚠️ But the DOM-clobbering + SSR-XSS family (GHSA-rcqx-6q8c-2c42, GHSA-pr6f-5x2q-rwfp, …) DO apply to 4.2.20 (introduced `0`, fixed `5.55.7`) — moved to the "APPLY" table above. |
| vite | CVE-2025-62522 / -58752 / -58751 / -46565 / -32395, GHSA-p9ff-h696-f583, etc. | various | <5.4.21 / 6.x+ | dev-only | All fixed at/before 5.4.21 or only affect 6.x+. |
| vitest | CVE-2025-24964 (GHSA-9crc-q9x8-hgqq) | Critical | <2.1.9 | dev-only | 2.1.9 is exactly the patch. Browser-mode 4.x CVEs N/A. |
| uuid | CVE-2026-41907 / CVE-2026-41988 | — | npm uuidjs <14.0.0 | n/a | **Name collision** — these affect the npm `uuid` package, NOT the Rust crate. |
| serde_json | RUSTSEC-2024-0012 | High 7.5 | `serde-json-wasm` (different crate) | n/a | Name disambiguation only. |
| sha1 | RUSTSEC-2025-0021 | — | `gix-features` (algorithmic SHA-1 note) | n/a | Not the sha1 crate; irrelevant to HMAC-SHA1 TOTP use. |

**No advisories of any kind** were found for: tauri-build, tauri-plugin-dialog, thiserror, argon2, hmac, base32, rqrr, xcap, @tauri-apps/api, @tauri-apps/plugin-dialog, @sveltejs/vite-plugin-svelte, @tsconfig/svelte, svelte-check, typescript.

### Runtime-shipped vs Dev/Build-only summary
- **Runtime-shipped dependencies: every Rust crate is fully patched (0 applicable Rust CVEs).** The one runtime npm package with applicable advisories is **`svelte 4.2.20`** — a family of **Moderate** XSS/DOM-clobbering advisories (`introduced 0`, `fixed 5.55.7`) flagged by `npm audit`. Per metadata they apply to 4.2.20, **but none of their attack paths are reachable in this app** (no SSR, no `@html`/`innerHTML`, no attribute spreading; all untrusted text auto-escaped). Effective runtime risk ≈ none; remediation requires the breaking svelte 5 upgrade (tracked, deferrable).
- **Dev/Build-only: 3 applicable CVEs** — `vite` (High + Medium) and `vitest` (Critical), none of which ship in the production Tauri binary. By severity: **1 Critical (vitest, dev), 1 High (vite, dev), 1 Medium (vite, dev).**
- **`npm audit` cross-check:** reports **10 advisories total = 1 runtime (svelte, Moderate, unreachable) + 9 dev/build-only** (svelte-hmr, @sveltejs/vite-plugin-svelte(+inspector), vite, vite-node, vitefu, @vitest/mocker, esbuild, vitest). Consistent with the above.

### Full-tree `cargo audit` cross-check (authoritative, all 555 crates)
Ran `cargo audit` (RustSec advisory-db, 1,139 advisories loaded) against the full `Cargo.lock`:
- **Vulnerabilities: 0.** No crate in the entire Rust tree has a known security vulnerability. This confirms the per-package deep-dive.
- **Warnings: 19** — informational only (no exploit): **17 `unmaintained` + 2 `unsound`**.
  - **GTK3/Linux cluster (11, NOT in the Windows build):** `atk`, `atk-sys`, `gdk`, `gdk-sys`, `gdkwayland-sys`, `gdkx11`, `gdkx11-sys`, `gtk`, `gtk-sys`, `gtk3-macros` (RUSTSEC-2024-041x, gtk-rs GTK3 bindings unmaintained) + `glib 0.18.5` (RUSTSEC-2024-0429 `unsound` VariantStrIter). Pulled by Tauri's Linux webview (`wry`/`webkit2gtk`); compiled only on the Linux target, absent from Windows/macOS builds. Upstream Tauri tracking issue; nothing the app controls.
  - **Build/proc-macro (2):** `paste` (RUSTSEC-2024-0436 unmaintained), `proc-macro-error` (RUSTSEC-2024-0370 unmaintained) — compile-time only.
  - **Unicode tables (5):** `unic-char-property`/`-range`/`-common`/`-ucd-ident`/`-ucd-version` 0.9.0 (RUSTSEC-2025-007x..010x unmaintained) — transitive, data-only.
  - **`lru 0.12.5`** (RUSTSEC-2026-0002 `unsound` IterMut/Stacked-Borrows) — transitive; potential UB under Miri, no known exploit.
- **None of the 19 is in a VaultOTP-authored code path**; all are transitive framework/build deps, and the largest group is Linux-only.

---

## 4. Malicious-behavior history

Per the per-package deep-dive (OSV / RustSec / GitHub Advisories / targeted web searches), **no credible malicious-behavior or supply-chain-compromise history was found for ANY audited package or its maintainers.** All checks returned `web_access_ok=true`, `confidence=high` — **no web-access gaps.**

| Package | Malicious history? | Notable context (with URL) |
|---|---|---|
| tauri | none found | TrapDoor (May 2026) involved unrelated typosquats; GH issue #7117 is an AV false-positive only. https://github.com/tauri-apps/tauri/security |
| tauri-build | none found | TrapDoor hit Sui/Move build helpers, not tauri. https://socket.dev/blog/trapdoor-crypto-stealer-npm-pypi-crates |
| tauri-plugin-dialog | none found | "Shai-Hulud 2.0" (Nov 2025) only inflated star counts of some Tauri repos; no artifact compromise. https://www.wiz.io/blog/shai-hulud-2-0-ongoing-supply-chain-attack |
| serde | none found | Aug 2023 `serde_derive` precompiled-binary controversy (no malware; reverted 1.0.184). https://github.com/serde-rs/serde/issues/2538 |
| serde_json | none found | Only unrelated incidents (TrapDoor, "onering"). |
| thiserror | none found | TrapDoor explicitly does not involve dtolnay. https://socket.dev/blog/trapdoor-crypto-stealer-npm-pypi-crates |
| argon2 | none found | RustCrypto, 35M+ downloads. https://crates.io/crates/argon2 |
| aes-gcm | none found | macOS.Gaslight implant *uses* aes-gcm 0.10.3 for C2 — legitimate third-party use, NOT a package compromise. NCC Group audit, no significant findings. https://www.sentinelone.com/labs/... |
| hmac | none found | https://github.com/RustCrypto/MACs |
| sha1 | none found | Unrelated typosquats (sha-rust, finch-rust by "face-lessssss"). https://github.com/RustCrypto/hashes/tree/master/sha1 |
| sha2 | none found | Only the 0.9.7 correctness bug; unrelated typosquats otherwise. https://osv.dev |
| base32 | none found | https://crates.io/crates/base32 |
| rqrr | none found | https://crates.io/crates/rqrr |
| image | none found | https://rustsec.org/packages/image.html |
| prost | none found | tokio-rs, ~466M downloads. https://github.com/tokio-rs/prost |
| zeroize | none found | https://osv.dev |
| uuid | none found | Malware hits were unrelated (Go typosquat, "onering"). https://rustsec.org/packages/uuid.html |
| xcap | none found | Legitimate screen-capture crate; v0.0.15 genuine release. https://github.com/nashaofu/xcap |
| rand | none found | https://github.com/rust-random/rand |
| windows | none found | Unrelated 2026 incidents (Miasma, Mastra, TrapDoor). https://rustsec.org/packages/windows.html |
| @tauri-apps/api | none found | Shai-Hulud / CISA Sept 2025 do not implicate the package. |
| @tauri-apps/plugin-dialog | none found | 2.7.1 tarball carries SLSA provenance + registry signatures. https://api.osv.dev/v1/query |
| @tauri-apps/cli | none found | Only the CVE-2023-46115 misconfiguration advisory (no malicious code). https://osv.dev/list?q=%40tauri-apps%2Fcli&ecosystem=npm |
| svelte | none found | NOT in any Shai-Hulud / Sept 2025 compromised list. https://security.snyk.io/package/npm/svelte |
| @sveltejs/vite-plugin-svelte | none found | Snyk + ReversingLabs scans clean. https://api.osv.dev/v1/query |
| @tsconfig/svelte | none found | Config-only (static JSON). https://github.com/advisories?query=%40tsconfig%2Fsvelte |
| svelte-check | none found | https://api.osv.dev/v1/query |
| typescript | none found | Official Microsoft compiler; unrelated typosquats only. |
| vite | none found | All vite advisories are ordinary security bugs, not malicious code. https://github.com/advisories?query=vite |
| vitest | none found | Cross-checked against the 3,290+-entry shai-hulud-detect list — neither `vitest` nor `@vitest/*` appears. https://github.com/Cobenian/shai-hulud-detect/blob/main/compromised-packages.txt |

**Single-maintainer concentration note:** the npm transitive tail includes the exact micro-utility category repeatedly hit in npm supply-chain attacks (`debug`/`ms` from qix-, `minimist`, `nanoid`, the lukeed portfolio, `picocolors`). All are at **current/patched versions** in this lockfile; the concern is latent concentration/blast-radius risk, not a present compromise.

---

## 5. Sans-IO compliance

**Overall verdict: Strong adherence for the protocol/crypto core; partial (verging on strong) for the project as a whole.**
**Crypto-core rating: strong.** The two hardest disciplines are nailed — the clock is fully injected and the crypto primitives are pure functions over byte slices. Remaining deviations are concentrated and consistent: entropy (RNG/UUID) is called inside logic modules instead of injected, and the Vault/backup orchestration interleaves `std::fs` with state transitions.

### Per-module table

| Module | Role | I/O kinds | Pure sans-IO? | Evidence (file:line) |
|---|---|---|---|---|
| `src-tauri/src/totp.rs` | TOTP/HOTP core (RFC 6238) | none | **yes** | `generate(account, unix_time)` takes clock as param (totp.rs:26); `remaining_seconds` (totp.rs:50). No SystemTime/rand/fs. |
| `src-tauri/src/secret.rs` | Base32 secret decode | none | **yes** | `decode_secret(s: &str)` pure on input (secret.rs:4). |
| `src-tauri/src/model.rs` | Data model (Account, Algorithm) | rng | no | `Account::new` calls `Uuid::new_v4()` (model.rs:34) — entropy hidden in constructor. Otherwise pure + Drop zeroize (46-50). |
| `src-tauri/src/error.rs` | Error enum + serde | none | **yes** | Pure types + Serialize (error.rs:19-23). |
| `src-tauri/src/otpauth.rs` | otpauth:// URI parser | rng | no | Pure string parse but mints id via `Uuid::new_v4()` (otpauth.rs:81). |
| `src-tauri/src/migration.rs` | Google Auth migration protobuf parser | rng | no | Pure protobuf/base64 decode but `Uuid::new_v4()` per account (migration.rs:71). |
| `src-tauri/src/vault/kdf.rs` | Argon2id KDF | none | **yes** | `derive_key(password, salt, params)` takes salt as param (kdf.rs:32); doc requires caller-supplied random salt (26-28). Returns Zeroizing key. |
| `src-tauri/src/vault/crypto.rs` | AES-256-GCM + vault (de)serialization | none | **yes** | `encrypt(key, header, plaintext)` (crypto.rs:64), `decrypt_with_key(password, file_bytes)` (96) on byte slices; nonce from `header.nonce`; `parse_header` pure (42). |
| `src-tauri/src/vault/mod.rs` | Vault state machine + persistence | file, rng | no | `create()` calls `thread_rng().fill_bytes` for salt (mod.rs:42) + persist; `persist()` nonce (93) + `write_atomic` (97); `unlock()` `read_file` (51); add/remove → persist (81,87). |
| `src-tauri/src/storage.rs` | Filesystem edge (atomic r/w) | file | no (edge) | `fs::read` (10), File::create+write_all+sync_all+rename (16-21), path.exists (6). Intended edge. |
| `src-tauri/src/qr.rs` | QR decode + screen capture | file, screen | no | `decode_image_bytes(bytes)` pure (qr.rs:3); but `decode_image_file` `fs::read` (21) and `capture_region` `xcap::Monitor::all()`/`capture_image()` (27,31). |
| `src-tauri/src/backup.rs` | Encrypted backup export/import | file, rng | no | `export_encrypted` `fill_bytes` salt+nonce (13-14) + `write_atomic` (19); `import_encrypted` `read_file` (23). |
| `src-tauri/src/session.rs` | OS session-lock listener | os-events, ipc | no (edge) | WTS session notifications + Win32 message pump (135,149); `app.emit('locked')` (24). |
| `src-tauri/src/commands.rs` | Tauri IPC / composition root | clock, ipc, file | no (edge) | `now_unix()` is the ONLY `SystemTime::now()` (44-49), injected via `to_code_view(a, now)` (34-42, 85). |
| `src-tauri/src/main.rs` | App bootstrap / composition root | clock, file, ipc, os-events | no (edge) | `create_dir_all` (38); 1s tick thread emits 'tick' (56-59); starts session listener (62). |
| `src/lib/ipc.ts` | Frontend IPC wrapper | ipc | no (edge) | `invoke(...)` (5-23), `listen('tick'/'locked')` (25-26). |
| `src/lib/settings.ts` | Frontend settings persistence | file | no | `localStorage.getItem/setItem` (6,9). |
| `src/routes/Main.svelte` | Main view: codes, idle-lock, tick | ipc, clock, os-events | no | `onTick(refresh)` (30), IPC calls, `setTimeout` idle lock (24), mousemove/keydown (32-33). |
| `src/components/AccountCard.svelte` | Account card UI + clipboard | ipc, clock | no | `navigator.clipboard.writeText/readText` (11,18,19), `setTimeout` auto-clear (16). |

### Key findings
- **Clock fully injected** — textbook sans-IO. `totp::generate`/`remaining_seconds` take time as a parameter; the sole `SystemTime::now()` is `now_unix()` at the IPC edge (commands.rs:44-49).
- **Crypto core pure** — `encrypt`/`decrypt_with_key` operate on byte slices (nonce via `header.nonce`, salt parsed from header); `kdf::derive_key` takes salt as a parameter with an explicit caller-supplies-random-salt contract.
- **Entropy is the main leak** — `thread_rng().fill_bytes` lives inside business modules (vault/mod.rs:42,93; backup.rs:13-14), and `Uuid::new_v4()` hides OS entropy in constructors/parsers (model.rs:34, otpauth.rs:81, migration.rs:71).
- **Vault state machine couples persistence with state logic** — interleaves `read_file`/`write_atomic` with state transitions; create/add/remove call `persist()` inline.
- **I/O edges correctly isolated** — storage.rs (fs), qr.rs capture (screen via xcap), session.rs (Win32 WTS + emit), commands.rs/main.rs (Tauri ipc/clock/bootstrap). `qr::decode_image_bytes` is kept pure and independently callable.

### Deviations
- vault/mod.rs:42 — `fill_bytes(&mut salt)` inside `Vault::create` (entropy in logic module).
- vault/mod.rs:93 — `fill_bytes(&mut nonce)` inside `persist()`.
- vault/mod.rs:51,97 — `read_file`/`write_atomic` called directly from the state machine.
- backup.rs:13-14 — `fill_bytes` salt+nonce inside `export_encrypted`.
- backup.rs:19,23 — `write_atomic`/`read_file` inside export/import logic.
- model.rs:34, otpauth.rs:81, migration.rs:71 — `Uuid::new_v4()` inside otherwise-pure constructors/parsers.
- qr.rs:21,27-31 — `fs::read` + xcap screen capture mixed into the module holding the pure `decode_image_bytes`.
- commands.rs:44-49 — `SystemTime::now()` in `now_unix()` (correct edge placement; listed for completeness).

### Recommendations (sans-IO)
1. **Make the Vault state machine pure:** have create/add/remove return new in-memory state (or serialized bytes) and let `commands.rs` perform `write_atomic`; or inject a storage trait + RNG source into `Vault` (vault/mod.rs:42,51,93,97).
2. **Inject crypto randomness:** pass salt (create) and nonce (persist/export) as parameters — the same way `derive_key` already takes salt; `encrypt()` already accepts `header.nonce`, so push generation up to the edge.
3. **Make backup pure:** `export_encrypted` returns `Vec<u8>` given accounts+password+salt+nonce; `import_encrypted` takes file bytes; `commands.rs` does the read/write.
4. **Inject ID generation:** pass ids in (or accept `Fn() -> String`) instead of calling `Uuid::new_v4()` in `Account::new`/`parse_otpauth`/`parse_migration` → deterministic, testable.
5. **Split qr.rs:** keep pure `decode_image_bytes` in a logic module; move `decode_image_file` (fs) and `capture_region` (screen) to an io/edge module.
6. *(Polish)* Optionally accept a `Fn() -> u64` time source in commands for testability; the core is already fully time-injected.

---

## 6. Overall risk assessment & recommended actions

**Overall posture: LOW risk.** Provenance is strong (15/20 Rust + 10/10 npm direct deps from first-party stewards; all crates resolve from crates.io with zero git/alternate-registry sources). The full-tree `cargo audit` finds **0 Rust vulnerabilities** (only 19 informational unmaintained/unsound warnings, mostly Linux-GTK-only). The only runtime dependency with applicable advisories is **`svelte 4.2.20`** (Moderate XSS/DOM-clobbering family) — and **none of those attack paths are reachable** in this client-only SPA (no SSR, no `@html`, no attribute spreading; text auto-escaped). No malicious history was found for any audited package. The sans-IO architecture is sound for the security-critical core (clock + crypto). Residual risk is concentrated in dev-only tooling CVEs, a deferrable svelte-5 runtime upgrade, and latent single-maintainer/abandoned-package supply-chain exposure.

### Prioritized actions

**P1 — Dev-tooling CVEs (low real-world exposure, but Critical/High by score):**
- **Upgrade `vite` to ≥ 6.4.3** (or 7.3.5 / 8.0.16). vite 5.x is EOL; two dev-server bypasses (CVE-2026-53571 High, CVE-2026-39365 Medium) will never be backported. Not in the shipped artifact, but the dev server should not be left on a permanently-unpatched line. *(Verify the major-version bump against the Svelte/Tauri toolchain.)*
- **Upgrade `vitest` to ≥ 3.2.6** to clear CVE-2026-47429 (Critical). Real exposure is low (headless `vitest run`, no UI/API/browser mode), but the fix is straightforward.

**P2 — Runtime `svelte` advisories (deferrable; zero reachable surface today):**
- `svelte 4.2.20` carries a Moderate XSS/DOM-clobbering advisory family (`introduced 0`, fixed `5.55.7`). No vector is reachable in the current code (verified: no SSR/`@html`/`{...}`-spread; auto-escaped text). Remediation is the **breaking upgrade to Svelte 5** — schedule it as tech-debt, and in the meantime keep the invariant: never render untrusted strings via `@html` and never spread untrusted attributes onto form elements.

**P2 — Supply-chain hygiene:**
- Treat the single-maintainer npm micro-utilities (`debug`/`ms`, `minimist`, `nanoid`, lukeed's `kleur`/`sade`/`mri`, `picocolors`) as a watchlist; pin via the lockfile and enable provenance/signature verification in CI. All are currently patched.
- **Prune the stale isaacs cluster** (`rimraf 2` → deprecated `glob 7` → deprecated `inflight`) and the `es6-promise 3.3.1` polyfill — reliability + reduced attack surface.

**P3 — Non-official direct Rust deps (security-relevant surface):**
- **`xcap 0.0.15`** grants screen-capture access and pulls a sizeable individual-maintained FFI stack (incl. `dlopen2` runtime dynamic loading). No advisories, but pre-1.0 and the highest-privilege non-official surface — keep current (latest 0.9.6) and review whether screen capture is essential vs. file-based QR import only.
- **`rqrr` + `g2p`/`g2gen`/`g2poly`** and **`base32`** are low-risk individual-maintained crates with no advisories; monitor but no action required.
- Consider reviewing/feature-gating the **AVIF/image codec stack** (rav1e/ravif/moxcms/pxfm/zmij/fax) — large unsafe/SIMD surface pulled by `image 0.25` for formats a TOTP app likely doesn't need; disabling unused `image` features would shrink the attack surface materially.

**P4 — Architecture (sans-IO):**
- Apply the sans-IO recommendations (§5) — inject RNG/IDs and lift persistence to the edges in `vault/mod.rs` and `backup.rs`, and split `qr.rs`. These improve testability and determinism; not a security defect today.

**No P0 items.** No Rust vulnerabilities (full-tree `cargo audit` = 0); no *reachable* runtime CVE; no compromised package; no malicious maintainer history.

---

*Full-tree CVE coverage across all 555 crates / 167 packages is provided separately by the controller's `cargo-audit` and `npm-audit` runs; this report covers per-package provenance/malicious-history deep-dives, the applicable-CVE analysis for the deep-dived set, and the sans-IO source audit.*

---

## 7. Addendum — new dependency added in Plan 3 (2026-06-29)

### `qrcode` 0.14.1

| Field | Value |
|---|---|
| Crate | qrcode |
| Version | 0.14.1 (resolved; `version = "0.14"` in `Cargo.toml`) |
| Source | crates.io (`index.crates.io`) |
| License | MIT OR Apache-2.0 (from crate's `Cargo.toml` `license` field) |
| Author | kennytm (kennytm@gmail.com) |
| Repository | https://github.com/kennytm/qrcode-rust |
| Maintenance | passively-maintained (badge in crate metadata) |
| Added for | `qr::encode_png` — encode an otpauth or migration URI as a QR PNG; pure bytes-in/bytes-out (no I/O). |
| Feature flags | `default-features = false` — disables the `image`, `svg`, and `pic` rendering backends. Only the pure encoder core is compiled. This avoids a potential `image`-version conflict with the already-present `image = "0.25"` direct dep (the crate's own `image` optional dep also targets 0.25, but to keep the dep-graph clean and the compile surface minimal, the feature is left off and PNG rendering is performed via the already-present `image` crate in `qr.rs` directly). |
| **Transitive deps (default-features=false)** | **None** — `cargo tree -p qrcode` with these features returns only `qrcode v0.14.1` (no children). The image/svg/pic optional deps are the only transitive surface; all are excluded by `default-features=false`. |
| Official? | Individual-maintained (single author kennytm). ~1.5M downloads. No org steward. |
| Provenance | Resolves from crates.io; no git source or alternate registry. |

#### RustSec / CVE check (2026-06-29)

- `cargo audit` run against the full `Cargo.lock` (with `qrcode 0.14.1` now present): **0 vulnerabilities, 19 informational warnings** — identical to the baseline audit (§3 above). No advisory of any kind for `qrcode` was found.
- Manual cross-check: https://rustsec.org/packages/qrcode.html — no advisories listed for the `qrcode` crate as of 2026-06-29.
- No malicious-behavior or supply-chain-compromise history found for `qrcode` or its author (kennytm).

#### Sans-IO compliance

`qr::encode_png` is pure: takes `&str`, returns `Result<Vec<u8>>`. No filesystem, no rand, no clock. Consistent with the existing `decode_image_bytes` pure function in the same module. The only I/O-adjacent behavior is `ImageBuffer` and `DynamicImage::write_to` (in-memory cursor), both of which are pure byte-buffer operations. The `qrcode` crate itself is also pure (bit manipulation only).

#### Risk assessment

**Low.** Single-author, individual-maintained, but well-known (1.5M+ downloads), no advisories, MIT/Apache-2.0 dual-licensed, zero transitive deps under `default-features=false`. Scope is narrow (QR matrix encoding — no crypto, no I/O, no FFI). The `passively-maintained` badge is informational; the 0.14.x line is current and stable. No action required beyond the standard lockfile + `cargo audit` monitoring.
