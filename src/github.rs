//! GitHub's REST API, read and written with the token at hand.
//!
//! One client for every command that speaks to GitHub: the base it asks is
//! `LYRN_GITHUB_API` when a test stands a local server in for GitHub, and the
//! token - from the environment, else the one `gh` is logged in with - goes to
//! GitHub and nowhere else.

use std::fmt;
use std::process::Command as Process;
use std::time::Duration;

use serde_json::Value;

/// Where the API lives.
pub const API: &str = "https://api.github.com";

/// The base the API is asked at: GitHub's, unless a test names its own.
pub fn api_base() -> String {
    std::env::var("LYRN_GITHUB_API").unwrap_or_else(|_| API.to_string())
}

/// A GitHub token, if one is at hand: the environment first, then the one
/// `gh` is logged in with.
pub fn token() -> Option<String> {
    for var in ["GH_TOKEN", "GITHUB_TOKEN"] {
        if let Some(value) = std::env::var(var).ok().filter(|v| !v.trim().is_empty()) {
            return Some(value.trim().to_string());
        }
    }
    let output = Process::new("gh").args(["auth", "token"]).output().ok()?;
    let token = String::from_utf8(output.stdout).ok()?.trim().to_string();
    (output.status.success() && !token.is_empty()).then_some(token)
}

/// What GitHub answered when it did not do what it was asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    /// The HTTP status; 0 when no answer came at all.
    pub status: u16,
    /// GitHub's own message, or why there was no answer.
    pub message: String,
}

impl Refusal {
    /// Whether the account's plan is what stands in the way, which GitHub says
    /// in words rather than by a status of its own: a ruleset or Pages on a
    /// private repository of a free account.
    pub fn is_the_plan(&self) -> bool {
        let message = self.message.to_ascii_lowercase();
        matches!(self.status, 403 | 422) && (message.contains("upgrade") || message.contains("plan"))
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.status {
            0 => write!(f, "{}", self.message),
            status => write!(f, "GitHub answered {status}: {}", self.message),
        }
    }
}

impl std::error::Error for Refusal {}

/// A client of the API, with the token it was given.
pub struct Api {
    client: reqwest::blocking::Client,
    base: String,
    token: Option<String>,
}

impl Api {
    /// A client of the API the environment names, with the token at hand
    /// when that API is GitHub's.
    pub fn new() -> Result<Self, String> {
        let base = api_base();
        let token = if base == API { token() } else { None };
        let client = reqwest::blocking::Client::builder()
            .user_agent(concat!("lyrn/", env!("CARGO_PKG_VERSION"), " (https://lyrn.lacodda.com)"))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            client,
            base: base.trim_end_matches('/').to_string(),
            token,
        })
    }

    /// Whether there is a token to write with.
    pub fn signed_in(&self) -> bool {
        self.token.is_some() || self.base != API
    }

    pub fn get(&self, path: &str) -> Result<Value, Refusal> {
        self.send("GET", path, None)
    }

    /// `path` read, or `None` when GitHub has nothing there.
    pub fn find(&self, path: &str) -> Result<Option<Value>, Refusal> {
        match self.get(path) {
            Ok(value) => Ok(Some(value)),
            Err(refusal) if refusal.status == 404 => Ok(None),
            Err(refusal) => Err(refusal),
        }
    }

    /// One request: its body as JSON, `Null` when it has none.
    pub fn send(&self, method: &str, path: &str, body: Option<&Value>) -> Result<Value, Refusal> {
        let method = reqwest::Method::from_bytes(method.as_bytes()).map_err(|e| Refusal {
            status: 0,
            message: e.to_string(),
        })?;
        let mut request = self
            .client
            .request(method, format!("{}/{}", self.base, path.trim_start_matches('/')))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28");
        if let Some(token) = &self.token {
            request = request.bearer_auth(token);
        }
        if let Some(body) = body {
            request = request.header("Content-Type", "application/json").body(body.to_string());
        }
        let response = request.send().map_err(|e| Refusal {
            status: 0,
            message: if e.is_timeout() {
                "GitHub gave no answer in time".to_string()
            } else {
                format!("could not reach GitHub: {}", e.to_string().lines().next().unwrap_or_default())
            },
        })?;
        let status = response.status().as_u16();
        let text = response.text().unwrap_or_default();
        let value = if text.trim().is_empty() {
            Value::Null
        } else {
            serde_json::from_str(&text).unwrap_or(Value::String(text))
        };
        if (200..300).contains(&status) {
            return Ok(value);
        }
        let mut message = value.get("message").and_then(Value::as_str).unwrap_or("no reason given").to_string();
        // A refused write names the field it refused in `errors`.
        if let Some(errors) = value.get("errors").and_then(Value::as_array) {
            let said: Vec<String> = errors
                .iter()
                .filter_map(|e| {
                    e.as_str()
                        .map(str::to_string)
                        .or_else(|| e.get("message").and_then(Value::as_str).map(str::to_string))
                })
                .collect();
            if !said.is_empty() {
                message = format!("{message} - {}", said.join("; "));
            }
        }
        Err(Refusal { status, message })
    }

    /// A GraphQL query, its `data`.
    pub fn graphql(&self, query: &str) -> Result<Value, Refusal> {
        let body = serde_json::json!({ "query": query });
        let answer = self.send("POST", "graphql", Some(&body))?;
        if let Some(errors) = answer.get("errors").and_then(Value::as_array).filter(|e| !e.is_empty()) {
            let said: Vec<&str> = errors.iter().filter_map(|e| e.get("message").and_then(Value::as_str)).collect();
            return Err(Refusal {
                status: 200,
                message: said.join("; "),
            });
        }
        Ok(answer.get("data").cloned().unwrap_or(Value::Null))
    }

    /// The bytes at a URL outside the API - an image GitHub serves - asked
    /// without the token, which belongs to the API alone.
    pub fn download(&self, url: &str) -> Result<Vec<u8>, Refusal> {
        let response = self.client.get(url).send().map_err(|e| Refusal {
            status: 0,
            message: format!("could not download {url}: {}", e.to_string().lines().next().unwrap_or_default()),
        })?;
        let status = response.status().as_u16();
        if !(200..300).contains(&status) {
            return Err(Refusal {
                status,
                message: format!("{url} could not be downloaded"),
            });
        }
        response.bytes().map(|b| b.to_vec()).map_err(|e| Refusal {
            status: 0,
            message: e.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_plan_is_told_by_its_words() {
        let plan = Refusal {
            status: 403,
            message: "Upgrade to GitHub Pro or make this repository public to enable this feature.".into(),
        };
        assert!(plan.is_the_plan());
        let other = Refusal {
            status: 403,
            message: "Resource not accessible by integration".into(),
        };
        assert!(!other.is_the_plan());
        let missing = Refusal {
            status: 404,
            message: "Not Found".into(),
        };
        assert!(!missing.is_the_plan());
    }
}
