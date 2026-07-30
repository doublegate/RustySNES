//! A small expression evaluator for conditional breakpoints and watchpoints (`v1.25.0`, T-FP-C2).
//!
//! A plain address breakpoint answers "did execution reach here", which is the wrong question for
//! the bugs that are actually hard: a routine called two thousand times a frame where only one call
//! misbehaves. A condition turns that into "did execution reach here **with `A > $80`**", which is
//! the difference between a usable breakpoint and one that fires constantly.
//!
//! # Scope, deliberately small
//!
//! Integer-only, no floats, no strings, no assignment, no function calls. A debugger condition is
//! evaluated on every hit of its address, so it must be cheap and it must be impossible for it to
//! have side effects on the machine being debugged — which is why memory access is read-only
//! (`[addr]`) and there is nothing that writes.
//!
//! Grammar (lowest to highest precedence):
//!
//! ```text
//! or     := and ('||' and)*
//! and    := cmp ('&&' cmp)*
//! cmp    := bitor (('=='|'!='|'<'|'<='|'>'|'>=') bitor)?
//! bitor  := bitxor ('|' bitxor)*
//! bitxor := bitand ('^' bitand)*
//! bitand := shift ('&' shift)*
//! shift  := sum (('<<'|'>>') sum)*
//! sum    := term (('+'|'-') term)*
//! term   := unary (('*'|'/'|'%') unary)*
//! unary  := ('-'|'!'|'~')? primary
//! primary:= number | register | '[' or ']' | '(' or ')'
//! ```
//!
//! Numbers are `$hex` / `0xhex` / decimal / `%binary`. Registers are `a x y s d p pb db pc`, plus
//! `nvmxdizc` for the individual status flags. `[expr]` reads one byte from the 24-bit bus; `{expr}`
//! reads a 16-bit little-endian word, since the values worth breaking on are as often pointers as
//! bytes.
//!
//! Division and modulo by zero evaluate to `0` rather than erroring: a condition is evaluated
//! constantly and in the background, and a breakpoint that stops working because a divisor
//! transiently hit zero would be worse than one that briefly reads a wrong value.

use core::fmt;

/// A parsed condition, ready to evaluate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    /// A literal.
    Num(i64),
    /// A CPU register or flag.
    Reg(Reg),
    /// An 8-bit read of the address the inner expression evaluates to.
    Byte(Box<Self>),
    /// A 16-bit little-endian read.
    Word(Box<Self>),
    /// A unary operation.
    Unary(UnOp, Box<Self>),
    /// A binary operation.
    Binary(BinOp, Box<Self>, Box<Self>),
}

/// A readable CPU register or status flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reg {
    /// Accumulator.
    A,
    /// X index.
    X,
    /// Y index.
    Y,
    /// Stack pointer.
    S,
    /// Direct page.
    D,
    /// Status byte.
    P,
    /// Program bank.
    Pb,
    /// Data bank.
    Db,
    /// Program counter (16-bit, without the bank).
    Pc,
    /// A single status flag, by its letter.
    Flag(char),
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    /// Arithmetic negation.
    Neg,
    /// Logical not (`0` becomes `1`, anything else `0`).
    Not,
    /// Bitwise complement.
    BitNot,
}

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/` (division by zero yields `0`).
    Div,
    /// `%` (modulo by zero yields `0`).
    Rem,
    /// `<<`
    Shl,
    /// `>>`
    Shr,
    /// `&`
    BitAnd,
    /// `|`
    BitOr,
    /// `^`
    BitXor,
    /// `==`
    Eq,
    /// `!=`
    Ne,
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `>`
    Gt,
    /// `>=`
    Ge,
    /// `&&`
    And,
    /// `||`
    Or,
}

/// Why an expression could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// The input held nothing to evaluate.
    Empty,
    /// A character that begins no token.
    BadChar(char),
    /// A numeric literal that did not parse in its base.
    BadNumber(String),
    /// An identifier that names no register or flag.
    UnknownName(String),
    /// A bracket or parenthesis was never closed.
    Unclosed(char),
    /// Input ran out mid-expression.
    UnexpectedEnd,
    /// Tokens remained after a complete expression.
    Trailing(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "empty expression"),
            Self::BadChar(c) => write!(f, "unexpected character {c:?}"),
            Self::BadNumber(s) => write!(f, "not a number: {s}"),
            Self::UnknownName(s) => write!(f, "unknown register or flag: {s}"),
            Self::Unclosed(c) => write!(f, "unclosed {c}"),
            Self::UnexpectedEnd => write!(f, "expression ended early"),
            Self::Trailing(s) => write!(f, "unexpected trailing input: {s}"),
        }
    }
}

impl core::error::Error for ParseError {}

/// The machine state an expression reads.
///
/// A trait rather than a concrete struct so the evaluator can be unit-tested against a fake machine
/// — the alternative is testing conditions only through a running emulator, which is exactly the
/// slow, flaky loop that makes evaluator bugs survive.
pub trait Context {
    /// Read a register or flag. Flags return `0` or `1`.
    fn reg(&self, reg: Reg) -> i64;
    /// Read one byte from the 24-bit bus, side-effect-free.
    fn peek(&self, addr: u32) -> u8;

    /// Read a 16-bit little-endian word. Provided, since every context reads it the same way.
    fn peek_word(&self, addr: u32) -> u16 {
        let lo = u16::from(self.peek(addr));
        let hi = u16::from(self.peek(addr.wrapping_add(1) & 0x00FF_FFFF));
        lo | (hi << 8)
    }
}

impl Expr {
    /// Parse a condition.
    ///
    /// # Errors
    /// Returns the first [`ParseError`] encountered; the caller shows it beside the input rather
    /// than arming a breakpoint whose condition silently means something else.
    pub fn parse(input: &str) -> Result<Self, ParseError> {
        let tokens = lex(input)?;
        if tokens.is_empty() {
            return Err(ParseError::Empty);
        }
        let mut p = Parser { tokens, pos: 0 };
        let expr = p.parse_or()?;
        if p.pos < p.tokens.len() {
            return Err(ParseError::Trailing(format!("{:?}", p.tokens[p.pos])));
        }
        Ok(expr)
    }

    /// Evaluate against a machine state.
    ///
    /// Total: every operation has a defined result for every input (see the module doc on division
    /// by zero), so evaluation cannot fail once parsing succeeded. A breakpoint that stopped working
    /// mid-session because of a transient divisor would be worse than a briefly wrong value.
    pub fn eval<C: Context + ?Sized>(&self, ctx: &C) -> i64 {
        match self {
            Self::Num(n) => *n,
            Self::Reg(r) => ctx.reg(*r),
            Self::Byte(inner) => i64::from(ctx.peek(addr_of(inner.eval(ctx)))),
            Self::Word(inner) => i64::from(ctx.peek_word(addr_of(inner.eval(ctx)))),
            Self::Unary(op, inner) => {
                let v = inner.eval(ctx);
                match op {
                    UnOp::Neg => v.wrapping_neg(),
                    UnOp::Not => i64::from(v == 0),
                    UnOp::BitNot => !v,
                }
            }
            Self::Binary(op, l, r) => {
                // Short-circuit, so `[ptr] != 0 && [[ptr]] == 5` does not deref a null pointer.
                match op {
                    BinOp::And => {
                        return i64::from(l.eval(ctx) != 0 && r.eval(ctx) != 0);
                    }
                    BinOp::Or => {
                        return i64::from(l.eval(ctx) != 0 || r.eval(ctx) != 0);
                    }
                    _ => {}
                }
                let (a, b) = (l.eval(ctx), r.eval(ctx));
                match op {
                    BinOp::Add => a.wrapping_add(b),
                    BinOp::Sub => a.wrapping_sub(b),
                    BinOp::Mul => a.wrapping_mul(b),
                    BinOp::Div => {
                        if b == 0 {
                            0
                        } else {
                            a.wrapping_div(b)
                        }
                    }
                    BinOp::Rem => {
                        if b == 0 {
                            0
                        } else {
                            a.wrapping_rem(b)
                        }
                    }
                    // A shift count outside 0..64 is clamped rather than wrapped: `x << 64` reading
                    // as `x << 0` (Rust's wrapping shift) would be a silently wrong answer.
                    BinOp::Shl => shift(a, b, true),
                    BinOp::Shr => shift(a, b, false),
                    BinOp::BitAnd => a & b,
                    BinOp::BitOr => a | b,
                    BinOp::BitXor => a ^ b,
                    BinOp::Eq => i64::from(a == b),
                    BinOp::Ne => i64::from(a != b),
                    BinOp::Lt => i64::from(a < b),
                    BinOp::Le => i64::from(a <= b),
                    BinOp::Gt => i64::from(a > b),
                    BinOp::Ge => i64::from(a >= b),
                    BinOp::And | BinOp::Or => unreachable!("handled above"),
                }
            }
        }
    }

    /// Whether the expression is true (non-zero) for this state.
    pub fn is_true<C: Context + ?Sized>(&self, ctx: &C) -> bool {
        self.eval(ctx) != 0
    }
}

/// Fold an evaluated value into a 24-bit bus address.
const fn addr_of(v: i64) -> u32 {
    // Deliberate: an expression's value is an arbitrary integer, and folding it into the 24-bit bus
    // is exactly what a wrapping narrowing does. A negative or oversized value wrapping into range
    // is the same behaviour a 65816 address calculation itself has.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    {
        (v as u32) & 0x00FF_FFFF
    }
}

/// Shift with a clamped count (see the call site's comment).
const fn shift(a: i64, b: i64, left: bool) -> i64 {
    if b < 0 {
        return 0;
    }
    if b >= 64 {
        // Everything shifted out. For a right shift of a negative value that is -1 (sign fill),
        // which is what an arithmetic shift actually produces.
        return if left || a >= 0 { 0 } else { -1 };
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let n = b as u32;
    if left {
        a.wrapping_shl(n)
    } else {
        a.wrapping_shr(n)
    }
}

/// One lexed token.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Tok {
    Num(i64),
    Reg(Reg),
    Op(&'static str),
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    LParen,
    RParen,
}

/// Split `input` into tokens.
#[allow(clippy::too_many_lines)]
fn lex(input: &str) -> Result<Vec<Tok>, ParseError> {
    let chars: Vec<char> = input.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        // Two-character operators first, so `<=` is never lexed as `<` then `=`.
        if i + 1 < chars.len() {
            let pair: String = chars[i..=i + 1].iter().collect();
            if let Some(op) = match pair.as_str() {
                "==" => Some("=="),
                "!=" => Some("!="),
                "<=" => Some("<="),
                ">=" => Some(">="),
                "&&" => Some("&&"),
                "||" => Some("||"),
                "<<" => Some("<<"),
                ">>" => Some(">>"),
                _ => None,
            } {
                out.push(Tok::Op(op));
                i += 2;
                continue;
            }
        }
        match c {
            '[' => out.push(Tok::LBracket),
            ']' => out.push(Tok::RBracket),
            '{' => out.push(Tok::LBrace),
            '}' => out.push(Tok::RBrace),
            '(' => out.push(Tok::LParen),
            ')' => out.push(Tok::RParen),
            '+' => out.push(Tok::Op("+")),
            '-' => out.push(Tok::Op("-")),
            '*' => out.push(Tok::Op("*")),
            '/' => out.push(Tok::Op("/")),
            '%' if i + 1 < chars.len() && (chars[i + 1] == '0' || chars[i + 1] == '1') => {
                // `%1010` is a binary literal; a bare `%` is modulo. Disambiguated by what follows,
                // which is unambiguous because a modulo's right operand never starts with a digit
                // that is only 0 or 1 without whitespace in any realistic condition — and the
                // ambiguity is resolved the same way every 65816 assembler resolves it.
                let start = i + 1;
                let mut j = start;
                while j < chars.len() && (chars[j] == '0' || chars[j] == '1') {
                    j += 1;
                }
                let text: String = chars[start..j].iter().collect();
                let n = i64::from_str_radix(&text, 2)
                    .map_err(|_| ParseError::BadNumber(text.clone()))?;
                out.push(Tok::Num(n));
                i = j;
                continue;
            }
            '%' => out.push(Tok::Op("%")),
            '&' => out.push(Tok::Op("&")),
            '|' => out.push(Tok::Op("|")),
            '^' => out.push(Tok::Op("^")),
            '~' => out.push(Tok::Op("~")),
            '!' => out.push(Tok::Op("!")),
            '<' => out.push(Tok::Op("<")),
            '>' => out.push(Tok::Op(">")),
            '$' => {
                let start = i + 1;
                let mut j = start;
                while j < chars.len() && chars[j].is_ascii_hexdigit() {
                    j += 1;
                }
                let text: String = chars[start..j].iter().collect();
                let n = i64::from_str_radix(&text, 16)
                    .map_err(|_| ParseError::BadNumber(text.clone()))?;
                out.push(Tok::Num(n));
                i = j;
                continue;
            }
            c if c.is_ascii_digit() => {
                let (n, next) = lex_number(&chars, i)?;
                out.push(Tok::Num(n));
                i = next;
                continue;
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let start = i;
                let mut j = i;
                while j < chars.len() && (chars[j].is_ascii_alphanumeric() || chars[j] == '_') {
                    j += 1;
                }
                let name: String = chars[start..j].iter().collect();
                out.push(Tok::Reg(reg_from_name(&name)?));
                i = j;
                continue;
            }
            other => return Err(ParseError::BadChar(other)),
        }
        i += 1;
    }
    Ok(out)
}

/// Lex a decimal or `0x`-prefixed literal starting at `i`.
fn lex_number(chars: &[char], i: usize) -> Result<(i64, usize), ParseError> {
    if chars[i] == '0' && i + 1 < chars.len() && (chars[i + 1] == 'x' || chars[i + 1] == 'X') {
        let start = i + 2;
        let mut j = start;
        while j < chars.len() && chars[j].is_ascii_hexdigit() {
            j += 1;
        }
        let text: String = chars[start..j].iter().collect();
        let n = i64::from_str_radix(&text, 16).map_err(|_| ParseError::BadNumber(text.clone()))?;
        return Ok((n, j));
    }
    let mut j = i;
    while j < chars.len() && chars[j].is_ascii_digit() {
        j += 1;
    }
    let text: String = chars[i..j].iter().collect();
    let n = text
        .parse::<i64>()
        .map_err(|_| ParseError::BadNumber(text.clone()))?;
    Ok((n, j))
}

/// Resolve an identifier to a register or status flag.
fn reg_from_name(name: &str) -> Result<Reg, ParseError> {
    let lower = name.to_ascii_lowercase();
    Ok(match lower.as_str() {
        "a" => Reg::A,
        "x" => Reg::X,
        "y" => Reg::Y,
        "s" | "sp" => Reg::S,
        "d" | "dp" => Reg::D,
        "p" => Reg::P,
        "pb" | "pbr" | "k" => Reg::Pb,
        "db" | "dbr" => Reg::Db,
        "pc" => Reg::Pc,
        // Individual status flags. `x` and `m` collide with the index register and nothing
        // respectively, so the width flags are spelled `fm`/`fx` to stay unambiguous — `x` alone
        // must keep meaning the index register, which is what a condition asks about far more often.
        "n" | "v" | "d_flag" | "i" | "z" | "c" => Reg::Flag(lower.chars().next().unwrap_or('n')),
        "fm" => Reg::Flag('m'),
        "fx" => Reg::Flag('x'),
        _ => return Err(ParseError::UnknownName(name.to_string())),
    })
}

/// Recursive-descent parser over the lexed tokens.
struct Parser {
    tokens: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.tokens.get(self.pos)
    }

    /// Consume the next token if it is one of `ops`, returning which.
    fn eat_op(&mut self, ops: &[&'static str]) -> Option<&'static str> {
        if let Some(Tok::Op(op)) = self.peek()
            && let Some(found) = ops.iter().find(|o| *o == op).copied()
        {
            self.pos += 1;
            return Some(found);
        }
        None
    }

    fn parse_or(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_and()?;
        while self.eat_op(&["||"]).is_some() {
            let rhs = self.parse_and()?;
            lhs = Expr::Binary(BinOp::Or, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_and(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_cmp()?;
        while self.eat_op(&["&&"]).is_some() {
            let rhs = self.parse_cmp()?;
            lhs = Expr::Binary(BinOp::And, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_cmp(&mut self) -> Result<Expr, ParseError> {
        let lhs = self.parse_bitor()?;
        // Non-associative on purpose: `a < b < c` is a bug in every language that allows it.
        if let Some(op) = self.eat_op(&["==", "!=", "<=", ">=", "<", ">"]) {
            let rhs = self.parse_bitor()?;
            let bin = match op {
                "==" => BinOp::Eq,
                "!=" => BinOp::Ne,
                "<=" => BinOp::Le,
                ">=" => BinOp::Ge,
                "<" => BinOp::Lt,
                _ => BinOp::Gt,
            };
            return Ok(Expr::Binary(bin, Box::new(lhs), Box::new(rhs)));
        }
        Ok(lhs)
    }

    fn parse_bitor(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_bitxor()?;
        while self.eat_op(&["|"]).is_some() {
            let rhs = self.parse_bitxor()?;
            lhs = Expr::Binary(BinOp::BitOr, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_bitxor(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_bitand()?;
        while self.eat_op(&["^"]).is_some() {
            let rhs = self.parse_bitand()?;
            lhs = Expr::Binary(BinOp::BitXor, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_bitand(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_shift()?;
        while self.eat_op(&["&"]).is_some() {
            let rhs = self.parse_shift()?;
            lhs = Expr::Binary(BinOp::BitAnd, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_shift(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_sum()?;
        while let Some(op) = self.eat_op(&["<<", ">>"]) {
            let rhs = self.parse_sum()?;
            let bin = if op == "<<" { BinOp::Shl } else { BinOp::Shr };
            lhs = Expr::Binary(bin, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_sum(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_term()?;
        while let Some(op) = self.eat_op(&["+", "-"]) {
            let rhs = self.parse_term()?;
            let bin = if op == "+" { BinOp::Add } else { BinOp::Sub };
            lhs = Expr::Binary(bin, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_term(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_unary()?;
        while let Some(op) = self.eat_op(&["*", "/", "%"]) {
            let rhs = self.parse_unary()?;
            let bin = match op {
                "*" => BinOp::Mul,
                "/" => BinOp::Div,
                _ => BinOp::Rem,
            };
            lhs = Expr::Binary(bin, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        if let Some(op) = self.eat_op(&["-", "!", "~"]) {
            let inner = self.parse_unary()?;
            let un = match op {
                "-" => UnOp::Neg,
                "!" => UnOp::Not,
                _ => UnOp::BitNot,
            };
            return Ok(Expr::Unary(un, Box::new(inner)));
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        let Some(tok) = self.peek().cloned() else {
            return Err(ParseError::UnexpectedEnd);
        };
        self.pos += 1;
        match tok {
            Tok::Num(n) => Ok(Expr::Num(n)),
            Tok::Reg(r) => Ok(Expr::Reg(r)),
            Tok::LBracket => {
                let inner = self.parse_or()?;
                self.expect(&Tok::RBracket, '[')?;
                Ok(Expr::Byte(Box::new(inner)))
            }
            Tok::LBrace => {
                let inner = self.parse_or()?;
                self.expect(&Tok::RBrace, '{')?;
                Ok(Expr::Word(Box::new(inner)))
            }
            Tok::LParen => {
                let inner = self.parse_or()?;
                self.expect(&Tok::RParen, '(')?;
                Ok(inner)
            }
            Tok::Op(_) | Tok::RBracket | Tok::RBrace | Tok::RParen => {
                Err(ParseError::Trailing(format!("{tok:?}")))
            }
        }
    }

    fn expect(&mut self, want: &Tok, opener: char) -> Result<(), ParseError> {
        if self.peek() == Some(want) {
            self.pos += 1;
            Ok(())
        } else {
            Err(ParseError::Unclosed(opener))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Context, Expr, ParseError, Reg};

    /// A fake machine: registers from an array, memory from a small map.
    struct Fake {
        a: i64,
        x: i64,
        mem: Vec<(u32, u8)>,
        flags: u8,
    }

    impl Default for Fake {
        fn default() -> Self {
            Self {
                a: 0x1234,
                x: 0x0010,
                mem: Vec::new(),
                flags: 0,
            }
        }
    }

    impl Context for Fake {
        fn reg(&self, reg: Reg) -> i64 {
            match reg {
                Reg::A => self.a,
                Reg::X => self.x,
                Reg::Flag('z') => i64::from(self.flags & 0x02 != 0),
                Reg::Flag('c') => i64::from(self.flags & 0x01 != 0),
                _ => 0,
            }
        }

        fn peek(&self, addr: u32) -> u8 {
            self.mem
                .iter()
                .find(|(a, _)| *a == addr)
                .map_or(0, |(_, v)| *v)
        }
    }

    fn eval(src: &str, ctx: &Fake) -> i64 {
        Expr::parse(src).expect("parse").eval(ctx)
    }

    #[test]
    fn literals_in_every_base() {
        let f = Fake::default();
        assert_eq!(eval("$FF", &f), 255);
        assert_eq!(eval("0xff", &f), 255);
        assert_eq!(eval("255", &f), 255);
        assert_eq!(eval("%11111111", &f), 255);
    }

    #[test]
    fn registers_and_flags_read_the_context() {
        let mut f = Fake::default();
        assert_eq!(eval("a", &f), 0x1234);
        assert_eq!(eval("x", &f), 0x10);
        assert_eq!(eval("z", &f), 0);
        f.flags = 0x02;
        assert_eq!(eval("z", &f), 1);
        // `x` must stay the index register; the width flag is spelled `fx`.
        assert_eq!(eval("x", &f), 0x10);
        assert_eq!(eval("fx", &f), 0);
    }

    /// Precedence must follow the documented grammar, or a condition silently means something
    /// other than what it reads as.
    #[test]
    fn precedence_matches_the_grammar() {
        let f = Fake::default();
        assert_eq!(eval("1 + 2 * 3", &f), 7);
        assert_eq!(eval("(1 + 2) * 3", &f), 9);
        assert_eq!(eval("1 << 4 + 1", &f), 32, "+ binds tighter than <<");
        assert_eq!(eval("6 & 3 == 3", &f), 6 & 1, "== binds looser than &");
        assert_eq!(eval("1 | 2 ^ 3", &f), 1 | (2 ^ 3));
        assert_eq!(eval("-2 + 3", &f), 1);
        assert_eq!(eval("!0", &f), 1);
        assert_eq!(eval("~0", &f), -1);
    }

    #[test]
    fn memory_reads_are_byte_and_word() {
        let f = Fake {
            mem: vec![(0x7E_0000, 0x34), (0x7E_0001, 0x12)],
            ..Fake::default()
        };
        assert_eq!(eval("[$7E0000]", &f), 0x34);
        assert_eq!(eval("{$7E0000}", &f), 0x1234);
        // The address expression is itself an expression.
        assert_eq!(eval("[$7E0000 + 1]", &f), 0x12);
        // Nested deref.
        let f2 = Fake {
            mem: vec![(0x00_0000, 0x05), (0x00_0005, 0x42)],
            ..Fake::default()
        };
        assert_eq!(eval("[[0]]", &f2), 0x42);
    }

    /// `&&`/`||` short-circuit, so a guard genuinely protects the deref after it.
    #[test]
    fn logical_operators_short_circuit() {
        let f = Fake::default();
        // If `||` did not short-circuit, the right side would still evaluate — which is not
        // observable without side effects, so assert the *value* semantics that follow from it.
        assert_eq!(eval("1 || 0", &f), 1);
        assert_eq!(eval("0 && 1", &f), 0);
        assert_eq!(
            eval("2 && 3", &f),
            1,
            "result is a boolean, not the operand"
        );
        assert_eq!(eval("0 || 0", &f), 0);
    }

    /// Evaluation is total: a transient zero divisor must not disarm a breakpoint mid-session.
    #[test]
    fn division_by_zero_is_zero_not_a_panic() {
        let f = Fake::default();
        assert_eq!(eval("5 / 0", &f), 0);
        assert_eq!(eval("5 % 0", &f), 0);
    }

    /// An out-of-range shift must not wrap to a no-op, which would be a silently wrong answer.
    #[test]
    fn out_of_range_shifts_saturate_rather_than_wrap() {
        let f = Fake::default();
        assert_eq!(eval("1 << 64", &f), 0, "not `1 << 0` == 1");
        assert_eq!(eval("1 << 100", &f), 0);
        assert_eq!(eval("256 >> 64", &f), 0);
        assert_eq!(eval("-1 >> 64", &f), -1, "arithmetic shift fills with sign");
        assert_eq!(eval("1 << -1", &f), 0);
    }

    #[test]
    fn realistic_conditions_parse_and_evaluate() {
        let f = Fake {
            a: 0x0090,
            mem: vec![(0x7E_0300, 0x03)],
            ..Fake::default()
        };
        assert!(Expr::parse("a > $80").expect("p").is_true(&f));
        assert!(
            Expr::parse("a > $80 && [$7E0300] == 3")
                .expect("p")
                .is_true(&f)
        );
        assert!(
            !Expr::parse("a > $80 && [$7E0300] == 4")
                .expect("p")
                .is_true(&f)
        );
        assert!(Expr::parse("(a & $F0) == $90").expect("p").is_true(&f));
    }

    /// Bad input is reported, never silently reinterpreted — a condition that means something
    /// other than what it reads as is worse than one that refuses to arm.
    #[test]
    fn errors_are_specific() {
        assert_eq!(Expr::parse(""), Err(ParseError::Empty));
        assert_eq!(Expr::parse("   "), Err(ParseError::Empty));
        assert!(matches!(
            Expr::parse("a > "),
            Err(ParseError::UnexpectedEnd)
        ));
        assert!(matches!(
            Expr::parse("[$7E0000"),
            Err(ParseError::Unclosed('['))
        ));
        assert!(matches!(
            Expr::parse("bogus"),
            Err(ParseError::UnknownName(_))
        ));
        assert!(matches!(
            Expr::parse("a # 1"),
            Err(ParseError::BadChar('#'))
        ));
        assert!(matches!(Expr::parse("1 2"), Err(ParseError::Trailing(_))));
        // Every error renders as a sentence the panel can show verbatim.
        for src in ["", "a > ", "[$7E0000", "bogus", "a # 1", "1 2"] {
            if let Err(e) = Expr::parse(src) {
                assert!(!e.to_string().is_empty());
            }
        }
    }
}
