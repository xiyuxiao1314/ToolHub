# Agent-ready delivery — 2026-10-04

Owner approved the six directions below. Base: fba33201197c5f892747e06713c750437172932c, branch codex/button-audit-20261001. Preserve the existing uncommitted fixes and all production registry/program data.

| Phase | Deliverable | Observable acceptance | State |
|---|---|---|---|
| 1 | Durable Agent discovery, integration diagnostics, versioned Windows package/install | Codex visible without terminal PATH; configured vs handshake vs successful call distinct; restart retains history; own binaries packaged with hashes; startup uses stable installation | Verified on Windows |
| 2 | Grouped installations, preferred path, health | Multiple copies expandable; missing copies retained; default only selects eligible instances; path checks never imply execution permission | Verified on Windows |
| 3 | Task/capability search, compact MCP responses | Chinese video/audio/PDF/OCR/archive tasks find providers; structured results preserve legacy text; bounded pagination | Verified on Windows |
| 4 | Rich Agent application proposals | Purpose, inputs, outputs, dependencies, examples survive proposal/select/save; explicit user confirmation; registration never grants execution | Verified on Windows |
| 5 | Managed task status/cancel/artifacts | Async tool execution retains existing policy/approval; owner-bound polling/cancel; bounded results; existing output files locatable; restart interrupts unfinished jobs | Verified on Windows |
| 6 | Reusable Skill library | Native file selection/import, requirement resolution, instruction preview/copy; safe packaged examples discoverable over MCP; no install hooks | Verified on Windows |

## Shared contract decisions

- Central THP methods, validation, MCP schemas, TypeScript SDK and docs change together. New metadata never grants trust.
- Agent host names from MCP clientInfo are self-reported observation labels, not authenticated identities. Show historical handshake/call timestamps, never infer current connectivity from a config file.
- Preserve installation records and the three user program entries. Grouping is presentation only.
- Long jobs use compatible ToolHub meta-tools; no claim that experimental MCP Tasks are implemented.
- Windows local delivery is unsigned; reboot and non-Windows checks must be reported separately if unverified.
- Verification: offline Rust tests/Clippy, frontend build, private native WebView flows, real stdio MCP, production read-only data comparison. No GitHub push/public release in this scope.

## Delivery evidence

Implementation and local verification completed on 2026-10-04. See docs/reviews/2026-10-04-AGENT-READY.md for exact package hashes, tests, preservation checks and limits. This is local implementation verification, not a formal B01/B02 external product-review round. Original review budget is unchanged.
