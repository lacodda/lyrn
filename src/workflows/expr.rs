//! GitHub's expression language: what sits between `${{` and `}}`, and what an
//! `if:` holds without them.
//!
//! Read the way the runner reads it, because a condition evaluated almost
//! right runs a step CI skips, or skips one CI runs. Strings compare without
//! regard to case, `&&` and `||` hand back one of their operands rather than a
//! boolean, a property a context does not have is null rather than an error,
//! and values of different kinds meet as numbers.
//!
//! A context nobody provided is an error, not null: `secrets.TOKEN` read as
//! an empty string would run a step with a blank where the token goes, and
//! the caller has to be able to say "this needs a secret" instead.

use std::collections::BTreeMap;

use yaml_rust2::Yaml;

/// A value an expression works with.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Value>),
    Object(Vec<(String, Value)>),
}

impl Value {
    pub fn string(text: impl Into<String>) -> Value {
        Value::String(text.into())
    }

    /// An object from its fields, in the order given.
    pub fn object<K: Into<String>>(fields: impl IntoIterator<Item = (K, Value)>) -> Value {
        Value::Object(fields.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }

    /// A field of an object, found without regard to case as GitHub finds
    /// it; null for anything else.
    pub fn field(&self, name: &str) -> Value {
        match self {
            Value::Object(fields) => fields
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(name))
                .map(|(_, v)| v.clone())
                .unwrap_or(Value::Null),
            _ => Value::Null,
        }
    }

    pub fn truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Bool(b) => *b,
            Value::Number(n) => *n != 0.0 && !n.is_nan(),
            Value::String(s) => !s.is_empty(),
            Value::Array(_) | Value::Object(_) => true,
        }
    }

    /// The value as text, the way it lands in a script.
    pub fn text(&self) -> String {
        match self {
            Value::Null => String::new(),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => number_text(*n),
            Value::String(s) => s.clone(),
            Value::Array(_) => "Array".to_string(),
            Value::Object(_) => "Object".to_string(),
        }
    }

    fn number(&self) -> f64 {
        match self {
            Value::Null => 0.0,
            Value::Bool(b) => f64::from(u8::from(*b)),
            Value::Number(n) => *n,
            Value::String(s) => {
                let s = s.trim();
                if s.is_empty() {
                    0.0
                } else if let Some(hex) = s.strip_prefix("0x") {
                    i64::from_str_radix(hex, 16).map_or(f64::NAN, |n| n as f64)
                } else {
                    s.parse().unwrap_or(f64::NAN)
                }
            }
            Value::Array(_) | Value::Object(_) => f64::NAN,
        }
    }

    /// YAML as an expression sees it: what a matrix or an input holds.
    pub fn from_yaml(yaml: &Yaml) -> Value {
        match yaml {
            Yaml::String(s) => Value::String(s.clone()),
            Yaml::Integer(i) => Value::Number(*i as f64),
            Yaml::Real(r) => r.parse().map_or_else(|_| Value::String(r.clone()), Value::Number),
            Yaml::Boolean(b) => Value::Bool(*b),
            Yaml::Array(items) => Value::Array(items.iter().map(Value::from_yaml).collect()),
            Yaml::Hash(map) => Value::Object(map.iter().filter_map(|(k, v)| Some((super::scalar(k)?, Value::from_yaml(v)))).collect()),
            _ => Value::Null,
        }
    }

    fn from_json(json: &serde_json::Value) -> Value {
        match json {
            serde_json::Value::Null => Value::Null,
            serde_json::Value::Bool(b) => Value::Bool(*b),
            serde_json::Value::Number(n) => Value::Number(n.as_f64().unwrap_or(f64::NAN)),
            serde_json::Value::String(s) => Value::String(s.clone()),
            serde_json::Value::Array(items) => Value::Array(items.iter().map(Value::from_json).collect()),
            serde_json::Value::Object(map) => Value::Object(map.iter().map(|(k, v)| (k.clone(), Value::from_json(v))).collect()),
        }
    }

    fn to_json(&self) -> serde_json::Value {
        match self {
            Value::Null => serde_json::Value::Null,
            Value::Bool(b) => serde_json::Value::Bool(*b),
            Value::Number(n) => serde_json::Number::from_f64(*n).map_or(serde_json::Value::Null, serde_json::Value::Number),
            Value::String(s) => serde_json::Value::String(s.clone()),
            Value::Array(items) => serde_json::Value::Array(items.iter().map(Value::to_json).collect()),
            Value::Object(fields) => serde_json::Value::Object(fields.iter().map(|(k, v)| (k.clone(), v.to_json())).collect()),
        }
    }
}

fn number_text(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        n.to_string()
    }
}

/// Why an expression could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// A context this evaluation was not given: `secrets`, `vars`, `inputs`.
    Unknown(String),
    /// Something GitHub would understand and this reading does not.
    Unsupported(String),
    /// Something GitHub would refuse too.
    Syntax(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Unknown(context) => write!(f, "needs `{context}`, which only GitHub has"),
            Error::Unsupported(what) => write!(f, "uses {what}, which lyrn does not evaluate"),
            Error::Syntax(what) => write!(f, "is not an expression: {what}"),
        }
    }
}

/// What an expression may read: the contexts by name, and how the job has
/// gone so far for `success()` and `failure()`.
#[derive(Debug, Clone, Default)]
pub struct Scope {
    contexts: BTreeMap<String, Value>,
    pub failed: bool,
}

impl Scope {
    pub fn new() -> Scope {
        Scope::default()
    }

    pub fn with(mut self, name: &str, value: Value) -> Scope {
        self.set(name, value);
        self
    }

    pub fn set(&mut self, name: &str, value: Value) {
        self.contexts.insert(name.to_ascii_lowercase(), value);
    }

    fn get(&self, name: &str) -> Result<Value, Error> {
        self.contexts
            .get(&name.to_ascii_lowercase())
            .cloned()
            .ok_or_else(|| Error::Unknown(name.to_string()))
    }
}

/// Every `${{ … }}` in `text` replaced by its value.
pub fn interpolate(text: &str, scope: &Scope) -> Result<String, Error> {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("${{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 3..];
        let end = closing(after).ok_or_else(|| Error::Syntax(format!("`${{{{` without `}}}}` in `{text}`")))?;
        out.push_str(&evaluate(&after[..end], scope)?.text());
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    Ok(out)
}

/// Where the `}}` closing an expression is, past any string that holds one.
fn closing(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut in_string = false;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\'' => in_string = !in_string,
            b'}' if !in_string && bytes.get(i + 1) == Some(&b'}') => return Some(i),
            _ => {}
        }
        i += 1;
    }
    None
}

/// An `if:`: an expression with or without `${{ }}` around it, true unless
/// it says otherwise, and - unless it names a status function - true only
/// while nothing before it has failed.
pub fn condition(text: Option<&str>, scope: &Scope) -> Result<bool, Error> {
    let Some(text) = text.map(str::trim).filter(|t| !t.is_empty()) else {
        return Ok(!scope.failed);
    };
    let inner = text
        .strip_prefix("${{")
        .and_then(|t| t.strip_suffix("}}"))
        .filter(|inner| !inner.contains("${{"))
        .unwrap_or(text);
    // A condition with text outside the braces is a string, and a string
    // that is not empty is true: GitHub's own trap, kept.
    if inner.contains("${{") {
        return interpolate(inner, scope).map(|s| !s.is_empty() && !scope.failed);
    }
    let tokens = lex(inner)?;
    let names_a_status = tokens.windows(2).any(|w| {
        matches!(&w[0], Token::Ident(name) if ["success", "failure", "always", "cancelled"].contains(&name.to_ascii_lowercase().as_str()))
            && w[1] == Token::Open
    });
    let value = Parser { tokens, at: 0, scope }.expression()?;
    Ok(if names_a_status { value.truthy() } else { value.truthy() && !scope.failed })
}

/// One expression's value.
pub fn evaluate(expression: &str, scope: &Scope) -> Result<Value, Error> {
    let tokens = lex(expression)?;
    let mut parser = Parser { tokens, at: 0, scope };
    let value = parser.expression()?;
    if parser.at != parser.tokens.len() {
        return Err(Error::Syntax(format!("unexpected `{}` in `{}`", parser.tokens[parser.at], expression.trim())));
    }
    Ok(value)
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Ident(String),
    Str(String),
    Num(f64),
    Dot,
    Star,
    Comma,
    Open,
    Close,
    OpenIndex,
    CloseIndex,
    Not,
    And,
    Or,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Token::Ident(s) => s.clone(),
            Token::Str(s) => format!("'{s}'"),
            Token::Num(n) => number_text(*n),
            Token::Dot => ".".into(),
            Token::Star => "*".into(),
            Token::Comma => ",".into(),
            Token::Open => "(".into(),
            Token::Close => ")".into(),
            Token::OpenIndex => "[".into(),
            Token::CloseIndex => "]".into(),
            Token::Not => "!".into(),
            Token::And => "&&".into(),
            Token::Or => "||".into(),
            Token::Eq => "==".into(),
            Token::Ne => "!=".into(),
            Token::Lt => "<".into(),
            Token::Le => "<=".into(),
            Token::Gt => ">".into(),
            Token::Ge => ">=".into(),
        };
        f.write_str(&text)
    }
}

fn lex(text: &str) -> Result<Vec<Token>, Error> {
    let chars: Vec<char> = text.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        let (token, width) = match c {
            c if c.is_whitespace() => {
                i += 1;
                continue;
            }
            '\'' => {
                let mut value = String::new();
                let mut j = i + 1;
                loop {
                    match chars.get(j) {
                        None => return Err(Error::Syntax(format!("a string never closes in `{}`", text.trim()))),
                        Some('\'') if chars.get(j + 1) == Some(&'\'') => {
                            value.push('\'');
                            j += 2;
                        }
                        Some('\'') => break,
                        Some(c) => {
                            value.push(*c);
                            j += 1;
                        }
                    }
                }
                (Token::Str(value), j + 1 - i)
            }
            '.' if !next.is_some_and(|n| n.is_ascii_digit()) => (Token::Dot, 1),
            '*' => (Token::Star, 1),
            ',' => (Token::Comma, 1),
            '(' => (Token::Open, 1),
            ')' => (Token::Close, 1),
            '[' => (Token::OpenIndex, 1),
            ']' => (Token::CloseIndex, 1),
            '!' if next == Some('=') => (Token::Ne, 2),
            '!' => (Token::Not, 1),
            '&' if next == Some('&') => (Token::And, 2),
            '|' if next == Some('|') => (Token::Or, 2),
            '=' if next == Some('=') => (Token::Eq, 2),
            '<' if next == Some('=') => (Token::Le, 2),
            '<' => (Token::Lt, 1),
            '>' if next == Some('=') => (Token::Ge, 2),
            '>' => (Token::Gt, 1),
            c if c.is_ascii_digit() || c == '-' || c == '.' => {
                let mut j = i + 1;
                while j < chars.len()
                    && (chars[j].is_ascii_alphanumeric() || chars[j] == '.' || ((chars[j] == '-' || chars[j] == '+') && matches!(chars[j - 1], 'e' | 'E')))
                {
                    j += 1;
                }
                let literal: String = chars[i..j].iter().collect();
                let value = Value::String(literal.clone()).number();
                if value.is_nan() {
                    return Err(Error::Syntax(format!("`{literal}` is not a number")));
                }
                (Token::Num(value), j - i)
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let mut j = i + 1;
                while j < chars.len() && (chars[j].is_ascii_alphanumeric() || chars[j] == '_' || chars[j] == '-') {
                    j += 1;
                }
                (Token::Ident(chars[i..j].iter().collect()), j - i)
            }
            other => return Err(Error::Syntax(format!("`{other}` in `{}`", text.trim()))),
        };
        tokens.push(token);
        i += width;
    }
    Ok(tokens)
}

struct Parser<'a> {
    tokens: Vec<Token>,
    at: usize,
    scope: &'a Scope,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.at)
    }

    fn eat(&mut self, token: &Token) -> bool {
        if self.peek() == Some(token) {
            self.at += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, token: &Token) -> Result<(), Error> {
        if self.eat(token) {
            Ok(())
        } else {
            Err(Error::Syntax(format!(
                "expected `{token}`, found {}",
                self.peek().map_or("the end".to_string(), |t| format!("`{t}`"))
            )))
        }
    }

    fn expression(&mut self) -> Result<Value, Error> {
        self.or()
    }

    fn or(&mut self) -> Result<Value, Error> {
        let mut left = self.and()?;
        while self.eat(&Token::Or) {
            let right = self.and()?;
            if !left.truthy() {
                left = right;
            }
        }
        Ok(left)
    }

    fn and(&mut self) -> Result<Value, Error> {
        let mut left = self.comparison()?;
        while self.eat(&Token::And) {
            let right = self.comparison()?;
            if left.truthy() {
                left = right;
            }
        }
        Ok(left)
    }

    fn comparison(&mut self) -> Result<Value, Error> {
        let left = self.unary()?;
        let op = match self.peek() {
            Some(op @ (Token::Eq | Token::Ne | Token::Lt | Token::Le | Token::Gt | Token::Ge)) => op.clone(),
            _ => return Ok(left),
        };
        self.at += 1;
        let right = self.unary()?;
        let result = match op {
            Token::Eq => equal(&left, &right),
            Token::Ne => !equal(&left, &right),
            _ => {
                let ordering = match (&left, &right) {
                    (Value::String(a), Value::String(b)) => Some(a.to_lowercase().cmp(&b.to_lowercase())),
                    _ => left.number().partial_cmp(&right.number()),
                };
                match (op, ordering) {
                    (_, None) => false,
                    (Token::Lt, Some(o)) => o.is_lt(),
                    (Token::Le, Some(o)) => o.is_le(),
                    (Token::Gt, Some(o)) => o.is_gt(),
                    (_, Some(o)) => o.is_ge(),
                }
            }
        };
        Ok(Value::Bool(result))
    }

    fn unary(&mut self) -> Result<Value, Error> {
        if self.eat(&Token::Not) {
            return Ok(Value::Bool(!self.unary()?.truthy()));
        }
        self.postfix()
    }

    fn postfix(&mut self) -> Result<Value, Error> {
        let mut value = self.primary()?;
        loop {
            if self.eat(&Token::Dot) {
                match self.peek().cloned() {
                    Some(Token::Ident(name)) => {
                        self.at += 1;
                        value = value.field(&name);
                    }
                    Some(Token::Star) => return Err(Error::Unsupported("an object filter (`.*`)".to_string())),
                    _ => return Err(Error::Syntax("a `.` with no property after it".to_string())),
                }
            } else if self.eat(&Token::OpenIndex) {
                let index = self.expression()?;
                self.expect(&Token::CloseIndex)?;
                value = match (&value, &index) {
                    (Value::Array(items), _) => {
                        let n = index.number();
                        if n.fract() == 0.0 && n >= 0.0 {
                            items.get(n as usize).cloned().unwrap_or(Value::Null)
                        } else {
                            Value::Null
                        }
                    }
                    (Value::Object(_), _) => value.field(&index.text()),
                    _ => Value::Null,
                };
            } else {
                return Ok(value);
            }
        }
    }

    fn primary(&mut self) -> Result<Value, Error> {
        let token = self.peek().cloned().ok_or_else(|| Error::Syntax("an expression ends too soon".to_string()))?;
        self.at += 1;
        match token {
            Token::Str(s) => Ok(Value::String(s)),
            Token::Num(n) => Ok(Value::Number(n)),
            Token::Open => {
                let value = self.expression()?;
                self.expect(&Token::Close)?;
                Ok(value)
            }
            Token::Ident(name) => {
                match name.as_str() {
                    "true" => return Ok(Value::Bool(true)),
                    "false" => return Ok(Value::Bool(false)),
                    "null" => return Ok(Value::Null),
                    _ => {}
                }
                if self.eat(&Token::Open) {
                    let mut args = Vec::new();
                    if !self.eat(&Token::Close) {
                        loop {
                            args.push(self.expression()?);
                            if self.eat(&Token::Close) {
                                break;
                            }
                            self.expect(&Token::Comma)?;
                        }
                    }
                    return self.call(&name, args);
                }
                self.scope.get(&name)
            }
            other => Err(Error::Syntax(format!("`{other}` where a value belongs"))),
        }
    }

    fn call(&self, name: &str, args: Vec<Value>) -> Result<Value, Error> {
        let arg = |i: usize| args.get(i).cloned().unwrap_or(Value::Null);
        let lower = |v: Value| v.text().to_lowercase();
        Ok(match name.to_ascii_lowercase().as_str() {
            "success" => Value::Bool(!self.scope.failed),
            "failure" => Value::Bool(self.scope.failed),
            "always" => Value::Bool(true),
            "cancelled" => Value::Bool(false),
            "contains" => Value::Bool(match arg(0) {
                Value::Array(items) => items.iter().any(|item| equal(item, &arg(1))),
                search => lower(search).contains(&lower(arg(1))),
            }),
            "startswith" => Value::Bool(lower(arg(0)).starts_with(&lower(arg(1)))),
            "endswith" => Value::Bool(lower(arg(0)).ends_with(&lower(arg(1)))),
            "format" => {
                let mut text = arg(0).text();
                for (i, value) in args.iter().enumerate().skip(1) {
                    text = text.replace(&format!("{{{}}}", i - 1), &value.text());
                }
                Value::String(text.replace("{{", "{").replace("}}", "}"))
            }
            "join" => {
                let separator = if args.len() > 1 { arg(1).text() } else { ",".to_string() };
                match arg(0) {
                    Value::Array(items) => Value::String(items.iter().map(Value::text).collect::<Vec<_>>().join(&separator)),
                    other => Value::String(other.text()),
                }
            }
            "tojson" => Value::String(serde_json::to_string_pretty(&arg(0).to_json()).unwrap_or_default()),
            "fromjson" => {
                let json: serde_json::Value = serde_json::from_str(&arg(0).text()).map_err(|e| Error::Syntax(format!("fromJSON: {e}")))?;
                Value::from_json(&json)
            }
            "hashfiles" => return Err(Error::Unsupported("hashFiles()".to_string())),
            other => return Err(Error::Unsupported(format!("the function `{other}()`"))),
        })
    }
}

/// `==` as GitHub has it: strings without regard to case, values of
/// different kinds as numbers, an object or array only equal to itself.
fn equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::String(x), Value::String(y)) => x.to_lowercase() == y.to_lowercase(),
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Array(_) | Value::Object(_), _) | (_, Value::Array(_) | Value::Object(_)) => false,
        _ => {
            let (x, y) = (a.number(), b.number());
            !x.is_nan() && x == y
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope() -> Scope {
        Scope::new()
            .with(
                "matrix",
                Value::object([("os", Value::string("ubuntu-latest")), ("addons", Value::string("keyring,self-update"))]),
            )
            .with("runner", Value::object([("os", Value::string("Linux")), ("temp", Value::string("/tmp/run"))]))
            .with(
                "steps",
                Value::object([("msrv", Value::object([("outputs", Value::object([("version", Value::string("1.88"))]))]))]),
            )
    }

    fn eval(text: &str) -> Value {
        evaluate(text, &scope()).unwrap()
    }

    #[test]
    fn contexts_and_their_properties_are_read() {
        assert_eq!(eval("matrix.os"), Value::string("ubuntu-latest"));
        assert_eq!(eval("steps.msrv.outputs.version"), Value::string("1.88"));
        assert_eq!(eval("matrix['os']"), Value::string("ubuntu-latest"));
        // A property the context lacks is null, not an error.
        assert_eq!(eval("matrix.nothing"), Value::Null);
        // A context nobody gave is an error: it is what tells a step needing
        // a secret from one that does not.
        assert_eq!(evaluate("secrets.TOKEN", &scope()), Err(Error::Unknown("secrets".into())));
    }

    #[test]
    fn strings_compare_without_case() {
        assert_eq!(eval("runner.os == 'linux'"), Value::Bool(true));
        assert_eq!(eval("runner.os != 'Windows'"), Value::Bool(true));
        assert_eq!(eval("contains(matrix.addons, 'KEYRING')"), Value::Bool(true));
        assert_eq!(eval("startsWith(matrix.os, 'ubuntu')"), Value::Bool(true));
    }

    #[test]
    fn and_and_or_hand_back_an_operand() {
        assert_eq!(eval("matrix.addons && 'yes'"), Value::string("yes"));
        assert_eq!(eval("matrix.nothing || 'none'"), Value::string("none"));
        assert_eq!(eval("'' && 'never'"), Value::string(""));
        assert_eq!(eval("!matrix.nothing"), Value::Bool(true));
    }

    #[test]
    fn different_kinds_meet_as_numbers() {
        assert_eq!(eval("1 == '1'"), Value::Bool(true));
        assert_eq!(eval("true == 1"), Value::Bool(true));
        assert_eq!(eval("null == 0"), Value::Bool(true));
        assert_eq!(eval("'a' == 1"), Value::Bool(false));
        assert_eq!(eval("2 > 10"), Value::Bool(false));
        assert_eq!(eval("0x10 == 16"), Value::Bool(true));
    }

    #[test]
    fn functions_are_the_runner_s() {
        assert_eq!(eval("format('{0}-{1}', matrix.os, 'x')"), Value::string("ubuntu-latest-x"));
        assert_eq!(eval("fromJSON('[\"a\",\"b\"]')[1]"), Value::string("b"));
        assert_eq!(eval("join(fromJSON('[1,2]'), '+')"), Value::string("1+2"));
        assert!(matches!(evaluate("hashFiles('**/Cargo.lock')", &scope()), Err(Error::Unsupported(_))));
    }

    #[test]
    fn text_is_interpolated_where_the_braces_are() {
        let text = "--path \"${{ runner.temp }}/demo\" --with ${{ matrix.addons }}";
        assert_eq!(interpolate(text, &scope()).unwrap(), "--path \"/tmp/run/demo\" --with keyring,self-update");
        // A `}}` inside a string does not close the expression.
        assert_eq!(interpolate("${{ format('{{0}}') }}", &scope()).unwrap(), "{0}");
        assert!(interpolate("${{ secrets.X }}", &scope()).is_err());
    }

    #[test]
    fn a_condition_holds_only_while_nothing_has_failed_unless_it_says_so() {
        let mut scope = scope();
        assert!(condition(None, &scope).unwrap());
        assert!(condition(Some("runner.os == 'Linux'"), &scope).unwrap());
        assert!(condition(Some("${{ runner.os == 'Linux' && contains(matrix.addons, 'keyring') }}"), &scope).unwrap());
        assert!(!condition(Some("runner.os == 'Windows'"), &scope).unwrap());
        scope.failed = true;
        assert!(!condition(None, &scope).unwrap());
        assert!(!condition(Some("runner.os == 'Linux'"), &scope).unwrap());
        assert!(condition(Some("always()"), &scope).unwrap());
        assert!(condition(Some("failure() && runner.os == 'Linux'"), &scope).unwrap());
    }

    #[test]
    fn what_github_would_refuse_is_refused() {
        assert!(matches!(evaluate("matrix.os ==", &scope()), Err(Error::Syntax(_))));
        assert!(matches!(evaluate("'open", &scope()), Err(Error::Syntax(_))));
        assert!(matches!(evaluate("matrix.os matrix.os", &scope()), Err(Error::Syntax(_))));
    }
}
