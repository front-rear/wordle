// 主流程，以及最后两个基本功能，由于实现得比较晚，没有再分文件
mod args;
mod builtin_words;
mod game_logic;
mod game_state;
mod status;
mod tui;
mod ui;
mod word_sets;

use clap::Parser;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use serde_json;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::io::{self, Write};

use args::Args;
use game_logic::{
    compute_feedback, is_valid_hard_mode, update_hard_mode_constraints, update_keyboard,
};
use game_state::{GameRecord, GameState, GameStats, RandomModeState};
use status::Status;
use ui::{is_tty, print_guess, print_keyboard};
use word_sets::{load_builtin_word_sets, load_word_set_from_file};

use crate::args::ConfigFile;

fn load_game_state(filename: &str) -> io::Result<GameState> {
    let content = match fs::read_to_string(filename) {
        Ok(content) => content,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(GameState::default()),
        Err(e) => return Err(e),
    };

    if content.trim().is_empty() || content.trim() == "{}" {
        return Ok(GameState::default());
    }

    let state: GameState = serde_json::from_str(&content).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Invalid JSON format: {}", e),
        )
    })?;
    Ok(state)
}

fn save_game_state(filename: &str, state: &GameState) -> io::Result<()> {
    let json = serde_json::to_string_pretty(state).map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to serialize state: {}", e),
        )
    })?;
    fs::write(filename, json)
}

fn load_config_file(filename: &str) -> io::Result<ConfigFile> {
    let content = fs::read_to_string(filename)?;
    let config: ConfigFile = serde_json::from_str(&content).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Invalid config file: {}", e),
        )
    })?;
    Ok(config)
}

fn merge_config(args: &mut Args, config: &ConfigFile) {
    // 如果命令行参数没有设置，使用配置文件中的值
    if config.random.is_some() && !args.random {
        args.random = config.random.unwrap();
    }

    if config.difficult.is_some() && !args.difficult {
        args.difficult = config.difficult.unwrap();
    }

    if config.stats.is_some() && !args.stats {
        args.stats = config.stats.unwrap();
    }

    if config.day.is_some() && args.day.is_none() {
        args.day = config.day;
    }

    if config.seed.is_some() && args.seed.is_none() {
        args.seed = config.seed;
    }

    if config.final_set.is_some() && args.final_set.is_none() {
        args.final_set = config.final_set.clone();
    }

    if config.acceptable_set.is_some() && args.acceptable_set.is_none() {
        args.acceptable_set = config.acceptable_set.clone();
    }

    if config.state.is_some() && args.state.is_none() {
        args.state = config.state.clone();
    }

    if config.word.is_some() && args.word.is_none() {
        args.word = config.word.clone();
    }

    if config.tui.is_some() && !args.tui {
        args.tui = config.tui.unwrap();
    }
}

fn main() -> io::Result<()> {
    let mut args = Args::parse();

    // 加载配置文件（如果指定了）
    if let Some(config_file) = &args.config {
        match load_config_file(config_file) {
            Ok(config) => {
                // 合并配置，命令行参数优先级更高
                merge_config(&mut args, &config);
            }
            Err(e) => {
                eprintln!("Failed to load config file: {}", e);
                std::process::exit(1);
            }
        }
    }

    // 参数冲突检查
    if args.tui && args.random {
        eprintln!("错误：TUI模式不支持随机模式，请去掉 --random 参数");
        std::process::exit(1);
    }

    // 加载游戏状态（如果指定了状态文件）
    let mut game_state = if let Some(state_file) = &args.state {
        match load_game_state(state_file) {
            Ok(state) => state,
            Err(e) => {
                eprintln!("Failed to load state file: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        // 没有指定状态文件，创建新状态
        GameState::default()
    };

    // 加载词库
    let (finals, acceptables) = if args.final_set.is_some() || args.acceptable_set.is_some() {
        let final_file = args.final_set.ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "必须同时指定 -f 和 -a 参数")
        })?;

        let acceptable_file = args.acceptable_set.ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "必须同时指定 -f 和 -a 参数")
        })?;

        let finals = load_word_set_from_file(&final_file, "候选词库")?;
        let acceptables = load_word_set_from_file(&acceptable_file, "可用词库")?;

        // 检查候选词库是否是可用词库的子集
        let finals_set: HashSet<&String> = finals.iter().collect();
        let acceptables_set: HashSet<&String> = acceptables.iter().collect();

        if !finals_set.is_subset(&acceptables_set) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "候选词库不是可用词库的子集",
            ));
        }

        (finals, acceptables)
    } else {
        load_builtin_word_sets()
    };

    // 参数冲突检查
    if args.random && args.word.is_some() {
        eprintln!("Cannot specify both -r/--random and -w/--word");
        std::process::exit(1);
    }

    if (args.day.is_some() || args.seed.is_some()) && args.word.is_some() {
        eprintln!("Cannot specify -d/--day or -s/--seed with -w/--word");
        std::process::exit(1);
    }

    let stdin = io::stdin();
    let mut lines = stdin.lines();

    // 转换为HashSet以便快速查找
    let finals_set: HashSet<String> = finals.clone().into_iter().collect(); // 修复：使用克隆后的finals
    let acceptables_set: HashSet<String> = acceptables.clone().into_iter().collect(); // 修复：使用克隆后的acceptables

    // 确定输出模式
    let is_interactive = ui::is_tty();
    let mut stats = GameStats::new();
    let mut played_words = HashSet::new();

    // 初始化随机模式状态
    let mut random_state = if args.random {
        let seed = args.seed.unwrap_or(0);
        let day = args.day.unwrap_or(1);

        // 获取所有已玩过的答案词
        let played_answers: HashSet<String> = game_state
            .games
            .iter()
            .map(|record| record.answer.clone())
            .collect();

        // 使用正确的词库（finals 而不是 finals_set）
        let mut available_words: Vec<String> = finals
            .iter()
            .filter(|word| !played_answers.contains(*word))
            .cloned()
            .collect();

        // 极其重要！！！！！！！！是先排序后筛词！！！！！
        let mut all_words: Vec<String> = finals
            .iter()
            //.filter(|word| !played_answers.contains(*word))
            .cloned()
            .collect();

        if available_words.is_empty() {
            if is_interactive {
                println!("所有单词已玩完！");
            } else {
                println!("FAILED All words played");
            }
            std::process::exit(1);
        }

        let mut rng = StdRng::seed_from_u64(seed);
        all_words.shuffle(&mut rng);

        let mut shuffled_words: VecDeque<String> = all_words.into_iter().collect();

        // Skip to the current day
        for _ in 0..(day as usize - 1) {
            if shuffled_words.pop_front().is_none() {
                if is_interactive {
                    println!("所有单词已玩完！");
                } else {
                    println!("FAILED All words played");
                }
                std::process::exit(1);
            }
        }

        Some(RandomModeState {
            shuffled_words,
            seed,
            initial_day: day,
            current_index: day,
        })
    } else {
        None
    };

    // 游戏主循环
    loop {
        // 确定答案
        let (answer, day) = if args.random {
            // 从随机状态中获取下一个单词
            if let Some(state) = &mut random_state {
                if state.shuffled_words.is_empty() {
                    if is_interactive {
                        println!("所有单词已玩完！");
                    } else {
                        println!("FAILED All words played");
                    }
                    break;
                }

                let answer = state.shuffled_words.pop_front().unwrap();
                let current_day = state.current_index;
                state.current_index += 1;

                (answer, current_day)
            } else {
                eprintln!("Random state not initialized");
                std::process::exit(1);
            }
        } else if let Some(w) = &args.word {
            // 指定单词模式
            let aw = w.trim().to_uppercase();
            if aw.len() != 5 || !finals_set.contains(&aw) {
                eprintln!("Invalid word: {}", w);
                std::process::exit(1);
            }
            (aw, 1)
        } else {
            // 交互模式或测试模式
            if is_interactive {
                println!("请输入答案词（5字母，来自候选词库）:");
            }

            let line = lines.next().expect("Failed to read answer").unwrap();
            let aw = line.trim().to_uppercase();
            if aw.len() != 5 {
                std::process::exit(1);
            }
            if is_interactive && !finals_set.contains(&aw) {
                eprintln!("Invalid answer word");
                std::process::exit(1);
            }
            (aw, 1)
        };

        // 检查随机模式下单词是否重复
        if args.random && played_words.contains(&answer) {
            if is_interactive {
                println!("单词 {} 已玩过，跳过", answer);
            }
            continue;
        }
        played_words.insert(answer.clone());

        // 游戏状态初始化
        let mut guesses: Vec<String> = Vec::new();
        let mut feedbacks: Vec<String> = Vec::new();
        let mut keyboard: HashMap<char, Status> = HashMap::new();
        for c in 'A'..='Z' {
            keyboard.insert(c, Status::Unknown);
        }

        let mut known_positions: [Option<char>; 5] = [None; 5];
        let mut min_counts: HashMap<char, usize> = HashMap::new();

        let mut attempt = 0;
        let mut won = false;

        // TUI模式分支
        if args.tui {
            match tui::run_tui(
                answer.clone(),
                args.difficult,
                finals_set.clone(),
                acceptables_set.clone(),
            ) {
                Ok((tui_won, tui_attempts, tui_guesses)) => {
                    won = tui_won;
                    attempt = tui_attempts;
                    guesses = tui_guesses;
                    
                    // 为了统计需要重构feedbacks
                    feedbacks = guesses.iter().map(|guess| {
                        game_logic::compute_feedback(guess, &answer)
                    }).collect();
                }
                Err(e) => {
                    eprintln!("TUI 错误: {}", e);
                    std::process::exit(1);
                }
            }
        } else {
            // 原有的命令行游戏循环
        'game: while attempt < 6 {
            if is_interactive {
                println!(
                    "请输入第 {} 次猜测（剩余 {} 次）:",
                    attempt + 1,
                    6 - attempt
                );
            }

            let input: String = match lines.next() {
                Some(Ok(line)) => line.trim().to_uppercase(),
                _ => break 'game,
            };

            if input.len() != 5 || !acceptables_set.contains(&input) {
                println!("INVALID");
                continue;
            }

            if args.difficult && !is_valid_hard_mode(&input, &known_positions, &min_counts) {
                println!("INVALID");
                continue;
            }

            // 计算反馈
            let feedback = game_logic::compute_feedback(&input, &answer);

            // 更新困难模式约束
            if args.difficult {
                game_logic::update_hard_mode_constraints(
                    &input,
                    &feedback,
                    &mut known_positions,
                    &mut min_counts,
                );
            }

            // 更新键盘状态
            game_logic::update_keyboard(&input, &feedback, &mut keyboard);

            guesses.push(input.clone());
            feedbacks.push(feedback.clone());
            attempt += 1;

            // 输出
            if is_interactive {
                ui::print_guess(&input, &feedback);
                ui::print_keyboard(&keyboard);
            } else {
                // 测试模式输出
                print!("{}", feedback);
                print!(" ");
                for c in 'A'..='Z' {
                    print!("{}", keyboard.get(&c).unwrap_or(&Status::Unknown).to_char());
                }
                println!();
            }

            // 检查是否猜对
            if input == answer {
                won = true;
                if is_interactive {
                    println!("🎉 恭喜！你在第 {} 次猜中了！", attempt);
                } else {
                    println!("CORRECT {}", attempt);
                }
                break 'game;
            }
        }
        } // 结束TUI/CLI模式分支

        // 游戏结束处理
        if !won {
            if is_interactive {
                println!("😭 游戏结束！正确答案是：{}", answer);
            } else {
                println!("FAILED {}", answer);
            }
        }

        // 更新游戏状态
        game_state.total_rounds += 1;
        game_state.games.push(GameRecord {
            answer: answer.clone(),
            guesses: guesses.clone(),
            ..Default::default()
        });

        // 保存状态（如果指定了状态文件）
        if let Some(state_file) = &args.state {
            if let Err(e) = save_game_state(state_file, &game_state) {
                eprintln!("Failed to save state: {}", e);
            }
        }

        // 更新统计数据
        stats.add_game(won, attempt as u32, &guesses);

        // 显示统计数据（包含所有游戏）
        if args.stats {
            if is_interactive {
                println!("当前统计数据:");
            }

            // 创建包含所有游戏的统计信息
            let mut all_stats = GameStats::new();
            for record in &game_state.games {
                let won = record
                    .guesses
                    .last()
                    .map_or(false, |last_guess| last_guess == &record.answer);
                let attempts = record.guesses.len() as u32;
                all_stats.add_game(won, attempts, &record.guesses);
            }

            println!("{}", all_stats);
        }

        // 检查是否继续游戏
        if args.word.is_some() {
            break;
        } else if args.random {
            if is_interactive {
                println!("是否继续下一局？(Y/N)");
            }

            match lines.next() {
                Some(Ok(line)) if line.trim().eq_ignore_ascii_case("Y") => continue,
                _ => break,
            }
        } else {
            if is_interactive {
                println!("是否继续下一局？(Y/N)");
            }

            match lines.next() {
                Some(Ok(line)) if line.trim().eq_ignore_ascii_case("Y") => continue,
                _ => break,
            }
        }
    }

    Ok(())
}