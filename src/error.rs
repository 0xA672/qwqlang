use crate::ast::Pos;
use std::fmt;

#[derive(Debug)]
pub enum Error {
    Syntax {
        pos: Pos,
        msg: String,
        input: String,
    },
    Compile {
        pos: Pos,
        msg: String,
    },
    Runtime {
        pos: Option<Pos>,
        msg: String,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Syntax { pos, msg, input } => {
                let line = input.lines().nth(pos.line - 1).unwrap_or("");
                let prefix = " ".repeat(pos.col - 1);
                let underline = "^".repeat(1);
                writeln!(f, "{}:{}: syntax error: {}", pos.line, pos.col, msg)?;
                writeln!(f, "{}", line)?;
                writeln!(f, "{}{}", prefix, underline)
            }
            Error::Compile { pos, msg } => {
                writeln!(f, "{}:{}: compile error: {}", pos.line, pos.col, msg)
            }
            Error::Runtime { pos, msg } => match pos {
                Some(p) => writeln!(f, "{}:{}: runtime error: {}", p.line, p.col, msg),
                None => writeln!(f, "runtime error: {}", msg),
            },
        }
    }
}

impl std::error::Error for Error {}

pub fn levenshtein(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let mut dp = vec![vec![0; b_chars.len() + 1]; a_chars.len() + 1];

    for i in 0..=a_chars.len() {
        dp[i][0] = i;
    }
    for j in 0..=b_chars.len() {
        dp[0][j] = j;
    }

    for i in 1..=a_chars.len() {
        for j in 1..=b_chars.len() {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            dp[i][j] = dp[i - 1][j].min(dp[i][j - 1]).min(dp[i - 1][j - 1] + cost);
        }
    }
    dp[a_chars.len()][b_chars.len()]
}
