use crate::game_logic::{compute_feedback, is_valid_hard_mode, update_keyboard};
use crate::status::Status;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame, Terminal,
};
use std::collections::{HashMap, HashSet};
use std::io;

pub struct TuiApp {
    // Game state
    pub answer: String,
    pub current_input: String,
    pub guesses: Vec<String>,
    pub feedbacks: Vec<String>,
    pub keyboard: HashMap<char, Status>,
    pub attempt: usize,
    pub max_attempts: usize,
    pub won: bool,
    pub finished: bool,
    pub message: String,
    
    // Game configuration
    pub is_hard_mode: bool,
    pub finals_set: HashSet<String>,
    pub acceptables_set: HashSet<String>,
    pub known_positions: [Option<char>; 5],
    pub min_counts: HashMap<char, usize>,
}

impl TuiApp {
    pub fn new(
        answer: String,
        is_hard_mode: bool,
        finals_set: HashSet<String>,
        acceptables_set: HashSet<String>,
    ) -> Self {
        let mut keyboard: HashMap<char, Status> = HashMap::new();
        for c in 'A'..='Z' {
            keyboard.insert(c, Status::Unknown);
        }

        Self {
            answer,
            current_input: String::new(),
            guesses: Vec::new(),
            feedbacks: Vec::new(),
            keyboard,
            attempt: 0,
            max_attempts: 6,
            won: false,
            finished: false,
            message: "猜测一个五字母单词，按回车确认".to_string(),
            is_hard_mode,
            finals_set,
            acceptables_set,
            known_positions: [None; 5],
            min_counts: HashMap::new(),
        }
    }

    pub fn handle_input(&mut self, key_code: KeyCode) -> bool {
        if self.finished {
            return matches!(key_code, KeyCode::Esc);
        }

        match key_code {
            KeyCode::Esc => return true, // Exit
            KeyCode::Enter => {
                if self.current_input.len() == 5 {
                    self.submit_guess();
                } else {
                    self.message = "请输入完整的五字母单词".to_string();
                }
            }
            KeyCode::Backspace => {
                self.current_input.pop();
                self.message = "猜测一个五字母单词，按回车确认".to_string();
            }
            KeyCode::Char(c) => {
                let c = c.to_ascii_uppercase();
                if c.is_ascii_alphabetic() && self.current_input.len() < 5 {
                    self.current_input.push(c);
                    self.message = "猜测一个五字母单词，按回车确认".to_string();
                }
            }
            _ => {}
        }
        false
    }

    fn submit_guess(&mut self) {
        let guess = self.current_input.clone();
        
        // Check if word is in acceptable set
        if !self.acceptables_set.contains(&guess) {
            self.message = "单词不在词库中".to_string();
            return;
        }

        // Check hard mode constraints
        if self.is_hard_mode && !is_valid_hard_mode(&guess, &self.known_positions, &self.min_counts) {
            self.message = "困难模式：必须使用已知信息".to_string();
            return;
        }

        // Compute feedback
        let feedback = compute_feedback(&guess, &self.answer);
        
        // Update hard mode constraints
        if self.is_hard_mode {
            crate::game_logic::update_hard_mode_constraints(
                &guess,
                &feedback,
                &mut self.known_positions,
                &mut self.min_counts,
            );
        }

        // Update keyboard
        update_keyboard(&guess, &feedback, &mut self.keyboard);

        // Store guess and feedback
        self.guesses.push(guess.clone());
        self.feedbacks.push(feedback.clone());
        self.attempt += 1;

        // Check win condition
        if guess == self.answer {
            self.won = true;
            self.finished = true;
            self.message = format!("🎉 恭喜！你在第 {} 次猜中了！按 ESC 退出", self.attempt);
        } else if self.attempt >= self.max_attempts {
            self.finished = true;
            self.message = format!("游戏结束！答案是 {}。按 ESC 退出", self.answer);
        } else {
            self.message = "猜测一个五字母单词，按回车确认".to_string();
        }

        // Clear current input
        self.current_input.clear();
    }
}

pub fn run_tui(
    answer: String,
    is_hard_mode: bool,
    finals_set: HashSet<String>,
    acceptables_set: HashSet<String>,
) -> io::Result<(bool, usize, Vec<String>)> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = TuiApp::new(answer, is_hard_mode, finals_set, acceptables_set);

    // Main loop
    let result = loop {
        terminal.draw(|f| ui(f, &app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                if app.handle_input(key.code) {
                    break (app.won, app.attempt, app.guesses);
                }
            }
        }
    };

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(result)
}

fn ui(f: &mut Frame, app: &TuiApp) {
    let size = f.area();

    // Create main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Current input area
            Constraint::Length(8),  // Keyboard area  
            Constraint::Min(10),    // Guess history area
            Constraint::Length(3),  // Message area
        ])
        .split(size);

    // Current input area
    render_input_area(f, chunks[0], app);

    // Keyboard area
    render_keyboard_area(f, chunks[1], app);

    // Guess history area
    render_history_area(f, chunks[2], app);

    // Message area
    render_message_area(f, chunks[3], app);
}

fn render_input_area(f: &mut Frame, area: Rect, app: &TuiApp) {
    let progress = format!("第 {}/{} 次猜测", app.attempt + 1, app.max_attempts);
    let input_display = format!("{:5}", app.current_input);
    
    let input_text = vec![
        Line::from(vec![
            Span::raw(progress),
            Span::raw("  当前输入: "),
            Span::styled(input_display, Style::default().add_modifier(Modifier::BOLD)),
        ])
    ];
    
    let input_paragraph = Paragraph::new(input_text)
        .block(Block::default().borders(Borders::ALL).title("输入区域"))
        .wrap(Wrap { trim: true });
    
    f.render_widget(input_paragraph, area);
}

fn render_keyboard_area(f: &mut Frame, area: Rect, app: &TuiApp) {
    let rows = vec![
        "QWERTYUIOP",
        "ASDFGHJKL",
        "ZXCVBNM",
    ];

    let mut lines = Vec::new();
    for row in rows {
        let mut spans = Vec::new();
        for ch in row.chars() {
            let status = app.keyboard.get(&ch).unwrap_or(&Status::Unknown);
            let style = match status {
                Status::Green => Style::default().bg(Color::Green).fg(Color::Black),
                Status::Yellow => Style::default().bg(Color::Yellow).fg(Color::Black),
                Status::Red => Style::default().bg(Color::Red).fg(Color::White),
                Status::Unknown => Style::default(),
            };
            spans.push(Span::styled(format!("{} ", ch), style));
        }
        lines.push(Line::from(spans));
    }

    let keyboard_paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("键盘状态"))
        .wrap(Wrap { trim: true });

    f.render_widget(keyboard_paragraph, area);
}

fn render_history_area(f: &mut Frame, area: Rect, app: &TuiApp) {
    let mut lines = Vec::new();
    
    for (i, (guess, feedback)) in app.guesses.iter().zip(app.feedbacks.iter()).enumerate() {
        let mut spans = Vec::new();
        spans.push(Span::raw(format!("{}. ", i + 1)));
        
        let guess_chars: Vec<char> = guess.chars().collect();
        let feedback_chars: Vec<char> = feedback.chars().collect();
        
        for j in 0..5 {
            let style = match feedback_chars[j] {
                'G' => Style::default().bg(Color::Green).fg(Color::Black),
                'Y' => Style::default().bg(Color::Yellow).fg(Color::Black),
                'R' => Style::default().bg(Color::Red).fg(Color::White),
                _ => Style::default(),
            };
            spans.push(Span::styled(format!("{} ", guess_chars[j]), style));
        }
        
        lines.push(Line::from(spans));
    }

    if lines.is_empty() {
        lines.push(Line::from("还没有猜测记录"));
    }

    let history_paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("猜测历史"))
        .wrap(Wrap { trim: true });

    f.render_widget(history_paragraph, area);
}

fn render_message_area(f: &mut Frame, area: Rect, app: &TuiApp) {
    let message_text = vec![Line::from(app.message.clone())];
    
    let message_paragraph = Paragraph::new(message_text)
        .block(Block::default().borders(Borders::ALL).title("消息"))
        .wrap(Wrap { trim: true });

    f.render_widget(message_paragraph, area);
}