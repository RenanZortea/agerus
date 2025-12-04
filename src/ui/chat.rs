use super::theme::*;
use crate::app::{App, MessageRole};
use crate::markdown::render_markdown;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    // Center the chat area horizontally to make it thinner (20% margin | 60% content | 20% margin)
    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ])
        .split(area);

    // Apply Vertical Margin/Padding to the chat column
    let vertical_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Top margin to unglue text
            Constraint::Min(0),    // Chat content
        ])
        .split(layout[1]);

    let area = vertical_layout[1];

    let mut lines = vec![];
    let max_width = area.width as usize;

    for (i, msg) in app.messages.iter().enumerate() {
        // Skip the initial system message in chat view to keep it clean
        if matches!(msg.role, MessageRole::System) && msg.content.starts_with("Ready") {
            continue;
        }

        match msg.role {
            MessageRole::System => {
                lines.push(Line::from(Span::styled(
                    format!("  >> {}", msg.content),
                    Style::default().fg(FG_SECONDARY),
                )));
            }
            MessageRole::Thinking => {
                // Pulse effect logic:
                // Only pulse if the app is currently processing AND this is the last message.
                let is_active = app.is_processing && (i == app.messages.len() - 1);

                let pulse_style = if is_active {
                    let t = app.spinner_frame as f64 * 0.2;
                    let brightness = 150.0 + 50.0 * t.sin();
                    let gray = brightness.clamp(0.0, 255.0) as u8;
                    Style::default()
                        .fg(Color::Rgb(gray, gray, gray))
                        .add_modifier(Modifier::ITALIC)
                } else {
                    Style::default()
                        .fg(FG_SECONDARY)
                        .add_modifier(Modifier::ITALIC)
                };

                // Arrow indicator
                let arrow = if msg.collapsed { "▶" } else { "▼" };

                // Header line
                lines.push(Line::from(vec![
                    Span::styled(format!("  {} Thinking... ", arrow), pulse_style),
                    if msg.collapsed {
                        Span::styled(
                            "(collapsed, Ctrl+t to toggle)",
                            Style::default().fg(Color::DarkGray),
                        )
                    } else {
                        Span::raw("")
                    },
                ]));

                // Only render content if expanded
                if !msg.collapsed {
                    let rendered = render_markdown(&msg.content, max_width - 4, pulse_style);
                    for line in rendered {
                        let mut spans = vec![Span::raw("    ")];
                        spans.extend(line.spans);
                        lines.push(Line::from(spans));
                    }
                }
            }
            _ => {
                let (name, style) = match msg.role {
                    MessageRole::User => (
                        "User",
                        Style::default()
                            .fg(ACCENT_BLUE)
                            .add_modifier(Modifier::BOLD),
                    ),
                    MessageRole::Assistant => (
                        "Agerus",
                        Style::default()
                            .fg(ACCENT_ORANGE)
                            .add_modifier(Modifier::BOLD),
                    ),
                    MessageRole::Error => ("Error", Style::default().fg(Color::Red)),
                    _ => ("System", Style::default().fg(FG_SECONDARY)),
                };

                // Header: Name + Time
                lines.push(Line::from(vec![
                    Span::styled(name, style),
                    Span::styled(
                        format!(" {}", chrono::Local::now().format("%H:%M")),
                        Style::default().fg(FG_SECONDARY),
                    ),
                ]));

                // Content
                if matches!(msg.role, MessageRole::Error) {
                    lines.push(Line::from(Span::styled(
                        &msg.content,
                        Style::default().fg(Color::Red),
                    )));
                } else {
                    let base_style = Style::default().fg(FG_PRIMARY);
                    let rendered = render_markdown(&msg.content, max_width, base_style);
                    lines.extend(rendered);
                }
            }
        }
        lines.push(Line::from("")); // Spacing
    }

    let scroll = if app.chat_stick_to_bottom {
        (lines.len() as u16).saturating_sub(area.height)
    } else {
        app.chat_scroll
    };

    f.render_widget(Paragraph::new(lines).scroll((scroll, 0)), area);
}
