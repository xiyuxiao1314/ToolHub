# R4 residual completion (owner-authorized)

**Date:** 2026-09-30  
**Branch:** `codex/toolhub-r4-final`  
**Authority:** Owner authorized completing residual R4 items (except macOS), updating the review ledger, and later merge/workbench actions after an independent subagent code review with no blocking defects.

## Scope completed

| Residual item | Outcome |
| --- | --- |
| Long soak fully observed | **Done.** `cargo test -p toolhub-daemon --test soak --locked -- --test-threads=1` → **5/5 pass** in 77s on Windows (rustc 1.98.1, MSVC 14.50). Evidence: [`evidence/nine-page/soak-result.txt`](evidence/nine-page/soak-result.txt). |
| Nine-page UI operation evidence | **Done.** Live `toolhubd` + production UI dist + Tauri invoke shim + Playwright. Screenshots + operation log under [`evidence/nine-page/`](evidence/nine-page/). |
| macOS host verification | **Out of scope** by owner direction. Not claimed. |
| Review ledger update | **Done.** [`REVIEW_LEDGER.md`](REVIEW_LEDGER.md) now records R4 at 4/4 and this residual window. |

## Soak design note

High-volume soak loops use a bounded `custom` scan root fixture so wall-clock hang detection stays meaningful on hosts where a real PATH `quick` scan takes tens of seconds (observed ~40s, ~1100–1300 candidates on this host). One dedicated test still runs a real `quick` scan end-to-end (`soak_one_real_quick_scan_still_works`).

## UI evidence method (honest)

- Production `apps/desktop/ui` build (`tsc && vite build`) served with a **browser Tauri invoke shim** that forwards `rpc`/`app_versions` to a live stdio `toolhubd`.
- This is **not** a native Tauri window capture; it is the same React UI and real daemon data over the reviewed RPC surface.
- Scripts: `scripts/ui-evidence/server.mjs`, `scripts/ui-evidence/capture.mjs`.
- Log: `docs/reviews/evidence/nine-page/nine-page-log.json` (scan click, post-scan status, per-page text snippets).

## Product fixes found while completing residuals

| Issue | Fix |
| --- | --- |
| Page switch rendered stale non-array onto Tools `map` → blank UI | `switchPage` clears data; `listFrom` guards list rendering |
| Re-clicking the active nav set `loading` without calling `load` → stuck “加载中” | Same-page switch re-invokes `load` |
| Trust column dumped raw `trust_json` | `trustLabel` parses to `level` |
| Clippy 1.98 `chunks_exact_to_as_chunks` | `as_chunks::<2>()` in `native_metadata.rs` |
| Soak custom-root params broke on Windows `\` paths | JSON-escape roots in soak fixture helper |

## Verification (this window)

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | pass (rustc/clippy 1.98.1) |
| `cargo test -p toolhub-daemon --test soak --locked` | **5/5 pass** |
| `npm run build` (`apps/desktop/ui`) | pass |
| Nine-page capture | 9/9 screenshots + log with live counts (tool_count 12, candidate_count 1120 after quick scan) |

## Still unverified / open

1. **macOS** — not performed (owner exclusion).
2. **Native Tauri window screenshots** — evidence uses browser + invoke shim, not `toolhub-desktop.exe` HWND captures.
3. **Formal Codex acceptance of this residual** — not a new formal review round; R4 remains changes_required with residuals now documented closed/verified where evidenced.
4. **Evidence server** (`scripts/ui-evidence/server.mjs`) is a local unauthenticated RPC proxy for capture only — do not leave running on shared machines.
5. **Independent pre-merge review:** subagent static review returned **GO** (no blocking defects; safety invariants pass). High-priority non-blocking item (UI concurrent-load race) was fixed with a monotonic `loadSeq` guard before merge.

## Independent pre-merge review record

- Reviewer: MiMo subagent (static read-only; no formal Codex round).
- Verdict: **GO** for merge + workbench after residual completion.
- Safety: no secret capture / no unknown-tool auto-exec / no policy weakening / soak does not install software — **PASS**.

## Revisions

- Implementation work continues on `codex/toolhub-r4-final`.
- Docs authority remains `main` until owner-authorized merge after review.
