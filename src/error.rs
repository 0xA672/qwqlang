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
                write_source_context(f, input, pos.line, pos.col, 1)?;
                write_suggestion(f, msg, input, pos.line, pos.col)
            }
            Error::Compile { pos, msg, input, filename } => {
                write_error_header(f, "compile error", filename.as_deref(), Some(pos))?;
                writeln!(f, "  {}", msg)?;
                if let Some(src) = input {
                    write_source_context(f, src, pos.line, pos.col, 1)?;
                    write_suggestion(f, msg, src, pos.line, pos.col)
                } else {
                    Ok(())
                }
            }
            Error::Runtime { pos, msg, input, filename } => {
                write_error_header(f, "runtime error", filename.as_deref(), pos.as_ref())?;
                writeln!(f, "  {}", msg)?;
                if let (Some(p), Some(src)) = (pos, input) {
                    write_source_context(f, src, p.line, p.col, 1)?;
                    write_suggestion(f, msg, src, p.line, p.col)
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
    let lines: Vec<&str> = input.lines().collect();
    let line_idx = line.saturating_sub(1);
    let line_num = line.to_string();
    let padding = " ".repeat(line_num.len());

    writeln!(f, "{} |", padding)?;
    
    // Show previous line if available
    if line_idx > 0 {
        let prev_line = lines.get(line_idx - 1).unwrap_or(&"");
        writeln!(f, "{} | {}", (line - 1).to_string(), prev_line)?;
    }
    
    // Current line
    let current_line = lines.get(line_idx).unwrap_or(&"");
    writeln!(f, "{} | {}", line_num, current_line)?;
    
    // Error pointer
    writeln!(
        f,
        "{} | {}{}",
        padding,
        " ".repeat(col.saturating_sub(1)),
        "^".repeat(width.max(1))
    )?;
    
    // Show next line if available
    if line_idx + 1 < lines.len() {
        let next_line = lines.get(line_idx + 1).unwrap_or(&"");
        writeln!(f, "{} | {}", (line + 1).to_string(), next_line)?;
    }
    
    writeln!(f, "{} |", padding)
}

fn write_suggestion(
    f: &mut fmt::Formatter,
    msg: &str,
    input: &str,
    line: usize,
    col: usize,
) -> fmt::Result {
    let lines: Vec<&str> = input.lines().collect();
    let current_line = lines.get(line.saturating_sub(1)).unwrap_or(&"");
    
    if msg.starts_with("undefined variable") || msg.starts_with("undefined function") {
        if let Some(word) = extract_word_at(current_line, col) {
            let candidates = ["let", "mut", "fn", "if", "else", "return", "while", "for", "loop",
                              "break", "continue", "true", "false", "null", "and", "or", "in",
                              "print", "len", "push", "pop", "Ok", "Err", "Some", "None"];
            if let Some(suggestion) = find_best_suggestion(&word, &candidates) {
                return writeln!(f, "  = help: did you mean '{}'?", suggestion);
            }
        }
    }
    
    if msg.starts_with("unexpected token") || msg.starts_with("expected") {
        if let Some(c) = current_line.chars().nth(col.saturating_sub(1)) {
            match c {
                ';' => {
                    return writeln!(f, "  = help: missing expression before ';'");
                }
                '}' => {
                    return writeln!(f, "  = help: unexpected closing brace - check for matching open brace");
                }
                ')' => {
                    return writeln!(f, "  = help: unexpected closing paren - check for matching open paren");
                }
                ']' => {
                    return writeln!(f, "  = help: unexpected closing bracket - check for matching open bracket");
                }
                _ => {}
            }
        }
    }
    
    if msg.starts_with("expected ';'") {
        return writeln!(f, "  = help: try adding ';' at the end of the statement");
    }
    
    if msg.starts_with("expected '='") {
        return writeln!(f, "  = help: try adding '=' to assign a value");
    }
    
    if msg.starts_with("expected '('") {
        return writeln!(f, "  = help: expected '(' - did you forget it?");
    }
    
    if msg.starts_with("expected ')'") {
        return writeln!(f, "  = help: expected ')' - check for mismatched parentheses");
    }
    
    if msg.starts_with("expected '{'") {
        return writeln!(f, "  = help: expected '{{' - did you forget the opening brace?");
    }
    
    if msg.starts_with("expected '}'") {
        return writeln!(f, "  = help: expected '}}' - check for mismatched braces");
    }
    
    Ok(())
}

fn extract_word_at(line: &str, col: usize) -> Option<String> {
    let idx = col.saturating_sub(1);
    let chars: Vec<char> = line.chars().collect();
    if idx >= chars.len() {
        return None;
    }
    
    // Expand left
    let mut start = idx;
    while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
        start -= 1;
    }
    
    // Expand right
    let mut end = idx;
    while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
        end += 1;
    }
    
    if start == end {
        return None;
    }
    
    Some(chars[start..end].iter().collect())
}

fn find_best_suggestion<'a>(word: &str, candidates: &'a [&'a str]) -> Option<&'a str> {
    let mut best = None;
    let mut best_dist = usize::MAX;
    
    for candidate in candidates {
        let dist = levenshtein(word, candidate);
        if dist < best_dist && dist <= 3 {
            best_dist = dist;
            best = Some(*candidate);
        }
    }
    
    best
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
