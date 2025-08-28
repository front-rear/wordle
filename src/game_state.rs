// Modified game_state.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::fmt;

#[derive(Debug)]
pub struct GameStats {
    total_games: u32,
    wins: u32,
    losses: u32,
    total_attempts: u32,
    word_frequency: HashMap<String, u32>,
}

impl GameStats {
    pub fn new() -> Self {
        GameStats {
            total_games: 0,
            wins: 0,
            losses: 0,
            total_attempts: 0,
            word_frequency: HashMap::new(),
        }
    }

    pub fn add_game(&mut self, won: bool, attempts: u32, words_used: &[String]) {
        self.total_games += 1;
        if won {
            self.wins += 1;
            self.total_attempts += attempts;
        } else {
            self.losses += 1;
        }

        for word in words_used {
            *self.word_frequency.entry(word.clone()).or_insert(0) += 1;
        }
    }

    fn average_attempts(&self) -> f64 {
        if self.wins == 0 {
            0.0
        } else {
            (self.total_attempts as f64) / (self.wins as f64)
        }
    }

    fn top_words(&self, count: usize) -> Vec<(&String, &u32)> {
        let mut words: Vec<_> = self.word_frequency.iter().collect();
        words.sort_by(|a, b| {
            let freq_cmp = b.1.cmp(a.1);
            if freq_cmp == std::cmp::Ordering::Equal {
                a.0.cmp(b.0)
            } else {
                freq_cmp
            }
        });
        words.into_iter().take(count).collect()
    }
}

impl fmt::Display for GameStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let avg_attempts = self.average_attempts();
        let top_words = self.top_words(5);
        writeln!(f, "{} {} {:.2}", self.wins, self.losses, avg_attempts)?;
        let word_list: Vec<String> = top_words
            .iter()
            .map(|(word, count)| format!("{} {}", word, count))
            .collect();
        write!(f, "{}", word_list.join(" "))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameState {
    #[serde(default)]
    pub total_rounds: u32,

    #[serde(default)]
    pub games: Vec<GameRecord>,

    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub other: HashMap<String, serde_json::Value>,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            total_rounds: 0,
            games: Vec::new(),
            other: HashMap::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameRecord {
    pub answer: String,
    pub guesses: Vec<String>,

    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub other: HashMap<String, serde_json::Value>,
}

impl Default for GameRecord {
    fn default() -> Self {
        Self {
            answer: String::new(),
            guesses: Vec::new(),
            other: HashMap::new(),
        }
    }
}

#[derive(Debug)]
pub struct RandomModeState {
    pub shuffled_words: VecDeque<String>,
    #[allow(dead_code)]
    pub seed: u64,
    #[allow(dead_code)]
    pub initial_day: u32,
    pub current_index: u32,
}
