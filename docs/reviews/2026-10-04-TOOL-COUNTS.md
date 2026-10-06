# Tool counters and completion-file recognition repair

Date: 2026-10-04. Branch: `codex/button-audit-20261001`.
Base/head commit: `fba33201197c5f892747e06713c750437172932c`; repairs are local, uncommitted changes on top of the existing desktop work.

## Findings

The screenshot showed 39 visible installations, 38 tagged CLI, and 44 registered records. Category chips are overlapping tags, not disjoint partitions: Java can belong to Runtime, CLI and SDK. Different installed copies count separately; the registry contained 11 FFmpeg and eight 7-Zip installations. The initial five missing paths were hidden by default but included in the sidebar total.

One genuine recognition error explained the extra visible non-CLI record: the no-extension `yt-dlp` file inside a UV package cache was Bash completion metadata, not an executable. Recognition used its basename. The file was inspected as text only and was never executed.

## Changes

- The sidebar now shows the global available count, registered installation count, unavailable count and number of distinct available tool definitions. The bar reflects available / registered records instead of an arbitrary multiplier. It remains pinned at the bottom.
- The list explains overlapping tags and separate installation paths; it shows the current filtered installation and definition counts, including zero-result searches. Search and category selection do not change the global sidebar count. Enabling hidden tools includes retired records in the list, with an unavailable warning on their cards.
- Portable discovery excludes UV caches and completion directories, and uses the existing platform executable convention. Unix discovery fixtures carry executable permissions but are never run.
- Built-in and resource recognition reject known shell-completion locations. An uncancelled scan revalidates only such previously available records through existing registry ingestion. Their historical record becomes missing/unavailable; no application, completion file or user launch entry is deleted.
- Existing UI category inference is retained. This repair does not change the shared domain, protocol, permissions, schema or execution policy.

Changed implementation files: `apps/desktop/ui/src/App.tsx`, `apps/desktop/ui/src/styles.css`, `crates/scanner/src/utilities.rs`, `crates/recognizer/src/lib.rs`, `apps/daemon/src/service.rs`.

## Verification

Windows, PowerShell 7, existing Node runtime and MSVC toolchain; locked offline Rust dependencies.

- `npm --prefix apps/desktop/ui run build`: TypeScript and Vite passed.
- `cargo test -p toolhub-recognizer -p toolhub-scanner -p toolhub-daemon -p toolhub-desktop --locked --offline -j 4`: 55 tests passed (recognizer 10, scanner 9, daemon 27 including integration/managed/soak, desktop 9). Recognition fixtures cover both built-in/resource rules; registry regression verifies cancellation, retirement, preservation of an unrelated instance and the original file.
- A Clippy warning in the new test's default initialization was repaired. The affected regression was rerun successfully with `cargo test -p toolhub-daemon --bin toolhubd scan_retires_completion_data --locked --offline -j 4`.
- `cargo build -p toolhub-daemon -p toolhub-desktop --locked --offline -j 4`: passed.
- `cargo clippy -p toolhub-recognizer -p toolhub-scanner -p toolhub-daemon -p toolhub-desktop --all-targets --locked --offline -j 4 -- -D warnings`: passed after the test correction.
- Real Tauri WebView QA on an isolated copied registry: all 38 / CLI 38; SDK filter two; 11 FFmpeg installations / one definition; empty search; hidden toggle (44 records / 14 historical definitions); retired completion warning; stable sidebar while scrolling; authenticated program refresh; no JavaScript errors.
- Production retirement used normal `scan.start` ingestion with an empty custom scope, avoiding an unnecessary disk crawl. After restart, the automatic scan also completed. Real data is 44 registered, 38 available, six unavailable, 13 available definitions. The three existing `program_entries` remained byte-for-byte identical to the pre-update backup, including their update timestamps. The completion file and the already running tdl application were preserved.
- The desktop and daemon were refreshed together after the final build. Actual production Tools and Programs pages were inspected: counts match the registry, all three launch entries load, and controller authentication succeeds. The app was left open on Tools, PID 54088, launched through the existing enabled login task.
- `git -c core.safecrlf=false diff --check`: passed. No commit, push or release was made in this repair.

Local evidence: `D:\codex\toolhub-counts-20261004\` contains source backups, the production registry backup, original/final inventories, regression/build scripts, `native-ui-result.json`, `tools-after.png`, `production-data-result.json` and `production-launch.json`.

This count audit does not prove successful execution of every registered binary. Windows was verified; macOS/Linux runtime behavior was not tested in this repair.
