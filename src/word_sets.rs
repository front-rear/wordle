use std::collections::HashSet;
use std::fs;
use std::io;

pub fn load_word_set_from_file(filename: &str, description: &str) -> io::Result<Vec<String>> {
    let content = fs::read_to_string(filename).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("无法读取{}文件 {}: {}", description, filename, e),
        )
    })?;

    let words: Vec<String> = content
        .lines()
        .map(|line| line.trim().to_uppercase())
        .collect();

    for word in &words {
        if word.len() != 5 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{}文件中的单词 '{}' 长度不是5个字母", description, word),
            ));
        }

        if !word.chars().all(|c| c.is_ascii_alphabetic()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{}文件中的单词 '{}' 包含非字母字符", description, word),
            ));
        }
    }

    let unique_words: HashSet<&String> = words.iter().collect();
    if unique_words.len() != words.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{}文件中存在重复的单词", description),
        ));
    }

    let mut sorted_words = words;
    sorted_words.sort();
    Ok(sorted_words)
}

pub fn load_builtin_word_sets() -> (Vec<String>, Vec<String>) {
    use crate::builtin_words::{ACCEPTABLE, FINAL};

    let finals: Vec<String> = FINAL.iter().map(|&s| s.to_uppercase()).collect();
    let acceptables: Vec<String> = ACCEPTABLE.iter().map(|&s| s.to_uppercase()).collect();

    (finals, acceptables)
}
