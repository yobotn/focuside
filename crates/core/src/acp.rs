//! ACP (Agent Client Protocol) client: JSON-RPC 2.0, one message per line over the agent's
//! stdio. The client side is small (initialize, sessions, prompt, cancel, permission answers),
//! so it is written here directly on `serde_json` instead of depending on the SDK crate, whose
//! API changes between major versions.

use crate::proc::ProcessTree;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, oneshot};

/// ACP protocol version this client speaks.
pub const PROTOCOL_VERSION: u64 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl std::fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (code {})", self.message, self.code)?;
        if let Some(d) = &self.data {
            write!(f, ": {d}")?;
        }
        Ok(())
    }
}

impl std::error::Error for RpcError {}

impl RpcError {
    pub fn method_not_found(method: &str) -> Self {
        Self {
            code: -32601,
            message: format!("Method not found: {method}"),
            data: None,
        }
    }

    /// ACP uses -32000 for "authentication required".
    pub fn is_auth_required(&self) -> bool {
        self.code == -32000
    }
}

/// A message from the agent that is not a response to one of our requests.
#[derive(Debug, Clone)]
pub enum Incoming {
    /// The agent asks something and waits for `Connection::respond`.
    Request {
        id: Value,
        method: String,
        params: Value,
    },
    Notification {
        method: String,
        params: Value,
    },
    /// The agent's stdout closed: the process exited or crashed.
    Closed,
}

type Pending = Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value, RpcError>>>>>;

/// Sending half of a JSON-RPC connection. Cheap to clone.
#[derive(Clone)]
pub struct Connection {
    out: mpsc::UnboundedSender<String>,
    pending: Pending,
    next_id: Arc<AtomicU64>,
    log: Option<mpsc::UnboundedSender<(bool, String)>>,
}

impl Connection {
    /// Wires a connection to a reader and writer. Returns the connection and the stream of
    /// requests and notifications from the agent. `log` receives every line (true = outgoing).
    pub fn start<R, W>(
        reader: R,
        mut writer: W,
        log: Option<mpsc::UnboundedSender<(bool, String)>>,
    ) -> (Self, mpsc::UnboundedReceiver<Incoming>)
    where
        R: tokio::io::AsyncRead + Unpin + Send + 'static,
        W: tokio::io::AsyncWrite + Unpin + Send + 'static,
    {
        let (out_tx, mut out_rx) = mpsc::unbounded_channel::<String>();
        let (in_tx, in_rx) = mpsc::unbounded_channel();
        let pending: Pending = Arc::default();

        tokio::spawn(async move {
            while let Some(line) = out_rx.recv().await {
                if writer.write_all(line.as_bytes()).await.is_err()
                    || writer.write_all(b"\n").await.is_err()
                    || writer.flush().await.is_err()
                {
                    break;
                }
            }
        });

        let pend = pending.clone();
        let log_in = log.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(reader).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if line.trim().is_empty() {
                    continue;
                }
                if let Some(l) = &log_in {
                    let _ = l.send((false, line.clone()));
                }
                let Ok(msg) = serde_json::from_str::<Value>(&line) else {
                    tracing::warn!("agent sent a line that is not JSON: {line}");
                    continue;
                };
                let method = msg.get("method").and_then(|m| m.as_str()).map(String::from);
                let id = msg.get("id").cloned().filter(|v| !v.is_null());
                match (method, id) {
                    (Some(method), Some(id)) => {
                        let params = msg.get("params").cloned().unwrap_or(Value::Null);
                        let _ = in_tx.send(Incoming::Request { id, method, params });
                    }
                    (Some(method), None) => {
                        let params = msg.get("params").cloned().unwrap_or(Value::Null);
                        let _ = in_tx.send(Incoming::Notification { method, params });
                    }
                    (None, Some(id)) => {
                        let Some(n) = id.as_u64() else { continue };
                        let tx = pend.lock().unwrap().remove(&n);
                        if let Some(tx) = tx {
                            let res = if let Some(err) = msg.get("error") {
                                Err(serde_json::from_value(err.clone()).unwrap_or(RpcError {
                                    code: -32603,
                                    message: err.to_string(),
                                    data: None,
                                }))
                            } else {
                                Ok(msg.get("result").cloned().unwrap_or(Value::Null))
                            };
                            let _ = tx.send(res);
                        }
                    }
                    (None, None) => {}
                }
            }
            // Fail everything still waiting, then tell the owner.
            for (_, tx) in pend.lock().unwrap().drain() {
                let _ = tx.send(Err(RpcError {
                    code: -32099,
                    message: "The agent stopped.".into(),
                    data: None,
                }));
            }
            let _ = in_tx.send(Incoming::Closed);
        });

        (
            Self {
                out: out_tx,
                pending,
                next_id: Arc::new(AtomicU64::new(1)),
                log,
            },
            in_rx,
        )
    }

    fn send(&self, msg: Value) {
        let line = msg.to_string();
        if let Some(l) = &self.log {
            let _ = l.send((true, line.clone()));
        }
        let _ = self.out.send(line);
    }

    /// Sends a request and waits for its response.
    pub async fn request(&self, method: &str, params: Value) -> Result<Value, RpcError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        self.pending.lock().unwrap().insert(id, tx);
        self.send(json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        rx.await.unwrap_or_else(|_| {
            Err(RpcError {
                code: -32099,
                message: "The agent stopped.".into(),
                data: None,
            })
        })
    }

    pub fn notify(&self, method: &str, params: Value) {
        self.send(json!({"jsonrpc": "2.0", "method": method, "params": params}));
    }

    pub fn respond(&self, id: Value, result: Result<Value, RpcError>) {
        match result {
            Ok(r) => self.send(json!({"jsonrpc": "2.0", "id": id, "result": r})),
            Err(e) => self.send(json!({"jsonrpc": "2.0", "id": id, "error": e})),
        }
    }
}

/// Where to find the agent and how to start it.
#[derive(Debug, Clone)]
pub struct AgentCommand {
    /// The ACP adapter executable (claude-agent-acp).
    pub adapter: PathBuf,
    /// Extra arguments for the adapter, for example a script path when the adapter is `node`.
    pub args: Vec<String>,
    /// The user's own `claude`, passed as CLAUDE_CODE_EXECUTABLE.
    pub claude: Option<PathBuf>,
    pub env: Vec<(String, String)>,
}

/// A running agent process with its connection.
pub struct AgentProcess {
    pub tree: ProcessTree,
    pub conn: Connection,
    pub incoming: mpsc::UnboundedReceiver<Incoming>,
    /// Last lines the agent wrote to stderr, for error messages.
    pub stderr: Arc<Mutex<VecDeque<String>>>,
}

impl AgentProcess {
    pub fn spawn(
        cmd: &AgentCommand,
        cwd: &Path,
        log: Option<mpsc::UnboundedSender<(bool, String)>>,
    ) -> Result<Self> {
        let mut c = Command::new(&cmd.adapter);
        c.args(&cmd.args).current_dir(cwd);
        if let Some(claude) = &cmd.claude {
            c.env("CLAUDE_CODE_EXECUTABLE", claude);
        }
        for (k, v) in &cmd.env {
            c.env(k, v);
        }
        let mut tree = ProcessTree::spawn_piped(c)
            .with_context(|| format!("start {}", cmd.adapter.display()))?;
        let child = tree.child_mut();
        let stdin = child.stdin.take().ok_or_else(|| anyhow!("no stdin"))?;
        let stdout = child.stdout.take().ok_or_else(|| anyhow!("no stdout"))?;
        let stderr_pipe = child.stderr.take().ok_or_else(|| anyhow!("no stderr"))?;
        let stderr: Arc<Mutex<VecDeque<String>>> = Arc::default();
        let buf = stderr.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr_pipe).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::debug!(target: "agent_stderr", "{line}");
                let mut b = buf.lock().unwrap();
                if b.len() >= 40 {
                    b.pop_front();
                }
                b.push_back(line);
            }
        });
        let (conn, incoming) = Connection::start(stdout, stdin, log);
        Ok(Self {
            tree,
            conn,
            incoming,
            stderr,
        })
    }

    pub fn stderr_tail(&self) -> String {
        self.stderr
            .lock()
            .unwrap()
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// The `initialize` request FocusIDE sends. No file system or terminal capabilities: the agent
/// reads and writes the disk itself, and FocusIDE measures changes from the disk.
pub fn initialize_params() -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "clientCapabilities": {
            "fs": { "readTextFile": false, "writeTextFile": false },
            "terminal": false
        },
        "clientInfo": { "name": "focuside", "title": "FocusIDE", "version": env!("CARGO_PKG_VERSION") }
    })
}

/// What the agent supports, read from the `initialize` response.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    pub load_session: bool,
    pub resume_session: bool,
    pub list_sessions: bool,
    pub images: bool,
    pub system_prompt: bool,
    pub auth_methods: Vec<Value>,
}

impl Capabilities {
    pub fn from_initialize(v: &Value) -> Self {
        let ac = &v["agentCapabilities"];
        let sc = &ac["sessionCapabilities"];
        Self {
            load_session: ac["loadSession"].as_bool().unwrap_or(false),
            resume_session: !sc["resume"].is_null(),
            list_sessions: !sc["list"].is_null(),
            images: ac["promptCapabilities"]["image"].as_bool().unwrap_or(false),
            system_prompt: !sc["systemPrompt"].is_null(),
            auth_methods: v["authMethods"].as_array().cloned().unwrap_or_default(),
        }
    }
}

/// A permission option offered by the agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionOption {
    pub option_id: String,
    pub name: String,
    pub kind: String,
}

pub fn permission_options(params: &Value) -> Vec<PermissionOption> {
    serde_json::from_value(params["options"].clone()).unwrap_or_default()
}

/// Response to `session/request_permission` choosing one option.
pub fn permission_selected(option_id: &str) -> Value {
    json!({ "outcome": { "outcome": "selected", "optionId": option_id } })
}

/// Response to `session/request_permission` when the turn was cancelled.
pub fn permission_cancelled() -> Value {
    json!({ "outcome": { "outcome": "cancelled" } })
}

/// True if a permission request is the plan approval (Claude Code's ExitPlanMode tool).
pub fn is_plan_request(params: &Value) -> bool {
    let tc = &params["toolCall"];
    let title = tc["title"].as_str().unwrap_or("");
    let kind = tc["kind"].as_str().unwrap_or("");
    tc["rawInput"]["plan"].is_string()
        || kind == "switch_mode"
        || title.contains("ExitPlanMode")
        || title.eq_ignore_ascii_case("ready to code?")
}

/// The plan text of a plan approval request.
pub fn plan_text(params: &Value) -> Option<String> {
    let tc = &params["toolCall"];
    if let Some(p) = tc["rawInput"]["plan"].as_str() {
        return Some(p.to_string());
    }
    // Some adapter versions put the plan in the tool call content.
    tc["content"].as_array().and_then(|items| {
        items.iter().find_map(|c| {
            c["content"]["text"]
                .as_str()
                .or(c["text"].as_str())
                .map(String::from)
        })
    })
}

/// Picks the "run the plan, accept edits" option: edits go through, shell commands still ask.
/// Prefers an accept-edits option without clearing context, then manual approval (the caller
/// switches the mode to acceptEdits right after).
pub fn run_plan_option(options: &[PermissionOption]) -> Option<(String, bool)> {
    let find = |pred: &dyn Fn(&PermissionOption) -> bool| options.iter().find(|o| pred(o));
    let lower = |o: &PermissionOption| (o.option_id.to_lowercase(), o.name.to_lowercase());
    if let Some(o) = find(&|o| {
        let (id, name) = lower(o);
        (id.contains("acceptedits") || name.contains("auto-accept edits"))
            && !id.contains("clear")
            && !name.contains("clear context")
    }) {
        return Some((o.option_id.clone(), false));
    }
    if let Some(o) = find(&|o| {
        let (id, name) = lower(o);
        (id.contains("default") || name.contains("manually approve")) && o.kind.starts_with("allow")
    }) {
        return Some((o.option_id.clone(), true));
    }
    find(&|o| o.kind.starts_with("allow") && !lower(o).1.contains("clear context"))
        .map(|o| (o.option_id.clone(), true))
}

/// Picks the "keep planning" (reject) option of a plan request.
pub fn reject_option(options: &[PermissionOption]) -> Option<String> {
    options
        .iter()
        .find(|o| o.kind.starts_with("reject"))
        .map(|o| o.option_id.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(list: &[(&str, &str, &str)]) -> Vec<PermissionOption> {
        list.iter()
            .map(|(id, name, kind)| PermissionOption {
                option_id: id.to_string(),
                name: name.to_string(),
                kind: kind.to_string(),
            })
            .collect()
    }

    #[test]
    fn run_plan_prefers_accept_edits() {
        let o = opts(&[
            (
                "exitPlanClearAcceptEdits",
                "Yes, clear context and auto-accept edits",
                "allow_always",
            ),
            (
                "exitPlanAcceptEdits",
                "Yes, auto-accept edits",
                "allow_always",
            ),
            (
                "exitPlanDefault",
                "Yes, manually approve edits",
                "allow_once",
            ),
            ("plan", "No, keep planning", "reject_once"),
        ]);
        assert_eq!(
            run_plan_option(&o),
            Some(("exitPlanAcceptEdits".into(), false))
        );
        assert_eq!(reject_option(&o), Some("plan".into()));
    }

    #[test]
    fn run_plan_falls_back_to_manual_then_mode_switch() {
        let o = opts(&[
            ("exitPlanAuto", "Yes, and use auto mode", "allow_always"),
            (
                "exitPlanDefault",
                "Yes, manually approve edits",
                "allow_once",
            ),
            ("plan", "No, keep planning", "reject_once"),
        ]);
        assert_eq!(run_plan_option(&o), Some(("exitPlanDefault".into(), true)));
    }

    #[tokio::test]
    async fn request_and_response_roundtrip() {
        let (client_side, agent_side) = tokio::io::duplex(4096);
        let (cr, cw) = tokio::io::split(client_side);
        let (conn, _incoming) = Connection::start(cr, cw, None);
        let (ar, mut aw) = tokio::io::split(agent_side);
        tokio::spawn(async move {
            let mut lines = BufReader::new(ar).lines();
            let line = lines.next_line().await.unwrap().unwrap();
            let v: Value = serde_json::from_str(&line).unwrap();
            let reply = json!({"jsonrpc":"2.0","id":v["id"],"result":{"ok":v["params"]["x"]}});
            aw.write_all(format!("{reply}\n").as_bytes()).await.unwrap();
        });
        let r = conn.request("ping", json!({"x": 7})).await.unwrap();
        assert_eq!(r["ok"], 7);
    }
}
