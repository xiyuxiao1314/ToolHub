# Frontend extension SDK

The desktop injects `toolhub.call`, `toolhub.notify` and `toolhub.navigate` inside an isolated extension document. `runtime.js` is host-managed; its token placeholder is replaced only by the document server. Do not import this file into the privileged app, modify it from a package, or treat its content as execution authority.

Copy `../examples/workspace.toolhub-extension.json`, change the package ID and every view ID, edit HTML/CSS/JavaScript, declare only necessary permissions, then preview/import in ToolHub. Use `../examples/graphite.toolhub-extension.json` for a theme. No npm dependency installation or host rebuild is required for either.

See [contracts, lifecycle and bridge methods](../../docs/development/EXTENSIONS.md).


## Native CLI forms (beta.4)

A package may declare `cli` alongside optional views/theme/skills. The host renders a navigation page and typed file/choice controls, then previews and authorizes one exact execution. This is **not a JavaScript SDK execution method**: frame code cannot call extension.cli_preview/start or arbitrary execute.tool. See `extensions/examples/ffmpeg.toolhub-extension.json` and the CLI section of `docs/development/EXTENSIONS.md`. Agents discover the included portable Skill through existing generic MCP tools.

## Full-window presentation (beta.7)

`await toolhub.presentation.set(true)` expands the current isolated view over the workbench; `set(false)` returns. This is a display-only request, not browser/OS fullscreen or native-window authority. The host retains its return bar, verifies source/token/current enabled package and digest on entry, and restores background interaction on exit/unmount. Existing iframe and its display state remain mounted. A host-managed `toolhub:presentation` event exposes `{expanded:boolean}` so the page can hide its editor chrome and restore it when the host return button is used. Treat the event as display state, never authorization for execution. Require min_toolhub_version >= 0.3.0-beta.7 and detect `toolhub.presentation` when supporting older hosts. Sandbox and CSP remain unchanged.

```js
previewButton.onclick = () => toolhub.presentation.set(true);
window.addEventListener('toolhub:presentation', event => {
  document.body.classList.toggle('previewing', event.detail.expanded);
});
// A child document must handle Escape itself; DOM key events do not cross frames.
```


## Editable presets and document export (beta.8)

`toolhub.storage` retains its package namespace/revision semantics and now permits 1 MiB UTF-8 JSON (previously 32 KiB). Clearing the WebView profile still removes this state; export important data. `await toolhub.files.exportText(name, text)` opens a native Save dialog and returns true after a successful write, false on cancel. Filenames must be plain .md/.json/.txt names (no paths, controls or alternate streams); content is limited to 2 MiB UTF-8. The iframe cannot select a destination or read its path. Source/token/current enabled package and digest are checked; native code rechecks after the dialog. Only one export request per mounted frame may be pending. The SDK waits up to five minutes for this user dialog. Sandbox/CSP and execution/network permissions stay unchanged. Require min_toolhub_version >= 0.3.0-beta.8 and feature-detect toolhub.files. Import may use a user-selected browser File input; never execute imported content.


## Navigation icons (beta.8)

Page/CLI `icon` strings may use `prompt`, `media` and `design` for host-rendered line icons. Secondary navigation renders them in the same 20×20 CSS pixel box and stroke style as kernel destinations. Existing ❞/▶/◧ declarations remain compatible through glyph aliases. Unknown glyphs retain a square centered fallback; requested icon size is respected in application cards and pin controls as well. This is presentation-only and grants no permissions.
