//! `hypra fmt`: one layout for every Hypra file. It works on the tokens rather than the syntax tree, so comments
//! and the author's line breaks stay. It sets the indentation by nesting, the spacing around punctuation, and at
//! most one blank line in a row; the tokens themselves never change.

use crate::diag::Diagnostic;
use crate::lexer::{Tok, Token, lex_with_comments};

const INDENT: usize = 2;

pub fn format(src: &str) -> Result<String, Diagnostic> {
    let tokens: Vec<Token> =
        lex_with_comments(src)?.into_iter().filter(|t| !matches!(t.tok, Tok::Newline | Tok::Eof)).collect();
    let source = Source::new(src);
    let mut out = String::new();
    // for each bracket still open, `{`, `(` or `[`, the indentation level of what's inside it; brackets opened on
    // one line all indent their contents by one level, so `div(data: [` indents its list once
    let mut open: Vec<usize> = vec![];
    let mut level = 0usize;
    // the token before, where it ends in the source, and how far the current line moved to the left or right
    let mut prev: Option<(&Tok, usize, usize)> = None;
    let mut shift = 0isize;

    for (i, t) in tokens.iter().enumerate() {
        let text = match &t.tok {
            Tok::Comment(comment) => comment.clone(),
            _ => source.text(t),
        };
        match prev {
            Some((before, end_line, end_col)) if t.pos.line == end_line => {
                let gap = t.pos.col.saturating_sub(end_col);
                out.push_str(&" ".repeat(space(before, &t.tok, gap)));
            }
            _ => {
                if let Some((before, end_line, _)) = prev {
                    out.push('\n');
                    // a blank line stays, except right after an opening or before a closing bracket
                    let blank = t.pos.line > end_line + 1;
                    if blank && !is_opening(before) && !is_closing(&t.tok) {
                        out.push('\n');
                    }
                }
                // closing brackets at the start of a line belong to the level they close
                let closing = tokens[i..].iter().take_while(|n| n.pos.line == t.pos.line && is_closing(&n.tok)).count();
                level = match open.len().checked_sub(closing + 1) {
                    Some(inner) => open[inner],
                    None => 0,
                };
                let indent = INDENT * level;
                out.push_str(&" ".repeat(indent));
                shift = indent as isize - (t.pos.col as isize - 1);
            }
        }
        out.push_str(&shift_lines(&text, shift));

        if is_opening(&t.tok) {
            open.push(level + 1);
        } else if is_closing(&t.tok) {
            open.pop();
        }
        let lines = text.matches('\n').count();
        let end_col = match text.rfind('\n') {
            Some(last) => text[last + 1..].chars().count() + 1,
            None => t.pos.col + t.len,
        };
        prev = Some((&t.tok, t.pos.line + lines, end_col));
    }
    if !out.is_empty() {
        out.push('\n');
    }
    Ok(out)
}

/// Spaces between two tokens on one line. `gap` is how many the author wrote; between items, one or two are
/// kept, so `a("/") "Home"  a("/shop") "Shop"` keeps its two spaces between the links.
fn space(before: &Tok, next: &Tok, gap: usize) -> usize {
    use Tok::*;
    match (before, next) {
        (_, Comment(_)) => gap.max(1),
        (LParen | LBracket, _) | (_, RParen | RBracket | Comma | Colon) => 0,
        (Ident(_), LParen) | (DotDot, _) | (_, DotDot) => 0,
        (Comma | Colon, _) | (Eq | EqEq | NotEq, _) | (_, Eq | EqEq | NotEq) => 1,
        (LBrace, _) | (_, LBrace | RBrace) => 1,
        _ => gap.clamp(1, 2),
    }
}

fn is_opening(tok: &Tok) -> bool {
    matches!(tok, Tok::LBrace | Tok::LParen | Tok::LBracket)
}

fn is_closing(tok: &Tok) -> bool {
    matches!(tok, Tok::RBrace | Tok::RParen | Tok::RBracket)
}

/// Moves the lines of a string over several lines along with the line it starts on. Its text stays the same,
/// since the closing `"""` moves too; if a line has too few spaces to move left, nothing moves.
fn shift_lines(text: &str, shift: isize) -> String {
    let mut lines = text.split('\n');
    let first = lines.next().unwrap_or_default();
    let rest: Vec<&str> = lines.collect();
    let left = shift.unsigned_abs();
    if rest.is_empty()
        || shift == 0
        || (shift < 0 && rest.iter().any(|l| !l.trim().is_empty() && !l.starts_with(&" ".repeat(left))))
    {
        return text.to_string();
    }
    let mut out = first.to_string();
    for line in rest {
        out.push('\n');
        if line.trim().is_empty() {
            continue;
        }
        if shift > 0 {
            out.push_str(&" ".repeat(left));
            out.push_str(line);
        } else {
            out.push_str(&line[left..]);
        }
    }
    out
}

/// The source by characters, to find a token's text from its line and column.
struct Source {
    chars: Vec<char>,
    line_starts: Vec<usize>,
}

impl Source {
    fn new(src: &str) -> Self {
        let chars: Vec<char> = src.chars().collect();
        let mut line_starts = vec![0];
        line_starts.extend(chars.iter().enumerate().filter(|(_, c)| **c == '\n').map(|(i, _)| i + 1));
        Source { chars, line_starts }
    }

    fn text(&self, t: &Token) -> String {
        let start = self.line_starts[t.pos.line - 1] + t.pos.col - 1;
        self.chars[start..start + t.len].iter().collect()
    }
}
