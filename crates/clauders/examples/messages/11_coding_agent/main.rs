//! An agentic coding CLI with a terminal UI, built on the Messages SDK.
//!
//! Same agent as the plain loop it grew from: the model has three real tools —
//! write files, read them back, and run `cargo` — so it can scaffold a small
//! Rust program, compile it, read the compiler errors, fix its own code, and
//! repeat until the program builds and runs.
//!
//! What this version adds is a **ratatui** terminal UI. The agent runs in a
//! background task and streams UI events over a channel; the foreground draws a
//! live transcript pane (colour-coded by who/what produced each line), a status
//! line with a spinner while the model is thinking, and a help bar. You watch
//! the agent work and scroll back through what it did.
//!
//! Nothing the agent does escapes a **sandbox directory**
//! (`./coding-agent-workspace` by default): every tool path is forced to stay
//! inside it and `cargo` runs with its working directory pinned there.
//!
//! Run:
//!
//! ```text
//! ANTHROPIC_API_KEY=sk-... cargo run --example 11_coding_agent
//! # or give it your own task:
//! ANTHROPIC_API_KEY=sk-... cargo run --example 11_coding_agent -- "write a Rust program that prints the first 20 primes"
//! ```
//!
//! Keys: `q`/`Esc` quit · `↑`/`↓` and `PageUp`/`PageDown` scroll · `End` follow.

use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use clauders::messages::MessageContent;
use clauders::messages::tools::{Tool, ToolChoice, ToolResultBlock};
use clauders::prelude::*;
use clauders::types::{SystemPrompt, ToolName};
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Paragraph, Wrap};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

/// Where the agent is allowed to work. Created if missing; never wiped.
const WORKSPACE: &str = "coding-agent-workspace";
/// Hard cap on agent turns so a stuck model cannot loop forever.
const MAX_TURNS: usize = 20;
/// Trim tool output before feeding it back, to keep token cost bounded.
const MAX_TOOL_OUTPUT: usize = 8_000;
/// Spinner frames, advanced each UI tick while the model is working.
const SPINNER: [char; 4] = ['|', '/', '-', '\\'];

const SYSTEM: &str = "\
You are a Rust coding agent working inside a sandbox directory. You have three tools:
- `write_file`  — create or overwrite a file (path relative to the sandbox root).
- `read_file`   — read a file back.
- `run_cargo`   — run a cargo subcommand in the sandbox (e.g. init, build, run, test).

Work step by step: scaffold the project with `cargo init`, write the code with
`write_file`, then compile and run it with `run_cargo`. If cargo reports errors,
read them, fix the file, and try again. When the program builds and runs
correctly, stop and give a short summary of what you built and how to run it.";

/// A message from the background agent to the UI.
enum AgentEvent {
    /// Replace the status line (what the agent is doing right now).
    Status(String),
    /// Append one styled entry to the transcript.
    Line(Kind, String),
    /// The agent has stopped; no more events will arrive.
    Done,
}

/// Who or what produced a transcript entry — drives its colour.
#[derive(Clone, Copy)]
enum Kind {
    User,
    Model,
    Tool,
    Ok,
    Err,
    Info,
}

impl Kind {
    fn style(self) -> Style {
        let color = match self {
            Self::User => Color::Cyan,
            Self::Model => Color::White,
            Self::Tool => Color::Yellow,
            Self::Ok => Color::Green,
            Self::Err => Color::Red,
            Self::Info => Color::DarkGray,
        };
        Style::default().fg(color)
    }

    const fn tag(self) -> &'static str {
        match self {
            Self::User => "you  ",
            Self::Model => "model",
            Self::Tool => "tool ",
            Self::Ok => "ok   ",
            Self::Err => "err  ",
            Self::Info => "info ",
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read the API key and build the client *before* entering the TUI, so any
    // setup error prints normally instead of being swallowed by raw mode.
    let api_key = ApiKey::new(std::env::var("ANTHROPIC_API_KEY")?)?;
    let client = Client::builder()?.api_key(api_key).build()?;

    // The coding task: everything after `--`, or a sensible default.
    let args: Vec<String> = std::env::args().skip(1).collect();
    let task = if args.is_empty() {
        "Create a Rust binary crate named `fizzbuzz` that prints FizzBuzz for \
         1..=20, then build and run it to prove it works."
            .to_string()
    } else {
        args.join(" ")
    };

    // Fresh-ish sandbox: create it if absent, reuse it otherwise.
    let root = std::env::current_dir()?.join(WORKSPACE);
    std::fs::create_dir_all(&root)?;

    // Run the agent in the background; it streams events to the UI.
    let (tx, rx) = mpsc::unbounded_channel();
    tokio::spawn(run_agent(client, task, root, tx));

    // The UI owns the terminal on the main thread. The multi-threaded runtime
    // keeps servicing the agent task while this blocks on input polling.
    let mut terminal = ratatui::init();
    let outcome = run_ui(&mut terminal, rx);
    ratatui::restore();
    outcome.map_err(Into::into)
}

// ---------------------------------------------------------------------------
// UI
// ---------------------------------------------------------------------------

/// Everything the UI draws.
struct App {
    lines: Vec<Line<'static>>,
    status: String,
    /// Manual scroll offset from the top, in lines.
    scroll: u16,
    /// While true, the view sticks to the newest line.
    follow: bool,
    spinner: usize,
    done: bool,
}

impl App {
    fn new() -> Self {
        Self {
            lines: Vec::new(),
            status: "starting…".to_string(),
            scroll: 0,
            follow: true,
            spinner: 0,
            done: false,
        }
    }

    /// Append an entry, splitting multi-line text into styled rows.
    fn push(&mut self, kind: Kind, text: &str) {
        for (i, row) in text.lines().enumerate() {
            let prefix = if i == 0 { kind.tag() } else { "     " };
            self.lines
                .push(Line::styled(format!("{prefix} {row}"), kind.style()));
        }
    }
}

/// The UI loop: draw, poll keys, drain agent events. Returns when the user quits.
fn run_ui(
    terminal: &mut DefaultTerminal,
    mut rx: UnboundedReceiver<AgentEvent>,
) -> std::io::Result<()> {
    let mut app = App::new();
    loop {
        terminal.draw(|frame| draw(frame, &app))?;

        // Poll for a keystroke; the timeout also paces the spinner.
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                        KeyCode::Up => {
                            app.follow = false;
                            app.scroll = app.scroll.saturating_sub(1);
                        }
                        KeyCode::Down => app.scroll = app.scroll.saturating_add(1),
                        KeyCode::PageUp => {
                            app.follow = false;
                            app.scroll = app.scroll.saturating_sub(10);
                        }
                        KeyCode::PageDown => app.scroll = app.scroll.saturating_add(10),
                        KeyCode::End => app.follow = true,
                        _ => {}
                    }
                }
            }
        }

        // Drain everything the agent has sent since the last frame.
        while let Ok(ev) = rx.try_recv() {
            match ev {
                AgentEvent::Status(s) => app.status = s,
                AgentEvent::Line(kind, text) => app.push(kind, &text),
                AgentEvent::Done => app.done = true,
            }
        }

        app.spinner = (app.spinner + 1) % SPINNER.len();
    }
}

fn draw(frame: &mut Frame, app: &App) {
    let areas = Layout::vertical([
        Constraint::Length(1), // status
        Constraint::Min(0),    // transcript
        Constraint::Length(1), // help
    ])
    .split(frame.area());

    // Status line: spinner + what the agent is doing, or a done marker.
    let status = if app.done {
        Line::styled(
            format!("● {}", app.status),
            Style::default().fg(Color::Green),
        )
    } else {
        Line::styled(
            format!("{} {}", SPINNER[app.spinner], app.status),
            Style::default().fg(Color::Cyan),
        )
    };
    frame.render_widget(Paragraph::new(status), areas[0]);

    // Transcript, bordered, with follow-to-bottom or manual scroll.
    let block = Block::bordered().title(" coding agent ");
    let inner_height = block.inner(areas[1]).height;
    let total = u16::try_from(app.lines.len()).unwrap_or(u16::MAX);
    let max_scroll = total.saturating_sub(inner_height);
    let scroll = if app.follow {
        max_scroll
    } else {
        app.scroll.min(max_scroll)
    };
    let transcript = Paragraph::new(app.lines.clone())
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));
    frame.render_widget(transcript, areas[1]);

    let help = Line::styled(
        " q/Esc quit · ↑/↓ PgUp/PgDn scroll · End follow",
        Style::default().fg(Color::DarkGray),
    );
    frame.render_widget(Paragraph::new(help), areas[2]);
}

// ---------------------------------------------------------------------------
// Agent
// ---------------------------------------------------------------------------

/// Drive the agent loop, reporting progress to the UI. Always sends `Done`.
async fn run_agent(client: Client, task: String, root: PathBuf, tx: UnboundedSender<AgentEvent>) {
    let _ = tx.send(AgentEvent::Line(
        Kind::Info,
        format!("workspace: {}", root.display()),
    ));
    let _ = tx.send(AgentEvent::Line(Kind::User, task.clone()));

    if let Err(err) = agent_loop(&client, task, &root, &tx).await {
        let _ = tx.send(AgentEvent::Line(Kind::Err, err));
    }
    let _ = tx.send(AgentEvent::Done);
}

/// The Messages-SDK agent loop. Errors bubble up so the wrapper can report them.
async fn agent_loop(
    client: &Client,
    task: String,
    root: &Path,
    tx: &UnboundedSender<AgentEvent>,
) -> Result<(), String> {
    let tools = tool_defs().map_err(|e| e.to_string())?;
    let mut history: Vec<(Role, MessageContent)> = vec![(Role::User, MessageContent::Text(task))];

    for turn in 1..=MAX_TURNS {
        let _ = tx.send(AgentEvent::Status(format!(
            "turn {turn}/{MAX_TURNS} — waiting for the model…"
        )));

        let mut builder = MessageRequest::builder()
            .model(ModelId::claude_sonnet_4_5())
            .max_tokens(MaxTokens::new(2048))
            .system(SystemPrompt::text(SYSTEM));
        for (role, content) in &history {
            builder = builder.add_message(*role, content.clone());
        }
        let req = builder
            .tools(tools.clone())
            .tool_choice(ToolChoice::Auto)
            .build();

        let msg = client
            .messages()
            .create(req)
            .await
            .map_err(|e| e.to_string())?;

        for block in &msg.content {
            if let ContentBlock::Text(t) = block {
                let _ = tx.send(AgentEvent::Line(Kind::Model, t.text.clone()));
            }
        }

        history.push((
            Role::Assistant,
            MessageContent::Blocks(
                ContentBlockParam::try_from_response(msg.content.clone())
                    .map_err(|e| e.to_string())?,
            ),
        ));

        // Any stop reason other than tool_use means the agent is finished.
        if msg.stop_reason != Some(StopReason::ToolUse) {
            let _ = tx.send(AgentEvent::Status("done".to_string()));
            return Ok(());
        }

        // Execute every tool call and feed the results back as one user turn.
        let mut results = Vec::new();
        for block in &msg.content {
            let ContentBlock::ToolUse(tu) = block else {
                continue;
            };
            let _ = tx.send(AgentEvent::Status(format!(
                "turn {turn}/{MAX_TURNS} — running {}",
                tu.name.as_str()
            )));
            let _ = tx.send(AgentEvent::Line(
                Kind::Tool,
                format!("→ {}({})", tu.name.as_str(), compact(&tu.input)),
            ));
            let result = match run_tool(root, tu.name.as_str(), &tu.input).await {
                Ok(out) => {
                    let _ = tx.send(AgentEvent::Line(Kind::Ok, summarize(&out)));
                    ToolResultBlock::text(tu.id.clone(), truncate(out))
                }
                Err(err) => {
                    let _ = tx.send(AgentEvent::Line(Kind::Err, err.clone()));
                    ToolResultBlock::err(tu.id.clone(), err)
                }
            };
            results.push(ContentBlockParam::ToolResult(result));
        }
        history.push((Role::User, MessageContent::Blocks(results)));
    }

    Err(format!("hit the {MAX_TURNS}-turn cap without finishing"))
}

/// The three tools the agent can call.
fn tool_defs() -> Result<Vec<Tool>, Box<dyn std::error::Error>> {
    let mk = |name: &str, description: &str, schema: serde_json::Value| -> Result<Tool, _> {
        ToolName::new(name).map(|name| Tool {
            name,
            description: description.into(),
            input_schema: schema,
            cache_control: None,
            strict: None,
            eager_input_streaming: None,
        })
    };

    Ok(vec![
        mk(
            "write_file",
            "Create or overwrite a text file inside the sandbox.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "relative to sandbox root" },
                    "contents": { "type": "string" }
                },
                "required": ["path", "contents"]
            }),
        )?,
        mk(
            "read_file",
            "Read a text file from inside the sandbox.",
            serde_json::json!({
                "type": "object",
                "properties": { "path": { "type": "string" } },
                "required": ["path"]
            }),
        )?,
        mk(
            "run_cargo",
            "Run a cargo subcommand in the sandbox, e.g. [\"init\"], [\"run\"], [\"test\"].",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "args": { "type": "array", "items": { "type": "string" } }
                },
                "required": ["args"]
            }),
        )?,
    ])
}

/// Dispatch one tool call. `Ok` becomes a tool result, `Err` an error result the
/// model can read and recover from.
async fn run_tool(root: &Path, name: &str, input: &serde_json::Value) -> Result<String, String> {
    match name {
        "write_file" => {
            let path = resolve(root, str_field(input, "path")?)?;
            let contents = str_field(input, "contents")?;
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::write(&path, contents).map_err(|e| e.to_string())?;
            Ok(format!(
                "wrote {} bytes to {}",
                contents.len(),
                path.display()
            ))
        }
        "read_file" => {
            let path = resolve(root, str_field(input, "path")?)?;
            std::fs::read_to_string(&path).map_err(|e| e.to_string())
        }
        "run_cargo" => {
            let args = input
                .get("args")
                .and_then(serde_json::Value::as_array)
                .ok_or("`args` must be an array of strings")?
                .iter()
                .map(|v| {
                    v.as_str()
                        .map(str::to_owned)
                        .ok_or("each arg must be a string")
                })
                .collect::<Result<Vec<_>, _>>()?;
            let out = tokio::process::Command::new("cargo")
                .args(&args)
                .current_dir(root)
                .output()
                .await
                .map_err(|e| e.to_string())?;
            Ok(format!(
                "exit: {}\n--- stdout ---\n{}\n--- stderr ---\n{}",
                out.status,
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr),
            ))
        }
        other => Err(format!("unknown tool: {other}")),
    }
}

/// Join `rel` onto the sandbox root, refusing anything that could escape it.
fn resolve(root: &Path, rel: &str) -> Result<PathBuf, String> {
    let candidate = Path::new(rel);
    if candidate.is_absolute()
        || candidate
            .components()
            .any(|c| matches!(c, Component::ParentDir))
    {
        return Err(format!(
            "path must be relative and stay inside the sandbox: {rel}"
        ));
    }
    Ok(root.join(candidate))
}

fn str_field<'a>(input: &'a serde_json::Value, key: &str) -> Result<&'a str, String> {
    input
        .get(key)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| format!("missing string field `{key}`"))
}

/// One-line rendering of a tool input for the transcript.
fn compact(input: &serde_json::Value) -> String {
    truncate_to(input.to_string(), 100)
}

/// Condense a tool's (possibly huge) output to its first line for the transcript.
fn summarize(out: &str) -> String {
    let first = out.lines().next().unwrap_or_default();
    truncate_to(first.to_string(), 100)
}

fn truncate(s: String) -> String {
    truncate_to(s, MAX_TOOL_OUTPUT)
}

fn truncate_to(s: String, max: usize) -> String {
    if s.len() <= max {
        return s;
    }
    // Cut on a char boundary at or before `max`.
    let mut end = max;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}… [truncated {} bytes]", &s[..end], s.len() - end)
}
