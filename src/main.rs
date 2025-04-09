mod game;

use std::io;
use std::time::{Duration, Instant};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{prelude::*, widgets::*};

use game::{Game, GameStatus};

fn main() -> Result<(), io::Error> {
    // Terminal configuration
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    // Create the terminal backend
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Defining minimum terminal requirements
    const MIN_WIDTH: u16 = 50; // Minimum width required
    const MIN_HEIGHT: u16 = 25; // Minimum height required

    // Check if the terminal has enough space
    let size = terminal.size()?;
    if size.width < MIN_WIDTH || size.height < MIN_HEIGHT {
        // Restore terminal before exiting
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        // Show error message
        println!("Error: Terminal too small for Wordle game.");
        println!(
            "Minimum size required: {}x{} characters",
            MIN_WIDTH, MIN_HEIGHT
        );
        println!("Current size: {}x{} characters", size.width, size.height);
        println!("\nPlease increase the terminal window size and try again.");

        return Ok(());
    }

    // Create game instance
    let mut game = Game::new();
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(250);

    // Main loop
    loop {
        // Capture any rendering errors and exit gracefully if needed
        if let Err(e) = terminal.draw(|f| ui(f, &game)) {
            disable_raw_mode()?;
            execute!(
                terminal.backend_mut(),
                LeaveAlternateScreen,
                DisableMouseCapture
            )?;
            terminal.show_cursor()?;

            println!("Error rendering the game: {}", e);
            println!("The game was terminated to avoid unexpected behavior.");
            return Ok(());
        }

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Esc => {
                            if game.status == GameStatus::Playing {
                                game.quit();
                            } else if game.status == GameStatus::Quitting {
                                // Cancel quitting and go back to the game
                                game.status = GameStatus::Playing;
                            } else {
                                // In won/lost state, start new game
                                game = Game::new();
                            }
                        }
                        KeyCode::Char(c) if c.is_ascii_alphabetic() => {
                            game.input_letter(c.to_ascii_uppercase());
                        }
                        KeyCode::Backspace => {
                            game.delete_letter();
                        }
                        KeyCode::Enter => {
                            game.submit_guess();
                            // If in quitting state and user presses Enter, exit
                            if game.status == GameStatus::Quitting {
                                break;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            game.on_tick();
            last_tick = Instant::now();
        }

        if game.should_quit {
            break;
        }
    }

    // Restore the terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

fn ui(f: &mut Frame, game: &Game) {
    const MIN_WIDTH: u16 = 50;
    const MIN_HEIGHT: u16 = 25;

    // Check if the terminal still has enough space
    let size = f.size();
    if size.width < MIN_WIDTH || size.height < MIN_HEIGHT {
        // Show warning message if terminal is too small
        let warning = format!(
            "Terminal too small ({}x{}). Minimum size: {}x{}",
            size.width, size.height, MIN_WIDTH, MIN_HEIGHT
        );

        let warning_text = Paragraph::new(warning)
            .style(Style::default().fg(Color::Red))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));

        f.render_widget(warning_text, size);
        return;
    }

    // Main layout
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(10),   // Game area
            Constraint::Length(3), // Messages and instructions
        ])
        .split(f.size());

    // Game title
    let title_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    let title = Paragraph::new("WORDLE")
        .block(title_block)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Yellow).bold());

    f.render_widget(title, main_layout[0]);

    // Game area
    let game_area = game.render();
    f.render_widget(game_area, main_layout[1]);

    // Instructions
    let instructions = if let Some(msg) = &game.message {
        Paragraph::new(msg.clone()).style(Style::default().fg(Color::Yellow))
    } else {
        match game.status {
            GameStatus::Won => Paragraph::new("You won! Press [ESC] to play again")
                .style(Style::default().fg(Color::Green)),
            GameStatus::Lost => {
                let text = format!(
                    "You lost! The word was {}. Press [ESC] to play again",
                    game.target_word
                );
                Paragraph::new(text).style(Style::default().fg(Color::Red))
            }
            GameStatus::Playing => {
                Paragraph::new("[Enter] Submit guess | [Backspace] Delete | [ESC] Exit")
            }
            GameStatus::Quitting => {
                Paragraph::new("Are you sure you want to exit? [Enter] Yes | [Esc] No")
            }
        }
    };

    let instructions_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    f.render_widget(
        instructions
            .alignment(Alignment::Center)
            .block(instructions_block),
        main_layout[2],
    );
}
