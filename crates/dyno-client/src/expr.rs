//! Small arithmetic expressions for rig data: secondary motion such as a
//! cape's sway or a wisp's orbiting motes is written in the rig as, say,
//! `"0.07 + flow*0.26 + sin(clock*4.5)*0.025*(1+flow)"`.
//!
//! Numbers, named variables, `+ - * /`, unary minus, brackets and the
//! functions `sin cos abs sqrt min max clamp`. Variables are resolved to
//! slots when parsed, so evaluating is a walk over a small tree.

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Num(f32),
    Var(usize),
    Neg(Box<Expr>),
    Bin(u8, Box<Expr>, Box<Expr>),
    Call(Fun, Vec<Expr>),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fun {
    Sin,
    Cos,
    Abs,
    Sqrt,
    Min,
    Max,
    Clamp,
}

impl Expr {
    /// Parse `src`; `var` gives the slot of a variable name (or `None` if
    /// there is no such variable).
    pub fn parse(src: &str, var: &dyn Fn(&str) -> Option<usize>) -> Result<Expr, String> {
        let toks = lex(src)?;
        let mut p = Parser {
            toks: &toks,
            at: 0,
            var,
        };
        let e = p.sum()?;
        if p.at != toks.len() {
            return Err(format!("`{src}`: unexpected {:?}", toks[p.at]));
        }
        Ok(e)
    }

    /// The value with variables from `vars`.
    pub fn eval(&self, vars: &[f32]) -> f32 {
        match self {
            Expr::Num(n) => *n,
            Expr::Var(i) => vars.get(*i).copied().unwrap_or(0.0),
            Expr::Neg(e) => -e.eval(vars),
            Expr::Bin(op, a, b) => {
                let (a, b) = (a.eval(vars), b.eval(vars));
                match op {
                    b'+' => a + b,
                    b'-' => a - b,
                    b'*' => a * b,
                    _ => a / b,
                }
            }
            Expr::Call(f, args) => {
                let a = |i: usize| args.get(i).map(|e| e.eval(vars)).unwrap_or(0.0);
                match f {
                    Fun::Sin => a(0).sin(),
                    Fun::Cos => a(0).cos(),
                    Fun::Abs => a(0).abs(),
                    Fun::Sqrt => a(0).sqrt(),
                    Fun::Min => a(0).min(a(1)),
                    Fun::Max => a(0).max(a(1)),
                    Fun::Clamp => a(0).clamp(a(1), a(2).max(a(1))),
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Num(f32),
    Name(String),
    Op(u8),
}

fn lex(src: &str) -> Result<Vec<Tok>, String> {
    let b = src.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c.is_ascii_whitespace() {
            i += 1;
        } else if c.is_ascii_digit() || c == b'.' {
            let start = i;
            while i < b.len()
                && (b[i].is_ascii_digit()
                    || b[i] == b'.'
                    || b[i] == b'e'
                    || ((b[i] == b'-' || b[i] == b'+') && i > start && b[i - 1] == b'e'))
            {
                i += 1;
            }
            let s = &src[start..i];
            out.push(Tok::Num(
                s.parse().map_err(|_| format!("`{src}`: bad number {s}"))?,
            ));
        } else if c.is_ascii_alphabetic() || c == b'_' {
            let start = i;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_' || b[i] == b'.') {
                i += 1;
            }
            out.push(Tok::Name(src[start..i].to_string()));
        } else if b"+-*/(),".contains(&c) {
            out.push(Tok::Op(c));
            i += 1;
        } else {
            return Err(format!("`{src}`: unexpected `{}`", c as char));
        }
    }
    Ok(out)
}

struct Parser<'a> {
    toks: &'a [Tok],
    at: usize,
    var: &'a dyn Fn(&str) -> Option<usize>,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.at)
    }

    fn eat(&mut self, op: u8) -> bool {
        if self.peek() == Some(&Tok::Op(op)) {
            self.at += 1;
            true
        } else {
            false
        }
    }

    fn sum(&mut self) -> Result<Expr, String> {
        let mut e = self.product()?;
        loop {
            let op = match self.peek() {
                Some(Tok::Op(o @ (b'+' | b'-'))) => *o,
                _ => return Ok(e),
            };
            self.at += 1;
            e = Expr::Bin(op, Box::new(e), Box::new(self.product()?));
        }
    }

    fn product(&mut self) -> Result<Expr, String> {
        let mut e = self.unary()?;
        loop {
            let op = match self.peek() {
                Some(Tok::Op(o @ (b'*' | b'/'))) => *o,
                _ => return Ok(e),
            };
            self.at += 1;
            e = Expr::Bin(op, Box::new(e), Box::new(self.unary()?));
        }
    }

    fn unary(&mut self) -> Result<Expr, String> {
        if self.eat(b'-') {
            return Ok(Expr::Neg(Box::new(self.unary()?)));
        }
        if self.eat(b'+') {
            return self.unary();
        }
        self.atom()
    }

    fn atom(&mut self) -> Result<Expr, String> {
        match self.toks.get(self.at).cloned() {
            Some(Tok::Num(n)) => {
                self.at += 1;
                Ok(Expr::Num(n))
            }
            Some(Tok::Op(b'(')) => {
                self.at += 1;
                let e = self.sum()?;
                if !self.eat(b')') {
                    return Err("missing `)`".into());
                }
                Ok(e)
            }
            Some(Tok::Name(name)) => {
                self.at += 1;
                if self.eat(b'(') {
                    let f = match name.as_str() {
                        "sin" => Fun::Sin,
                        "cos" => Fun::Cos,
                        "abs" => Fun::Abs,
                        "sqrt" => Fun::Sqrt,
                        "min" => Fun::Min,
                        "max" => Fun::Max,
                        "clamp" => Fun::Clamp,
                        _ => return Err(format!("no function {name}")),
                    };
                    let mut args = Vec::new();
                    if !self.eat(b')') {
                        loop {
                            args.push(self.sum()?);
                            if self.eat(b')') {
                                break;
                            }
                            if !self.eat(b',') {
                                return Err("expected `,` or `)`".into());
                            }
                        }
                    }
                    return Ok(Expr::Call(f, args));
                }
                match name.as_str() {
                    "pi" => Ok(Expr::Num(std::f32::consts::PI)),
                    _ => (self.var)(&name)
                        .map(Expr::Var)
                        .ok_or_else(|| format!("no variable {name}")),
                }
            }
            t => Err(format!("unexpected {t:?}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluates() {
        let var = |n: &str| match n {
            "flow" => Some(0),
            "clock" => Some(1),
            "veil.ox" => Some(2),
            _ => None,
        };
        let e = Expr::parse("0.07 + flow*0.26 - -2*(1+clock)/4", &var).unwrap();
        assert!((e.eval(&[1.0, 3.0, 0.0]) - (0.07 + 0.26 + 2.0)).abs() < 1e-6);
        let e = Expr::parse("clamp(max(veil.ox, 0.2), 0, 1) + sin(0)", &var).unwrap();
        assert_eq!(e.eval(&[0.0, 0.0, 0.1]), 0.2);
        assert!(Expr::parse("nope", &var).is_err());
        assert!(Expr::parse("1e-3*2", &var).unwrap().eval(&[]) - 0.002 < 1e-9);
    }
}
