# Bounded project dependency discovery — owner-approved local batch

Base: 504abf9; branch codex/project-dependency-discovery. Codex owns this bounded implementation; the owner's direct request supersedes historical dispatch/review gates for this scope. No delegation, main merge or publication.

Register the selected existing Electron using supported native flows, then discover Electron from explicitly selected project manifests, package metadata and binaries. Preserve real versions, canonical identities and many-project associations. Record missing package/binary separately; never execute or grant trust. Keep global node_modules exclusions. Support explicit distribution overrides without executing package JavaScript. Scope: core DTO, scanner, recognizer, registry migration, daemon/protocol/SDK, CLI, desktop tools page, docs and tests. Contract changes land together. Validate bounded reads, traversal/link confinement, overrides, missing/full/multiple versions, dedup, rescan and trust preservation. Build and activate paired Windows binaries with backups, preserve user records/configuration, verify actual MCP search/inspect. macOS runtime verification is unavailable.

Owner additionally requested recycling the incomplete Atria Electron 33.4.11 package; exact version/path verified and moved to Windows Recycle Bin. Project source, manifest and lock retained. Initial supported extension+custom scan registered 37.10.3 path as unknown; old native reader missed its version resource beyond 96 MiB. Replace temporary recognition with native rule and bounded Windows version resource reads.

Verified local delivery: [results and limitations](CODEX-PROJECT-DEPS-20261017-DELIVERY.md).
