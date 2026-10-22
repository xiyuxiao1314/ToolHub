# Photo gallery / bounded image assets — 2026-10-22

Goal: independent 3D photo-gallery extension with user-selected import, persistent display copies, individual removal and full-window viewing. Initial library limit 50; compare 50/100 rendering costs. Owner requested reference inspection, not copying its code/media or publishing to GitHub.

Bases: ToolHub 3203807dc01ff347358a96e5c8fdfa2972bafc58; ToolHub-Extensions bf209f42b2cc9bccf9be916d506649618781cef5. Branches codex/photo-gallery. Existing sandbox, storage/presentation SDK and installation digest contract remain.

Core scope: desktop UI extensionImages IndexedDB store, ExtensionFrame image bridge, SDK runtime/docs, named gallery icon, beta.9 version metadata and focused native acceptance. Extension scope: packages/photo-gallery/0.1.0, catalog/checksums, docs and UI/performance harness. Registry/model/THP/MCP, native file access, trust and execution policies unchanged.

Contract: images.list/get/add/remove operate only in the installed calling package namespace after token/source/current digest validation. JPEG display copies only, bounded dimensions/bytes, 50 entries / 32 MiB per package, 128 MiB aggregate; atomic revision checks, separate metadata/pixel stores. Browser file input is user-selected; no original paths, reads or filesystem deletion are exposed. IndexedDB remains in the main WebView profile, outside SQLite package snapshots and ordinary JSON settings; profile clearing removes display copies. Uninstall retains namespace for reinstall. No network or arbitrary asset types.

Acceptance: typecheck/build; metadata/revision/quota and cross-package native image checks; real File input import, duplicate/bad input, removal/cancel, restart persistence, full-window interaction and hidden/idle rendering; 50/100 synthetic benchmark with bounded resident pictures. Preserve prior prompt empty state, unrelated installed packages, registry rows and appearance on local activation. Native macOS remains unverified. Deliver exact local commits, first-version package, concise docs and private evidence; no GitHub push unless requested.
