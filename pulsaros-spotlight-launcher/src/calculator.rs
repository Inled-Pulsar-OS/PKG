use std::f64;

pub struct Calculator;

impl Calculator {
    pub fn evaluate(query: &str) -> Option<(String, String)> {
        let q = query.trim();
        if q.is_empty() {
            return None;
        }

        // Clean query to convert human operations
        let mut clean = q.replace('×', "*")
            .replace('÷', "/")
            .replace("**", "^");

        // Simple validation: must contain at least one operator or math function/constant
        let has_op = clean.chars().any(|c| "+-*/%^".contains(c))
            || ["sqrt", "abs", "sin", "cos", "tan", "log", "pi", "π", "e"].iter().any(|&f| clean.to_lowercase().contains(f));

        if !has_op {
            return None;
        }

        let mut parser = Parser::new(&clean);
        match parser.parse() {
            Ok(val) => {
                // If it's a whole number, format as integer
                let val_str = if val.fract() == 0.0 {
                    format!("{}", val as i64)
                } else {
                    format!("{:.6}", val).trim_end_matches('0').trim_end_matches('.').to_string()
                };
                Some((val_str.clone(), format!("{} = {}", q, val_str)))
            }
            Err(_) => None,
        }
    }
}

struct Parser<'a> {
    input: &'a str,
    chars: Vec<char>,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            chars: input.chars().collect(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn next_char(&mut self) -> Option<char> {
        if self.pos < self.chars.len() {
            let c = self.chars[self.pos];
            self.pos += 1;
            Some(c)
        } else {
            None
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.next_char();
            } else {
                break;
            }
        }
    }

    fn parse(&mut self) -> Result<f64, ()> {
        let val = self.parse_expr()?;
        self.skip_whitespace();
        if self.pos < self.chars.len() {
            return Err(()); // Trailing garbage
        }
        Ok(val)
    }

    fn parse_expr(&mut self) -> Result<f64, ()> {
        let mut val = self.parse_term()?;
        loop {
            self.skip_whitespace();
            match self.peek() {
                Some('+') => {
                    self.next_char();
                    val += self.parse_term()?;
                }
                Some('-') => {
                    self.next_char();
                    val -= self.parse_term()?;
                }
                _ => break,
            }
        }
        Ok(val)
    }

    fn parse_term(&mut self) -> Result<f64, ()> {
        let mut val = self.parse_factor()?;
        loop {
            self.skip_whitespace();
            match self.peek() {
                Some('*') => {
                    self.next_char();
                    val *= self.parse_factor()?;
                }
                Some('/') => {
                    self.next_char();
                    let denom = self.parse_factor()?;
                    if denom == 0.0 {
                        return Err(());
                    }
                    val /= denom;
                }
                Some('%') => {
                    self.next_char();
                    let denom = self.parse_factor()?;
                    if denom == 0.0 {
                        return Err(());
                    }
                    val %= denom;
                }
                _ => break,
            }
        }
        Ok(val)
    }

    fn parse_factor(&mut self) -> Result<f64, ()> {
        let mut val = self.parse_unary()?;
        self.skip_whitespace();
        if self.peek() == Some('^') {
            self.next_char();
            let exponent = self.parse_factor()?;
            val = val.powf(exponent);
        }
        Ok(val)
    }

    fn parse_unary(&mut self) -> Result<f64, ()> {
        self.skip_whitespace();
        match self.peek() {
            Some('-') => {
                self.next_char();
                Ok(-self.parse_unary()?)
            }
            Some('+') => {
                self.next_char();
                self.parse_unary()
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Result<f64, ()> {
        self.skip_whitespace();
        if let Some(c) = self.peek() {
            if c == '(' {
                self.next_char();
                let val = self.parse_expr()?;
                self.skip_whitespace();
                if self.next_char() != Some(')') {
                    return Err(());
                }
                return Ok(val);
            }

            if c.is_ascii_digit() || c == '.' {
                return self.parse_number();
            }

            if c.is_alphabetic() || c == 'π' {
                return self.parse_identifier();
            }
        }
        Err(())
    }

    fn parse_number(&mut self) -> Result<f64, ()> {
        let mut start = self.pos;
        let mut has_dot = false;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.next_char();
            } else if c == '.' && !has_dot {
                has_dot = true;
                self.next_char();
            } else {
                break;
            }
        }
        let s: String = self.chars[start..self.pos].iter().collect();
        s.parse::<f64>().map_err(|_| ())
    }

    fn parse_identifier(&mut self) -> Result<f64, ()> {
        let start = self.pos;
        if self.peek() == Some('π') {
            self.next_char();
            return Ok(f64::consts::PI);
        }

        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                self.next_char();
            } else {
                break;
            }
        }

        let id: String = self.chars[start..self.pos].iter().collect();
        let lower = id.to_lowercase();

        match lower.as_str() {
            "pi" => Ok(f64::consts::PI),
            "e" => Ok(f64::consts::E),
            "sqrt" | "abs" | "sin" | "cos" | "tan" | "log" | "log10" => {
                self.skip_whitespace();
                if self.peek() != Some('(') {
                    return Err(());
                }
                self.next_char(); // skip '('
                let arg = self.parse_expr()?;
                self.skip_whitespace();
                if self.next_char() != Some(')') {
                    return Err(());
                }
                match lower.as_str() {
                    "sqrt" => Ok(arg.sqrt()),
                    "abs" => Ok(arg.abs()),
                    "sin" => Ok(arg.sin()),
                    "cos" => Ok(arg.cos()),
                    "tan" => Ok(arg.tan()),
                    "log" => Ok(arg.ln()),
                    "log10" => Ok(arg.log10()),
                    _ => Err(()),
                }
            }
            _ => Err(()),
        }
    }
}
