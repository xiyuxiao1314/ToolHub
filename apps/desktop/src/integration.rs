//! Embed the portable Agent skill and add the current installation's connection details.
use serde_json::{json, Value};
use std::path::Path;

const SKILL: &str = include_str!("../../../skills/toolhub-integration/SKILL.md");

fn connection_info(cli: &Path) -> Result<Value, String> {
    let cli_path = cli.to_str().ok_or("CLI 路径包含无法表示的字符")?;
    let config = json!({"mcpServers":{"toolhub":{"command":cli_path,"args":["mcp","serve"]}}});
    let config_text = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    let mimo_config = serde_json::to_string_pretty(&json!({"mcp":{"toolhub":{"type":"local","command":[cli_path,"mcp","serve"],"enabled":true}}})).map_err(|e|e.to_string())?;
    let check = format!(
        "$toolhub = '{}'\n& $toolhub --json status\n& $toolhub --json search python",
        cli_path.replace('\'', "''")
    );
    let local = format!("# ToolHub 接入\n\n## 本机连接信息\n\nCLI：`{cli_path}`\n\n```powershell\n{check}\n```\n\n通用 MCP 配置（按宿主格式合并）：\n\n```json\n{config_text}\n```\n\nMiMo / mimocode 配置（~/.config/mimocode/mimocode.jsonc，合并 mcp.toolhub）：\n\n```json\n{mimo_config}\n```\n");
    let skill = SKILL.replacen("# ToolHub 接入\n", &local, 1);
    let codex_config = format!(
        "[mcp_servers.toolhub]\ncommand = {}\nargs = [\"mcp\", \"serve\"]\n",
        serde_json::to_string(cli_path).map_err(|e| e.to_string())?
    );
    let host_profiles = toolhub_agent_bridge::hosts::templates(cli_path);
    let bootstrap = format!(
        r#"请帮我把本机 ToolHub 接入你正在使用的 Agent 宿主，并按宿主支持创建本机的 toolhub-integration Skill 或任务指引。

1. 宿主支持 Skills 时，如果有 skill-creator（创建 Skills 的技能），先使用它；否则按宿主的 Skill 规范创建。不支持 Skills 时将下方正文作为任务指引，继续配置 MCP。确认本机 CLI 和同目录 toolhubd 存在，并运行下方只读检查。不要创建另一个 ToolHub 应用，也不要安装无关依赖。
2. 宿主支持 Skills 时，根据本机宿主实际识别的用户 Skill 目录保存下面的完整 SKILL.md，保留本机绝对路径；若同名 Skill 已存在，先阅读并合并，不重复创建。保持允许隐式调用，让需要本机工具的任务优先查询 ToolHub。支持 agents/openai.yaml 的宿主可用 allow_implicit_invocation: true。使用创建技能的验证器检查。
3. 先确认实际宿主和它的 MCP 配置格式。支持本地 stdio MCP 的其他客户端也可接入。接入说明的客户端选择器提供当前模板；不支持 Skills 的宿主用同一正文作为任务指引。将 toolhub stdio MCP 服务合并到当前宿主配置；保留其他 MCP 服务和权限设置。command 是下面的 CLI 路径，args 为 ["mcp", "serve"]。通用 JSON、Codex TOML 和 MiMo JSONC 示例都在下面，选宿主实际支持的格式。MiMo 的 command 是包含 CLI、mcp、serve 的数组，保存在 mcp.toolhub 中。若已配置 toolhub，核对路径后只更新该服务。
4. 实际验证 MCP initialize / initialized、tools/list（应包含 propose_program 等固定数量的元工具），再调用 search_tools 查询 ffmpeg。配置写入不等于连接成功；若宿主需要重新打开对话或重载，明确告诉我，并尽可能用直接 stdio 检查验证服务。不能将没有找到 FFmpeg 说成连接失败。
5. 以后需要 FFmpeg、解压、PDF、OCR 或编程环境时先查询/解析，再检查并按策略调用。应用成果验证完成后，通过 propose_program 提交真实启动入口到“待确认”，提醒我到 ToolHub 程序页刷新、勾选并保存。提交不代表已收录，不自动运行或绕过确认。

本机只读检查：
```powershell
{check}
```

通用 MCP 配置：
```json
{config_text}
```

MiMo / mimocode JSONC 示例（合并到 ~/.config/mimocode/mimocode.jsonc）：
```json
{mimo_config}
```

Codex TOML 示例：
```toml
{codex_config}```

将以下全文保存为 SKILL.md（不要把上面的安装任务写入 Skill 正文）：
{skill}

完成后报告 Skill 实际保存路径、MCP 配置位置、连接验证结果和需要我执行的重载步骤。"#
    );
    Ok(
        json!({"host_profiles":host_profiles,"cli_path":cli_path,"cli_available":cli.is_file(),"mcp_config":config_text,"mimo_config":mimo_config,"codex_config":codex_config,"cli_check":check,"skill_markdown":skill,"bootstrap_prompt":bootstrap}),
    )
}

#[tauri::command]
pub fn agent_integration() -> Result<Value, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let cli = exe
        .parent()
        .ok_or("应用目录不可用")?
        .join(if cfg!(windows) {
            "toolhub.exe"
        } else {
            "toolhub"
        });
    connection_info(&cli)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn configuration_preserves_unicode_spaces_and_shell_quotes() {
        let path = Path::new("C:\\Agent's 项目\\Tool Hub\\toolhub.exe");
        let info = connection_info(path).unwrap();
        let config: Value = serde_json::from_str(info["mcp_config"].as_str().unwrap()).unwrap();
        assert_eq!(
            config["mcpServers"]["toolhub"]["command"],
            path.to_str().unwrap()
        );
        assert_eq!(
            config["mcpServers"]["toolhub"]["args"],
            json!(["mcp", "serve"])
        );
        assert!(info["cli_check"]
            .as_str()
            .unwrap()
            .starts_with("$toolhub = 'C:\\Agent''s 项目\\Tool Hub\\toolhub.exe'\n"));
        let mimo: Value = serde_json::from_str(info["mimo_config"].as_str().unwrap()).unwrap();
        assert_eq!(
            mimo["mcp"]["toolhub"]["command"],
            json!([path.to_str().unwrap(), "mcp", "serve"])
        );
        assert_eq!(mimo["mcp"]["toolhub"]["type"], "local");
        let markdown = info["skill_markdown"].as_str().unwrap();
        assert!(markdown.starts_with("---\nname: toolhub-integration\n"));
        assert!(markdown.contains(info["mcp_config"].as_str().unwrap()));
        assert!(markdown.contains("普通 Agent 无权自动选择、保存或启动程序条目"));
        assert!(info["bootstrap_prompt"]
            .as_str()
            .unwrap()
            .contains(markdown));
        assert!(info["bootstrap_prompt"]
            .as_str()
            .unwrap()
            .contains("保留其他 MCP 服务和权限设置"));
    }
}

#[tauri::command]
pub async fn agent_mcp_check() -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(||{
  use std::io::{BufReader,Write};use std::process::{Command,Stdio};
  let exe=std::env::current_exe().map_err(|e|e.to_string())?;let cli=exe.parent().ok_or("installation directory unavailable")?.join(if cfg!(windows){"toolhub.exe"}else{"toolhub"});
  let mut command=Command::new(cli);command.args(["mcp","serve"]).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null());
  #[cfg(windows)] {use std::os::windows::process::CommandExt;command.creation_flags(0x08000000);}
  struct Owned(std::process::Child);impl Drop for Owned{fn drop(&mut self){let _=self.0.kill();let _=self.0.wait();}}
  let mut child=Owned(command.spawn().map_err(|e|e.to_string())?);let mut input=child.0.stdin.take().ok_or("stdin unavailable")?;let output=child.0.stdout.take().ok_or("stdout unavailable")?;
  let messages=[json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"ToolHub self-test","version":"1"}}}),json!({"jsonrpc":"2.0","method":"notifications/initialized"}),json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"search_tools","arguments":{"query":"ffmpeg","limit":5,"detail":"summary"}}})];
  for message in messages {writeln!(input,"{message}").map_err(|e|e.to_string())?;}drop(input);
  let (sender,receiver)=std::sync::mpsc::channel();std::thread::spawn(move||{let mut reader=BufReader::new(output);for _ in 0..3 {let line=toolhub_ipc::read_bounded_line(&mut reader).map_err(|e|e.to_string()).and_then(|v|v.ok_or("missing response".into())).and_then(|v|serde_json::from_slice::<Value>(&v).map_err(|e|e.to_string()));if sender.send(line).is_err(){break}}});
  let mut responses=vec![];for _ in 0..3{responses.push(receiver.recv_timeout(std::time::Duration::from_secs(5)).map_err(|_|"MCP self-test deadline")??);}
  if responses.iter().any(|r|r.get("error").is_some())||responses[2]["result"]["isError"]==true {return Err("MCP self-test failed".into())}
  Ok(json!({"status":"success","tool_count":responses[1]["result"]["tools"].as_array().map(Vec::len),"matching_tools":responses[2]["result"]["structuredContent"]["result"].as_array().map(Vec::len),"note":"ToolHub server works; host configuration alone does not prove the host has reloaded"}))
 }).await.map_err(|e|e.to_string())?
}
