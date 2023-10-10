use std::{error::Error, io, string, fmt::format};

/// A simple example demonstrating how to handle user input. This is
/// a bit out of the scope of the library as it does not provide any
/// input handling out of the box. However, it may helps some to get
/// started.
///
/// This is a very simple example:
///   * An input box always focused. Every character you type is registered
///   here.
///   * An entered character is inserted at the cursor position.
///   * Pressing Backspace erases the left character before the cursor position
///   * Pressing Enter pushes the current input in the history of previous
///   messages.
/// **Note: ** as this is a relatively simple example unicode characters are unsupported and
/// their use will result in undefined behaviour.
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, EventStream},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::FutureExt;
use ratatui::{prelude::*, widgets::*};
use tokio::{runtime, select};
use tokio_stream::StreamExt;

use crate::comms::{CommsLink, Messages};

#[derive(PartialEq, Eq)]
enum InputMode {
    Normal,
    FileName,
    Messages,
}

impl InputMode {
    pub fn is_writing(&self) -> bool {
        match self {
            InputMode::Normal => { false },
            _ => { true }
        }
    }
}

/// App holds the state of the application
struct App {
    /// Current value of the input box
    input: String,
    /// Position of cursor in the editor area.
    cursor_position: usize,
    /// Current input mode
    input_mode: InputMode,

    os_message: Vec<String>,

    /// History of recorded messages
    messages: Vec<String>,

    /// Out comms to the *os* thread
    comms: CommsLink,

    file_name: String,
    file_contents: String,

    list_state: ListState
}


impl App {
    fn new(comms: CommsLink) -> App {
        App {
            input: String::new(),
            input_mode: InputMode::Normal,
            os_message: Vec::new(),
            messages: Vec::new(),
            cursor_position: 0,
            comms,
            file_name: String::new(),
            file_contents: String::new(),
            list_state: ListState::default()
        }
    }

    pub fn get_user_title(&self) -> String {
        match self.input_mode {
            InputMode::Messages => { format!("Editing file: {}", self.file_name) }
            _ => format!("Text editor"),
        }
    }

    fn enter_edit(&mut self) {
        // self.list_state = ListState::default();
        self.input_mode = InputMode::FileName;
        self.file_name = String::new();
        self.file_contents = String::new();
    }

    fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.cursor_position.saturating_sub(1);
        self.cursor_position = self.clamp_cursor(cursor_moved_left);
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.cursor_position.saturating_add(1);
        self.cursor_position = self.clamp_cursor(cursor_moved_right);
    }

    fn move_cursor_up(&mut self) {
        if self.messages.is_empty() {
            self.list_state.select(None)
        }

        if let Some(curr_selected) = self.list_state.selected() {
            if curr_selected == 0 {
                self.list_state.select(None);
                return;
            }

            let selecting = Some(curr_selected.saturating_sub(1));
            self.list_state.select(selecting);
        }
    }

    fn move_cursor_down(&mut self) {
        if self.messages.is_empty() {
            self.list_state.select(None)
        }

        let mut selecting = Some(0);
        if let Some(curr_selected) = self.list_state.selected() {
            selecting = Some((curr_selected + 1).clamp(0, self.messages.len() - 1));
        }

        self.list_state.select(selecting);
    }

    fn enter_char(&mut self, new_char: char) {
        self.input.insert(self.cursor_position, new_char);

        self.move_cursor_right();
    }

    fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.cursor_position != 0;
        if is_not_cursor_leftmost {
            // Method "remove" is not used on the saved text for deleting the selected char.
            // Reason: Using remove on String works on bytes instead of the chars.
            // Using remove would require special care because of char boundaries.

            let current_index = self.cursor_position;
            let from_left_to_current_index = current_index - 1;

            // Getting all characters before the selected character.
            let before_char_to_delete = self.input.chars().take(from_left_to_current_index);
            // Getting all characters after selected character.
            let after_char_to_delete = self.input.chars().skip(current_index);

            // Put all characters together except the selected one.
            // By leaving the selected one out, it is forgotten and therefore deleted.
            self.input = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.input.len())
    }

    fn reset_cursor(&mut self) {
        self.cursor_position = 0;
    }

    fn submit_message(&mut self) {
        if self.input_mode == InputMode::FileName {
            self.file_name = self.input.clone();
            self.messages = vec![];
        } else if self.input_mode == InputMode::Messages {
            self.file_contents.push_str(&self.input);
            self.file_contents.push_str("\n");

            self.messages.push(self.input.clone());
        }

        if self.input_mode == InputMode::FileName {
            self.input_mode = InputMode::Messages;
        }

        self.input.clear();
        self.reset_cursor();
    }

    async fn esc_pressed(&mut self) {
        if self.input_mode == InputMode::Messages {
            let update_message = format!("UI thread submitting a file write");

            self.messages = vec![update_message];
            self.file_contents.push_str(&self.input);

            let create_mesasge = Messages::FileWrite(self.file_name.clone(), self.file_contents.clone());

            self.comms.tx.send(create_mesasge).await.unwrap();

            self.file_name.clear();
            self.file_contents.clear();
        }

        self.input_mode = InputMode::Normal;
    }
}

pub fn create_terminal(comms: CommsLink) -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let rt = runtime::Builder::new_current_thread().enable_all().build().unwrap();

    let app = App::new(comms);
    rt.block_on(async {
        run_app(&mut terminal, app).await.unwrap();
    });

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    let mut reader = EventStream::new();

    loop {
        terminal.draw(|f| ui(f, &mut app))?;
        let event = reader.next().fuse();
        let os_event = app.comms.rx.recv().fuse();

        select! {
            Some(Ok(Event::Key(key))) = event => {
                match app.input_mode {
                    InputMode::Normal => match key.code {
                        KeyCode::Char('e') => {
                            app.enter_edit();
                        }
                        KeyCode::Char('q') => {
                            return Ok(());
                        }
                        KeyCode::Up => {
                            app.move_cursor_up();
                        }
                        KeyCode::Down => {
                            app.move_cursor_down();
                        }
                        _ => {}
                    },
                    _ if key.kind == KeyEventKind::Press => match key.code {
                        KeyCode::Enter => app.submit_message(),
                        KeyCode::Char(to_insert) => {
                            app.enter_char(to_insert);
                        }
                        KeyCode::Backspace => {
                            app.delete_char();
                        }
                        KeyCode::Left => {
                            app.move_cursor_left();
                        }
                        KeyCode::Right => {
                            app.move_cursor_right();
                        }
                        KeyCode::Esc => {
                            app.esc_pressed().await;
                        }
                        _ => {}
                    },
                    _ => {}
                }
            },
            Some(message) = os_event => {
                if let Messages::String(str) = message {
                    app.os_message.push(format!("{str}"));
                }
            }
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Min(1),
            ]
            .as_ref(),
        )
        .split(f.size());

    let inner_types = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50)
        ])
        .split(chunks[2]);

    let (msg, style) = match app.input_mode {
        InputMode::Normal => (
            vec![
                "Press ".into(),
                "q".bold(),
                " to exit, ".into(),
                "e".bold(),
                " to start editing.".bold(),
            ],
            Style::default().add_modifier(Modifier::RAPID_BLINK),
        ),
        InputMode::FileName => (
            vec![
                "Press ".into(),
                "Esc".bold(),
                " to stop changing file name, ".into(),
                "Enter".bold(),
                " to pick a file name".into(),
            ],
            Style::default(),
        ),
        InputMode::Messages => (
            vec![
                "Press ".into(),
                "Esc".bold(),
                " to stop editing, ".into(),
                "Enter".bold(),
                " to record the message".into(),
            ],
            Style::default(),
        ),
    };
    let mut text = Text::from(Line::from(msg));
    text.patch_style(style);
    let help_message = Paragraph::new(text);
    f.render_widget(help_message, chunks[0]);

    if app.input_mode.is_writing() {
        let input = Paragraph::new(app.input.as_str())
            .style(match app.input_mode {
                InputMode::Normal => Style::default(),
                InputMode::FileName => Style::default().fg(Color::Green),
                InputMode::Messages => Style::default().fg(Color::Yellow),
            })
            .block(Block::default().borders(Borders::ALL).title("Input"));
        f.render_widget(input, chunks[1]);
        match app.input_mode {
            InputMode::Normal =>
                // Hide the cursor. `Frame` does this by default, so we don't need to do anything here
                {}

            _ => {
                // Make the cursor visible and ask ratatui to put it at the specified coordinates after
                // rendering
                f.set_cursor(
                    // Draw the cursor at the current position in the input field.
                    // This position is can be controlled via the left and right arrow key
                    chunks[1].x + app.cursor_position as u16 + 1,
                    // Move one line down, from the border to the input line
                    chunks[1].y + 1,
                )
            }
        }
    }

    let messages: Vec<ListItem> = app
        .messages
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let content = Line::from(Span::raw(format!("{i}: {m}")));
            ListItem::new(content)
        })
        .collect();

    let messages =
        List::new(messages).block(Block::default().borders(Borders::ALL).title(app.get_user_title()))
        .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
        .highlight_symbol(">>");
    f.render_stateful_widget(messages, inner_types[0], &mut app.list_state);

    let os_messages: Vec<ListItem> = app
        .os_message
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let content = Line::from(Span::raw(format!("{i}: {m}")));
            ListItem::new(content)
        })
        .collect();

    let os_messages =
        List::new(os_messages).block(Block::default().borders(Borders::ALL).title("OS Messages"))
        .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
        .highlight_symbol(">>");
    f.render_stateful_widget(os_messages, inner_types[1], &mut app.list_state);

}
