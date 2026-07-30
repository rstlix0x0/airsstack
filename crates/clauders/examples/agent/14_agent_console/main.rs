//! An interactive agent console: a terminal UI over a live `claude` session.
//!
//! This is the capstone. Everything the earlier examples showed in isolation is
//! wired together into something usable:
//!
//! - a long-lived `Client` in a background task, streaming frames to the UI;
//! - a `PermissionPolicy` that does not decide anything itself — it hands the
//!   request to the UI and waits for a keystroke, which is how a real host
//!   turns tool gating into a user-facing prompt;
//! - a `PreToolUse` hook that logs every tool the agent reaches for;
//! - `interrupt()` bound to a key, so a runaway turn can be cut short;
//! - `set_model()` bound to a slash command, changing the model mid-session;
//! - live cost and token counters read off each result frame.
//!
//! Type a prompt and press Enter. While the agent is working, `Ctrl+X`
//! interrupts. When the agent asks permission for a tool, press `y` or `n`.
//!
//! Run:
//!
//! ```text
//! cargo run -p clauders --example agent_14_agent_console
//! ```
//!
//! Keys: `Enter` send · `Ctrl+X` interrupt · `Ctrl+C` quit · `↑`/`↓`,
//! `PgUp`/`PgDn` scroll · `End` follow. Commands: `/model haiku|sonnet|opus`,
//! `/quit`.

use std::time::Duration;

use clauders::agent::cancel::CancelSignal;
use clauders::agent::error::AgentError;
use clauders::agent::{
    Client, ContentBlock, Hook, HookEvent, HookInput, HookOutput, Message, Options,
    PermissionContext, PermissionDecision, PermissionMode, PermissionPolicy,
};
use clauders::types::ModelId;
use futures_util::StreamExt;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Paragraph, Wrap};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::sync::oneshot;

/// Spinner frames shown while a turn is in flight.
const SPINNER: [&str; 4] = ["|", "/", "-", "\\"];

// ---------------------------------------------------------------------------
// Messages between the UI and the agent task
// ---------------------------------------------------------------------------

/// What the agent task tells the UI.
enum UiEvent {
    /// A transcript line.
    Line(Kind, String),
    /// Replace the status line.
    Status(String),
    /// A tool call needs a yes/no answer; the sender carries it back.
    Permission {
        /// Tool the agent wants to run.
        tool: String,
        /// Arguments it wants to run with, rendered compactly.
        detail: String,
        /// Channel the UI answers on. `true` allows.
        reply: oneshot::Sender<bool>,
    },
    /// The turn ended; totals to show in the status bar.
    TurnDone {
        /// Cumulative session cost in USD, when the binary reported one.
        cost_usd: Option<f64>,
        /// Turns taken by the run that just finished.
        turns: u32,
    },
}

/// What the UI asks the agent task to do.
enum Command {
    /// Send a user turn.
    Prompt(String),
    /// Interrupt whatever is running.
    Interrupt,
    /// Switch the active model.
    SetModel(ModelId),
    /// Shut the session down.
    Quit,
}

/// Transcript line categories, which is all the styling this UI needs.
#[derive(Clone, Copy)]
enum Kind {
    /// A prompt the user sent.
    User,
    /// Model prose.
    Agent,
    /// Summarized model thinking.
    Thinking,
    /// A tool the agent asked to run.
    Tool,
    /// The outcome of a tool call.
    ToolOut,
    /// A permission question or its answer.
    Gate,
    /// SDK/session information.
    Info,
    /// Something failed.
    Error,
}

impl Kind {
    /// Fixed-width tag printed at the start of the line.
    const fn tag(self) -> &'static str {
        match self {
            Self::User => "you  ",
            Self::Agent => "claude",
            Self::Thinking => "think",
            Self::Tool => "tool ",
            Self::ToolOut => "out  ",
            Self::Gate => "gate ",
            Self::Info => "info ",
            Self::Error => "error",
        }
    }

    /// Colour for this category.
    fn style(self) -> Style {
        let colour = match self {
            Self::User => Color::Cyan,
            Self::Agent => Color::White,
            Self::Thinking => Color::Magenta,
            Self::Tool => Color::Yellow,
            Self::ToolOut => Color::Green,
            Self::Gate => Color::LightRed,
            Self::Info => Color::DarkGray,
            Self::Error => Color::Red,
        };
        Style::default().fg(colour)
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// Routes every permission request to the UI and blocks on the answer.
struct AskTheUser {
    /// Channel to the UI.
    tx: UnboundedSender<UiEvent>,
}

#[async_trait::async_trait]
impl PermissionPolicy for AskTheUser {
    async fn can_use_tool(
        &self,
        tool: &str,
        input: &serde_json::Value,
        _ctx: PermissionContext,
        cancel: CancelSignal,
    ) -> Result<PermissionDecision, AgentError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        let detail = compact_json(input);
        if self
            .tx
            .send(UiEvent::Permission {
                tool: tool.to_owned(),
                detail,
                reply: reply_tx,
            })
            .is_err()
        {
            // The UI is gone; refusing is the safe default.
            return Ok(PermissionDecision::deny("console closed"));
        }

        // Wait for the keystroke, but give up if the binary withdraws the
        // request first. Cancellation is cooperative: nothing kills this task.
        tokio::select! {
            answer = reply_rx => match answer {
                Ok(true) => Ok(PermissionDecision::allow()),
                Ok(false) => Ok(PermissionDecision::deny("denied at the console")),
                Err(_) => Ok(PermissionDecision::deny("console closed")),
            },
            () = cancel.cancelled() => Err(AgentError::Interrupted),
        }
    }
}

/// Logs each tool call to the transcript without changing the outcome.
struct LogTools {
    /// Channel to the UI.
    tx: UnboundedSender<UiEvent>,
}

#[async_trait::async_trait]
impl Hook for LogTools {
    async fn call(
        &self,
        input: HookInput,
        _cancel: CancelSignal,
    ) -> Result<HookOutput, AgentError> {
        let name = input
            .data
            .get("tool_name")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("<tool>");
        let _ = self.tx.send(UiEvent::Line(
            Kind::Info,
            format!("hook: about to run {name}"),
        ));
        Ok(HookOutput::default())
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (ui_tx, ui_rx) = mpsc::unbounded_channel();
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();

    let options = Options::builder()
        .system_prompt(
            "You are running inside a small terminal console. Keep answers short. \
             Use tools when they help; the user approves each one.",
        )
        // `Default` is what routes tool calls through the policy below.
        .permission_mode(PermissionMode::Default)
        .allowed_tools(vec![
            "Bash".to_owned(),
            "Read".to_owned(),
            "Glob".to_owned(),
            "Grep".to_owned(),
        ])
        .permission_policy(std::sync::Arc::new(AskTheUser { tx: ui_tx.clone() }))
        .hook(
            HookEvent::PreToolUse,
            None,
            std::sync::Arc::new(LogTools { tx: ui_tx.clone() }),
        )
        .max_turns(30)
        .build();

    // The agent lives in a background task so the UI can own the terminal.
    tokio::spawn(run_agent(options, cmd_rx, ui_tx));

    let mut terminal = ratatui::init();
    let outcome = run_ui(&mut terminal, ui_rx, &cmd_tx);
    ratatui::restore();
    let _ = cmd_tx.send(Command::Quit);
    outcome.map_err(Into::into)
}

// ---------------------------------------------------------------------------
// UI
// ---------------------------------------------------------------------------

/// Everything the UI draws.
struct App {
    /// Styled transcript rows.
    lines: Vec<Line<'static>>,
    /// Current status text.
    status: String,
    /// What the user is typing.
    input: String,
    /// Manual scroll offset from the top, in rows.
    scroll: u16,
    /// While true, the view sticks to the newest row.
    follow: bool,
    /// Spinner frame index.
    spinner: usize,
    /// True while a turn is in flight.
    busy: bool,
    /// The pending permission question, if any.
    pending: Option<(String, oneshot::Sender<bool>)>,
    /// Cumulative session cost, when reported.
    cost_usd: Option<f64>,
}

impl App {
    /// A console with an empty transcript.
    fn new() -> Self {
        Self {
            lines: Vec::new(),
            status: "connecting…".to_owned(),
            input: String::new(),
            scroll: 0,
            follow: true,
            spinner: 0,
            busy: false,
            pending: None,
            cost_usd: None,
        }
    }

    /// Append an entry, splitting multi-line text into styled rows.
    fn push(&mut self, kind: Kind, text: &str) {
        for (index, row) in text.lines().enumerate() {
            let prefix = if index == 0 { kind.tag() } else { "     " };
            self.lines
                .push(Line::styled(format!("{prefix} {row}"), kind.style()));
        }
    }
}

/// The UI loop: draw, read keys, drain agent events. Returns when the user quits.
fn run_ui(
    terminal: &mut DefaultTerminal,
    mut rx: UnboundedReceiver<UiEvent>,
    cmd: &UnboundedSender<Command>,
) -> std::io::Result<()> {
    let mut app = App::new();
    app.push(Kind::Info, "type a prompt and press Enter");

    loop {
        terminal.draw(|frame| draw(frame, &app))?;

        // The poll timeout also paces the spinner.
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press
                    && !handle_key(&mut app, key.code, key.modifiers, cmd)
                {
                    return Ok(());
                }
            }
        }

        while let Ok(event) = rx.try_recv() {
            apply_event(&mut app, event);
        }

        app.spinner = (app.spinner + 1) % SPINNER.len();
    }
}

/// Handle one keystroke. Returns `false` when the console should exit.
fn handle_key(
    app: &mut App,
    code: KeyCode,
    modifiers: KeyModifiers,
    cmd: &UnboundedSender<Command>,
) -> bool {
    // A pending permission question takes over y/n before anything else.
    if app.pending.is_some() {
        if let KeyCode::Char(answer @ ('y' | 'n')) = code {
            if let Some((tool, reply)) = app.pending.take() {
                let allow = answer == 'y';
                let _ = reply.send(allow);
                app.push(
                    Kind::Gate,
                    &format!("{} {tool}", if allow { "allowed" } else { "denied" }),
                );
            }
            return true;
        }
    }

    if modifiers.contains(KeyModifiers::CONTROL) {
        match code {
            KeyCode::Char('c') => return false,
            KeyCode::Char('x') => {
                let _ = cmd.send(Command::Interrupt);
                app.push(Kind::Info, "interrupt requested");
            }
            _ => {}
        }
        return true;
    }

    match code {
        KeyCode::Enter => submit(app, cmd),
        KeyCode::Char(ch) => app.input.push(ch),
        KeyCode::Backspace => {
            app.input.pop();
        }
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
    true
}

/// Send the typed line as a prompt, or run it as a slash command.
fn submit(app: &mut App, cmd: &UnboundedSender<Command>) {
    let line = std::mem::take(&mut app.input);
    let line = line.trim();
    if line.is_empty() {
        return;
    }

    if let Some(rest) = line.strip_prefix("/model ") {
        let model = match rest.trim() {
            "haiku" => Some(ModelId::claude_haiku_4_5()),
            "sonnet" => Some(ModelId::claude_sonnet_4_5()),
            "opus" => Some(ModelId::claude_opus_4_8()),
            other => ModelId::custom(other).ok(),
        };
        match model {
            Some(model) => {
                app.push(Kind::Info, &format!("switching to {}", model.as_str()));
                let _ = cmd.send(Command::SetModel(model));
            }
            None => app.push(Kind::Error, &format!("not a model id: {rest}")),
        }
        return;
    }
    if line == "/quit" {
        let _ = cmd.send(Command::Quit);
        app.push(Kind::Info, "closing the session");
        return;
    }

    app.push(Kind::User, line);
    app.busy = true;
    "working…".clone_into(&mut app.status);
    let _ = cmd.send(Command::Prompt(line.to_owned()));
}

/// Fold one agent event into the UI state.
fn apply_event(app: &mut App, event: UiEvent) {
    match event {
        UiEvent::Line(kind, text) => app.push(kind, &text),
        UiEvent::Status(text) => app.status = text,
        UiEvent::Permission {
            tool,
            detail,
            reply,
        } => {
            app.push(Kind::Gate, &format!("allow {tool}? {detail}  [y/n]"));
            app.pending = Some((tool, reply));
        }
        UiEvent::TurnDone { cost_usd, turns } => {
            app.busy = false;
            if cost_usd.is_some() {
                app.cost_usd = cost_usd;
            }
            app.status = format!("idle · {turns} turn(s)");
        }
    }
}

/// Draw the status bar, transcript, input line, and help bar.
fn draw(frame: &mut Frame, app: &App) {
    let areas = Layout::vertical([
        Constraint::Length(1), // status
        Constraint::Min(0),    // transcript
        Constraint::Length(1), // input
        Constraint::Length(1), // help
    ])
    .split(frame.area());

    let cost = app
        .cost_usd
        .map_or_else(|| "—".to_owned(), |value| format!("${value:.4}"));
    let marker = if app.busy { SPINNER[app.spinner] } else { "*" };
    let status = Line::styled(
        format!("{marker} {} · cost {cost}", app.status),
        Style::default().fg(if app.busy { Color::Cyan } else { Color::Green }),
    );
    frame.render_widget(Paragraph::new(status), areas[0]);

    let block = Block::bordered().title(" agent console ");
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

    let prompt = if app.pending.is_some() {
        Line::styled(
            " answer y or n above".to_owned(),
            Style::default().fg(Color::LightRed),
        )
    } else {
        Line::from(format!("> {}", app.input))
    };
    frame.render_widget(Paragraph::new(prompt), areas[2]);

    let help = Line::styled(
        " Enter send · Ctrl+X interrupt · Ctrl+C quit · PgUp/PgDn scroll · /model <id> · /quit",
        Style::default().fg(Color::DarkGray),
    );
    frame.render_widget(Paragraph::new(help), areas[3]);
}

// ---------------------------------------------------------------------------
// Agent task
// ---------------------------------------------------------------------------

/// Own the session and service commands until told to quit.
async fn run_agent(
    options: Options,
    mut commands: UnboundedReceiver<Command>,
    tx: UnboundedSender<UiEvent>,
) {
    let client = match Client::connect(options).await {
        Ok(client) => client,
        Err(error) => {
            let _ = tx.send(UiEvent::Line(
                Kind::Error,
                format!("connect failed: {error}"),
            ));
            let _ = tx.send(UiEvent::Status("not connected".to_owned()));
            return;
        }
    };

    let _ = tx.send(UiEvent::Line(
        Kind::Info,
        format!(
            "connected · {} model(s), {} command(s) advertised",
            client.supported_models().len(),
            client.supported_commands().len()
        ),
    ));
    let _ = tx.send(UiEvent::Status("idle".to_owned()));

    while let Some(command) = commands.recv().await {
        match command {
            Command::Prompt(text) => run_turn(&client, &text, &tx, &mut commands).await,
            Command::SetModel(model) => {
                let label = model.as_str().to_owned();
                match client.set_model(model).await {
                    Ok(()) => {
                        let _ = tx.send(UiEvent::Line(Kind::Info, format!("model is now {label}")));
                    }
                    Err(error) => {
                        let _ = tx.send(UiEvent::Line(Kind::Error, format!("set_model: {error}")));
                    }
                }
            }
            // Nothing is running, so there is nothing to interrupt.
            Command::Interrupt => {}
            Command::Quit => break,
        }
    }
}

/// Stream one turn, staying responsive to interrupt and quit while it runs.
async fn run_turn(
    client: &Client,
    prompt: &str,
    tx: &UnboundedSender<UiEvent>,
    commands: &mut UnboundedReceiver<Command>,
) {
    let mut stream = match client.query(prompt).await {
        Ok(stream) => stream,
        Err(error) => {
            let _ = tx.send(UiEvent::Line(Kind::Error, format!("query: {error}")));
            let _ = tx.send(UiEvent::TurnDone {
                cost_usd: None,
                turns: 0,
            });
            return;
        }
    };

    loop {
        tokio::select! {
            // Commands keep flowing during the turn — this is what makes
            // Ctrl+X able to reach `interrupt()` mid-stream.
            Some(command) = commands.recv() => match command {
                Command::Interrupt => match client.interrupt().await {
                    Ok(Some(receipt)) => {
                        let _ = tx.send(UiEvent::Line(
                            Kind::Info,
                            format!("interrupted; {} item(s) still queued", receipt.still_queued.len()),
                        ));
                    }
                    Ok(None) => {
                        let _ = tx.send(UiEvent::Line(Kind::Info, "interrupted".to_owned()));
                    }
                    Err(error) => {
                        let _ = tx.send(UiEvent::Line(Kind::Error, format!("interrupt: {error}")));
                    }
                },
                Command::Quit => break,
                // A prompt typed mid-turn is dropped rather than queued.
                Command::Prompt(_) | Command::SetModel(_) => {}
            },
            frame = stream.next() => match frame {
                None => break,
                Some(Err(error)) => {
                    let _ = tx.send(UiEvent::Line(Kind::Error, error.to_string()));
                }
                Some(Ok(message)) => {
                    if forward(message, tx) {
                        return;
                    }
                }
            },
        }
    }

    let _ = tx.send(UiEvent::TurnDone {
        cost_usd: None,
        turns: 0,
    });
}

/// Render one frame into transcript lines. Returns `true` on a result frame,
/// which ends the turn.
fn forward(message: Message, tx: &UnboundedSender<UiEvent>) -> bool {
    match message {
        Message::Assistant(assistant) => {
            for block in &assistant.content {
                match block {
                    ContentBlock::Text { text } => {
                        let _ = tx.send(UiEvent::Line(Kind::Agent, text.clone()));
                    }
                    ContentBlock::Thinking { thinking } => {
                        let _ = tx.send(UiEvent::Line(Kind::Thinking, thinking.clone()));
                    }
                    ContentBlock::ToolUse { name, input, .. } => {
                        let _ = tx.send(UiEvent::Line(
                            Kind::Tool,
                            format!("{name} {}", compact_json(input)),
                        ));
                    }
                    ContentBlock::ToolResult {
                        content, is_error, ..
                    } => {
                        let kind = if *is_error {
                            Kind::Error
                        } else {
                            Kind::ToolOut
                        };
                        let _ = tx.send(UiEvent::Line(kind, compact_json(content)));
                    }
                    _ => {}
                }
            }
            false
        }
        Message::Result(result) => {
            if !result.result.is_empty() {
                let _ = tx.send(UiEvent::Line(
                    Kind::Info,
                    format!("done: {:?}", result.subtype),
                ));
            }
            let _ = tx.send(UiEvent::TurnDone {
                cost_usd: result.total_cost_usd,
                turns: result.num_turns,
            });
            true
        }
        Message::User(_) | Message::System(_) | Message::StreamEvent(_) | Message::Other(_) => {
            false
        }
    }
}

/// One-line rendering of a JSON value, clipped so it fits a transcript row.
fn compact_json(value: &serde_json::Value) -> String {
    let text = value.to_string();
    if text.chars().count() > 160 {
        let head: String = text.chars().take(157).collect();
        format!("{head}...")
    } else {
        text
    }
}
