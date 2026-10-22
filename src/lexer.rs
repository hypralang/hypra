use crate::diag::Diagnostic;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Pos {
    /// Index into the compiled project's source files; 0 is the entry file.
    pub file: usize,
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Ident(String),
    Str(String),
    Num(String),
    LBrace,
    RBrace,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Colon,
    Comma,
    DotDot,
    Eq,
    /// `==` and `!=`, in a condition.
    EqEq,
    NotEq,
    /// Statement separator. Never emitted inside parentheses, so argument lists may span lines.
    Newline,
    /// `// ...`, kept only for the formatter.
    Comment(String),
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub tok: Tok,
    pub pos: Pos,
    pub len: usize,
}

struct Lexer {
    file: usize,
    chars: Vec<char>,
    i: usize,
    line: usize,
    col: usize,
}

impl Lexer {
    fn peek(&self, offset: usize) -> Option<char> {
        self.chars.get(self.i + offset).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek(0)?;
        self.i += 1;
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(c)
    }

    fn pos(&self) -> Pos {
        Pos { file: self.file, line: self.line, col: self.col }
    }
}

/// The escape after a backslash in a string, e.g. `n` for a line break.
fn escape(lx: &mut Lexer, start: Pos) -> Result<char, Diagnostic> {
    let at = lx.pos();
    match lx.bump() {
        Some('n') => Ok('\n'),
        Some('t') => Ok('\t'),
        Some(c @ ('"' | '\\')) => Ok(c),
        Some(c) => Err(Diagnostic::error("LEX-003", format!("unknown escape `\\{c}`"), at, 2)
            .help("strings know \\\" \\\\ \\n and \\t; write \\\\ for a backslash")),
        None => Err(Diagnostic::error("LEX-002", "unterminated string", start, 1)),
    }
}

/// A string over several lines, between `"""` lines:
///
/// ```text
/// p """
///   First line
///   Second line
///   """
/// ```
///
/// The closing `"""` sets the indentation: it's removed from every line, so the text can be indented with the
/// code around it. The line breaks after the opening and before the closing `"""` aren't part of the text.
/// Backslashes are text too, so code keeps its own escapes: `join("\n")` stays `join("\n")`.
fn multiline(lx: &mut Lexer, start: Pos) -> Result<String, Diagnostic> {
    let unterminated =
        |help: &str| Diagnostic::error("LEX-002", "unterminated string", start, 3).help(help.to_string());
    for _ in 0..3 {
        lx.bump();
    }
    while matches!(lx.peek(0), Some(' ' | '\t' | '\r')) {
        lx.bump();
    }
    if lx.bump() != Some('\n') {
        return Err(unterminated("the text of a \"\"\" string starts on the next line"));
    }
    let mut lines: Vec<String> = vec![];
    loop {
        let mut indent = String::new();
        while let Some(c @ (' ' | '\t')) = lx.peek(0) {
            indent.push(c);
            lx.bump();
        }
        if lx.peek(0) == Some('"') && lx.peek(1) == Some('"') && lx.peek(2) == Some('"') {
            for _ in 0..3 {
                lx.bump();
            }
            let mut text = vec![];
            for line in lines {
                match line.strip_prefix(indent.as_str()) {
                    Some(rest) => text.push(rest.to_string()),
                    None if line.trim().is_empty() => text.push(String::new()),
                    None => {
                        return Err(Diagnostic::error(
                            "LEX-002",
                            "a line is indented less than the closing \"\"\"",
                            start,
                            3,
                        )
                        .help("indent every line at least as far as the closing \"\"\""));
                    }
                }
            }
            return Ok(text.join("\n"));
        }
        let mut line = indent;
        loop {
            match lx.bump() {
                None => return Err(unterminated("close the string with \"\"\" on a line of its own")),
                Some('\n') => break,
                Some('\r') if lx.peek(0) == Some('\n') => {}
                Some(c) => line.push(c),
            }
        }
        lines.push(line);
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

pub fn lex(src: &str, file: usize) -> Result<Vec<Token>, Diagnostic> {
    tokens(src, file, false)
}

/// The tokens with comments kept, for the formatter.
pub fn lex_with_comments(src: &str) -> Result<Vec<Token>, Diagnostic> {
    tokens(src, 0, true)
}

fn tokens(src: &str, file: usize, keep_comments: bool) -> Result<Vec<Token>, Diagnostic> {
    let mut lx = Lexer { file, chars: src.chars().collect(), i: 0, line: 1, col: 1 };
    let mut out: Vec<Token> = Vec::new();
    // inside parentheses, line breaks don't end anything, so arguments can span lines
    let mut depth = 0usize;

    while let Some(c) = lx.peek(0) {
        let pos = lx.pos();
        let start = lx.i;
        let tok = match c {
            '\n' => {
                lx.bump();
                let after_newline = matches!(out.last().map(|t| &t.tok), Some(Tok::Newline) | None);
                if depth > 0 || after_newline {
                    continue;
                }
                Tok::Newline
            }
            ' ' | '\t' | '\r' => {
                lx.bump();
                continue;
            }
            '/' if lx.peek(1) == Some('/') => {
                let mut text = String::new();
                while let Some(c) = lx.peek(0).filter(|c| *c != '\n') {
                    text.push(c);
                    lx.bump();
                }
                if !keep_comments {
                    continue;
                }
                Tok::Comment(text.trim_end().to_string())
            }
            '.' if lx.peek(1) == Some('.') => {
                lx.bump();
                lx.bump();
                Tok::DotDot
            }
            '=' | '!' if lx.peek(1) == Some('=') => {
                lx.bump();
                lx.bump();
                if c == '=' { Tok::EqEq } else { Tok::NotEq }
            }
            '"' if lx.peek(1) == Some('"') && lx.peek(2) == Some('"') => Tok::Str(multiline(&mut lx, pos)?),
            '"' => Tok::Str(string(&mut lx, pos)?),
            c if c.is_ascii_digit() || (c == '-' && lx.peek(1).is_some_and(|d| d.is_ascii_digit())) => {
                Tok::Num(number(&mut lx))
            }
            c if is_ident_start(c) => Tok::Ident(take_ident(&mut lx)),
            c => {
                let Some(tok) = punctuation(c) else {
                    let d = Diagnostic::error("LEX-001", format!("unexpected character `{c}`"), pos, 1);
                    return Err(match c {
                        '@' | '.' | '#' => {
                            d.help("attribute names with characters like @ go in quotes: \"@click\": \"open()\"")
                        }
                        _ => d,
                    });
                };
                lx.bump();
                match tok {
                    Tok::LParen | Tok::LBracket => depth += 1,
                    Tok::RParen | Tok::RBracket => depth = depth.saturating_sub(1),
                    _ => {}
                }
                tok
            }
        };
        out.push(Token { tok, pos, len: lx.i - start });
    }

    out.push(Token { tok: Tok::Eof, pos: lx.pos(), len: 1 });
    Ok(out)
}

fn punctuation(c: char) -> Option<Tok> {
    Some(match c {
        '{' => Tok::LBrace,
        '}' => Tok::RBrace,
        '(' => Tok::LParen,
        ')' => Tok::RParen,
        '[' => Tok::LBracket,
        ']' => Tok::RBracket,
        ':' => Tok::Colon,
        ',' => Tok::Comma,
        '=' => Tok::Eq,
        _ => return None,
    })
}

/// A string on one line: `"Hello {user.name}"`.
fn string(lx: &mut Lexer, start: Pos) -> Result<String, Diagnostic> {
    lx.bump();
    let mut s = String::new();
    loop {
        match lx.bump() {
            None | Some('\n') => {
                return Err(Diagnostic::error("LEX-002", "unterminated string", start, 1)
                    .help("close the string with `\"` on the same line, or use \"\"\" for text over several lines"));
            }
            Some('"') => return Ok(s),
            Some('\\') => s.push(escape(lx, start)?),
            Some(c) => s.push(c),
        }
    }
}

/// A number as written, optionally negative: `42`, `13.40`, `-1`.
fn number(lx: &mut Lexer) -> String {
    let mut s = String::new();
    if lx.peek(0) == Some('-') {
        s.push('-');
        lx.bump();
    }
    while let Some(c) = lx.peek(0) {
        let decimal_point = c == '.' && lx.peek(1).is_some_and(|d| d.is_ascii_digit());
        if !(c.is_ascii_digit() || decimal_point) {
            break;
        }
        s.push(c);
        lx.bump();
    }
    s
}

/// Identifiers may contain dots (`product.name`, `cart.add`) and inner dashes (`SEO-003`, `x-cloak`).
fn take_ident(lx: &mut Lexer) -> String {
    let mut s = String::new();
    while let Some(c) = lx.peek(0) {
        let next = lx.peek(1);
        let ok = c.is_ascii_alphanumeric()
            || c == '_'
            || (c == '.' && next.is_some_and(is_ident_start))
            || (c == '-' && next.is_some_and(|n| n.is_ascii_alphanumeric()));
        if !ok {
            break;
        }
        s.push(c);
        lx.bump();
    }
    s
}
