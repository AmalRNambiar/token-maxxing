// Polls local client session logs and emits aggregated token usage to the UI.
//
// Adapter model: each supported client writes JSONL session logs locally. We
// tail those files (tracking a byte offset per file) and normalize every new
// line into token deltas, summed per (client, model).
//
// Semantics: usage is the *all-time total* across every session log on disk. On
// first sight of a file we read it whole; the logs are the source of truth, so
// totals are recomputed from scratch each launch (no persistence, no double
// counting within a run).

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};

const POLL_INTERVAL: Duration = Duration::from_millis(1500);

#[derive(Default, Clone)]
struct Totals {
    input: u64,
    output: u64,
}

#[derive(Serialize, Clone)]
struct UsageRow {
    client: String,
    model: String,
    input: u64,
    output: u64,
    total: u64,
}

#[derive(Serialize, Clone)]
struct Payload {
    today: u64,
    total: u64,
    rows: Vec<UsageRow>,
}

pub fn run(app: AppHandle) {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return,
    };
    let claude_glob = format!("{}/.claude/projects/**/*.jsonl", home.display());
    let codex_glob = format!("{}/.codex/sessions/**/rollout-*.jsonl", home.display());

    // Per-file byte offset already consumed.
    let mut offsets: HashMap<PathBuf, u64> = HashMap::new();
    // Codex reports *cumulative* usage per session; track last seen to derive deltas.
    let mut codex_last: HashMap<PathBuf, (u64, u64)> = HashMap::new();
    // Resolved model name per Codex session file.
    let mut codex_model: HashMap<PathBuf, String> = HashMap::new();
    // Aggregated all-time totals keyed by (client, model).
    let mut totals: HashMap<(String, String), Totals> = HashMap::new();
    // Tokens used today (UTC); resets when the date rolls over.
    let mut today_total: u64 = 0;
    let mut today_date = utc_today();

    loop {
        // Re-assert always-on-top: some Linux WMs drop the hint on focus changes.
        if let Some(win) = app.get_webview_window("main") {
            let _ = win.set_always_on_top(true);
            let _ = win.set_visible_on_all_workspaces(true);
        }

        // Midnight rollover: drop yesterday's tally; new events accrue fresh.
        let now_date = utc_today();
        if now_date != today_date {
            today_date = now_date;
            today_total = 0;
        }

        let mut changed = false;

        // ---- Claude Code ----
        for path in glob::glob(&claude_glob).into_iter().flatten().flatten() {
            if let Some(chunk) = read_new(&path, &mut offsets) {
                for line in chunk.lines() {
                    if let Some((model, inp, out)) = parse_claude_line(line) {
                        let t = totals.entry(("claude".into(), model)).or_default();
                        t.input += inp;
                        t.output += out;
                        if line_is_today(line, &today_date) {
                            today_total += inp + out;
                        }
                        changed = true;
                    }
                }
            }
        }

        // ---- Codex CLI ----
        for path in glob::glob(&codex_glob).into_iter().flatten().flatten() {
            if let Some(chunk) = read_new(&path, &mut offsets) {
                if !codex_model.contains_key(&path) {
                    let m = scan_codex_model(&path).unwrap_or_else(|| "codex".into());
                    codex_model.insert(path.clone(), m);
                }
                // A chunk may also carry a fresher model line.
                if let Some(m) = find_codex_model(&chunk) {
                    codex_model.insert(path.clone(), m);
                }
                let model = codex_model.get(&path).cloned().unwrap_or_else(|| "codex".into());

                for line in chunk.lines() {
                    if let Some((cum_in, cum_out)) = parse_codex_usage(line) {
                        // Baseline 0 on first sample => first event counts the
                        // session's full cumulative usage (all-time).
                        let (pi, po) = codex_last.get(&path).copied().unwrap_or((0, 0));
                        let (din, dout) =
                            (cum_in.saturating_sub(pi), cum_out.saturating_sub(po));
                        codex_last.insert(path.clone(), (cum_in, cum_out));
                        if din > 0 || dout > 0 {
                            let t = totals.entry(("codex".into(), model.clone())).or_default();
                            t.input += din;
                            t.output += dout;
                            if line_is_today(line, &today_date) {
                                today_total += din + dout;
                            }
                            changed = true;
                        }
                    }
                }
            }
        }

        if changed {
            let mut rows: Vec<UsageRow> = totals
                .iter()
                .map(|((client, model), t)| UsageRow {
                    client: client.clone(),
                    model: model.clone(),
                    input: t.input,
                    output: t.output,
                    total: t.input + t.output,
                })
                .collect();
            rows.sort_by(|a, b| b.total.cmp(&a.total));
            let total = rows.iter().map(|r| r.total).sum();
            let _ = app.emit(
                "usage-update",
                Payload {
                    today: today_total,
                    total,
                    rows,
                },
            );
        }

        std::thread::sleep(POLL_INTERVAL);
    }
}

/// Read bytes appended since the last poll. Returns only complete lines.
/// On first sight of a file, reads it whole (offset 0) to capture all history.
fn read_new(path: &PathBuf, offsets: &mut HashMap<PathBuf, u64>) -> Option<String> {
    let size = std::fs::metadata(path).ok()?.len();

    // First sight => start at 0 so the entire file is read.
    let off = offsets.get(path).copied().unwrap_or(0);

    if size < off {
        // File truncated or rotated; resync.
        offsets.insert(path.clone(), size);
        return None;
    }
    if size == off {
        return None;
    }

    let mut f = File::open(path).ok()?;
    f.seek(SeekFrom::Start(off)).ok()?;
    let mut buf = String::new();
    f.read_to_string(&mut buf).ok()?;

    // Only consume through the last newline so we never parse a partial line.
    let idx = buf.rfind('\n')?;
    let complete = &buf[..=idx];
    offsets.insert(path.clone(), off + complete.len() as u64);
    Some(complete.to_string())
}

/// Parse a Claude Code assistant line -> (model, input_total, output_total).
fn parse_claude_line(line: &str) -> Option<(String, u64, u64)> {
    let v: Value = serde_json::from_str(line).ok()?;
    if v.get("type")?.as_str()? != "assistant" {
        return None;
    }
    let msg = v.get("message")?;
    let model = msg.get("model")?.as_str()?.to_string();
    let u = msg.get("usage")?;
    let input = field(u, "input_tokens")
        + field(u, "cache_read_input_tokens")
        + field(u, "cache_creation_input_tokens");
    let output = field(u, "output_tokens");
    Some((model, input, output))
}

/// Parse a Codex line carrying cumulative usage -> (cum_input, cum_output).
fn parse_codex_usage(line: &str) -> Option<(u64, u64)> {
    let v: Value = serde_json::from_str(line).ok()?;
    let payload = v.get("payload").unwrap_or(&v);
    let ttu = payload.get("info")?.get("total_token_usage")?;
    Some((field(ttu, "input_tokens"), field(ttu, "output_tokens")))
}

/// Look for a model name within a freshly read chunk.
fn find_codex_model(chunk: &str) -> Option<String> {
    chunk.lines().find_map(|line| {
        let v: Value = serde_json::from_str(line).ok()?;
        let payload = v.get("payload").unwrap_or(&v);
        payload.get("model")?.as_str().map(|s| s.to_string())
    })
}

/// One-time full scan of a Codex session file for its model name.
fn scan_codex_model(path: &PathBuf) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    find_codex_model(&content)
}

fn field(v: &Value, key: &str) -> u64 {
    v.get(key).and_then(Value::as_u64).unwrap_or(0)
}

/// Current UTC date as "YYYY-MM-DD".
fn utc_today() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

/// True if the line's top-level ISO `timestamp` falls on `today` (UTC date).
fn line_is_today(line: &str, today: &str) -> bool {
    serde_json::from_str::<Value>(line)
        .ok()
        .and_then(|v| {
            v.get("timestamp")
                .and_then(Value::as_str)
                .map(|ts| ts.len() >= 10 && &ts[..10] == today)
        })
        .unwrap_or(false)
}
