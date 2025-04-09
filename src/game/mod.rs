use rand::seq::SliceRandom;
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, Widget},
};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use unicode_width::UnicodeWidthStr;

const MAX_ATTEMPTS: usize = 6;
const WORD_LENGTH: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LetterStatus {
    Correct, // Correct letter in correct position
    Present, // Correct letter in wrong position
    Absent,  // Letter is not in the word
    Unused,  // Letter not yet used
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GameStatus {
    Playing,
    Won,
    Lost,
    Quitting,
}

pub struct Game {
    pub attempts: Vec<Vec<char>>,
    pub letter_statuses: [[LetterStatus; WORD_LENGTH]; MAX_ATTEMPTS],
    pub current_attempt: usize,
    pub target_word: String,
    pub status: GameStatus,
    pub should_quit: bool,
    pub message: Option<String>,
    pub message_timer: u8,
}

impl Game {
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();

        let words = Self::load_words_from_file("./data/words.txt");

        let target_word = match words.choose(&mut rng) {
            Some(word) => word.to_string(),
            None => {
                let fallback_words = vec![
                    "PROVA",
                    // "OLHAR", "SORTE", "TEMPO", "PULAR", "FALAR",
                    // "JOGAR", "QUERO", "MUNDO", "LIVRO", "VIVER",
                ];
                fallback_words.choose(&mut rng).unwrap().to_string()
            }
        };

        Game {
            attempts: vec![Vec::new(); MAX_ATTEMPTS],
            letter_statuses: [[LetterStatus::Unused; WORD_LENGTH]; MAX_ATTEMPTS],
            current_attempt: 0,
            target_word,
            status: GameStatus::Playing,
            should_quit: false,
            message: None,
            message_timer: 0,
        }
    }

    fn load_words_from_file(filename: &str) -> Vec<String> {
        let path = Path::new(filename);

        // Try to open the file
        let file = match File::open(path) {
            Ok(file) => file,
            Err(_) => return Vec::new(), // Return empty vector if file can't be opened
        };

        let reader = BufReader::new(file);

        // Read words, convert to uppercase, and filter by length
        reader
            .lines()
            .filter_map(Result::ok) // Skip lines that can't be read
            .map(|line| line.trim().to_uppercase())
            .filter(|word| word.len() == WORD_LENGTH)
            .collect()
    }

    pub fn input_letter(&mut self, c: char) {
        if self.status != GameStatus::Playing {
            return;
        }

        if self.attempts[self.current_attempt].len() < WORD_LENGTH {
            self.attempts[self.current_attempt].push(c);
        }
    }

    pub fn delete_letter(&mut self) {
        if self.status != GameStatus::Playing {
            return;
        }

        if !self.attempts[self.current_attempt].is_empty() {
            self.attempts[self.current_attempt].pop();
        }
    }

    pub fn submit_guess(&mut self) {
        if self.status != GameStatus::Playing {
            return;
        }

        if self.attempts[self.current_attempt].len() != WORD_LENGTH {
            return; // Incomplete word
        }

        // Removed the check if the word is in the list to allow
        // any 5-letter attempt
        // let current_word: String = self.attempts[self.current_attempt].iter().collect();
        // if !WORDS.contains(&current_word.as_str()) {
        //     return; // Word is not in the list
        // }

        // Evaluate the guess
        self.evaluate_guess();

        // Check if won
        if self.attempts[self.current_attempt]
            .iter()
            .collect::<String>()
            == self.target_word
        {
            self.status = GameStatus::Won;
            return;
        }

        // Move to next attempt
        self.current_attempt += 1;

        // Check if lost
        if self.current_attempt >= MAX_ATTEMPTS {
            self.status = GameStatus::Lost;
            // No need to do anything else, as we've used all attempts
        }
    }

    fn evaluate_guess(&mut self) {
        // Ensure we don't try to evaluate out of bounds
        if self.current_attempt >= MAX_ATTEMPTS {
            return;
        }

        let guess = &self.attempts[self.current_attempt];
        let target: Vec<char> = self.target_word.chars().collect();
        let mut used = vec![false; WORD_LENGTH];

        // First step: mark correct letters
        for i in 0..WORD_LENGTH {
            if i < guess.len() && guess[i] == target[i] {
                self.letter_statuses[self.current_attempt][i] = LetterStatus::Correct;
                used[i] = true;
            }
        }

        // Second step: mark letters present in another position
        for i in 0..guess.len() {
            if self.letter_statuses[self.current_attempt][i] == LetterStatus::Correct {
                continue;
            }

            let mut found = false;
            for j in 0..WORD_LENGTH {
                if !used[j] && guess[i] == target[j] {
                    self.letter_statuses[self.current_attempt][i] = LetterStatus::Present;
                    used[j] = true;
                    found = true;
                    break;
                }
            }

            if !found {
                self.letter_statuses[self.current_attempt][i] = LetterStatus::Absent;
            }
        }
    }

    pub fn render(&self) -> impl Widget + '_ {
        GameWidget { game: self }
    }

    pub fn quit(&mut self) {
        self.status = GameStatus::Quitting;
    }

    pub fn on_tick(&mut self) {
        // Update the temporary message timer
        if self.message_timer > 0 {
            self.message_timer -= 1;
            if self.message_timer == 0 {
                self.message = None;
            }
        }
    }

    // Utilities for getting the keyboard status map
    pub fn get_keyboard_status(&self) -> [LetterStatus; 26] {
        let mut keyboard_status = [LetterStatus::Unused; 26];

        // Limit to valid attempts (min of current_attempt or MAX_ATTEMPTS)
        let max_attempt = self.current_attempt.min(MAX_ATTEMPTS);

        for attempt_idx in 0..max_attempt {
            for (letter_idx, letter) in self.attempts[attempt_idx].iter().enumerate() {
                if letter.is_ascii_alphabetic() {
                    let idx = (*letter as u8 - b'A') as usize;
                    if idx < 26 {
                        let current_status = self.letter_statuses[attempt_idx][letter_idx];
                        // Only update if the status is "better" than the current one
                        match (keyboard_status[idx], current_status) {
                            (LetterStatus::Unused, _) => keyboard_status[idx] = current_status,
                            (
                                LetterStatus::Absent,
                                LetterStatus::Present | LetterStatus::Correct,
                            ) => keyboard_status[idx] = current_status,
                            (LetterStatus::Present, LetterStatus::Correct) => {
                                keyboard_status[idx] = current_status
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        keyboard_status
    }
}

struct GameWidget<'a> {
    game: &'a Game,
}

impl<'a> Widget for GameWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Create a layout for the grid of attempts and the virtual keyboard
        let game_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(70), // Attempts grid
                Constraint::Percentage(30), // Virtual keyboard
            ])
            .split(area);

        // Render the attempts grid
        self.render_grid(game_layout[0], buf);

        // Render the virtual keyboard
        self.render_keyboard(game_layout[1], buf);
    }
}

impl<'a> GameWidget<'a> {
    fn render_grid(&self, area: Rect, buf: &mut Buffer) {
        let cell_width = 5;
        let cell_height = 3;
        let horizontal_gap = 1;

        let grid_width = WORD_LENGTH * cell_width + (WORD_LENGTH - 1) * horizontal_gap;
        let grid_height = MAX_ATTEMPTS * cell_height;

        // Calculate the starting point to center the grid
        let start_x = area.x + (area.width as usize - grid_width) as u16 / 2;
        let start_y = area.y + (area.height as usize - grid_height) as u16 / 2;

        for attempt_idx in 0..MAX_ATTEMPTS {
            for letter_idx in 0..WORD_LENGTH {
                let x = start_x + (letter_idx * (cell_width + horizontal_gap)) as u16;
                let y = start_y + (attempt_idx * cell_height) as u16;

                let cell_area = Rect::new(x, y, cell_width as u16, cell_height as u16);

                // Determine cell style based on letter status
                let style = if attempt_idx < self.game.current_attempt {
                    match self.game.letter_statuses[attempt_idx][letter_idx] {
                        LetterStatus::Correct => Style::default().bg(Color::Green).fg(Color::Black),
                        LetterStatus::Present => {
                            Style::default().bg(Color::Yellow).fg(Color::Black)
                        }
                        LetterStatus::Absent => {
                            Style::default().bg(Color::DarkGray).fg(Color::White)
                        }
                        LetterStatus::Unused => Style::default().bg(Color::Black).fg(Color::White),
                    }
                } else if attempt_idx == self.game.current_attempt {
                    Style::default().bg(Color::Black).fg(Color::White)
                } else {
                    Style::default().bg(Color::Black).fg(Color::DarkGray)
                };

                // Draw cell with border
                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Plain)
                    .style(style);

                block.render(cell_area, buf);

                // Draw letter if it exists
                if attempt_idx < self.game.attempts.len()
                    && attempt_idx < self.game.current_attempt + 1 // Ensure we don't access beyond valid attempts
                    && letter_idx < self.game.attempts[attempt_idx].len()
                {
                    let letter = self.game.attempts[attempt_idx][letter_idx].to_string();
                    let width = letter.width() as u16;
                    let letter_x = x + (cell_width as u16 - width) / 2;
                    let letter_y = y + 1;

                    buf.set_string(letter_x, letter_y, letter, style);
                }
            }
        }
    }

    fn render_keyboard(&self, area: Rect, buf: &mut Buffer) {
        let keyboard_layout = ["QWERTYUIOP", "ASDFGHJKL", "ZXCVBNM"];

        let key_width = 3;
        let key_height = 3;
        let horizontal_gap = 1;
        let vertical_gap = 1;

        let keyboard_status = self.game.get_keyboard_status();

        // Calculate keyboard dimensions
        let max_row_len = keyboard_layout.iter().map(|row| row.len()).max().unwrap();
        let keyboard_width = max_row_len * key_width + (max_row_len - 1) * horizontal_gap;
        let keyboard_height =
            keyboard_layout.len() * key_height + (keyboard_layout.len() - 1) * vertical_gap;

        // Starting position to center keyboard
        let start_x = area.x + (area.width as usize - keyboard_width) as u16 / 2;
        let start_y = area.y + (area.height as usize - keyboard_height) as u16 / 2;

        for (row_idx, row) in keyboard_layout.iter().enumerate() {
            // Center each row horizontally
            let row_width = row.len() * key_width + (row.len() - 1) * horizontal_gap;
            let row_start_x = start_x + (keyboard_width - row_width) as u16 / 2;

            for (key_idx, key) in row.chars().enumerate() {
                let x = row_start_x + (key_idx * (key_width + horizontal_gap)) as u16;
                let y = start_y + (row_idx * (key_height + vertical_gap)) as u16;

                let key_area = Rect::new(x, y, key_width as u16, key_height as u16);

                // Get key status
                let key_char_idx = (key as u8 - b'A') as usize;
                let status = if key_char_idx < keyboard_status.len() {
                    keyboard_status[key_char_idx]
                } else {
                    LetterStatus::Unused
                };

                // Set style based on key status
                let style = match status {
                    LetterStatus::Correct => Style::default().bg(Color::Green).fg(Color::Black),
                    LetterStatus::Present => Style::default().bg(Color::Yellow).fg(Color::Black),
                    LetterStatus::Absent => Style::default().bg(Color::DarkGray).fg(Color::White),
                    LetterStatus::Unused => Style::default().bg(Color::Black).fg(Color::White),
                };

                // Draw key
                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Plain)
                    .style(style);

                block.render(key_area, buf);

                // Draw letter
                let letter = key.to_string();
                let width = letter.width() as u16;
                let letter_x = x + (key_width as u16 - width) / 2;
                let letter_y = y + 1;

                buf.set_string(letter_x, letter_y, letter, style);
            }
        }
    }
}
