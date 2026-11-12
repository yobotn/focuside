//! M0.5 agent spike: Claude Code over ACP with no UI.
//!
//! focuside-spike caps
//! focuside-spike run <dir> "<prompt>" [--plan] [--discard] [--cancel-after <secs>] [--raw]
//! focuside-spike resume <dir> <session-id> "<prompt>" [--raw]
//! focuside-spike list <dir>
//! focuside-spike kill <dir>

use anyhow::{anyhow, bail, Result};
use focuside_core::acp::{self, AgentProcess, Capabilities, Incoming};
use focuside_core::agent;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

struct Opts {
    plan: bool,
    discard: bool,
    cancel_after: Option<u64>,
    raw: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let flag = |f: &str| args.iter().any(|a| a == f);
    let opts = Opts {
        plan: flag("--plan"),
        discard: flag("--discard"),
        cancel_after: args
            .iter()
            .position(|a| a == "--cancel-after")
            .and_then(|i| args.get(i + 1))
            .and_then(|s| s.parse().ok()),
        raw: flag("--raw"),
    };
    let pos: Vec<&String> = args
        .iter()
        .enumerate()
        .filter(|(i, a)| !a.starts_with("--") && (*i == 0 || args[i - 1] != "--cancel-after"))
        .map(|(_, a)| a)
        .collect();

    match pos.first().map(|s| s.as_str()) {
        Some("caps") => caps().await,
        Some("run") => {
            let dir = PathBuf::from(pos.get(1).ok_or_else(|| anyhow!("missing dir"))?);
            let prompt = pos.get(2).ok_or_else(|| anyhow!("missing prompt"))?;
            run(&dir, None, prompt, &opts).await
        }
        Some("resume") => {
            let dir = PathBuf::from(pos.get(1).ok_or_else(|| anyhow!("missing dir"))?);
            let id = pos.get(2).ok_or_else(|| anyhow!("missing session id"))?;
            let prompt = pos.get(3).ok_or_else(|| anyhow!("missing prompt"))?;
            run(&dir, Some(id.as_str()), prompt, &opts).await
        }
        Some("list") => {
            let dir = PathBuf::from(pos.get(1).ok_or_else(|| anyhow!("missing dir"))?);
            list(&dir).await
        }
        Some("kill") => {
            let dir = PathBuf::from(pos.get(1).ok_or_else(|| anyhow!("missing dir"))?);
            kill(&dir).await
        }
        _ => {
            eprintln!("usage: focuside-spike caps | run <dir> <prompt> [--plan] [--discard] [--cancel-after N] [--raw] | resume <dir> <id> <prompt> | list <dir> | kill <dir>");
            Ok(())
        }
    }
}

async fn start(dir: &Path, raw: bool) -> Result<(AgentProcess, Capabilities)> {
    let cmd = agent::claude_command().map_err(|m| anyhow!("missing: {m:?}"))?;
    println!("adapter: {}", cmd.adapter.display());
    println!("claude:  {:?}", cmd.claude);
    let log = if raw {
        let (tx, mut rx) = mpsc::unbounded_channel::<(bool, String)>();
        tokio::spawn(async move {
            while let Some((out, line)) = rx.recv().await {
                let short: String = line.chars().take(600).collect();
                println!("{} {short}", if out { ">>" } else { "<<" });
            }
        });
        Some(tx)
    } else {
        None
    };
    let p = AgentProcess::spawn(&cmd, dir, log)?;
    let t = Instant::now();
    let init = p
        .conn
        .request("initialize", acp::initialize_params())
        .await
        .map_err(|e| anyhow!("initialize: {e}\n{}", p.stderr_tail()))?;
    println!("initialize in {:?}", t.elapsed());
    let caps = Capabilities::from_initialize(&init);
    Ok((p, caps))
}

async fn caps() -> Result<()> {
    let dir = std::env::current_dir()?;
    let (mut p, caps) = start(&dir, false).await?;
    let init = p
        .conn
        .request("initialize", acp::initialize_params())
        .await?;
    println!("{}", serde_json::to_string_pretty(&init)?);
    println!("parsed: {caps:?}");
    p.tree.kill().await;
    Ok(())
}

async fn list(dir: &Path) -> Result<()> {
    let (mut p, caps) = start(dir, false).await?;
    if !caps.list_sessions {
        println!("session/list not supported");
    } else {
        let r = p
            .conn
            .request("session/list", json!({ "cwd": dir }))
            .await?;
        for s in r["sessions"].as_array().cloned().unwrap_or_default() {
            println!(
                "{}  {}  {}",
                s["sessionId"].as_str().unwrap_or(""),
                s["updatedAt"].as_str().unwrap_or(""),
                s["title"].as_str().unwrap_or("")
            );
        }
    }
    p.tree.kill().await;
    Ok(())
}

async fn kill(dir: &Path) -> Result<()> {
    let (mut p, caps) = start(dir, false).await?;
    let s = p
        .conn
        .request(
            "session/new",
            agent::new_session_params(dir, caps.system_prompt),
        )
        .await?;
    let sid = s["sessionId"].as_str().unwrap_or_default().to_string();
    let conn = p.conn.clone();
    let prompt = tokio::spawn(async move {
        conn.request(
            "session/prompt",
            json!({"sessionId": sid, "prompt": [{"type": "text", "text": "Reply with one word: ok"}]}),
        )
        .await
    });
    tokio::time::sleep(Duration::from_secs(4)).await;
    let before = p.tree.pids();
    println!(
        "processes in tree before kill: {} {:?}",
        before.len(),
        before
    );
    p.tree.kill().await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    let alive: Vec<u32> = before
        .iter()
        .copied()
        .filter(|pid| focuside_core::proc::is_alive(*pid))
        .collect();
    println!("still alive after kill: {alive:?}");
    println!("prompt result: {:?}", prompt.await?.map(|_| ()));
    if !alive.is_empty() {
        bail!("processes left behind");
    }
    Ok(())
}

async fn run(dir: &Path, resume: Option<&str>, text: &str, o: &Opts) -> Result<()> {
    let (mut p, caps) = start(dir, o.raw).await?;
    println!("caps: {caps:?}");
    let session = match resume {
        None => {
            p.conn
                .request(
                    "session/new",
                    agent::new_session_params(dir, caps.system_prompt),
                )
                .await
        }
        Some(id) => {
            let method = if caps.resume_session {
                "session/resume"
            } else {
                "session/load"
            };
            println!("resuming with {method}");
            p.conn
                .request(
                    method,
                    agent::load_session_params(dir, id, caps.system_prompt),
                )
                .await
                .map(|mut v| {
                    if v["sessionId"].is_null() {
                        v["sessionId"] = json!(id);
                    }
                    v
                })
        }
    };
    let session = match session {
        Ok(s) => s,
        Err(e) if e.is_auth_required() => bail!("not signed in: {e}"),
        Err(e) => bail!("session: {e}\n{}", p.stderr_tail()),
    };
    let sid = session["sessionId"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    println!("session: {sid}");
    println!(
        "modes: {}",
        session["modes"]["availableModes"]
            .as_array()
            .map(|m| m
                .iter()
                .map(|x| x["id"].as_str().unwrap_or("?").to_string())
                .collect::<Vec<_>>()
                .join(", "))
            .unwrap_or_default()
    );
    println!("models: {}", session["models"]);
    if !session["configOptions"].is_null() {
        println!("configOptions: {}", session["configOptions"]);
    }

    if o.plan {
        let r = p
            .conn
            .request(
                "session/set_mode",
                json!({"sessionId": sid, "modeId": "plan"}),
            )
            .await;
        println!("set_mode plan: {:?}", r.map(|_| "ok"));
    }

    let conn = p.conn.clone();
    let sid2 = sid.clone();
    let prompt_text = text.to_string();
    let mut prompt = tokio::spawn(async move {
        conn.request(
            "session/prompt",
            json!({"sessionId": sid2, "prompt": [{"type": "text", "text": prompt_text}]}),
        )
        .await
    });

    let started = Instant::now();
    let mut cancelled = false;
    let mut reads = BTreeSet::new();
    let mut writes = BTreeSet::new();
    let mut text_buf = String::new();
    let mut kinds: std::collections::HashMap<String, String> = Default::default();
    let mut want_accept_edits = false;
    let result = loop {
        if let Some(secs) = o.cancel_after {
            if !cancelled && started.elapsed() > Duration::from_secs(secs) {
                println!("\n-- cancelling after {secs}s");
                p.conn.notify("session/cancel", json!({"sessionId": sid}));
                cancelled = true;
            }
        }
        tokio::select! {
            r = &mut prompt => break r?,
            msg = p.incoming.recv() => {
                let Some(msg) = msg else { break Err(acp::RpcError{code:-1,message:"closed".into(),data:None}) };
                match msg {
                    Incoming::Notification { method, params } if method == "session/update" => {
                        let u = &params["update"];
                        if let (Some(id), Some(k)) = (u["toolCallId"].as_str(), u["kind"].as_str()) { kinds.insert(id.to_string(), k.to_string()); }
                        if want_accept_edits && u["sessionUpdate"] == "current_mode_update" && u["currentModeId"] != "acceptEdits" {
                            want_accept_edits = false;
                            let r = p.conn.request("session/set_mode", json!({"sessionId": sid, "modeId": "acceptEdits"})).await;
                            println!("-> set_mode acceptEdits after mode update: {:?}", r.map(|_| "ok"));
                        }
                        on_update(u, &kinds, &mut text_buf, &mut reads, &mut writes);
                    }
                    Incoming::Notification { method, params } => println!("\n[notify {method}] {params}"),
                    Incoming::Request { id, method, params } if method == "session/request_permission" => {
                        flush(&mut text_buf);
                        let options = acp::permission_options(&params);
                        println!("\n[permission] {} kind={} options={:?}",
                            params["toolCall"]["title"], params["toolCall"]["kind"],
                            options.iter().map(|o| format!("{}={}", o.option_id, o.name)).collect::<Vec<_>>());
                        if acp::is_plan_request(&params) {
                            let plan = acp::plan_text(&params).unwrap_or_default();
                            println!("---- PLAN ----\n{plan}\n--------------");
                            let parsed = focuside_core::plan::parse(&plan);
                            println!("parsed plan: {parsed:#?}");
                            if o.discard {
                                let rej = acp::reject_option(&options).unwrap_or_default();
                                p.conn.respond(id, Ok(acp::permission_selected(&rej)));
                                p.conn.notify("session/cancel", json!({"sessionId": sid}));
                                println!("-> discard ({rej}) and cancel");
                            } else if let Some((opt, switch)) = acp::run_plan_option(&options) {
                                p.conn.respond(id, Ok(acp::permission_selected(&opt)));
                                println!("-> run plan with {opt}");
                                if switch { want_accept_edits = true; }
                            }
                        } else {
                            let allow = options.iter().find(|o| o.kind == "allow_once").or(options.first());
                            let id_opt = allow.map(|o| o.option_id.clone()).unwrap_or_default();
                            println!("-> allow once ({id_opt})");
                            p.conn.respond(id, Ok(acp::permission_selected(&id_opt)));
                        }
                    }
                    Incoming::Request { id, method, .. } => {
                        println!("\n[request {method}] not supported");
                        p.conn.respond(id, Err(acp::RpcError::method_not_found(&method)));
                    }
                    Incoming::Closed => {
                        println!("\n[agent closed] {}", p.stderr_tail());
                    }
                }
            }
        }
    };
    flush(&mut text_buf);
    match result {
        Ok(r) => println!(
            "\n== stopReason: {} in {:?}",
            r["stopReason"],
            started.elapsed()
        ),
        Err(e) => println!("\n== prompt error: {e}"),
    }
    println!("reads:  {reads:?}");
    println!("writes: {writes:?}");
    println!("session id for resume: {sid}");
    let before = p.tree.pids();
    p.tree.kill().await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    let alive: Vec<u32> = before
        .into_iter()
        .filter(|x| focuside_core::proc::is_alive(*x))
        .collect();
    println!("left after kill: {alive:?}");
    Ok(())
}

fn flush(buf: &mut String) {
    if !buf.is_empty() {
        println!("{buf}");
        buf.clear();
    }
}

fn on_update(
    u: &Value,
    kinds: &std::collections::HashMap<String, String>,
    text: &mut String,
    reads: &mut BTreeSet<String>,
    writes: &mut BTreeSet<String>,
) {
    let kind = u["sessionUpdate"].as_str().unwrap_or("");
    match kind {
        "agent_message_chunk" => {
            if text.is_empty() {
                print!("\n[agent] ");
            }
            text.push_str(u["content"]["text"].as_str().unwrap_or(""));
        }
        "agent_thought_chunk" => {}
        "tool_call" | "tool_call_update" => {
            flush(text);
            let locs: Vec<String> = u["locations"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|l| l["path"].as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            let tk = u["toolCallId"]
                .as_str()
                .and_then(|id| kinds.get(id))
                .map(|s| s.as_str())
                .unwrap_or("");
            for l in &locs {
                match tk {
                    "read" | "search" => {
                        reads.insert(l.clone());
                    }
                    "edit" | "delete" | "move" => {
                        writes.insert(l.clone());
                    }
                    _ => {}
                }
            }
            println!(
                "[{kind}] id={} kind={tk} status={} title={} locations={locs:?}",
                u["toolCallId"].as_str().unwrap_or(""),
                u["status"].as_str().unwrap_or(""),
                u["title"].as_str().unwrap_or(""),
            );
        }
        "plan" => {
            flush(text);
            let items: Vec<String> = u["entries"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .map(|e| {
                            format!(
                                "{}:{}",
                                e["status"].as_str().unwrap_or(""),
                                e["content"].as_str().unwrap_or("")
                            )
                        })
                        .collect()
                })
                .unwrap_or_default();
            println!("[todo] {items:?}");
        }
        other => {
            flush(text);
            let s = u.to_string();
            println!("[{other}] {}", &s[..s.len().min(300)]);
        }
    }
}
