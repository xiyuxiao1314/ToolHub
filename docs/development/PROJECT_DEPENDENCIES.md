# 项目级工具依赖发现 / Project tool dependency discovery

ToolHub beta.5 增加 Electron 适配器。全盘扫描继续跳过 `node_modules`；它不代表所有项目依赖已被发现。项目扫描只读取用户明确选择的项目根目录，不执行 `npm`、安装脚本、`index.js`、Electron 或版本探针，也不自动授予信任。

## 桌面使用

在“工具 → 项目工具依赖”选择项目目录，再点击“扫描项目依赖”。支持逐个添加多个项目、重新扫描已选项目，并在工具详情查看项目归属。自定义下载位置可选“自定义 Electron 分发目录”；明确选中的目录随观察记录保存，点击重新扫描时沿用。取消自定义位置后可恢复默认扫描。

扫描状态独立于工具安装实例：完整安装为 `available`；只有 npm 包为 `missing_binary`；依赖声明存在而包缺失为 `missing_package`；损坏/越界包和非原生二进制分别为 `invalid_package` / `invalid_binary`。缺失记录不会伪装成可执行实例，不会增加可用工具总数。版本从二进制静态元数据优先读取，其次采用已安装包版本；依赖声明范围不当作实际版本。两者不同时保存并提示。

## CLI / SDK / THP

```powershell
& 'C:\path\to\toolhub.exe' --json scan --mode project --roots 'D:\projects\app-one' --roots 'D:\projects\app-two'
& 'C:\path\to\toolhub.exe' --json scan --mode project --roots 'D:\projects\app-one' --electron-dist 'D:\Electron dist'
```

```json
{"method":"scan.start","params":{"mode":"project","roots":["D:/projects/app"],"project_overrides":[{"project_root":"D:/projects/app","electron_dist":"D:/Electron dist"}]}}
```

`registry.project_dependencies {}` 返回持久化的项目状态；`registry.inspect_instance {id}` / MCP `inspect_tool {id}` 返回 `projects` 数组。`search_tools {query:"Electron"}` 查询完整安装，默认不返回缺失实例。SDK 提供 `scanProjectDependencies(roots, overrides?, options?)` 和 `projectDependencies(options?)`，保持 16 个通用 MCP 工具，不为每个项目增加工具定义。

## 边界与限制

- 每次 1–64 个明确项目根目录；每个项目只读 `package.json`、`node_modules/electron/package.json`、`path.txt` 和定位到的原生二进制。无目录树遍历。
- 清单各限 1 MiB、`path.txt` 限 4 KiB；拒绝绝对路径、盘符、UNC、ADS、`..` 及分发目录外的链接。
- 项目内 pnpm 链接可以解析；链接到项目外的 npm 包不能隐式访问，需选择其所属项目。不沿父目录搜索 hoisted/workspace 依赖；后续适配器可沿相同显式范围扩展。
- `ELECTRON_OVERRIDE_DIST_PATH` 只有在所选项目内部才被接受；项目外分发目录必须显式选择/传入。不会读取或执行 `.env`、shell 配置、npm 配置和包内脚本。
- Windows 使用只读版本资源 API，解决大型 Electron 资源超过旧 96 MiB 前缀的问题；其他平台读取已有静态格式及包版本。macOS/Linux 的真实桌面运行尚未实测。
- 原生路径去重：同一二进制被多项目引用只登记一个实例，保留每个项目的版本声明与关联；不同路径/版本独立保留。完整重扫只替换相应项目观察；中断或清单不可读不删除旧观察。人工信任/阻止决定在重扫时保留。
- 此适配器不安装或修复缺失依赖、不修改项目清单或锁文件、不提升信任。未知实例仍受既有执行策略约束。

## English

Select a project under **Tools → Project tool dependencies**. The Electron adapter reads exact manifests, `path.txt` and native binary metadata; it never recursively traverses node_modules or runs package code. An npm package without its distribution is reported separately from available executable instances. Explicit external distribution selections are supported; ambient settings cannot silently grant external scope. Canonical binaries are deduplicated while retaining all project references. Rescanning preserves human trust decisions. See the CLI/THP examples above and inspect `projects` through the existing MCP interface. No Electron download or redistribution is included.
