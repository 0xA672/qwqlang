use crate::ast::Pos;
use std::fmt;

#[derive(Debug)]
pub enum Error {
    Syntax {
        pos: Pos,
        msg: String,
        input: String,
        filename: Option<String>,
    },
    Compile {
        pos: Pos,
        msg: String,
        input: Option<String>,
        filename: Option<String>,
    },
    Runtime {
        pos: Option<Pos>,
        msg: String,
        input: Option<String>,
        filename: Option<String>,
    },
}

impl Error {
    pub fn with_filename(mut self, filename: String) -> Self {
        match &mut self {
            Error::Syntax { filename: f, .. } => *f = Some(filename),
            Error::Compile { filename: f, .. } => *f = Some(filename),
            Error::Runtime { filename: f, .. } => *f = Some(filename),
        }
        self
    }

    pub fn with_input(mut self, input: String) -> Self {
        match &mut self {
            Error::Syntax { .. } => {}
            Error::Compile { input: i, .. } => *i = Some(input),
            Error::Runtime { input: i, .. } => *i = Some(input),
        }
        self
    }

    pub fn syntax(pos: Pos, msg: String, input: &str) -> Self {
        Error::Syntax {
            pos,
            msg,
            input: input.to_string(),
            filename: None,
        }
    }

    pub fn compile(pos: Pos, msg: String) -> Self {
        Error::Compile {
            pos,
            msg,
            input: None,
            filename: None,
        }
    }

    pub fn runtime(msg: String) -> Self {
        Error::Runtime {
            pos: None,
            msg,
            input: None,
            filename: None,
        }
    }

    pub fn runtime_at(pos: Pos, msg: String) -> Self {
        Error::Runtime {
            pos: Some(pos),
            msg,
            input: None,
            filename: None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Syntax { pos, msg, input, filename } => {
                write_error_header(f, "syntax error", filename.as_deref(), Some(pos))?;
                writeln!(f, "  {}", msg)?;
                write_source_context(f, input, pos.line, pos.col, 1)
            }
            Error::Compile { pos, msg, input, filename } => {
                write_error_header(f, "compile error", filename.as_deref(), Some(pos))?;
                writeln!(f, "  {}", msg)?;
                if let Some(src) = input {
                    write_source_context(f, src, pos.line, pos.col, 1)
                } else {
                    Ok(())
                }
            }
            Error::Runtime { pos, msg, input, filename } => {
                write_error_header(f, "runtime error", filename.as_deref(), pos.as_ref())?;
                writeln!(f, "  {}", msg)?;
                if let (Some(p), Some(src)) = (pos, input) {
                    write_source_context(f, src, p.line, p.col, 1)
                } else {
                    Ok(())
                }
            }
        }
    }
}

impl std::error::Error for Error {}

fn write_error_header(
    f: &mut fmt::Formatter,
    kind: &str,
    filename: Option<&str>,
    pos: Option<&Pos>,
) -> fmt::Result {
    match (filename, pos) {
        (Some(file), Some(p)) => write!(f, "{}:{}:{}: {}", file, p.line, p.col, kind),
        (Some(file), None) => write!(f, "{}: {}", file, kind),
        (None, Some(p)) => write!(f, "{}:{}: {}", p.line, p.col, kind),
        (None, None) => write!(f, "{}", kind),
    }
}

fn write_source_context(
    f: &mut fmt::Formatter,
    input: &str,
    line: usize,
    col: usize,
    width: usize,
) -> fmt::Result {
    let line_text = input.lines().nth(line.saturating_sub(1)).unwrap_or("");
    let line_num = line.to_string();
    let padding = " ".repeat(line_num.len());

    writeln!(f, "{} |", padding)?;
    writeln!(f, "{} | {}", line_num, line_text)?;
    writeln!(
        f,
        "{} | {}{}",
        padding,
        " ".repeat(col.saturating_sub(1)),
        "^".repeat(width.max(1))
    )?;
    writeln!(f, "{} |", padding)
}

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
            dp[i][j] = (dp[i - 1][j] + 1).min(dp[i][j - 1] + 1).min(dp[i - 1][j - 1] + cost);
        }
    }
    dp[a_chars.len()][b_chars.len()]
}
