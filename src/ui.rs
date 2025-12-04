use crate::app::{App, AppMode, MessageRole};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Padding, Paragraph}, // Removed Wrap
    Frame,
};
use textwrap::wrap;

// --- Theme / Color Palette (Codex / GitHub Dark Dimmed Style) ---
const BG_MAIN: Color = Color::Rgb(13, 17, 23); // #0d1117 (Deep Navy)
const BG_SIDEBAR: Color = Color::Rgb(22, 27, 34); // #161b22 (Lighter Navy)
const BORDER_COLOR: Color = Color::Rgb(48, 54, 61); // #30363d (Subtle Gray)
const FG_PRIMARY: Color = Color::Rgb(201, 209, 217); // #c9d1d9 (Soft White)
const FG_SECONDARY: Color = Color::Rgb(139, 148, 158); // #8b949e (Dimmed Gray)
const ACCENT_CYAN: Color = Color::Rgb(88, 166, 255); // #58a6ff (Bright Blue/Cyan)
const ACCENT_GREEN: Color = Color::Rgb(63, 185, 80); // #3fb950 (Success Green)
const ACCENT_RED: Color = Color::Rgb(248, 81, 73); // #f85149 (Error Red)
const CODE_COLOR: Color = Color::Rgb(255, 123, 114); // #ff7b72 (Code Pinkish)

// Braille pattern for a rotating circle effect
const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();

    // 1. Global Background
    let bg_block = Block::default().bg(BG_MAIN);
    f.render_widget(bg_block, area);

    // 2. Main Split: Sidebar (Left) vs Content (Right)
    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(32), // Fixed sidebar width
            Constraint::Min(1),     // Fluid content width
        ])
        .split(area);

    let sidebar_area = main_layout[0];
    let content_area = main_layout[1];

    draw_sidebar(f, app, sidebar_area);
    draw_main_content(f, app, content_area);
}

fn draw_sidebar(f: &mut Frame, app: &App, area: Rect) {
    // Sidebar Background
    let sidebar_block = Block::default()
        .bg(BG_SIDEBAR)
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(BORDER_COLOR));
    f.render_widget(sidebar_block.clone(), area);

    let inner_area = sidebar_block.inner(area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // Header Box (Codex Style)
            Constraint::Length(2), // Spacer
            Constraint::Min(1),    // Navigation / History
            Constraint::Length(3), // Footer Status
        ])
        .split(inner_area);

    // --- 1. Codex Header Box ---
    // Mimics the ">_ OpenAI Codex" box from the reference
    let header_text = vec![
        Line::from(vec![
            Span::styled(
                ">_ ",
                Style::default()
                    .fg(ACCENT_GREEN)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Agerus Agent",
                Style::default().fg(FG_PRIMARY).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""), // Spacer
        Line::from(vec![
            Span::styled("model: ", Style::default().fg(FG_SECONDARY)),
            Span::styled(app.config.model.clone(), Style::default().fg(ACCENT_CYAN)),
        ]),
        Line::from(vec![
            Span::styled("cwd:   ", Style::default().fg(FG_SECONDARY)),
            // Truncate workspace path for visual cleanliness
            Span::styled(
                app.config
                    .workspace_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy(),
                Style::default().fg(FG_PRIMARY),
            ),
        ]),
    ];

    let header_box = Paragraph::new(header_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER_COLOR))
            .padding(Padding::new(1, 1, 1, 1)),
    );
    f.render_widget(header_box, chunks[0]);

    // --- 2. Navigation ---
    let active_style = Style::default()
        .fg(ACCENT_CYAN)
        .add_modifier(Modifier::BOLD);
    let inactive_style = Style::default().fg(FG_SECONDARY);

    let nav_items = vec![
        ListItem::new(if app.mode == AppMode::Chat {
            "● Chat"
        } else {
            "○ Chat"
        })
        .style(if app.mode == AppMode::Chat {
            active_style
        } else {
            inactive_style
        }),
        ListItem::new(if app.mode == AppMode::Terminal {
            "● Terminal"
        } else {
            "○ Terminal"
        })
        .style(if app.mode == AppMode::Terminal {
            active_style
        } else {
            inactive_style
        }),
        ListItem::new(""),
        ListItem::new(Line::from(vec![Span::styled(
            "Commands:",
            Style::default()
                .fg(FG_SECONDARY)
                .add_modifier(Modifier::UNDERLINED),
        )])),
        ListItem::new(Line::from(vec![
            Span::styled("/init", Style::default().fg(FG_PRIMARY)),
            Span::raw(" - setup workspace"),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("/reset", Style::default().fg(FG_PRIMARY)),
            Span::raw(" - clear chat"),
        ])),
    ];

    let nav_list = List::new(nav_items).block(Block::default().padding(Padding::horizontal(1)));
    f.render_widget(nav_list, chunks[2]);

    // --- 3. Flashing Footer Status (Animation only) ---
    let (symbol, style) = if app.is_processing {
        let frame_idx = app.spinner_frame % SPINNER.len();
        (
            SPINNER[frame_idx],
            Style::default()
                .fg(ACCENT_CYAN)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        ("●", Style::default().fg(ACCENT_GREEN)) // Static filled circle when idle
    };

    let footer = Paragraph::new(Line::from(Span::styled(symbol, style))).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(BORDER_COLOR))
            .padding(Padding::new(1, 1, 0, 0)),
    );
    f.render_widget(footer, chunks[3]);
}

fn draw_main_content(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),    // Chat Area
            Constraint::Length(1), // Separator Line
            Constraint::Length(3), // Input Area (Fixed height like a prompt)
        ])
        .split(area);

    // 1. View Area (Chat or Terminal)
    match app.mode {
        AppMode::Chat => draw_chat_view(f, app, chunks[0]),
        AppMode::Terminal => draw_terminal_view(f, app, chunks[0]),
    }

    // 2. Separator
    let separator = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(BORDER_COLOR));
    f.render_widget(separator, chunks[1]);

    // 3. Input Area
    draw_input_bar(f, app, chunks[2]);
}

fn draw_chat_view(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default().padding(Padding::new(2, 2, 1, 1));
    f.render_widget(block.clone(), area);
    let inner_area = block.inner(area);

    let max_width = inner_area.width as usize;
    let mut lines = vec![];

    for msg in &app.messages {
        match msg.role {
            MessageRole::System => {
                lines.push(Line::from(Span::styled(
                    format!("  >> {}", msg.content),
                    Style::default().fg(FG_SECONDARY),
                )));
            }
            MessageRole::Thinking => {
                lines.push(Line::from(vec![Span::styled(
                    "  ⚡ Thinking...",
                    Style::default()
                        .fg(FG_SECONDARY)
                        .add_modifier(Modifier::ITALIC),
                )]));
                // Indent thinking content
                for line in msg.content.lines() {
                    lines.push(Line::from(vec![
                        Span::raw("    "),
                        Span::styled(
                            line,
                            Style::default()
                                .fg(FG_SECONDARY)
                                .add_modifier(Modifier::ITALIC),
                        ),
                    ]));
                }
            }
            _ => {
                // Header: "User" or "Agent"
                let (name, style) = if matches!(msg.role, MessageRole::User) {
                    (
                        "User",
                        Style::default()
                            .fg(ACCENT_GREEN)
                            .add_modifier(Modifier::BOLD),
                    )
                } else if matches!(msg.role, MessageRole::Error) {
                    (
                        "Error",
                        Style::default().fg(ACCENT_RED).add_modifier(Modifier::BOLD),
                    )
                } else {
                    (
                        "Agerus",
                        Style::default()
                            .fg(ACCENT_CYAN)
                            .add_modifier(Modifier::BOLD),
                    )
                };

                lines.push(Line::from(vec![
                    Span::styled(format!("{} ", name), style),
                    Span::styled(
                        chrono::Local::now().format("%H:%M").to_string(),
                        Style::default().fg(FG_SECONDARY),
                    ),
                ]));

                // Content
                let content_color = if matches!(msg.role, MessageRole::Error) {
                    ACCENT_RED
                } else {
                    FG_PRIMARY
                };
                let mut in_code_block = false;

                for line in msg.content.lines() {
                    let wrapped = wrap(line, max_width);
                    if wrapped.is_empty() {
                        lines.push(Line::from(""));
                    }

                    for w_line in wrapped {
                        let (parsed_line, new_state) =
                            parse_markdown_line(&w_line, in_code_block, content_color);
                        in_code_block = new_state;
                        lines.push(parsed_line);
                    }
                }
            }
        }
        lines.push(Line::from("")); // Margin between messages
    }

    // Stick to bottom logic
    let total_height = lines.len() as u16;
    let view_height = inner_area.height;
    let scroll = if app.chat_stick_to_bottom {
        if total_height > view_height {
            total_height - view_height
        } else {
            0
        }
    } else {
        app.chat_scroll
    };

    let paragraph = Paragraph::new(lines).scroll((scroll, 0));
    f.render_widget(paragraph, inner_area);
}

fn draw_terminal_view(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .terminal_lines
        .iter()
        .map(|l| ListItem::new(Line::from(Span::styled(l, Style::default().fg(FG_PRIMARY)))))
        .collect();

    let list = List::new(items).block(Block::default().padding(Padding::new(1, 1, 1, 1)));

    let mut state = app.term_scroll.clone();
    f.render_stateful_widget(list, area, &mut state);
}

fn draw_input_bar(f: &mut Frame, app: &App, area: Rect) {
    // A clean prompts line: "> [Input]"
    let prompt_symbol = "> ";
    let prompt_style = if app.mode == AppMode::Chat {
        Style::default()
            .fg(ACCENT_CYAN)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(ACCENT_GREEN)
            .add_modifier(Modifier::BOLD)
    };

    let input_text = vec![
        Span::styled(prompt_symbol, prompt_style),
        Span::styled(app.input_buffer.as_str(), Style::default().fg(FG_PRIMARY)),
        // Blinking cursor simulation could go here if we manually rendered it,
        // but Ratatui handles the terminal cursor position via `f.set_cursor` usually.
        // For visual block cursor:
        Span::styled("▋", Style::default().fg(FG_SECONDARY)),
    ];

    let p = Paragraph::new(Line::from(input_text))
        .block(Block::default().padding(Padding::horizontal(1))); // No borders, just clean text
    f.render_widget(p, area);
}

// --- Helpers ---

fn parse_markdown_line(
    line: &str,
    in_code_block: bool,
    base_color: Color,
) -> (Line<'static>, bool) {
    // 1. Code Fences
    if line.trim().starts_with("```") {
        return (
            Line::from(Span::styled(
                line.to_string(),
                Style::default().fg(FG_SECONDARY),
            )),
            !in_code_block,
        );
    }

    // 2. Inside Code Block
    if in_code_block {
        return (
            Line::from(Span::styled(
                line.to_string(),
                Style::default().fg(CODE_COLOR),
            )),
            true,
        );
    }

    // 3. Regular Text Parsing (Bold headers, bullet points)
    let style = Style::default().fg(base_color);

    // Headers (# )
    if line.starts_with("# ") || line.starts_with("## ") {
        return (
            Line::from(Span::styled(
                line.to_string(),
                style.add_modifier(Modifier::BOLD).fg(ACCENT_CYAN),
            )),
            false,
        );
    }

    // Bold text (**...**) - Simple split parser
    let parts: Vec<&str> = line.split("**").collect();
    let mut spans = vec![];
    for (i, part) in parts.iter().enumerate() {
        let s = if i % 2 == 1 {
            style.add_modifier(Modifier::BOLD).fg(Color::White)
        } else {
            style
        };
        spans.push(Span::styled(part.to_string(), s));
    }

    (Line::from(spans), false)
}
