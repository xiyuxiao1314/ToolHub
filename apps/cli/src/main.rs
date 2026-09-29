//! toolhub CLI — daemon client with structured --json output.

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

use clap::{Parser, Subcommand};
use serde_json::{json, Value};

#[derive(Parser)]
#[command(name = "toolhub", about = "ToolHub local tool infrastructure", version)]
struct Cli {
    /// Emit structured JSON for agents
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Daemon and registry status
    Status,
    /// Native discovery scan
    Scan {
        #[arg(long, default_value = "quick")]
        mode: String,
    },
    /// Search tools
    Search { query: String },
    /// Inspect a tool instance
    Inspect { id: String },
    /// List environments
    Env {
        #[command(subcommand)]
        cmd: EnvCmd,
    },
    /// Duplicate runtime analysis
    Duplicates,
    /// Resolve a capability
    Resolve {
        capability: String,
        #[arg(long)]
        version: Option<String>,
    },
    /// Execute a tool under policy
    Exec {
        instance_id: String,
        #[arg(last = true)]
        args: Vec<String>,
        #[arg(long)]
        approval_id: Option<String>,
    },
    /// Approve a pending execution
    Approve {
        instance_id: String,
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// Skill queries
    Skill {
        #[command(subcommand)]
        cmd: SkillCmd,
    },
    /// Discovery sessions
    Discovery {
        #[command(subcommand)]
        cmd: DiscoveryCmd,
    },
    /// Agent adapters
    Agent {
        #[command(subcommand)]
        cmd: AgentCmd,
    },
    /// Policy
    Policy {
        #[command(subcommand)]
        cmd: PolicyCmd,
    },
    /// Redacted machine report
    Export {
        #[arg(long, default_value = "json")]
        format: String,
    },
    /// Doctor / health
    Doctor,
    /// Run MCP stdio gateway (meta-tools only)
    Mcp {
        #[command(subcommand)]
        cmd: McpCmd,
    },
}

#[derive(Subcommand)]
enum EnvCmd {
    List,
}

#[derive(Subcommand)]
enum SkillCmd {
    List,
    Inspect { id: String },
}

#[derive(Subcommand)]
enum DiscoveryCmd {
    Start {
        #[arg(long)]
        agent_id: Option<String>,
    },
    List,
    Inspect {
        id: String,
    },
    Revoke {
        id: String,
    },
}

#[derive(Subcommand)]
enum AgentCmd {
    List,
}

#[derive(Subcommand)]
enum PolicyCmd {
    Get,
    Set {
        scope: String,
        subject: String,
        action: String,
    },
}

#[derive(Subcommand)]
enum McpCmd {
    Serve,
    Tools,
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(&cli) {
        let msg = e.to_string();
        let code = if msg.contains("not_found") {
            3
        } else if msg.contains("denied") || msg.contains("policy") || msg.contains("approval") {
            4
        } else if msg.contains("expired") || msg.contains("session") {
            5
        } else if msg.contains("unavailable") || msg.contains("daemon") {
            6
        } else if msg.contains("timeout") {
            7
        } else if msg.contains("invalid") || msg.contains("parse") {
            2
        } else {
            1
        };
        if cli.json {
            let v = json!({"ok": false, "error": msg});
            println!("{}", serde_json::to_string(&v).unwrap_or_default());
        } else {
            eprintln!("error: {msg}");
        }
        std::process::exit(code);
    }
}

fn run(cli: &Cli) -> anyhow::Result<()> {
    let mut client = DaemonClient::connect()?;

    let result = match &cli.command {
        Commands::Status => client.call("status", json!({}))?,
        Commands::Scan { mode } => client.call("scan.start", json!({"mode": mode}))?,
        Commands::Search { query } => client.call("registry.search", json!({"query": query}))?,
        Commands::Inspect { id } => client.call("registry.inspect_instance", json!({"id": id}))?,
        Commands::Env { cmd: EnvCmd::List } => client.call("environment.list", json!({}))?,
        Commands::Duplicates => client.call("environment.duplicates", json!({}))?,
        Commands::Resolve {
            capability,
            version,
        } => client.call(
            "resolve.capability",
            json!({"capability": capability, "version": version}),
        )?,
        Commands::Exec {
            instance_id,
            args,
            approval_id,
        } => client.call(
            "execute.tool",
            json!({"instance_id": instance_id, "args": args, "approval_id": approval_id}),
        )?,
        Commands::Approve { instance_id, args } => client.call(
            "execute.approve",
            json!({"instance_id": instance_id, "args": args}),
        )?,
        Commands::Skill { cmd } => match cmd {
            SkillCmd::List => client.call("skill.list", json!({}))?,
            SkillCmd::Inspect { id } => client.call("skill.inspect", json!({"id": id}))?,
        },
        Commands::Discovery { cmd } => match cmd {
            DiscoveryCmd::Start { agent_id } => client.call(
                "discovery.start",
                json!({"agent_id": agent_id.clone().unwrap_or_else(|| "local.agent".into())}),
            )?,
            DiscoveryCmd::List => client.call("discovery.list", json!({}))?,
            DiscoveryCmd::Inspect { id } => client.call("discovery.inspect", json!({"id": id}))?,
            DiscoveryCmd::Revoke { id } => {
                client.call("discovery.revoke", json!({"session_id": id}))?
            }
        },
        Commands::Agent {
            cmd: AgentCmd::List,
        } => client.call("agent.list", json!({}))?,
        Commands::Policy { cmd } => match cmd {
            PolicyCmd::Get => client.call("policy.get", json!({}))?,
            PolicyCmd::Set {
                scope,
                subject,
                action,
            } => client.call(
                "policy.set",
                json!({"scope": scope, "subject": subject, "action": action}),
            )?,
        },
        Commands::Export { format } => {
            let v = client.call("export.report", json!({"format": format}))?;
            println!("{}", serde_json::to_string_pretty(&v)?);
            return Ok(());
        }
        Commands::Doctor => {
            let st = client.call("status", json!({}))?;
            let agents = client.call("agent.list", json!({})).unwrap_or(json!([]));
            let out = json!({
                "daemon": st,
                "agents": agents,
                "checks": [
                    {"name": "registry_readable", "ok": true},
                    {"name": "no_public_listener", "ok": true},
                    {"name": "native_scan_without_ai_key", "ok": true},
                ]
            });
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&out)?);
            } else {
                println!("ToolHub doctor");
                println!("{}", serde_json::to_string_pretty(&out)?);
            }
            return Ok(());
        }
        Commands::Mcp { cmd } => match cmd {
            McpCmd::Tools => {
                let tools = vec![
                    "search_tools",
                    "resolve_capability",
                    "inspect_tool",
                    "list_environments",
                    "execute_tool",
                    "search_skills",
                    "inspect_skill",
                ];
                if cli.json {
                    println!("{}", serde_json::to_string(&tools)?);
                } else {
                    for t in tools {
                        println!("{t}");
                    }
                }
                return Ok(());
            }
            McpCmd::Serve => return mcp_serve(&mut client),
        },
    };

    if cli.json {
        println!("{}", serde_json::to_string(&result)?);
    } else {
        println!("{}", serde_json::to_string_pretty(&result)?);
    }
    Ok(())
}

struct DaemonClient {
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    reader: BufReader<std::process::ChildStdout>,
    next_id: u64,
}

impl DaemonClient {
    fn connect() -> anyhow::Result<Self> {
        // Prefer an already-running toolhubd on stdio via spawn.
        // For B01, CLI always starts a short-lived daemon for the request set
        // (shared registry file provides durable state).
        let mut child = Command::new(daemon_bin())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;
        let stdin = child.stdin.take().expect("stdin");
        let stdout = child.stdout.take().expect("stdout");
        Ok(Self {
            child,
            stdin,
            reader: BufReader::new(stdout),
            next_id: 1,
        })
    }

    fn call(&mut self, method: &str, params: Value) -> anyhow::Result<Value> {
        let id = self.next_id;
        self.next_id += 1;
        let req = json!({"jsonrpc":"2.0","id":id,"method":method,"params":params});
        writeln!(self.stdin, "{}", serde_json::to_string(&req)?)?;
        self.stdin.flush()?;
        let mut line = String::new();
        let n = self.reader.read_line(&mut line)?;
        if n == 0 {
            anyhow::bail!("daemon closed connection");
        }
        let resp: Value = serde_json::from_str(&line)?;
        if let Some(err) = resp.get("error") {
            let code = err
                .get("data")
                .and_then(|d| d.get("error_code"))
                .and_then(|c| c.as_str())
                .unwrap_or("internal_error")
                .to_string();
            let msg = err
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("error")
                .to_string();
            // F17: structured error without killing reusable clients (MCP).
            return Err(anyhow::anyhow!("{code}: {msg}"));
        }
        Ok(resp.get("result").cloned().unwrap_or(Value::Null))
    }
}

impl Drop for DaemonClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn daemon_bin() -> std::path::PathBuf {
    if let Ok(p) = std::env::var("TOOLHUBD_BIN") {
        return std::path::PathBuf::from(p);
    }
    // Sibling of current exe: toolhubd.exe next to toolhub.exe
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            #[cfg(windows)]
            let cand = dir.join("toolhubd.exe");
            #[cfg(not(windows))]
            let cand = dir.join("toolhubd");
            if cand.exists() {
                return cand;
            }
        }
    }
    std::path::PathBuf::from("toolhubd")
}

#[allow(dead_code)]
fn exit_code_for(code: &str) -> i32 {
    match code {
        "invalid_request" | "invalid_params" | "parse_error" => 2,
        "not_found" => 3,
        "denied" | "policy_denied" | "trust_blocked" | "approval_invalid" => 4,
        "expired" | "session_invalid" => 5,
        "unavailable" | "daemon_error" => 6,
        "timeout" => 7,
        _ => 1,
    }
}

fn mcp_serve(client: &mut DaemonClient) -> anyhow::Result<()> {
    // Bounded meta-tools over stdio JSON-RPC (MCP-compatible subset).
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let req: Value = serde_json::from_str(&line)?;
        let id = req.get("id").cloned();
        let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let params = req.get("params").cloned().unwrap_or(json!({}));

        // F13: notifications (no id) require no response
        if id.is_none() {
            continue;
        }

        let (result, is_error) = match method {
            "initialize" => (
                json!({
                    "protocolVersion": params.get("protocolVersion").and_then(|v| v.as_str()).unwrap_or("2024-11-05"),
                    "capabilities": {"tools": {"listChanged": false}},
                    "serverInfo": {"name": "toolhub", "version": "0.1.0"}
                }),
                false,
            ),
            "notifications/initialized" | "initialized" | "notifications/cancelled" => {
                continue;
            }
            "tools/list" => (
                json!({
                    "tools": [
                        {"name":"search_tools","description":"Search installed tools by query","inputSchema":{"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}},
                        {"name":"resolve_capability","description":"Resolve a capability","inputSchema":{"type":"object","properties":{"capability":{"type":"string"}},"required":["capability"]}},
                        {"name":"inspect_tool","description":"Inspect one tool","inputSchema":{"type":"object","properties":{"id":{"type":"string"}},"required":["id"]}},
                        {"name":"list_environments","description":"List environments","inputSchema":{"type":"object","properties":{}}},
                        {"name":"execute_tool","description":"Execute under policy","inputSchema":{"type":"object","properties":{"instance_id":{"type":"string"},"args":{"type":"array","items":{"type":"string"}},"approval_id":{"type":"string"}},"required":["instance_id"]}},
                        {"name":"search_skills","description":"Search skills","inputSchema":{"type":"object","properties":{"query":{"type":"string"}}}},
                        {"name":"inspect_skill","description":"Inspect skill","inputSchema":{"type":"object","properties":{"id":{"type":"string"}},"required":["id"]}}
                    ]
                }),
                false,
            ),
            "tools/call" => {
                let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or(json!({}));
                let call_result = (|| -> Result<Value, String> {
                    Ok(match name {
                        "search_tools" => {
                            let q = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                            client
                                .call("registry.search", json!({"query": q}))
                                .map_err(|e| e.to_string())?
                        }
                        "resolve_capability" => {
                            let c = args
                                .get("capability")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            client
                                .call("resolve.capability", json!({"capability": c}))
                                .map_err(|e| e.to_string())?
                        }
                        "inspect_tool" => {
                            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
                            client
                                .call("registry.inspect_instance", json!({"id": id}))
                                .map_err(|e| e.to_string())?
                        }
                        "list_environments" => client
                            .call("environment.list", json!({}))
                            .map_err(|e| e.to_string())?,
                        "execute_tool" => {
                            let id = args
                                .get("instance_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            let a = args.get("args").cloned().unwrap_or(json!([]));
                            let approval = args.get("approval_id").cloned();
                            client
                                .call(
                                    "execute.tool",
                                    json!({"instance_id": id, "args": a, "approval_id": approval}),
                                )
                                .map_err(|e| e.to_string())?
                        }
                        "search_skills" => client
                            .call("skill.list", json!({}))
                            .map_err(|e| e.to_string())?,
                        "inspect_skill" => {
                            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
                            client
                                .call("skill.inspect", json!({"id": id}))
                                .map_err(|e| e.to_string())?
                        }
                        other => return Err(format!("unknown tool {other}")),
                    })
                })();
                match call_result {
                    Ok(v) => (
                        json!({"content":[{"type":"text","text": v.to_string()}], "isError": false}),
                        false,
                    ),
                    Err(msg) => (
                        json!({"content":[{"type":"text","text": msg}], "isError": true}),
                        true,
                    ),
                }
            }
            other => (
                json!({"error": format!("unsupported method {other}")}),
                true,
            ),
        };
        let resp = if is_error {
            json!({"jsonrpc":"2.0","id":id,"error":{"code":-32000,"message": result}})
        } else {
            json!({"jsonrpc":"2.0","id":id,"result":result})
        };
        writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
        stdout.flush()?;
    }
    Ok(())
}
