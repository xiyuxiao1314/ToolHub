<div align="center">
  <img src="apps/desktop/icons/icon.png" width="96" alt="ToolHub logo" />
  <h1>ToolHub</h1>
  <p><strong>Find the tools already on your machine before your agent starts working.</strong></p>
  <p>A local tool hub for people and AI agents. Discover, resolve and reuse what's already on your machine.</p>
  <p>
    <a href="LICENSE"><img alt="MIT license" src="https://img.shields.io/badge/license-MIT-blue" /></a>
    <a href="https://github.com/xiyuxiao1314/ToolHub/releases"><img alt="Release" src="https://img.shields.io/github/v/release/xiyuxiao1314/ToolHub?include_prereleases" /></a>
    <img alt="Platforms" src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS-lightgrey" />
    <img alt="Beta status" src="https://img.shields.io/badge/status-beta-orange" />
  </p>
  <p><a href="README.md">简体中文</a> | <strong>English</strong></p>
  <p><a href="https://github.com/xiyuxiao1314/ToolHub/releases/tag/v0.3.0-beta.1">Download v0.3.0-beta.1</a> · <a href="#screenshots">Screenshots</a> · <a href="#connect-an-ai-agent">Agent setup</a> · <a href="#build-from-source">Build from source</a> · <a href="https://github.com/xiyuxiao1314/ToolHub/issues">Report an issue</a></p>
</div>

## Why ToolHub?

When you ask an agent to compress a video, process a PDF or run a project, it may look for FFmpeg again, install another Python runtime or leave a startup script buried in a project directory. As the number of tools and agents grows, the capabilities you already have become harder to find.

ToolHub organizes local tools into a searchable catalog: their paths, versions, environments, capabilities and availability. People use the desktop interface; agents query the same catalog through local MCP or the CLI. Applications you create can be added to the program shelf. Reusable procedures can be saved as Skills, and capability packages combine those procedures with their tool dependencies for sharing.

ToolHub reuses existing tools. Discovery does not require an AI service, and ToolHub does not automatically download FFmpeg, install language runtimes or clean up your computer. Agents still need their own instructions to use ToolHub; it cannot force every agent to change how it selects tools.

## Screenshots

These screenshots show the **v0.3.0-beta.1 Windows release** with an independent demonstration dataset. The desktop UI is currently in Simplified Chinese. Click an image to view it at full size.

### Tool catalog

Group installations by tool, browse categories, search and paginate, then inspect an instance's version, path, environment and trust status.

![ToolHub tool catalog with grouped installations, pagination and Git instance details](docs/images/tools.png)

### Program shelf

Keep file and command launch entries together with their working directories, favorites, background launch and folder actions. The project entries shown here are demonstration examples.

![ToolHub program shelf with file and command entries, favorites and working directories](docs/images/programs.png)

### Portable Skill library

Inspect reusable procedures, inputs and outputs, supported platforms, declared permissions and local tool dependencies before copying instructions to an agent.

![ToolHub Skill library showing a media workflow, portability declarations and dependency checks](docs/images/skills.png)

### Capability package market

Browse bundled packages, preview procedures and dependency results, then add them to your Skill library with confirmation or copy/export them for sharing.

![ToolHub capability market with categories, a media workflow and local dependency inspection](docs/images/market.png)

## Current features

| Module | Implemented features |
| --- | --- |
| Tools | Local discovery; search by name, path or task; capability resolution; version and environment details; availability checks; preferred path selection |
| Tool list | Group by tool or show individual installations; category filters; 6 / 12 / 24 items per page; saved page-size preferences |
| Utilities | Recognize media, document and automation tools such as FFmpeg, FFprobe, 7-Zip, ImageMagick, Pandoc, Tesseract and Poppler |
| Program shelf | File or command launch entries, working directories, arguments, favorites, background launch, folder opening and invalid-path indicators |
| Agent proposals | Submit application launch entries over MCP to a pending list; users select and review them before adding them |
| Agents | Host detection, configured connections and historical handshakes/calls; setup templates and MCP connection checks |
| Environments | Inspect tool environments, duplicate installations and runtimes; no automatic deletion or merging |
| Skill library | Import reusable procedures; check platform, host and local tool dependencies; search and inspect through MCP |
| Market | Eight bundled capability packages; preview instructions and dependencies; add with explicit confirmation; import/export for sharing |
| Tasks and approvals | Policy-controlled tool execution, pending approvals, background task status/cancellation and activity records |
| Settings | Configuration import/export and display options; Windows current-user login startup settings and diagnostics |

Different installation paths count as separate instances of the same tool. Categories such as CLI and Runtime overlap, so their counts cannot be added to obtain the total.

### An agent workflow

```text
The user requests a task
    ↓
The agent queries ToolHub for existing tools, reusable Skills and capabilities
    ↓
It checks actual instances, paths, dependencies and permissions
    ↓
It executes under policy and requests desktop approval when needed
    ↓
After creating an application, it proposes a launch entry
    ↓
The user selects, reviews and adds it to the program shelf
```

### Portable Skills and the market

A shared Skill describes a procedure that different agents can understand, such as compressing video with an existing FFmpeg installation. It declares inputs, outputs, platforms, tool dependencies and permissions. ToolHub does not scan and register every agent's private Skills, or treat a host-specific image generator, browser or plugin API as available to all agents.

The market is currently a local catalog shipped with the application. You can import, preview, add and share `.toolhub-skill.json` packages. Bundled packages cover media processing, image conversion/OCR, PDF text/OCR, document conversion, archive extraction and CSV checks. There is no online community index, account system, paid content or automatic dependency installation yet.

## Downloads and platform support

The first open-source preview is **v0.3.0-beta.1**. Download files from the Assets section of [GitHub Releases](https://github.com/xiyuxiao1314/ToolHub/releases/tag/v0.3.0-beta.1).

| Your system | Recommended download | Notes |
| --- | --- | --- |
| Windows 10 / 11, 64-bit | `ToolHub-v0.3.0-beta.1-windows-x64.zip` | Desktop, CLI, daemon and capability packages; native maintainer-host verification |
| Mac, Apple Silicon (M series) | `ToolHub-v0.3.0-beta.1-macos-arm64.dmg` | macOS 13+; preview support; ZIP also available |
| Mac, Intel | `ToolHub-v0.3.0-beta.1-macos-x64.dmg` | macOS 13+; preview support; ZIP also available |
| CLI / MCP only | `ToolHub-CLI-v0.3.0-beta.1-<platform>.zip` | CLI and paired daemon; no desktop approval interface |
| Checksums | `SHA256SUMS.txt` | Compare SHA-256 values to check download integrity |

Windows and macOS share code, data models and MCP interfaces, with the following platform differences:

| Capability | Windows | macOS |
| --- | --- | --- |
| Discovery, registration, capability resolution, CLI / MCP | Supported | Supported; verified by native CI builds and tests |
| Desktop lists, portable Skills, capability market | Supported | Preview support |
| Native file/directory selection and package saving | Supported | Native AppleScript dialogs |
| Background program launch | `.exe/.bat/.cmd/.lnk/.ps1` and CMD commands | Executable files, `.sh/.command` and sh commands; whole `.app` launch entries are not supported yet |
| Open a terminal in a tool directory | CMD | Terminal; macOS may request permission to control Terminal |
| Interactive program launch in a terminal | Supported | Not supported yet; use background launch or run it in a terminal yourself |
| Controlled tool execution / execution approvals (CLI, MCP) | Supported | Not supported yet; executable identity pinning is not implemented, with no fallback to ordinary path execution |
| Login startup and native EXE icon extraction | Supported | Not supported yet |
| Default project program scan scope | Local fixed/removable disks, excluding system and dependency directories | User home directory; project directories can be selected explicitly |

On macOS you can query tools, Skills and capability packages, and use the program shelf. `execute_tool` and execution approvals explicitly return unavailable; they do not bypass executable identity checks. If an agent calls a discovered tool directly, it must still follow its own host's execution permissions.

macOS packages are built on GitHub's native macOS runners, with build and test results recorded for the release. Full interactive acceptance on a physical Mac has not been completed. No Linux, Windows ARM64 or mobile packages are provided yet.

### Windows setup

1. Download the desktop ZIP and extract it to a location you intend to keep.
2. Open `toolhub-desktop.exe`. Keep `toolhubd.exe`, `toolhub.exe`, `skills` and the other bundled files together.
3. Open **工具 (Tools)** and select **扫描本机 (Scan this computer)**. Discovery is read-only; unknown programs are not automatically executed to check their versions.
4. To add a project launcher, use **程序 (Programs)** to select a file/working directory, or select candidates after a scan.

For a desktop shortcut and a versioned installation, run the following in the extracted directory using **PowerShell 7**:

```powershell
./install-local.ps1 -VerifyOnly
./install-local.ps1
```

The installer uses the current user's `%LOCALAPPDATA%\ToolHub\versions` directory and preserves existing user data. Enable login startup explicitly in Settings; the installer only migrates an existing ToolHub login task. It does not configure all agent hosts automatically. The optional `-UpdateCodexMcp` flag is for explicitly updating an existing Codex ToolHub MCP entry.

Windows requires Microsoft Edge WebView2 Runtime. It is usually already installed; if missing, use [Microsoft's official WebView2 download page](https://developer.microsoft.com/en-us/microsoft-edge/webview2/). Windows executables in this preview do not have publisher code signing and may trigger publisher prompts. Check the source and checksums.

### macOS setup

1. Choose the DMG for your chip, open it and drag `ToolHub.app` to Applications.
2. Open ToolHub. The CLI is at `/Applications/ToolHub.app/Contents/MacOS/toolhub`.
3. Use that absolute CLI path in your MCP configuration. After moving the app, update the host's path and reload the connection.

The application is ad-hoc signed, with **no Apple Developer signing or notarization**. If macOS blocks it, check the official release and SHA-256 values, then use the allow-open flow in System Settings → Privacy & Security. You do not need to disable system-wide security checks.

## Connect an AI agent

ToolHub provides **local stdio MCP** for clients with compatible MCP support. Setup templates are available for Codex, MiMo / OpenCode, Claude Code / Desktop, Cursor, Gemini CLI, Copilot, VS Code and other hosts. Templates and detection indicate configuration support, not proof of a successful call from a particular host. Verify the connection and actual tool calls.

Open **智能体 → 接入说明 (Agents → Integration instructions)** in the desktop app and copy the current installation's connection information and setup instructions to your agent. The instructions ask it to preserve other host settings, use the correct CLI path, and check the handshake and tool list.

### Common JSON configuration

Merge the following into your host's configuration, replacing the path. Configuration file locations and enablement differ by client; follow that client's documentation.

```json
{
  "mcpServers": {
    "toolhub": {
      "command": "C:\\Tools\\ToolHub\\toolhub.exe",
      "args": ["mcp", "serve"]
    }
  }
}
```

On macOS, set `command` to `/Applications/ToolHub.app/Contents/MacOS/toolhub`. Keep the CLI paired with its daemon. Desktop users should configure the CLI from their desktop bundle rather than simultaneously using a CLI-only bundle in a different directory for the same user service: the service checks the paired daemon's path and identity. Read-only agent queries do not require the desktop window to stay open; interactive approvals require the desktop.

### Codex TOML

```toml
[mcp_servers.toolhub]
command = 'C:\Tools\ToolHub\toolhub.exe'
args = ["mcp", "serve"]
```

### MiMo / OpenCode JSONC

These hosts use a local `command` array rather than separate `command` and `args` fields:

```json
{
  "mcp": {
    "toolhub": {
      "type": "local",
      "command": ["C:\\Tools\\ToolHub\\toolhub.exe", "mcp", "serve"],
      "enabled": true
    }
  }
}
```

VS Code uses a `servers` structure; its template is available in the desktop integration instructions. Reload the MCP connection or start a new session after configuring it.

### Integration Skill

[toolhub-integration/SKILL.md](skills/toolhub-integration/SKILL.md) contains general integration instructions. Hosts with Skill support can install it in their own Skill directory; other hosts can use it as task instructions. Fill in your local installation path and ask the agent to search for tools or resolve capabilities, inspect actual instances, and use them under its permissions. After creating an application, it should submit a pending launch entry.

The gateway exposes 12 fixed meta-tools rather than a separate MCP tool for every installed application:

| Purpose | MCP tools |
| --- | --- |
| Discovery and inspection | `search_tools`, `resolve_capability`, `inspect_tool`, `list_environments` |
| Execution and approval | `execute_tool`, `request_execution_approval` |
| Background tasks | `get_task`, `cancel_task` |
| Portable Skills | `search_skills`, `inspect_skill` |
| Project applications | `propose_program`, `search_programs` |

Host names and historical observations are for display and do not grant execution permissions. `propose_program` only submits a declaration; it does not add, launch or trust a program. Sharing program information with an agent does not authorize its execution.

## CLI examples

Windows PowerShell:

```powershell
$toolhubPath = 'C:\Tools\ToolHub\toolhub.exe'
& $toolhubPath --version
& $toolhubPath --json status
& $toolhubPath --json scan --mode quick
& $toolhubPath --json search ffmpeg
& $toolhubPath --json resolve media.video.transcode
& $toolhubPath --json env list
& $toolhubPath --json skill list
& $toolhubPath doctor
```

macOS Terminal:

```sh
toolhub_bin='/Applications/ToolHub.app/Contents/MacOS/toolhub'
"$toolhub_bin" --json status
"$toolhub_bin" --json search ffmpeg
"$toolhub_bin" --json resolve media.video.transcode
"$toolhub_bin" mcp serve
```

Instance IDs come from query results. Execution is subject to policy, the working directory, instance integrity and any required user approval. `--json` produces agent-friendly output. See `toolhub --help` and each subcommand's `--help` for the full argument list.

## Local data and security boundaries

- Registry data, program entries and Skill metadata are stored by default in `~/.toolhub/registry.sqlite`; capability package caches also live under this user data directory. Removing the application does not delete your user data.
- Scanning does not automatically install or delete software, modify PATH or elevate privileges. Unknown executables are not automatically run for version identification.
- File selection or candidate selection comes before editing and adding an entry. Scanning uses project markers and launcher structure; **files alone cannot prove an AI agent created them**.
- Tool execution uses local connection authentication, instance checks, policy and one-time approval bound to the specific invocation. Discovering a tool, importing a Skill or declaring permissions does not authorize execution.
- Child processes receive a cleaned environment. ToolHub's permission boundaries cover its own interfaces; they do not replace the agent host's shell permissions or the operating system's sandbox.
- Skills and external capability packages are untrusted instructions. Importing them does not execute installation hooks. Portability checks use declarations and known host references; they cannot prove arbitrary text safe or fully portable.
- Data is primarily local. Optional AI-assisted flows and user-initiated exports have separate boundaries. Review exported content before sharing it.

See [SECURITY.md](SECURITY.md) for implementation constraints. Do not upload your registry database, agent configuration, API keys or private logs as source code.

## Build from source

Stack: Rust / Tokio / SQLite core, Tauri 2 desktop, React / TypeScript frontend, a separate stdio MCP gateway and a TypeScript SDK.

Requirements: Rust 1.90+ (current stable recommended), Node.js 22+ and native development tools for your platform. On Windows, install Visual Studio C++ Build Tools and WebView2. On macOS, install Xcode Command Line Tools. See the [official Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

```sh
git clone https://github.com/xiyuxiao1314/ToolHub.git
cd ToolHub
npm ci --prefix apps/desktop/ui
npm run build --prefix apps/desktop/ui
cargo build --release -p toolhub-cli -p toolhub-daemon -p toolhub-desktop --locked
```

Windows binaries are `target/release/toolhub-desktop.exe`, `toolhubd.exe` and `toolhub.exe`. macOS binaries have the same names without `.exe`.

Verification and packaging:

```sh
cargo test --release --workspace --locked -- --test-threads=1
node --test packages/sdk-typescript/test/client.test.js
python scripts/package-release.py --platform windows-x64
# On a Mac of the corresponding architecture: --platform macos-arm64 or --platform macos-x64
```

The packaging script requires Python 3.11+, uses previously built native binaries, and writes to `dist/release/<platform>` without changing user configuration. Start with an empty output directory each time. macOS packages must be built on macOS. GitHub Actions builds and tests all three native platforms and uploads artifacts; see the [release workflow](.github/workflows/release.yml).

### Repository layout

```text
apps/daemon/             Local service, policy execution, tasks and program entries
apps/cli/                CLI and MCP stdio entry point
apps/desktop/            Tauri desktop and React UI
crates/                  Domain models, scanning, recognition, registry, resolution, IPC, MCP, Skills
packages/sdk-typescript/ TypeScript client
skills/                  Integration Skill, portable Skill examples and market packages
schemas/                 JSON Schemas for exchange formats
scripts/                 Local installation, verification and native release packaging
docs/                    Design, implementation notes and historical development/verification records
```

The [system design](docs/design/TOOLHUB_SYSTEM_DESIGN.md) describes the full product direction, including goals not yet implemented. This README and the [release notes](docs/releases/v0.3.0-beta.1.md) describe the current feature set. Linked design and release documents are currently in Chinese or mixed languages.

## Known limitations and next steps

This is a beta preview. Compatibility across real agent hosts and macOS desktop interactions needs more feedback. There is no automatic updater, online capability package index or publisher-signed installer, and missing dependencies are not automatically installed. Host detection reads known user configuration locations; project overrides, remote MCP and custom paths may require manual verification.

Next priorities include macOS interactive acceptance and feature parity, release signing, clearer tool capability/dependency diagnostics, and capability package version/source management. Contributions of tool recognition rules, portable Skills and real host compatibility results are welcome.

## Contributing

Open an [Issue](https://github.com/xiyuxiao1314/ToolHub/issues) or Pull Request. For bug reports, include the version, OS/architecture, reproduction steps, expected behavior and actual behavior. Remove account details, tokens, private paths and sensitive output first. Keep domain models centralized, discovery read-only and approval boundaries intact, and add tests relevant to changed behavior.

## License

ToolHub source code is licensed under the [MIT License](LICENSE). Third-party dependencies retain their respective licenses. ToolHub packages do not bundle discovered software such as FFmpeg, Python or Node.js, or grant redistribution rights for it. See [NOTICE](NOTICE) for brand assets and third-party dependency information.
