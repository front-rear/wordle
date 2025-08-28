use clap::Parser;
use serde::Deserialize;

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Args {
    #[arg(short = 'w', long)]
    pub word: Option<String>,

    #[arg(short = 'r', long)]
    pub random: bool,

    #[arg(short = 'D', long)]
    pub difficult: bool,

    #[arg(short = 't', long)]
    pub stats: bool,

    #[arg(short = 'd', long)]
    pub day: Option<u32>,

    #[arg(short = 's', long)]
    pub seed: Option<u64>,

    #[arg(short = 'f', long)]
    pub final_set: Option<String>,

    #[arg(short = 'a', long)]
    pub acceptable_set: Option<String>,

    #[arg(short = 'S', long)]
    pub state: Option<String>,

    #[arg(short = 'c', long)]
    pub config: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ConfigFile {
    pub random: Option<bool>,
    pub difficult: Option<bool>,
    pub stats: Option<bool>,
    pub day: Option<u32>,
    pub seed: Option<u64>,
    pub final_set: Option<String>,
    pub acceptable_set: Option<String>,
    pub state: Option<String>,
    pub word: Option<String>,
}
