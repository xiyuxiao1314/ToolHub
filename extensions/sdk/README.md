# Frontend extension SDK

The desktop injects `toolhub.call`, `toolhub.notify` and `toolhub.navigate` inside an isolated extension document. `runtime.js` is host-managed; its token placeholder is replaced only by the document server. Do not import this file into the privileged app, modify it from a package, or treat its content as execution authority.

Copy `../examples/workspace.toolhub-extension.json`, change the package ID and every view ID, edit HTML/CSS/JavaScript, declare only necessary permissions, then preview/import in ToolHub. Use `../examples/graphite.toolhub-extension.json` for a theme. No npm dependency installation or host rebuild is required for either.

See [contracts, lifecycle and bridge methods](../../docs/development/EXTENSIONS.md).


## Native CLI forms (beta.4)

A package may declare `cli` alongside optional views/theme/skills. The host renders a navigation page and typed file/choice controls, then previews and authorizes one exact execution. This is **not a JavaScript SDK execution method**: frame code cannot call extension.cli_preview/start or arbitrary execute.tool. See `extensions/examples/ffmpeg.toolhub-extension.json` and the CLI section of `docs/development/EXTENSIONS.md`. Agents discover the included portable Skill through existing generic MCP tools.
