use crate::status::Status;
use std::collections::HashMap;

pub fn print_guess(guess: &str, feedback: &str) {
    let g_chars: Vec<char> = guess.chars().collect();
    let f_chars: Vec<char> = feedback.chars().collect();

    for i in 0..5 {
        let color = match f_chars[i] {
            'G' => "\x1b[42m",
            'Y' => "\x1b[43m",
            'R' => "\x1b[41m",
            _ => "",
        };
        print!("{}{}\x1b[0m ", color, g_chars[i]);
    }
    println!();
}

pub fn print_keyboard(keyboard: &HashMap<char, Status>) {
    let rows = vec![
        "QWERTYUIOP".to_string(),
        "ASDFGHJKL".to_string(),
        "ZXCVBNM".to_string(),
    ];

    println!("键盘状态:");
    for row in rows {
        for ch in row.chars() {
            let st = keyboard.get(&ch).unwrap_or(&Status::Unknown);
            let color = match st {
                Status::Green => "\x1b[42m",
                Status::Yellow => "\x1b[43m",
                Status::Red => "\x1b[41m",
                Status::Unknown => "",
            };
            print!("{}{}\x1b[0m ", color, ch);
        }
        println!();
    }
}

pub fn is_tty() -> bool {
    atty::is(atty::Stream::Stdout)
}
