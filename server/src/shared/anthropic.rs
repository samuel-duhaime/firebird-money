//! A minimal Claude API (Messages) client — just enough for statement import. Rust has no official
//! Anthropic SDK, so this talks to `POST /v1/messages` over the shared `reqwest` stack.
//!
//! Requests are streamed, so a caller can show progress while a long answer is still being written.
//!
//! Privacy: nothing here logs request or response content, only token counts.

use std::time::Duration;

use log::info;
use serde::Deserialize;
use serde_json::Value;

const MESSAGES_URL: &str = "https://api.anthropic.com/v1/messages";
const API_VERSION: &str = "2023-06-01";

/// Used when `STATEMENT_IMPORT_MODEL` is unset.
const DEFAULT_MODEL: &str = "claude-opus-5-5";

/// Server-side refusal fallback: when a model's safety classifier declines a request, the API
/// re-runs it on Anthropic's recommended fallback model inside the same call instead of failing.
const FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";

/// A streamed answer can take minutes in total, so there's no overall timeout — only a cap on how
/// long the stream may go silent (the API sends `ping` events while it works).
const READ_TIMEOUT: Duration = Duration::from_secs(60);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Why a Claude call didn't produce usable text.
#[derive(Debug)]
pub enum AnthropicError {
    /// `ANTHROPIC_API_KEY` isn't set, so the feature is off.
    NotConfigured,
    /// Couldn't reach the API, or it timed out.
    Request(reqwest::Error),
    /// The API answered with a non-2xx status.
    Status(reqwest::StatusCode),
    /// Claude declined the request (`stop_reason: "refusal"`), even after any fallback.
    Refusal,
    /// The answer hit `max_tokens` and was cut off.
    Truncated,
    /// The API reported an error partway through the stream (e.g. `overloaded_error`).
    StreamError,
    /// The caller stopped listening (e.g. the user left the page), so the call was abandoned.
    Cancelled,
    /// A 200 with no text, or a stream that isn't Messages events.
    UnexpectedResponse,
}

impl std::fmt::Display for AnthropicError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotConfigured => write!(f, "ANTHROPIC_API_KEY is not set"),
            Self::Request(e) => write!(f, "request failed: {e}"),
            Self::Status(status) => write!(f, "API answered {status}"),
            Self::Refusal => write!(f, "Claude declined the request"),
            Self::Truncated => write!(f, "answer was cut off at max_tokens"),
            Self::StreamError => write!(f, "API reported an error mid-stream"),
            Self::Cancelled => write!(f, "cancelled by the caller"),
            Self::UnexpectedResponse => write!(f, "unexpected response shape"),
        }
    }
}

/// The parts of a streamed event this client reads; every other field (and event type) is ignored.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum StreamEvent {
    MessageStart {
        message: StartedMessage,
    },
    ContentBlockDelta {
        delta: Delta,
    },
    MessageDelta {
        delta: MessageDeltaBody,
        usage: OutputUsage,
    },
    Error,
    /// `ping`, `content_block_start`/`stop` (including a `fallback` marker — a fallback model
    /// continues the same text, so nothing needs resetting), `message_stop`, ...
    #[serde(other)]
    Other,
}

#[derive(Deserialize)]
struct StartedMessage {
    usage: InputUsage,
}

#[derive(Deserialize)]
struct InputUsage {
    input_tokens: u64,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Delta {
    TextDelta {
        text: String,
    },
    /// Thinking and other deltas — nothing statement import reads.
    #[serde(other)]
    Other,
}

#[derive(Deserialize)]
struct MessageDeltaBody {
    stop_reason: Option<String>,
}

#[derive(Deserialize)]
struct OutputUsage {
    output_tokens: u64,
}

/// Built once in `main` and shared with the handlers.
pub struct AnthropicClient {
    http: reqwest::Client,
    api_key: Option<String>,
    model: String,
}

impl AnthropicClient {
    /// Reads `ANTHROPIC_API_KEY` and `STATEMENT_IMPORT_MODEL`. A missing key isn't a startup error:
    /// the server runs fine without statement import, which then answers `503`.
    pub fn from_env() -> Self {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .ok()
            .map(|key| key.trim().to_string())
            .filter(|key| !key.is_empty());
        let model = std::env::var("STATEMENT_IMPORT_MODEL")
            .ok()
            .map(|model| model.trim().to_string())
            .filter(|model| !model.is_empty())
            .unwrap_or_else(|| DEFAULT_MODEL.to_string());
        Self::new(api_key, model)
    }

    pub fn new(api_key: Option<String>, model: String) -> Self {
        let http = reqwest::Client::builder()
            .read_timeout(READ_TIMEOUT)
            .connect_timeout(CONNECT_TIMEOUT)
            .build()
            .expect("failed to build the Claude API HTTP client");
        Self {
            http,
            api_key,
            model,
        }
    }

    pub fn is_configured(&self) -> bool {
        self.api_key.is_some()
    }

    /// Streams `body` (a Messages request without `model`) and returns the answer's full text.
    /// Fills in `model` and `stream`, and opts into the server-side refusal fallback where the
    /// model supports it. `on_text` is called with the text received so far after every new piece;
    /// returning `false` abandons the call (`Cancelled`).
    pub async fn stream_message(
        &self,
        mut body: Value,
        mut on_text: impl FnMut(&str) -> bool,
    ) -> Result<String, AnthropicError> {
        let api_key = self
            .api_key
            .as_deref()
            .ok_or(AnthropicError::NotConfigured)?;

        body["model"] = Value::String(self.model.clone());
        body["stream"] = Value::Bool(true);
        let mut request = self
            .http
            .post(MESSAGES_URL)
            .header("x-api-key", api_key)
            .header("anthropic-version", API_VERSION);
        if supports_server_fallback(&self.model) {
            body["fallbacks"] = Value::String("default".to_string());
            request = request.header("anthropic-beta", FALLBACK_BETA);
        }

        let mut response = request
            .json(&body)
            .send()
            .await
            .map_err(AnthropicError::Request)?;
        if !response.status().is_success() {
            return Err(AnthropicError::Status(response.status()));
        }

        let mut buffer: Vec<u8> = Vec::new();
        let mut text = String::new();
        let mut stop_reason: Option<String> = None;
        let (mut input_tokens, mut output_tokens) = (0, 0);

        while let Some(chunk) = response.chunk().await.map_err(AnthropicError::Request)? {
            buffer.extend_from_slice(&chunk);
            // Server-sent events are separated by a blank line; keep any trailing partial event.
            while let Some(end) = find_event_end(&buffer) {
                let event: Vec<u8> = buffer.drain(..end + 2).collect();
                let Some(event) = parse_event(&event) else {
                    continue;
                };
                match event {
                    StreamEvent::MessageStart { message } => {
                        input_tokens = message.usage.input_tokens;
                    }
                    StreamEvent::ContentBlockDelta {
                        delta: Delta::TextDelta { text: piece },
                    } => {
                        text.push_str(&piece);
                        if !on_text(&text) {
                            return Err(AnthropicError::Cancelled);
                        }
                    }
                    StreamEvent::MessageDelta { delta, usage } => {
                        stop_reason = delta.stop_reason;
                        output_tokens = usage.output_tokens;
                    }
                    StreamEvent::Error => return Err(AnthropicError::StreamError),
                    StreamEvent::ContentBlockDelta { .. } | StreamEvent::Other => {}
                }
            }
        }

        info!(
            "claude call model={} input_tokens={input_tokens} output_tokens={output_tokens}",
            self.model
        );

        match stop_reason.as_deref() {
            Some("refusal") => Err(AnthropicError::Refusal),
            Some("max_tokens") => Err(AnthropicError::Truncated),
            Some(_) if !text.is_empty() => Ok(text),
            _ => Err(AnthropicError::UnexpectedResponse),
        }
    }
}

/// Index of the `\n\n` that ends the first complete event in `buffer`, if there is one.
fn find_event_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(2).position(|pair| pair == b"\n\n")
}

/// The JSON on an event's `data:` line, if it's one this client understands.
fn parse_event(event: &[u8]) -> Option<StreamEvent> {
    let event = std::str::from_utf8(event).ok()?;
    let data = event
        .lines()
        .find_map(|line| line.trim_end_matches('\r').strip_prefix("data:"))?;
    serde_json::from_str(data.trim()).ok()
}

/// The models that accept `fallbacks: "default"` on the Claude API. Sending it to any other model
/// is a 400, so an override like `claude-haiku-4-5` goes without.
fn supports_server_fallback(model: &str) -> bool {
    matches!(
        model,
        "claude-opus-5-5" | "claude-opus-5" | "claude-fable-5-1" | "claude-sonnet-5-5"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_model_gets_the_refusal_fallback() {
        assert!(supports_server_fallback(DEFAULT_MODEL));
    }

    #[test]
    fn older_models_do_not_get_the_refusal_fallback() {
        assert!(!supports_server_fallback("claude-haiku-4-5"));
        assert!(!supports_server_fallback("claude-sonnet-5"));
    }

    #[tokio::test]
    async fn stream_message_without_a_key_is_not_configured() {
        let client = AnthropicClient::new(None, DEFAULT_MODEL.to_string());

        assert!(!client.is_configured());
        assert!(matches!(
            client.stream_message(serde_json::json!({}), |_| true).await,
            Err(AnthropicError::NotConfigured)
        ));
    }

    #[test]
    fn find_event_end_waits_for_a_blank_line() {
        assert_eq!(find_event_end(b"event: ping\ndata: {}"), None);
        assert_eq!(
            find_event_end(b"event: ping\ndata: {}\n\nevent: x"),
            Some(20)
        );
    }

    #[test]
    fn parse_event_reads_a_text_delta() {
        let event = parse_event(
            b"event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hi\"}}\n\n",
        );
        assert!(matches!(
            event,
            Some(StreamEvent::ContentBlockDelta { delta: Delta::TextDelta { text } }) if text == "Hi"
        ));
    }

    #[test]
    fn parse_event_ignores_unknown_events() {
        assert!(matches!(
            parse_event(b"event: ping\ndata: {\"type\": \"ping\"}\n\n"),
            Some(StreamEvent::Other)
        ));
        assert!(matches!(
            parse_event(b"event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"\"}}\n\n"),
            Some(StreamEvent::ContentBlockDelta { delta: Delta::Other })
        ));
    }
}
