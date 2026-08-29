//! Expression evaluator (frozen v5 P6.4): a small C-like expression language
//! over symbols, registers and target memory. Not a full C parser.

use crate::executor::ExecutorHandle;
use crate::op::OperationKind;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct EvalResult {
    pub value: u64,
    pub text: String,
}

pub async fn evaluate(
    expr: &str,
    executor: &ExecutorHandle,
    symbols: Option<&cmsis_dap_core::symbols::SymbolDatabase>,
) -> Result<EvalResult, String> {
    let tokens = tokenize(expr)?;
    let mut parser = Parser { tokens, pos: 0 };
    let ast = parser.parse_expr()?;
    if parser.pos != parser.tokens.len() {
        return Err(format!(
            "unexpected token after expression: {:?}",
            parser.tokens[parser.pos]
        ));
    }
    let value = eval_ast(&ast, executor, symbols).await?;
    Ok(EvalResult {
        value,
        text: format!("0x{value:x} ({value})"),
    })
}

// ---------------------------------------------------------------------------
// Lexer
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(u64),
    Ident(String),
    Reg(String),
    Op(String),
    LParen,
    RParen,
}

fn tokenize(s: &str) -> Result<Vec<Tok>, String> {
    let mut toks = Vec::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c == '(' {
            toks.push(Tok::LParen);
            i += 1;
            continue;
        }
        if c == ')' {
            toks.push(Tok::RParen);
            i += 1;
            continue;
        }
        if c == '$' {
            let start = i + 1;
            let mut j = start;
            while j < chars.len() && chars[j].is_ascii_alphanumeric() {
                j += 1;
            }
            if j == start {
                return Err("expected register name after '$'".into());
            }
            toks.push(Tok::Reg(chars[start..j].iter().collect()));
            i = j;
            continue;
        }
        if c.is_ascii_digit() {
            let start = i;
            let mut j = i;
            if c == '0' && j + 1 < chars.len() && (chars[j + 1] == 'x' || chars[j + 1] == 'X') {
                j += 2;
                while j < chars.len() && chars[j].is_ascii_hexdigit() {
                    j += 1;
                }
                let digits: String = chars[start + 2..j].iter().collect();
                let v = u64::from_str_radix(&digits, 16)
                    .map_err(|_| "invalid hex literal".to_string())?;
                toks.push(Tok::Num(v));
                i = j;
                continue;
            }
            if c == '0' && j + 1 < chars.len() && (chars[j + 1] == 'b' || chars[j + 1] == 'B') {
                j += 2;
                while j < chars.len() && (chars[j] == '0' || chars[j] == '1') {
                    j += 1;
                }
                let digits: String = chars[start + 2..j].iter().collect();
                let v = u64::from_str_radix(&digits, 2)
                    .map_err(|_| "invalid bin literal".to_string())?;
                toks.push(Tok::Num(v));
                i = j;
                continue;
            }
            while j < chars.len() && chars[j].is_ascii_digit() {
                j += 1;
            }
            let digits: String = chars[start..j].iter().collect();
            let v = digits
                .parse::<u64>()
                .map_err(|_| "invalid number".to_string())?;
            toks.push(Tok::Num(v));
            i = j;
            continue;
        }
        if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            let mut j = i;
            while j < chars.len() && (chars[j].is_ascii_alphanumeric() || chars[j] == '_') {
                j += 1;
            }
            toks.push(Tok::Ident(chars[start..j].iter().collect()));
            i = j;
            continue;
        }
        // operators (longest first)
        let two: String = chars[i..(i + 2).min(chars.len())].iter().collect();
        let two_ops = ["<<", ">>", "==", "!=", "<=", ">=", "&&", "||"];
        if two_ops.contains(&two.as_str()) {
            toks.push(Tok::Op(two));
            i += 2;
            continue;
        }
        if "+-*/%&|^<>!~".contains(c) {
            toks.push(Tok::Op(c.to_string()));
            i += 1;
            continue;
        }
        if c == '*' && i + 1 < chars.len() && chars[i + 1] == '*' {
            toks.push(Tok::Op("**".into()));
            i += 2;
            continue;
        }
        return Err(format!("unexpected character '{c}'"));
    }
    Ok(toks)
}

// ---------------------------------------------------------------------------
// Parser (precedence climbing)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum Ast {
    Num(u64),
    Var(String),
    Reg(String),
    Deref(Box<Ast>),
    Unary(String, Box<Ast>),
    Binary(String, Box<Ast>, Box<Ast>),
}

struct Parser {
    tokens: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<Tok> {
        self.tokens.get(self.pos).cloned()
    }
    fn next(&mut self) -> Option<Tok> {
        let t = self.tokens.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    fn parse_expr(&mut self) -> Result<Ast, String> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Ast, String> {
        let mut left = self.parse_and()?;
        while let Some(Tok::Op(op)) = self.peek() {
            if op == "||" {
                self.next();
                let right = self.parse_and()?;
                left = Ast::Binary(op.clone(), Box::new(left), Box::new(right));
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Ast, String> {
        let mut left = self.parse_cmp()?;
        while let Some(Tok::Op(op)) = self.peek() {
            if op == "&&" {
                self.next();
                let right = self.parse_cmp()?;
                left = Ast::Binary(op.clone(), Box::new(left), Box::new(right));
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_cmp(&mut self) -> Result<Ast, String> {
        let mut left = self.parse_bitor()?;
        while let Some(Tok::Op(op)) = self.peek() {
            if matches!(op.as_str(), "==" | "!=" | "<" | "<=" | ">" | ">=") {
                self.next();
                let right = self.parse_bitor()?;
                left = Ast::Binary(op.clone(), Box::new(left), Box::new(right));
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_bitor(&mut self) -> Result<Ast, String> {
        let mut left = self.parse_bitxor()?;
        while let Some(Tok::Op(op)) = self.peek() {
            if op == "|" {
                self.next();
                let right = self.parse_bitxor()?;
                left = Ast::Binary(op.clone(), Box::new(left), Box::new(right));
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_bitxor(&mut self) -> Result<Ast, String> {
        let mut left = self.parse_bitand()?;
        while let Some(Tok::Op(op)) = self.peek() {
            if op == "^" {
                self.next();
                let right = self.parse_bitand()?;
                left = Ast::Binary(op.clone(), Box::new(left), Box::new(right));
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_bitand(&mut self) -> Result<Ast, String> {
        let mut left = self.parse_add()?;
        while let Some(Tok::Op(op)) = self.peek() {
            if op == "&" {
                self.next();
                let right = self.parse_add()?;
                left = Ast::Binary(op.clone(), Box::new(left), Box::new(right));
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_add(&mut self) -> Result<Ast, String> {
        let mut left = self.parse_mul()?;
        while let Some(Tok::Op(op)) = self.peek() {
            if op == "+" || op == "-" {
                self.next();
                let right = self.parse_mul()?;
                left = Ast::Binary(op.clone(), Box::new(left), Box::new(right));
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_mul(&mut self) -> Result<Ast, String> {
        let mut left = self.parse_shift()?;
        while let Some(Tok::Op(op)) = self.peek() {
            if matches!(op.as_str(), "*" | "/" | "%") {
                self.next();
                let right = self.parse_shift()?;
                left = Ast::Binary(op.clone(), Box::new(left), Box::new(right));
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_shift(&mut self) -> Result<Ast, String> {
        let mut left = self.parse_unary()?;
        while let Some(Tok::Op(op)) = self.peek() {
            if op == "<<" || op == ">>" {
                self.next();
                let right = self.parse_unary()?;
                left = Ast::Binary(op.clone(), Box::new(left), Box::new(right));
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Ast, String> {
        if let Some(Tok::Op(op)) = self.peek() {
            if matches!(op.as_str(), "-" | "!" | "~") {
                self.next();
                let inner = self.parse_unary()?;
                return Ok(Ast::Unary(op.clone(), Box::new(inner)));
            }
            if op == "*" {
                self.next();
                let inner = self.parse_unary()?;
                return Ok(Ast::Deref(Box::new(inner)));
            }
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Ast, String> {
        let mut node = self.parse_primary()?;
        while let Some(Tok::Op(op)) = self.peek() {
            if op == "*" {
                self.next();
                node = Ast::Deref(Box::new(node));
            } else {
                break;
            }
        }
        Ok(node)
    }

    fn parse_primary(&mut self) -> Result<Ast, String> {
        match self.next() {
            Some(Tok::Num(v)) => Ok(Ast::Num(v)),
            Some(Tok::Ident(name)) => Ok(Ast::Var(name)),
            Some(Tok::Reg(name)) => Ok(Ast::Reg(name)),
            Some(Tok::LParen) => {
                let e = self.parse_expr()?;
                match self.next() {
                    Some(Tok::RParen) => Ok(e),
                    _ => Err("expected ')'".into()),
                }
            }
            _ => Err("expected expression".into()),
        }
    }
}

// ---------------------------------------------------------------------------
// Evaluator
// ---------------------------------------------------------------------------

async fn eval_ast(
    ast: &Ast,
    executor: &ExecutorHandle,
    symbols: Option<&cmsis_dap_core::symbols::SymbolDatabase>,
) -> Result<u64, String> {
    match ast {
        Ast::Num(v) => Ok(*v),
        Ast::Var(name) => {
            // Try symbol first, then fall back to a register name.
            if let Some(db) = symbols {
                if let Some(s) = db.resolve_name(name) {
                    return Ok(s.address);
                }
            }
            if let Ok(v) = executor
                .call_async(
                    OperationKind::RegisterRead,
                    serde_json::json!({ "name": name }),
                )
                .await
            {
                if let Some(val) = v.get("value").and_then(|x| x.as_u64()) {
                    return Ok(val);
                }
            }
            Err(format!("unknown symbol or register: {name}"))
        }
        Ast::Reg(name) => {
            let v = executor
                .call_async(
                    OperationKind::RegisterRead,
                    serde_json::json!({ "name": name }),
                )
                .await
                .map_err(|e| e.to_string())?;
            v.get("value")
                .and_then(|x| x.as_u64())
                .ok_or_else(|| format!("cannot read register {name}"))
        }
        Ast::Deref(inner) => {
            let addr = Box::pin(eval_ast(inner, executor, symbols)).await?;
            read32(executor, addr).await
        }
        Ast::Unary(op, inner) => {
            let v = Box::pin(eval_ast(inner, executor, symbols)).await?;
            match op.as_str() {
                "-" => Ok(v.wrapping_neg()),
                "!" => Ok(if v == 0 { 1 } else { 0 }),
                "~" => Ok(!v),
                _ => Err(format!("unknown unary op {op}")),
            }
        }
        Ast::Binary(op, l, r) => {
            // Short-circuit && / ||
            if op == "&&" {
                let lv = Box::pin(eval_ast(l, executor, symbols)).await?;
                if lv == 0 {
                    return Ok(0);
                }
                return Ok(if Box::pin(eval_ast(r, executor, symbols)).await? != 0 {
                    1
                } else {
                    0
                });
            }
            if op == "||" {
                let lv = Box::pin(eval_ast(l, executor, symbols)).await?;
                if lv != 0 {
                    return Ok(1);
                }
                return Ok(if Box::pin(eval_ast(r, executor, symbols)).await? != 0 {
                    1
                } else {
                    0
                });
            }
            let lv = Box::pin(eval_ast(l, executor, symbols)).await?;
            let rv = Box::pin(eval_ast(r, executor, symbols)).await?;
            match op.as_str() {
                "+" => Ok(lv.wrapping_add(rv)),
                "-" => Ok(lv.wrapping_sub(rv)),
                "*" => Ok(lv.wrapping_mul(rv)),
                "/" => lv.checked_div(rv).ok_or_else(|| "division by zero".into()),
                "%" => lv.checked_rem(rv).ok_or_else(|| "modulo by zero".into()),
                "&" => Ok(lv & rv),
                "|" => Ok(lv | rv),
                "^" => Ok(lv ^ rv),
                "<<" => Ok(lv.wrapping_shl(rv as u32)),
                ">>" => Ok(lv.wrapping_shr(rv as u32)),
                "==" => Ok(if lv == rv { 1 } else { 0 }),
                "!=" => Ok(if lv != rv { 1 } else { 0 }),
                "<" => Ok(if lv < rv { 1 } else { 0 }),
                "<=" => Ok(if lv <= rv { 1 } else { 0 }),
                ">" => Ok(if lv > rv { 1 } else { 0 }),
                ">=" => Ok(if lv >= rv { 1 } else { 0 }),
                _ => Err(format!("unknown binary op {op}")),
            }
        }
    }
}

async fn read32(executor: &ExecutorHandle, address: u64) -> Result<u64, String> {
    let v = executor
        .call_async(
            OperationKind::MemoryRead,
            serde_json::json!({ "address": address, "width": "u32", "count": 1 }),
        )
        .await
        .map_err(|e| e.to_string())?;
    let bytes = v
        .get("bytes")
        .and_then(|b| b.as_array())
        .ok_or("no bytes")?;
    if bytes.len() < 4 {
        return Err("short read".into());
    }
    Ok(bytes[0].as_u64().unwrap_or(0)
        | (bytes[1].as_u64().unwrap_or(0) << 8)
        | (bytes[2].as_u64().unwrap_or(0) << 16)
        | (bytes[3].as_u64().unwrap_or(0) << 24))
}

#[allow(dead_code)]
fn _unused(_: HashMap<String, u64>) {}
