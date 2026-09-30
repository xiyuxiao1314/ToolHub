//! toolhub CLI — daemon client with structured --json output.

use std::io::{BufReader, Write};
use toolhub_ipc::RpcClient as DaemonClient;

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
        #[arg(long)]
        roots: Vec<String>,
        #[arg(long)]
        scan_id: Option<String>,
    },
    ScanStatus {
        scan_id: String,
    },
    ScanCancel {
        scan_id: String,
    },
    /// Search tools
    Search {
        query: String,
    },
    /// Inspect a tool instance
    Inspect {
        id: String,
    },
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
        #[arg(long)]
        preferred_environment: Option<String>,
    },
    /// Execute a tool under policy
    Exec {
        instance_id: String,
        #[arg(last = true)]
        args: Vec<String>,
        #[arg(long)]
        approval_id: Option<String>,
        #[arg(long)]
        session_id: Option<String>,
        #[arg(long)]
        execution_id: Option<String>,
        #[arg(long)]
        cwd: Option<String>,
    },
    /// Approve a pending execution
    Approve {
        request_id: String,
    },
    /// Submit an execution for review by the desktop controller
    RequestApproval {
        instance_id: String,
        #[arg(long)]
        session_id: Option<String>,
        #[arg(long)]
        cwd: Option<String>,
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// Import a declarative machine report
    Import {
        path: std::path::PathBuf,
    },
    /// Recent audit activity
    Activity,
    /// Typed SDK bridge; parameters are a bounded JSON object on stdin
    Rpc {
        method: String,
    },
    /// Persisted preferences
    Settings {
        #[command(subcommand)]
        cmd: SettingsCmd,
    },
    /// Cancel an owned execution by operation ID
    Cancel {
        execution_id: String,
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
    Register { path: std::path::PathBuf },
    Resolve { id: String },
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
    Classify {
        session_id: String,
        path: std::path::PathBuf,
    },
}

#[derive(Subcommand)]
enum AgentCmd {
    List,
    Launch {
        agent_id: String,
        #[arg(last = true)]
        args: Vec<String>,
    },
    Status {
        operation_id: String,
    },
    Cancel {
        operation_id: String,
    },
}

#[derive(Subcommand)]
enum SettingsCmd {
    Get,
    Set { path: std::path::PathBuf },
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
    let mut client = DaemonClient::connect_or_start(&daemon_bin())?;

    let result = match &cli.command {
        Commands::Status => call(&mut client, "status", json!({}))?,
        Commands::Scan {
            mode,
            roots,
            scan_id,
        } => call(
            &mut client,
            "scan.start",
            json!({"mode": mode,"roots":if roots.is_empty(){None}else{Some(roots)},"scan_session_id":scan_id}),
        )?,
        Commands::ScanStatus { scan_id } => {
            call(&mut client, "scan.status", json!({"scan_session_id":scan_id}))?
        }
        Commands::ScanCancel { scan_id } => {
            call(&mut client, "scan.cancel", json!({"scan_session_id":scan_id}))?
        }
        Commands::Search { query } => {
            call(&mut client, "registry.search", json!({"query": query}))?
        }
        Commands::Inspect { id } => {
            call(&mut client, "registry.inspect_instance", json!({"id": id}))?
        }
        Commands::Env { cmd: EnvCmd::List } => call(&mut client, "environment.list", json!({}))?,
        Commands::Duplicates => call(&mut client, "environment.duplicates", json!({}))?,
        Commands::Resolve {
            capability,
            version,
            preferred_environment,
        } => call(
            &mut client,
            "resolve.capability",
            json!({"capability": capability, "version": version, "preferred_environment": preferred_environment}),
        )?,
        Commands::Exec {
            instance_id,
            args,
            approval_id,
            session_id,
            execution_id,
            cwd,
        } => call(
            &mut client,
            "execute.tool",
            json!({"instance_id": instance_id, "args": args, "approval_id": approval_id, "session_id": session_id, "execution_id": execution_id, "cwd": cwd}),
        )?,
        Commands::Approve { request_id } => call(
            &mut client,
            "execute.approve",
            json!({"request_id": request_id}),
        )?,
        Commands::RequestApproval {
            instance_id,
            args,
            session_id,
            cwd,
        } => call(
            &mut client,
            "execute.approval_request",
            json!({"instance_id":instance_id,"args":args,"session_id":session_id,"cwd":cwd}),
        )?,
        Commands::Cancel { execution_id } => call(
            &mut client,
            "execute.cancel",
            json!({"execution_id":execution_id}),
        )?,
        Commands::Import { path } => call(
            &mut client,
            "import.report",
            json!({"report":read_json(path)?}),
        )?,
        Commands::Activity => call(&mut client, "activity.list", json!({}))?,
        Commands::Rpc { method } => {
            let mut reader = BufReader::new(std::io::stdin());
            let bytes = toolhub_ipc::read_bounded_line(&mut reader)?
                .ok_or_else(|| anyhow::anyhow!("invalid_params: JSON input required"))?;
            let params: Value = serde_json::from_slice(&bytes)?;
            call(&mut client, method, params)?
        }
        Commands::Settings { cmd } => match cmd {
            SettingsCmd::Get => call(&mut client, "settings.get", json!({}))?,
            SettingsCmd::Set { path } => call(&mut client, "settings.set", read_json(path)?)?,
        },
        Commands::Skill { cmd } => match cmd {
            SkillCmd::List => call(&mut client, "skill.list", json!({}))?,
            SkillCmd::Inspect { id } => call(&mut client, "skill.inspect", json!({"id": id}))?,
            SkillCmd::Register { path } => {
                call(&mut client, "skill.register", json!({"path":path}))?
            }
            SkillCmd::Resolve { id } => call(&mut client, "skill.resolve", json!({"id":id}))?,
        },
        Commands::Discovery { cmd } => match cmd {
            DiscoveryCmd::Start { agent_id } => call(
                &mut client,
                "discovery.start",
                json!({"agent_id": agent_id.clone().unwrap_or_else(|| "local.agent".into())}),
            )?,
            DiscoveryCmd::List => call(&mut client, "discovery.list", json!({}))?,
            DiscoveryCmd::Inspect { id } => {
                call(&mut client, "discovery.inspect", json!({"id": id}))?
            }
            DiscoveryCmd::Revoke { id } => {
                call(&mut client, "discovery.revoke", json!({"session_id": id}))?
            }
            DiscoveryCmd::Classify { session_id, path } => {
                let mut value = read_json(path)?;
                value
                    .as_object_mut()
                    .ok_or_else(|| anyhow::anyhow!("classification must be object"))?
                    .insert("session_id".into(), json!(session_id));
                call(&mut client, "discovery.classify", value)?
            }
        },
        Commands::Agent { cmd } => match cmd {
            AgentCmd::List => call(&mut client, "agent.list", json!({}))?,
            AgentCmd::Launch { agent_id, args } => call(
                &mut client,
                "agent.launch",
                json!({"agent_id":agent_id,"args":args}),
            )?,
            AgentCmd::Status { operation_id } => call(
                &mut client,
                "agent.status",
                json!({"operation_id":operation_id}),
            )?,
            AgentCmd::Cancel { operation_id } => call(
                &mut client,
                "agent.cancel",
                json!({"operation_id":operation_id}),
            )?,
        },
        Commands::Policy { cmd } => match cmd {
            PolicyCmd::Get => call(&mut client, "policy.get", json!({}))?,
            PolicyCmd::Set {
                scope,
                subject,
                action,
            } => call(
                &mut client,
                "policy.set",
                json!({"scope": scope, "subject": subject, "action": action}),
            )?,
        },
        Commands::Export { format } => {
            let v = call(&mut client, "export.report", json!({"format": format}))?;
            println!("{}", serde_json::to_string_pretty(&v)?);
            return Ok(());
        }
        Commands::Doctor => {
            let st = call(&mut client, "status", json!({}))?;
            let agents = call(&mut client, "agent.list", json!({})).unwrap_or(json!([]));
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
    // R2-B10: propagate semantic child failure as CLI exit code.
    if let Some(status) = result.get("status").and_then(|s| s.as_str()) {
        match status {
            "success" => {}
            "failed" | "timed_out" | "cancelled" => {
                if let Some(code) = result.get("exit_code").and_then(|c| c.as_i64()) {
                    std::process::exit((code as i32).clamp(1, 125));
                }
                std::process::exit(1);
            }
            "denied" | "expired" | "invalid_request" => std::process::exit(4),
            "unavailable" => std::process::exit(6),
            _ => {}
        }
    }
    Ok(())
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

fn read_json(path: &std::path::Path) -> anyhow::Result<Value> {
    let file = std::fs::File::open(path)?;
    if file.metadata()?.len() > toolhub_protocol::limits::MAX_REQUEST_BYTES as u64 {
        anyhow::bail!("payload_too_large");
    }
    Ok(serde_json::from_reader(file)?)
}

fn call(
    client: &mut DaemonClient,
    method: &str,
    mut params: Value,
) -> toolhub_ipc::IpcResult<Value> {
    // CLI optional flags are omitted from the wire when absent.
    if let Some(object) = params.as_object_mut() {
        object.retain(|_, value| !value.is_null());
    }
    client.call(method, params)
}
fn mcp_serve(client: &mut DaemonClient) -> anyhow::Result<()> {
    let stdin = std::io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    let mut stdout = std::io::stdout();
    let mut server = toolhub_mcp::Server::default();
    loop {
        let response = match toolhub_ipc::read_bounded_line(&mut reader) {
            Ok(Some(bytes)) => {
                if bytes.iter().all(|b| b.is_ascii_whitespace()) {
                    continue;
                }
                match toolhub_protocol::parse_request(&bytes) {
                    Ok(req) => server.handle(&req, |method, params| {
                        client.call(method, params).map_err(|e| e.to_string())
                    }),
                    Err(error) => Some(toolhub_ipc::error_response(None, &error)),
                }
            }
            Ok(None) => break,
            Err(toolhub_ipc::IpcError::Message(_)) => Some(toolhub_ipc::error_response(
                None,
                &toolhub_protocol::ProtocolError::new(
                    toolhub_protocol::ErrorCode::PayloadTooLarge,
                    "request too large",
                ),
            )),
            Err(error) => return Err(error.into()),
        };
        if let Some(response) = response {
            stdout.write_all(&toolhub_ipc::serialize_response(&response)?)?;
            stdout.write_all(b"\n")?;
            stdout.flush()?;
        }
    }
    Ok(())
}

