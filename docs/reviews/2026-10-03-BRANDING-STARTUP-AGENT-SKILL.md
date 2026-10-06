# A branding, login startup, and copyable Agent integration

Implemented and verified locally on Windows, 2026-10-03. Development branch: `codex/button-audit-20261001`; base commit `fba3320`. The prior sidebar and startup-timeout corrections remain in the working tree.

## Result

- Adopted selected concept A: a white interlocking TH monogram on a blue/violet rounded tile. Sidebar uses the 128px asset; Windows application resources use an ICO containing 16/20/24/32/40/48/64/128/256px frames. Replaced the old 70-byte placeholder. The application currently has no notification-area tray implementation; this replaces the executable/window/taskbar icon shown in the original screenshot.
- Settings now has a Windows login-startup switch independent of automatic scanning. It reads the current user's `Run` entry, quotes the current executable path, and changes only the `ToolHub` value on an explicit toggle. A previous application location can be updated. Loading/error states cannot falsely show a successful change. Configuration imports/resets do not change OS startup registration.
- Expanded Agent connection help has buttons to copy MCP JSON and a complete SKILL.md with the current CLI path, PowerShell checks, seven MCP meta-tools, permission-aware execution, user-selected program intake, and declarative Skill registration. The portable Skill is also saved at `skills/toolhub-integration/SKILL.md`. The application reports a missing adjacent CLI; it does not claim that detecting an adapter establishes a connection.
- Fixed an existing Skill registration failure discovered while exercising the guide: the CLI supplies the manifest file, but registry asset validation needs its containing directory. Missing declared assets still fail before overwriting an existing registration. No policy or confinement checks were relaxed.

## Verification

- `npm run build`: TypeScript and production frontend build passed.
- `cargo test -p toolhub-desktop --locked --offline`: three tests passed, including an isolated HKCU registry roundtrip, Unicode/spaced paths, and JSON/PowerShell quoting.
- Daemon regression `manifest_file_registers_sibling_assets_from_its_package_directory`: passed; registers sibling SKILL.md and preserves the existing record after a missing-asset failure.
- Desktop and daemon Clippy with `-D warnings`: passed. Desktop, CLI and daemon rebuilt offline.
- Browser fixture: successful enable, failed disable preserving displayed state, retry/read, then successful disable passed.
- Actual native WebView: both copy buttons were pasted into a temporary text field and compared with the full expected payload; passed. The copied SKILL.md passed the skill-creator validator.
- Actual CLI/MCP in a private registry: initialize, seven tools, search, environment listing, Skill registration via skill.json, Skill search/inspection passed. The intentionally missing Python provider was reported as `missing_capabilities`. This validation also launched from Windows System32 to avoid relying on a project working directory.
- The real login-startup registry entry was unchanged by testing and is currently absent. No Windows reboot/login test was performed; enabling the actual startup setting remains the user's choice.

Evidence: `D:\codex\toolhub-brand-settings\output\native-results.json`, `mcp-results.json`, `startup-ui.json`, and native screenshots. Test registry/WebView profiles are separate from user data.

## Image generation record

Used the built-in ImageGen editor against approved A, preserving its monogram and gradient. Final outputs:

- `apps/desktop/icons/toolhub-a-master.png` — generated RGBA master, 1254px.
- `apps/desktop/icons/icon.png` — 512px PNG.
- `apps/desktop/icons/icon.ico` — Windows multi-size packaging.
- `apps/desktop/ui/src/assets/toolhub-a.png` — 128px sidebar asset.

Final prompt:

> Use case: background-extraction. Asset type: production Windows application icon and sidebar brand mark. Edit target: the LARGE rounded-square blue/violet TH icon on the LEFT of the attached approved A concept board. Extract and reproduce ONLY this single icon as a square high-resolution PNG with genuinely transparent space outside its rounded-square silhouette. Keep the exact approved monogram: white intertwined geometric T and H, T on the left, H on the right, identical central angled interlock/notch, bold proportions and rounded corners. Preserve the blue-to-violet gradient. The rounded-square tile should occupy about 94% of a square canvas, centered, equal minimal padding on all sides. Flat front view, crisp edges, no perspective, no shadow or glow. Do NOT include any ToolHub wordmark, A label, taskbar, board, previews, or additional symbols. The inside of the tile is fully opaque; only the exterior is transparent. This is an asset extraction, not a new logo design. Match the LARGE left icon, not the smaller wordmark icon.

PNG resizing and ICO conversion preserve the generated alpha. Only technical asset packaging used Pillow.
