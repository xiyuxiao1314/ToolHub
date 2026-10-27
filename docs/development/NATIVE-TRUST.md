# 系统工具信任与默认版本（beta.11）

识别名称、核验来源、选择默认版本和执行授权是四件不同的事。AI 可协助解释候选及来源线索，不能证明文件无害或替代原生签名检查。版本最新也不代表安全。

本次修复的系统 cURL：真实 Windows System32 由 GetSystemDirectoryW 得出，规范路径、PE 产品/原始文件名、微软受信签名与当前 SHA256 一起核验。支持嵌入签名和 Windows 目录签名；通过 WinVerifyTrust 使用本机缓存，不发送本机工具清单或主动联网拉取证书。证据标记离线核验，不能保证最新撤销状态或没有漏洞。复制到其他位置的同名 cURL 不自动获得系统信任，未知/阻止程序仍需对应核验或明确用户信任。

根因是原识别器严格匹配 ProductName=cURL，而实际系统文件是 The curl executable、CompanyName=curl, https://curl.se/。修复没有仅增加一个可伪造的名称别名；必须有系统路径和有效签名。资源规则不能覆盖已经取得的这份原生证明。扫描仍不运行未知程序，人工阻止/信任决定保留。

每次执行前，当前规范路径及执行器计算的哈希需与签名证据一致；变化后停止批准并要求重新扫描。现有固定文件句柄、精确参数授权和 Deny 规则继续生效。签名认证程序来源，不代表任意 Python 脚本或 cURL 参数都无害。

多版本策略：先过滤阻止、未知、版本不满足和显式架构不匹配的实例；保留手动选择和项目偏好；其他候选优先稳定正式版、真实版本号较高者，再按信任和路径排序。每个工具定义推荐一个默认实例，旧版仍作为可选项，项目可继续钉选旧版。搜索摘要暴露 trust_status 与 recommended；推荐不是执行授权。没有可信候选时不捏造可运行默认项。

验证：实际系统 cURL 8.21.0 的签名、SHA256、识别和受控 --version；隔离的伪 cURL 拒绝执行；十个 Python 的最新稳定选择、预览/未知排除、显式旧版与版本约束；执行前签名哈希失配不产生批准。Windows 原生检查通过，macOS 未实测。本次不自动安装工具、扫描并上传文件、删除旧版本或执行安全性不明的内容。

原生 API 依据：[WinVerifyTrust](https://learn.microsoft.com/en-us/windows/win32/api/wintrust/nf-wintrust-winverifytrust)、[Catalog hash lookup](https://learn.microsoft.com/en-us/windows/win32/api/mscat/nf-mscat-cryptcatadminenumcatalogfromhash)、[Windows 自带 curl](https://curl.se/windows/microsoft.html)。
