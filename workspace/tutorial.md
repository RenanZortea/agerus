# Ratatui Basics Tutorial

## Introduction

Ratatui is a Rust crate for building terminal user interfaces (TUIs). This tutorial covers the basics of creating a simple TUI application.

## Prerequisites

- Rust installed (https://www.rust-lang.org/learn/get-started)
- Cargo project setup

## 1. Project Setup

Create a new Cargo project:

```bash
cargo new ratatui_tutorial
cd ratatui_tutorial
```

Add Ratatui as a dependency in `Cargo.toml`:

```toml
[dependencies]
ratatui = "0.20"
```

## 2. Hello Ratatui

Create a basic UI with a paragraph:

```rust
use ratatui::prelude::*;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let backend = Terminal::new(Config::default())?
        .into_raw_mode()?;
    let mut backend = backend;
    let mut terminal = Terminal::new(backend)?;

    let mut frame = Frame::new(Paragraph::new("Hello, Ratatui!"), terminal.size()?);
    terminal.draw(|f| f.render_widget(frame, f.size()))?;

    Ok(())
}
```

## 3. Handling Input

Add keyboard input handling:

```rust
use crossterm::event::{self, Event, KeyCode};

// In your main loop:
loop {
    if let Event::Key(event) = event::poll(Duration::from_millis(100))?
        .expect("Event expected") {
        match event.code {
            KeyCode::Char('q') => break,
            _ => continue,
        }
    }
    // Render UI
}
```

## 4. Layouts

Use layouts to organize widgets:

```rust
use ratatui::layout::{Constraint, Direction, Layout, Position};

let layout = Layout::default()
    .direction(Direction::Vertical)
    .constraints(
        [
            Constraint::Length(3),
            Constraint::Min(0),
        ]
        .as_ref()
    )
    .split(terminal.size()?);
```

## Resources

- [Official Tutorials](https://ratatui.rs/tutorials/)
- [GitHub Repository](https://github.com/ratatui/ratatui)
- [Beginner's Guide](https://kdheepak.com/blog/the-basic-building-blocks-of-ratatui-part-1/)

```

```
