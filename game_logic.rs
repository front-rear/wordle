use crate::status::Status;
use std::collections::HashMap;

pub fn compute_feedback(guess: &str, answer: &str) -> String {
    let g_chars: Vec<char> = guess.chars().collect();
    let a_chars: Vec<char> = answer.chars().collect();
    let mut result = vec!['R'; 5];
    let mut answer_count: HashMap<char, i32> = HashMap::new();

    for i in 0..5 {
        if g_chars[i] == a_chars[i] {
            result[i] = 'G';
        } else {
            *answer_count.entry(a_chars[i]).or_insert(0) += 1;
        }
    }

    for i in 0..5 {
        if result[i] != 'G' {
            if let Some(count) = answer_count.get_mut(&g_chars[i]) {
                if *count > 0 {
                    result[i] = 'Y';
                    *count -= 1;
                }
            }
        }
    }

    result.into_iter().collect()
}

pub fn update_keyboard(guess: &str, feedback: &str, keyboard: &mut HashMap<char, Status>) {
    let g_chars: Vec<char> = guess.chars().collect();
    let f_chars: Vec<char> = feedback.chars().collect();
    let mut local_max: HashMap<char, Status> = HashMap::new();

    for i in 0..5 {
        let st = Status::from_feedback(f_chars[i]);
        let current = local_max.entry(g_chars[i]).or_insert(Status::Unknown);
        if st.priority() > current.priority() {
            *current = st;
        }
    }

    for (ch, st) in local_max {
        if let Some(global) = keyboard.get_mut(&ch) {
            if st.priority() > global.priority() {
                *global = st;
            }
        }
    }
}

pub fn is_valid_hard_mode(
    guess: &str,
    known_positions: &[Option<char>; 5],
    min_counts: &HashMap<char, usize>,
) -> bool {
    let g_chars: Vec<char> = guess.chars().collect();

    for i in 0..5 {
        if let Some(c) = known_positions[i] {
            if g_chars[i] != c {
                return false;
            }
        }
    }

    let mut counts: HashMap<char, usize> = HashMap::new();
    for &ch in &g_chars {
        *counts.entry(ch).or_insert(0) += 1;
    }

    for (&ch, &min) in min_counts {
        if counts.get(&ch).copied().unwrap_or(0) < min {
            return false;
        }
    }

    true
}

pub fn update_hard_mode_constraints(
    guess: &str,
    feedback: &str,
    known_positions: &mut [Option<char>; 5],
    min_counts: &mut HashMap<char, usize>,
) {
    let g_chars: Vec<char> = guess.chars().collect();
    let f_chars: Vec<char> = feedback.chars().collect();

    for i in 0..5 {
        if f_chars[i] == 'G' {
            known_positions[i] = Some(g_chars[i]);
        }
    }

    let mut conf: HashMap<char, usize> = HashMap::new();
    for i in 0..5 {
        if f_chars[i] == 'G' || f_chars[i] == 'Y' {
            *conf.entry(g_chars[i]).or_insert(0) += 1;
        }
    }

    for (&ch, &c) in &conf {
        let entry = min_counts.entry(ch).or_insert(0);
        if c > *entry {
            *entry = c;
        }
    }
}
