# ToolHub 完整系统设计

## 0. 一句话定义

**ToolHub 是面向 AI Agent 与用户的本机工具基础设施层。**

它统一发现、识别、描述和调用 Windows/macOS 电脑中已经存在的：

- CLI
- GUI 软件
- 编程语言 Runtime
- Compiler / Linker
- SDK
- Debugger
- Decompiler
- Package Manager
- Container Runtime
- Database
- AI Agent
- AI Runtime
- MCP Server
- Local API
- 开发工具
- 媒体工具
- 自动化工具
- 其他能够帮助 AI 完成任务的软件

ToolHub **不负责安装这些软件**，而负责告诉用户和 AI：

> 这台电脑已经有什么工具、它们在哪里、属于哪个环境、提供什么能力、怎么调用、是否可信，以及哪个 Agent 创建了重复环境。

---

# 1. ToolHub 的核心原则

整个项目从第一天开始遵循十条原则。

### 1. 本机优先

Agent 在创建沙箱、下载依赖、重新安装 Runtime 之前，应优先查询 ToolHub：

```text
用户需求
   ↓
查询 ToolHub
   ↓
本机已有？
 ┌─YES──────────────┐
 │                  ↓
 │          使用本机工具
 │
 └─NO
    ↓
Agent 自己决定
    ↓
Sandbox / 临时环境 / 自行安装
```

ToolHub 不阻止 Agent 使用沙箱。

---

### 2. ToolHub 是 Preferred Runtime，不是 Mandatory Runtime

默认：

```text
Agent
  ↓
ToolHub
  ↓
本机工具
```

但出现：

- ToolHub 故障
- 工具没有被收录
- Agent 已经有合适沙箱
- 用户明确要求隔离
- 工作属于一次性任务

时允许：

```text
Agent → Direct Execution

或者

Agent → Sandbox
```

---

### 3. 不负责安装

ToolHub 可以：

```text
发现
识别
注册
描述
查询
调用
判断缺失
```

不能：

```text
自动安装
自动卸载
自动升级
自动清理环境
自动修改 PATH
```

Skill 缺少依赖时只报告：

```text
Skill requires:

✓ Java >= 17
✓ adb
✕ android.apk.decompile
```

而不是自动安装 JADX。

---

### 4. 扫描必须 AI 可选

没有任何 API Key 时：

```text
Native Scanner
```

必须已经能完整工作。

AI 的职责是：

```text
识别未知软件
理解复杂工具
补充 Capability
分类
读取文档
判断 CLI 用法
```

而不是承担整个扫描流程。

---

### 5. 扫描默认只读

Discovery Mode 禁止：

```text
install
uninstall
delete
write config
modify registry
modify PATH
elevation
```

允许：

```text
读取目录
读取 metadata
读取 Registry
读取 package metadata
读取 bundle metadata
计算 hash
读取签名
已知工具 --version
已知工具 --help
```

---

### 6. Tool 和 Capability 分离

Skill 不应该说：

```text
我需要 FFmpeg
```

而应该尽量说：

```text
我需要 video.transcode
```

ToolHub 再解析：

```text
video.transcode

→ FFmpeg
→ HandBrakeCLI
→ Blender
→ 其他实现
```

---

### 7. 一个工具可以存在多个 Instance

例如：

```text
Python
├── CPython 3.13 / System
├── CPython 3.12 / Homebrew
├── Python 3.11 / Conda
├── Python 3.12 / Cursor Sandbox
└── Python 3.13 / Other Agent
```

ToolHub 不应该擅自合并或删除。

---

### 8. 用户与 AI 看到的是同一 Registry

Desktop UI、CLI、MCP、Agent Bridge 都读取同一份数据。

不存在：

```text
UI 一套数据
MCP 一套数据
CLI 一套数据
```

---

### 9. AI Key 默认不落盘

如果用户必须使用 API Key：

```text
输入
 ↓
进程内存
 ↓
AI Discovery Session
 ↓
结束
 ↓
销毁
```

长期 Credential Store 不作为 ToolHub 的核心功能。

---

### 10. 开放标准与桌面产品分开

ToolHub Desktop 是产品。

ToolHub Protocol / Manifest / Capability Schema 则可以逐渐发展成开放规范。

---

# 2. 总体架构

完整结构：

```text
                     ┌──────────────────────┐
                     │   ToolHub Desktop    │
                     │  Tauri + Web UI      │
                     └──────────┬───────────┘
                                │
                         Local IPC Protocol
                                │
                    ┌───────────▼───────────┐
                    │    ToolHub Daemon     │
                    │        Rust           │
                    └───────────┬───────────┘
                                │
       ┌────────────────────────┼─────────────────────────┐
       │                        │                         │
       ▼                        ▼                         ▼
 Scanner Engine           Tool Registry            Policy Engine
       │                        │                         │
       │                        ▼                         │
       │                  Capability DB                   │
       │                        │                         │
       └────────────────────────┼─────────────────────────┘
                                │
                          Resolver Engine
                                │
                          Execution Engine
                                │
            ┌───────────────────┼──────────────────────┐
            │                   │                      │
            ▼                   ▼                      ▼
           CLI                 MCP               Agent Bridge
            │                   │                      │
            └───────────────────┼──────────────────────┘
                                │
                       External AI Agents
                       Codex / OpenCode
                       Claude / others
```

还有独立的：

```text
Skill Engine
Environment Analyzer
Ownership Analyzer
Plugin System
Audit System
Updater
Protocol Layer
```

---

# 3. 项目拆成 14 个核心系统

整个工程建议拆成 **14 个正式子系统**。

不是版本，而是完整 ToolHub 的组成部分。

---

# 3.1 Core Daemon

这是整个项目真正的核心。

桌面 UI 关闭以后，Agent 仍然应该能够：

```bash
toolhub search python
```

因此不能把业务逻辑全部放在桌面应用里。

应该存在用户级后台进程：

```text
toolhubd
```

职责：

```text
Registry
Scanner
Resolver
Executor
Policy
Skills
Agent sessions
IPC
Events
Logging
```

Desktop 本质上只是：

```text
ToolHub Daemon Client
```

CLI 同样是：

```text
ToolHub Daemon Client
```

---

# 3.2 Scanner Engine

Scanner 是最大的系统之一。

拆成：

```text
Scanner Engine
├── Discovery Scanner
├── Runtime Scanner
├── Application Scanner
├── Package Scanner
├── Environment Scanner
├── Agent Environment Scanner
├── MCP Scanner
└── AI Assisted Scanner
```

## Windows Scanner

检查：

```text
PATH
Registry
App Paths
Program Files
Program Files (x86)
AppData
WindowsApps
PowerShell commands
winget metadata
Chocolatey
Scoop
Visual Studio
Windows SDK
WSL
Docker
AI application directories
```

## macOS Scanner

检查：

```text
PATH
/Applications
~/Applications
/usr/bin
/usr/local/bin
/opt/homebrew
Homebrew
MacPorts
Xcode
CommandLineTools
LaunchServices
Application Bundles
Frameworks
AI application directories
```

---

# 3.3 Candidate System

Scanner 不应该发现文件以后直接创建 Tool。

中间增加：

```text
Candidate
```

例如：

```text
Candidate

Path:
/usr/local/bin/abc

Metadata:
Executable
Universal Binary

Hash:
xxxx

Version:
unknown

Recognized:
false
```

识别流程：

```text
Filesystem
   ↓
Candidate
   ↓
Recognizer
   ↓
Tool Definition
   ↓
Tool Instance
```

这样 Unknown Tool 不会污染 Registry。

---

# 3.4 Tool Recognition Engine

负责回答：

> 这个 executable / app 到底是什么？

识别方式按照可信程度排序。

```text
Known signatures
↓
Bundle / Publisher metadata
↓
Package manager metadata
↓
Known path patterns
↓
Executable metadata
↓
Static fingerprint
↓
Known safe probe
↓
AI classification
↓
User classification
```

输出：

```text
Tool Definition:
FFmpeg

Instance:
FFmpeg 8.x

Confidence:
0.99

Evidence:
Homebrew metadata
Executable name
Version output
```

所有自动判断都保留 Evidence。

---

# 3.5 Tool Registry

Registry 不存简单的：

```text
name + path
```

而是完整关系模型。

核心实体：

```text
ToolDefinition
ToolInstance
Interface
Capability
Environment
Owner
Origin
Evidence
Trust
Alias
Dependency
Agent
Skill
ExecutionRecord
```

---

# 4. Tool 数据模型

## ToolDefinition

描述“这个软件是什么”。

```yaml
id: org.ffmpeg.ffmpeg

name: FFmpeg

type:
  - media_tool
  - cli

vendor: FFmpeg

capabilities:
  - media.video.transcode
  - media.audio.transcode
  - media.video.probe
  - media.frame.extract
```

---

## ToolInstance

描述“电脑里的这一份 FFmpeg”。

```yaml
id: instance_uuid

tool: org.ffmpeg.ffmpeg

version: "8.x"

platform: macos
arch: arm64

path:
  /opt/homebrew/bin/ffmpeg

canonical_path:
  /opt/homebrew/Cellar/ffmpeg/.../ffmpeg

environment:
  homebrew.default

origin:
  homebrew

scope:
  user

trust:
  known

status:
  available
```

---

# 5. Interface 模型

一个 Tool 可以有多个 Interface。

例如 Blender：

```text
Tool
Blender

Interfaces
├── GUI
├── CLI
├── Python
└── File Association
```

Interface 类型建议：

```text
CLI
GUI
API
HTTP
Local Socket
MCP
Python API
Shell
Plugin
Library
File Handler
Automation
```

CLI Interface：

```yaml
type: cli

executable:
  /Applications/Blender.app/Contents/MacOS/Blender

supports:
  stdin: true
  stdout: true
  batch: true
```

---

# 6. Capability System

这是 ToolHub 最值得标准化的部分。

Capability 命名采用 namespace：

```text
language.python.execute

code.compile.c
code.compile.cpp

binary.decompile.java
binary.decompile.native

android.apk.decompile
android.device.control

media.video.transcode
media.image.convert

archive.extract
archive.create

network.http.request

container.run

database.sqlite.query
```

Tool：

```text
FFmpeg
↓
media.video.transcode
media.audio.transcode
media.video.probe
```

Skill：

```text
Create Video Thumbnail

requires:
media.video.frame.extract
```

Agent：

```text
我需要提取视频帧
↓
toolhub resolve media.video.frame.extract
↓
FFmpeg
```

Agent 不需要知道 FFmpeg 的安装路径。

---

# 7. Environment System

这部分解决你最开始提出的重复安装问题。

Environment 类型：

```text
system
user
homebrew
winget
scoop
chocolatey
conda
venv
uv
pyenv
nvm
pnpm
npm
cargo
docker
wsl
agent_sandbox
application_bundle
project_local
unknown
```

例如：

```text
Machine

├── System
│
├── Homebrew
│   ├── FFmpeg
│   ├── Python
│   └── Node
│
├── Conda
│   └── ML
│       ├── Python
│       └── CUDA tools
│
├── Cursor
│   └── Sandbox A
│       ├── Python
│       └── Node
│
└── Agent X
    └── Sandbox
        └── Python
```

---

# 8. Ownership Analyzer

需要独立模块：

```text
Ownership Analyzer
```

判断：

```text
这个 Python 为什么在这里？
是谁管理？
属于谁？
```

但是 Owner 必须允许：

```text
known
probable
unknown
```

不能仅仅看到某个目录就在 UI 中武断地说：

> Cursor 安装了它。

例如：

```yaml
owner:
  type: application
  id: cursor

confidence: 0.72

evidence:
  - parent_directory
  - environment_metadata
```

用户才能区分事实和推测。

---

# 9. Duplicate Analyzer

不会删除任何东西。

只负责：

```text
发现重复
统计
可视化
分析来源
```

例如：

```text
Python

6 instances

System           1
Homebrew         1
Conda            1
Agent Sandboxes  3
```

还可以计算：

```text
Executable size
Environment size
Last used
Last detected
Owner
Version
Architecture
```

最后形成：

## Environment Map

这是 ToolHub 一个非常有辨识度的 UI 功能。

---

# 10. AI Assisted Discovery

AI 不能直接成为 Scanner。

正确结构：

```text
Native Scan
   ↓
Known Tools ─────────────→ Registry
   ↓
Unknown Candidates
   ↓
AI Discovery
   ↓
Classification
   ↓
Registry
```

用户可以选择：

```text
AI Discovery Provider

● Connected Agent
    OpenCode
    Codex
    Other Agent

○ Temporary API

○ Disabled
```

---

# 11. Discovery Session

AI 扫描应该创建临时 Session。

```text
DiscoverySession

ID:
xxxx

Expires:
30 minutes

Permissions:
candidate.list
candidate.inspect
metadata.read
known_probe.version
classification.submit
```

ToolHub 给 Agent 的能力不是：

```text
execute_everything()
```

而是：

```text
list_candidates()
inspect_candidate()
probe_version()
search_registry()
submit_classification()
```

---

# 12. Connected Agent 模式

这是整个项目非常重要的设计。

例如 OpenCode。

ToolHub 可以检测：

```text
opencode
```

然后用户点击：

```text
Use OpenCode to identify unknown tools
```

ToolHub 创建 Discovery Session。

生成任务：

```text
You are performing a ToolHub discovery session.

Goal:
Identify useful software already installed on this machine.

Rules:
Do not install software.
Do not uninstall software.
Do not change system configuration.
Do not modify PATH.
Do not delete files.
Use ToolHub discovery interfaces whenever possible.

Unknown candidates:
...
```

然后由 Agent 调用：

```bash
toolhub discovery list
toolhub discovery inspect xxx
toolhub discovery classify xxx
```

这意味着：

**模型属于 Agent。**

ToolHub 不关心：

```text
OpenAI
Anthropic
Gemini
免费模型
本地模型
```

只关心：

```text
这个 Agent 是否能够运行 ToolHub CLI/MCP。
```

像 OpenCode 这样的 Agent 已经提供程序化运行、shell 权限以及 MCP 接入，因此 ToolHub 可以通过适配器与这类 Agent 协作，而不需要取得它背后模型的 API Key。

---

# 13. Agent Adapter System

不要把：

```text
OpenCode integration
Codex integration
```

写死到 Core。

定义：

```text
AgentAdapter
```

接口：

```text
detect()

get_version()

get_capabilities()

launch_discovery_session()

generate_install_instructions()

generate_mcp_config()

health_check()
```

例如：

```text
Adapters

├── OpenCode
├── Codex
├── Claude Code
├── Generic MCP Client
└── Generic CLI Agent
```

以后新增 Agent 不需要修改 Scanner。

---

# 14. Execution Engine

执行时：

```text
Agent Request
     ↓
Resolver
     ↓
Instance Selection
     ↓
Policy Engine
     ↓
Environment Builder
     ↓
Process Executor
     ↓
stdout / stderr
     ↓
Agent
```

---

# 15. Resolver

Agent 可以请求：

```bash
toolhub resolve language.python.execute
```

返回：

```text
Candidate 1

Python 3.13
System
Trusted
Global

Candidate 2

Python 3.12
Conda ML
Project Environment
```

自动选择可以参考：

```text
用户设置
Agent request
Working directory
Project environment
Version requirements
Trust
Scope
Architecture
Availability
```

但所有 Instance 都继续暴露。

---

# 16. Execution Policy

工具执行权限：

```text
Allow
Ask
Deny
```

可以按：

```text
Tool
Capability
Agent
Directory
Environment
Command
```

配置。

例如：

```text
FFmpeg
allow

Python
ask

PowerShell
ask

rm
deny
```

---

# 17. Environment Sanitizer

ToolHub 执行工具时不能简单继承整个环境。

需要：

```text
Environment Sanitizer
```

控制：

```text
PATH
HOME
TEMP
API Keys
Proxy
Working Directory
Locale
Tool-specific ENV
```

避免无意把：

```text
OPENAI_API_KEY
AWS_SECRET_ACCESS_KEY
GITHUB_TOKEN
```

全部传进未知程序。

---

# 18. Execution Audit

记录：

```text
Agent
Tool
Instance
Command
Arguments
Working directory
Start time
Duration
Exit code
```

默认不保存敏感 stdout。

例如：

```text
OpenCode

used

FFmpeg 8

Capability:
media.video.transcode

Duration:
18.3 sec
```

这也能让 UI 展示：

> 哪些工具实际上被 AI 使用过。

---

# 19. Fallback System

ToolHub Executor 失败：

```text
ToolHub execution failed
```

只返回结构化状态：

```json
{
  "status": "unavailable",
  "reason": "daemon_error",
  "fallback_allowed": true
}
```

Agent 可以自己决定：

```text
Direct execution
Sandbox
Alternative Tool
```

ToolHub 不应该控制 Agent 的全部行为。

---

# 20. Skill Engine

Skill Engine 支持三类：

```text
Instruction Skill
MCP Skill
Package Skill
```

它们统一成：

```text
ToolHub Skill Manifest
```

---

# 21. Skill Manifest

例如：

```yaml
schema: toolhub.skill/v1

id: reverse.android.apk
name: Android APK Analysis

description:
  Analyze APK structure and decompile application code.

requires:

  - capability: android.apk.decompile

  - capability: archive.extract

  - capability: language.java.runtime
    version: ">=17"

optional:

  - capability: android.device.control

  - capability: binary.reverse

interfaces:

  instruction:
    file: SKILL.md

  mcp:
    server: ./mcp.json
```

ToolHub 做：

```text
Parse Skill
↓
Resolve Capabilities
↓
Find Local Tools
↓
Show availability
```

不安装依赖。

---

# 22. Skill Center

UI：

```text
Skills

Installed
Downloaded
Local
Available
```

Skill Detail：

```text
Android APK Analysis

Requirements

✓ Java
✓ Archive extraction
✓ APK decompiler

Optional

✕ Android device control

Compatible tools

Java
OpenJDK 21

APK Decompiler
JADX
```

下载 Skills 的按钮最终进入这里。

---

# 23. ToolHub CLI

完整 CLI：

```bash
toolhub status

toolhub scan
toolhub scan --deep

toolhub list
toolhub list --type cli
toolhub list --capability media.video.transcode

toolhub search python

toolhub show <tool>
toolhub inspect <instance>

toolhub resolve <capability>

toolhub exec <tool>
toolhub exec --capability <capability>

toolhub env list
toolhub env show <id>

toolhub duplicates

toolhub skill list
toolhub skill inspect
toolhub skill resolve

toolhub agent list
toolhub agent connect

toolhub discovery start
toolhub discovery list
toolhub discovery inspect
toolhub discovery classify

toolhub mcp serve

toolhub daemon status
toolhub daemon restart

toolhub export

toolhub doctor
```

所有 CLI 都支持：

```text
--json
```

方便 Agent 调用。

---

# 24. MCP Gateway

不要把每个 Tool 都变成 MCP Tool。

否则用户电脑有：

```text
500 tools
```

就会给模型塞入几百个 tool definitions。

ToolHub MCP 只暴露少量“元工具”。

例如：

```text
search_tools
resolve_capability
inspect_tool
list_environments
execute_tool
search_skills
inspect_skill
```

模型需要 FFmpeg 时：

```text
search_tools("video conversion")
```

然后再：

```text
execute_tool(...)
```

而不是让模型启动时直接看到：

```text
ffmpeg
python
git
clang
java
...
500 个 MCP Tool
```

这样可以显著减少 Context 污染。

---

# 25. ToolHub Protocol

建议定义开放协议：

## THP

```text
ToolHub Protocol
```

使用：

```text
JSON-RPC
```

Transport：

```text
stdin/stdout
Unix Domain Socket
Windows Named Pipe
```

后续可以增加：

```text
localhost HTTP
WebSocket
```

---

# 26. ToolHub Manifest

定义：

```text
THM
ToolHub Manifest
```

描述：

```text
Tool
Interface
Capability
Environment
Skill
Agent
```

Schemas 放在：

```text
schemas.toolhub.dev
```

未来即使桌面软件不存在，第三方也可以生成：

```text
toolhub-tool.json
```

让 ToolHub 直接识别。

---

# 27. Plugin System

第三方扩展应该针对 Scanner，而不是修改 Core。

例如：

```text
AndroidScanner
CUDA Scanner
Unity Scanner
Unreal Scanner
SecurityTools Scanner
Bioinformatics Scanner
EDA Scanner
```

Plugin 权限需要非常有限：

```text
discover
inspect
classify
```

不能直接得到无限制写权限。

---

# 28. Native Scanner Plugin

内置 Scanner 也使用类似接口。

```rust
trait Scanner {
    detect(...)
    scan(...)
    identify(...)
}
```

这样：

```text
Windows Scanner
macOS Scanner
Python Scanner
Node Scanner
Docker Scanner
Agent Scanner
```

结构保持一致。

---

# 29. Desktop UI

技术架构：

```text
Tauri
+
Rust
+
TypeScript
+
React
```

UI 不直接扫描磁盘。

全部通过 Daemon。

---

# 30. Desktop 页面

建议正式拥有九个一级页面：

```text
Overview

Tools

Environments

Capabilities

Skills

Agents

Activity

Security

Settings
```

---

# 31. Overview

首页：

```text
ToolHub

428 tools

CLI        GUI        Runtime       SDK
126         84          23          34

AI Agents
5

Environments
17

Duplicate runtimes
12

Last scan
3 minutes ago
```

下面显示：

```text
Recently discovered

Recently used

Unknown tools

Environment changes
```

---

# 32. Tools

过滤：

```text
All

Applications
CLI
Runtime
Compiler
SDK
Decompiler
Debugger
Database
Media
Containers
AI
```

支持：

```text
Search
Capability Search
Vendor
Environment
Owner
Trust
Architecture
```

---

# 33. Tool Detail

例如：

```text
Python

CPython 3.13

Interfaces
CLI

Capabilities
language.python.execute
language.python.package

Path
...

Environment
System

Owner
User

Detected by
PATH Scanner

Trust
Known

Used by
OpenCode
Codex
```

---

# 34. Environment Map

视觉上是树/图：

```text
Mac

System
├── Git
└── Python

Homebrew
├── Node
├── FFmpeg
└── Python

Conda
└── Python

OpenCode
└── Sandbox
    └── Python
```

这是整个软件最适合成为宣传截图的页面之一。

---

# 35. Agents 页面

ToolHub 可以识别：

```text
OpenCode
Codex
Claude Code
Cursor
其他 Agent
```

展示：

```text
Installed
Integration available
MCP connected
Last activity
Tools used
Environments detected
```

---

# 36. Activity

时间线：

```text
14:02
OpenCode used Python 3.13

13:58
New environment detected

13:41
Codex requested video.transcode

13:20
Native scan completed
```

---

# 37. Security 页面

显示：

```text
Execution permissions

Discovery permissions

Unknown executable policy

Environment variable policy

Agent permissions

Temporary AI credentials
```

---

# 38. API Key 设计

默认：

```text
Temporary Provider
```

Key：

```text
不写 SQLite
不写日志
不写 config
不进入 crash report
```

生命周期：

```text
UI input
↓
Sensitive memory object
↓
Provider process/session
↓
destroy
```

最好连 UI State persistence 都禁止。

---

# 39. Trust Model

Tool Instance Trust：

```text
Verified
Known
User Trusted
Unknown
Blocked
```

Evidence 可以来自：

```text
package manager
publisher
signature
bundle id
known hash
known executable
user decision
```

---

# 40. Unknown Executable

Unknown executable 默认：

```text
不自动执行
```

即使：

```text
abc.exe --version
```

理论上也可以有副作用。

只有：

```text
Known Tool
```

可以安全进行预定义 probe。

Unknown Tool：

```text
Static metadata
AI reasoning
User approval
```

之后才能 probe。

---

# 41. Scan Modes

完整产品提供三个扫描模式。

## Quick Scan

```text
OS metadata
PATH
Known applications
Known package managers
Known runtimes
```

## Full Scan

额外：

```text
Known environment locations
Agent application directories
Project toolchains
Local runtimes
MCP configs
```

## AI Discovery

处理：

```text
Unknown candidates
Ambiguous software
Capability enrichment
```

不建议默认递归扫描整个磁盘。

应该依赖：

```text
OS indexes
known directories
package metadata
environment managers
```

避免 ToolHub 本身变成高 IO 软件。

---

# 42. Incremental Scanner

第一次扫描之后：

```text
Full discovery
```

之后：

```text
Filesystem events
PATH changes
Registry changes
Applications changes
Package manager state
```

增量更新。

ToolHub 不应该每次启动重新扫描整个电脑。

---

# 43. Database

建议：

```text
SQLite
```

核心 tables：

```text
tool_definitions
tool_instances
interfaces
capabilities
tool_capabilities

environments
environment_tools

owners

evidence
trust_records

skills
skill_requirements

agents

execution_records

scan_sessions
scan_candidates

settings
```

Migration 必须从项目第一天存在。

---

# 44. Local IPC

Desktop：

```text
UI
↓
IPC
↓
toolhubd
```

CLI：

```text
toolhub
↓
IPC
↓
toolhubd
```

推荐：

```text
macOS
Unix Domain Socket

Windows
Named Pipe
```

禁止默认监听公网端口。

---

# 45. Daemon Failure

CLI 检测：

```text
Daemon unavailable
```

执行：

```text
尝试启动 daemon
```

仍失败：

```text
返回结构化错误
```

而不是整个 CLI 卡死。

某些只读命令未来可以实现：

```text
--direct
```

读取 Registry。

---

# 46. 日志

三类日志：

```text
System Log
Scanner Log
Execution Audit
```

敏感字段必须 redaction。

例如：

```text
--token
--password
--api-key
Authorization
```

不能进入日志。

---

# 47. Export / Import

ToolHub 可以导出：

```text
Machine Tool Report
```

但默认隐藏：

```text
Username
Home directory
Secrets
Sensitive arguments
```

例如：

```bash
toolhub export --format json
```

可以给 Agent、开发者或者 Bug Report 使用。

---

# 48. 技术仓库结构

建议使用 Monorepo：

```text
toolhub/
│
├── apps/
│   ├── desktop/
│   ├── daemon/
│   └── cli/
│
├── crates/
│   ├── core/
│   ├── protocol/
│   ├── registry/
│   ├── scanner/
│   ├── scanner-windows/
│   ├── scanner-macos/
│   ├── recognizer/
│   ├── capability/
│   ├── environment/
│   ├── ownership/
│   ├── resolver/
│   ├── executor/
│   ├── policy/
│   ├── skills/
│   ├── agent-bridge/
│   ├── mcp/
│   ├── ipc/
│   └── audit/
│
├── packages/
│   ├── ui/
│   ├── schemas/
│   └── sdk-typescript/
│
├── scanners/
│   ├── python/
│   ├── node/
│   ├── java/
│   ├── dotnet/
│   ├── rust/
│   ├── go/
│   ├── android/
│   ├── container/
│   └── ai-agents/
│
├── resources/
│   ├── capabilities/
│   ├── signatures/
│   └── tool-definitions/
│
├── schemas/
│   ├── tool.schema.json
│   ├── capability.schema.json
│   ├── skill.schema.json
│   └── protocol.schema.json
│
├── docs/
│   ├── architecture/
│   ├── protocol/
│   ├── security/
│   └── development/
│
└── tests/
    ├── fixtures/
    ├── windows/
    ├── macos/
    └── integration/
```

---

# 49. 工程技术栈

核心：

```text
Rust
```

用于：

```text
Scanner
Daemon
CLI
Executor
Registry
IPC
Security
```

Desktop：

```text
Tauri
React
TypeScript
```

Database：

```text
SQLite
```

Async Runtime：

```text
Tokio
```

CLI：

```text
Clap
```

Serialization：

```text
Serde
```

Logging：

```text
tracing
```

具体 crate 不需要写进开放规范。

---

# 50. 为什么 Core 必须 Rust 优先

这个项目最大的复杂度并不是 UI，而是：

```text
process management
filesystem
OS API
path resolution
signals
IPC
permissions
concurrency
binary inspection
cross-platform behavior
```

因此 Core 和 UI 应该彻底分开。

---

# 51. 自动测试体系

必须有：

## Unit Tests

```text
Registry
Resolver
Capability
Policy
Manifest parser
```

## Scanner Fixtures

虚拟：

```text
fake Program Files
fake Homebrew
fake PATH
fake Registry dump
```

否则扫描器非常难稳定测试。

## Integration Tests

```text
Scanner
→ Registry
→ Resolver
→ Executor
```

## Platform CI

```text
Windows
macOS Intel
macOS ARM
```

---

# 52. Scanner 不应该依赖开发者自己的机器

应该建立：

```text
Tool Fixture Library
```

例如：

```text
Python 3.11 metadata
Python 3.13 metadata
FFmpeg
Node
Java
JADX
Docker
Git
```

用于稳定测试 Recognition。

---

# 53. 更新系统

三个东西独立版本：

```text
ToolHub Desktop

ToolHub Core

Recognition Database
```

例如 Tool Definition 数据库可以更新，而不需要整个 App 更新。

未来：

```text
ToolHub
1.8

Recognition DB
2026.09.29
```

---

# 54. Privacy

ToolHub 默认：

```text
Local First
```

扫描结果不上传。

AI Discovery 如果使用外部模型，应明确显示：

```text
哪些 metadata 会发送出去。
```

Connected Agent 模式下：

```text
数据处理遵循该 Agent 自己的行为。
```

ToolHub 不能宣称能够控制一个已经拥有主机 shell 权限的第三方 Agent。

它只能限制：

> Agent 通过 ToolHub 能做什么。

---

# 55. 产品完整功能边界

ToolHub 做：

```text
Discover
Identify
Catalog
Describe
Resolve
Execute
Audit
Visualize
Integrate
```

ToolHub 不做：

```text
Package Manager
System Cleaner
Sandbox Provider
AI Model Provider
IDE
General-purpose Agent
```

这条边界非常重要。

---

# 56. Agent 工程拆分

如果真的使用多个 Coding Agent 并行构建，我建议拆成 **12 个 Agent 工作域 + 1 个 Integration Agent**。

也就是总共：

**13 个工程 Agent。**

---

# Agent 01 — Architecture / Protocol

负责：

```text
Domain model
Tool schema
Capability schema
Skill schema
IPC protocol
Error model
Versioning
```

只拥有：

```text
crates/protocol
schemas
docs/architecture
```

这个 Agent 是最先工作的。

---

# Agent 02 — Registry / Database

负责：

```text
SQLite
Migrations
Repositories
ToolDefinition
ToolInstance
Capability
Environment
Evidence
```

目录：

```text
crates/registry
```

---

# Agent 03 — Scanner Framework

负责：

```text
Scanner trait
Candidate pipeline
Scan orchestration
Incremental scanning
Scanner events
```

目录：

```text
crates/scanner
```

不负责 OS 特定扫描。

---

# Agent 04 — Windows Discovery

负责：

```text
PATH
Registry
Installed Apps
winget
Scoop
Chocolatey
Windows SDK
Visual Studio
WSL
```

目录：

```text
crates/scanner-windows
```

---

# Agent 05 — macOS Discovery

负责：

```text
Applications
PATH
Homebrew
Xcode
CommandLineTools
LaunchServices
Bundle metadata
```

目录：

```text
crates/scanner-macos
```

---

# Agent 06 — Recognition / Capability

负责：

```text
Known tool definitions
Fingerprint
Evidence
Classification
Capability database
Version probe rules
```

目录：

```text
crates/recognizer
crates/capability
resources/
```

---

# Agent 07 — Environment / Ownership

负责：

```text
Environment detection
Duplicate analysis
Origin
Owner
Agent sandbox detection
Environment graph
```

目录：

```text
crates/environment
crates/ownership
```

---

# Agent 08 — Execution / Security

负责：

```text
Resolver
Process executor
Policy
Environment sanitizer
Trust
Audit
```

目录：

```text
crates/resolver
crates/executor
crates/policy
crates/audit
```

这是安全敏感度最高的 Agent。

---

# Agent 09 — CLI / Daemon / IPC

负责：

```text
toolhubd
toolhub CLI
Named Pipe
Unix Socket
Lifecycle
Health check
```

目录：

```text
apps/daemon
apps/cli
crates/ipc
```

---

# Agent 10 — AI / MCP / Agent Bridge

负责：

```text
MCP Server
Discovery Sessions
AgentAdapter
OpenCode adapter
Generic adapters
Temporary credentials
AI classification workflow
```

目录：

```text
crates/mcp
crates/agent-bridge
```

---

# Agent 11 — Skills

负责：

```text
Skill parser
SKILL.md
MCP Skill
Package Skill
Capability requirements
Skill resolver
```

目录：

```text
crates/skills
```

---

# Agent 12 — Desktop UX

负责：

```text
Tauri
React
Overview
Tools
Tool Detail
Environment Map
Skills
Agents
Activity
Security
Settings
```

目录：

```text
apps/desktop
packages/ui
```

---

# Agent 13 — Integration / QA

这个 Agent 不负责主要 Feature。

负责：

```text
merge review
cross-module integration
test architecture
CI
release build
protocol compliance
security regression
```

并且禁止随意重构其他 Agent 的代码。

---

# 57. Agent 依赖关系

不能 13 个 Agent 第一秒一起乱写。

依赖图：

```text
             Agent 01
          Architecture
               │
       ┌───────┼────────┐
       ▼       ▼        ▼
      02      03       06
   Registry Scanner Recognition
       │       │        │
       │    ┌──┴──┐     │
       │    ▼     ▼     │
       │   04     05    │
       │ Windows macOS  │
       │                │
       └──────┬─────────┘
              ▼
             07
      Environment/Owner
              │
              ▼
             08
      Execution/Security
              │
       ┌──────┼──────────┐
       ▼      ▼          ▼
      09     10         11
   CLI/IPC  AI/MCP     Skills
       │      │          │
       └──────┼──────────┘
              ▼
             12
          Desktop
              │
              ▼
             13
       Integration/QA
```

Desktop 可以提前做 Mock UI，但不能提前决定 Domain Model。

---

# 58. Agent 开发规则

所有 Agent 共用：

```text
ARCHITECTURE.md
CONTRIBUTING.md
DOMAIN.md
PROTOCOL.md
SECURITY.md
```

禁止：

```text
Agent 自己新增重复 Domain Model
Agent 自己偷偷修改 Schema
Agent 跨目录大规模重构
```

Schema 修改必须由 Architecture Agent 审核。

---

# 59. 每个 Agent 的标准任务格式

给 Agent 的任务不能写：

```text
帮我开发 Scanner。
```

应该写：

```text
Domain:
Windows Scanner

Allowed directories:
crates/scanner-windows/**
tests/windows/**

Read-only dependencies:
crates/protocol
crates/scanner

Responsibilities:
- discover PATH executables
- enumerate known application metadata
- output ScanCandidate
- never write Registry
- never install software

Must not:
- modify database schema
- execute unknown binaries
- edit desktop application

Required tests:
...
```

这样多个 Agent 才不会互相破坏。

---

# 60. Integration Contract

所有模块之间只通过 Domain 类型通信。

例如 Scanner：

```text
scan()
↓
Vec<ScanCandidate>
```

Recognizer：

```text
ScanCandidate
↓
RecognitionResult
```

Registry：

```text
RecognitionResult
↓
ToolInstance
```

Resolver：

```text
CapabilityRequest
↓
ResolvedInstance
```

Executor：

```text
ExecutionRequest
↓
ExecutionResult
```

Desktop 永远不要绕过这些接口。

---

# 61. 项目开发顺序

不是把产品砍成 MVP，而是为了避免工程依赖混乱。

第一轮完成：

```text
Domain
Protocol
Registry
```

第二轮：

```text
Scanner Framework
Recognition
Windows
macOS
```

第三轮：

```text
Environment
Ownership
Duplicate analysis
```

第四轮：

```text
Resolver
Executor
Policy
Audit
```

第五轮：

```text
Daemon
CLI
MCP
Agent Bridge
```

第六轮：

```text
Skills
```

第七轮：

```text
Desktop
```

第八轮：

```text
Integration
QA
Packaging
Updates
```

最终仍然是一个完整产品。

---

# 62. ToolHub 最核心的数据流

最终整套系统最重要的流程只有：

```text
DISCOVER
      ↓
CANDIDATE
      ↓
RECOGNIZE
      ↓
REGISTER
      ↓
DESCRIBE
      ↓
RESOLVE
      ↓
AUTHORIZE
      ↓
EXECUTE
      ↓
AUDIT
```

AI 只是插在：

```text
RECOGNIZE
```

和部分：

```text
DESCRIBE
```

中。

这能保证即使完全没有 AI：

**ToolHub 本身仍然是一款完整的软件。**

---

# 63. ToolHub 的真正核心资产

最后，这个项目真正长期有价值的不是 Desktop UI。

也不是 MCP Server。

更不是某个 AI Provider。

而是四个东西：

```text
1. Tool Registry Model

2. Capability Taxonomy

3. Recognition Database

4. ToolHub Protocol
```

如果这四个东西设计稳定：

```text
OpenCode
Codex
Claude Code
Cursor
未来 Agent
```

都只是 Adapter。

Windows 和 macOS 也只是 Scanner Backend。

GUI 也只是 Registry 的一种可视化方式。

因此整个项目应该围绕：

**Tool → Instance → Interface → Capability → Environment → Agent**

这条主线构建。

最终 ToolHub 所做的事情可以浓缩为：

> **让任何 AI 在使用电脑之前，先知道电脑已经会什么。**