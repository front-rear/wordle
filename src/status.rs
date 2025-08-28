#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Status {
    Green,
    Yellow,
    Red,
    Unknown,
}

impl Status {
    pub fn to_char(&self) -> char {
        match self {
            Status::Green => 'G',
            Status::Yellow => 'Y',
            Status::Red => 'R',
            Status::Unknown => 'X',
        }
    }

    pub fn priority(&self) -> i32 {
        match self {
            Status::Green => 3,
            Status::Yellow => 2,
            Status::Red => 1,
            Status::Unknown => 0,
        }
    }

    pub fn from_feedback(c: char) -> Self {
        match c {
            'G' => Status::Green,
            'Y' => Status::Yellow,
            'R' => Status::Red,
            _ => Status::Unknown,
        }
    }
}
