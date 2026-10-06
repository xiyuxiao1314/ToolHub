# Tool icons — 2026-10-03

Tool list rows and the selected-tool detail now display an image instead of a colored abbreviation tile.

- On Windows, read the icon resource from the registered `.exe` or `.ico` without launching it. `javac.exe` can use the sibling `java.exe` icon from the same JDK when it has no own icon.
- Cargo, Rust, Git, Python and Node.js have bundled official brand images as fallback. Other tools use a neutral terminal symbol when no image is available.
- Native extraction runs on a blocking worker, independently of registry/daemon requests. The UI loads visible icons lazily and shares a bounded in-memory cache between list and detail.
- Transparent RGBA conversion handles both alpha icons and legacy masks. Image failures fall back gracefully. Bundled assets work offline; their sources and attribution are documented beside the assets.

Validation on Windows:

- TypeScript and Vite production build passed.
- Six desktop unit tests passed, including resource extraction, transparency conversion, and missing/unsupported file fallback.
- Desktop Rust build and Clippy with `-D warnings` passed, using the locked offline dependency set.
- Real Tauri/WebView validation used a consistent copy of the existing registry: all 16 entries displayed images without abbreviation text; list and detail navigation passed; no page errors were recorded.
- Native icons rendered for Java, Java compiler, both Node.js entries and installed Python. Cargo, Git and Rustc launchers without embedded resources used the bundled brand images. Missing paths fell back successfully.

The original registry and user preferences were preserved. Native extraction on non-Windows platforms returns no image, leaving the bundled/generic fallback available; those platforms were not tested.
