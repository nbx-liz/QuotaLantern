use serde::{Deserialize, Deserializer, de};

use crate::{
    EphemeralId, Percent, PositiveSafeUint, SCHEMA_VERSION, SafeUint, UtcTimestamp, WindowSeconds,
};

/// One normalized event from an adapter.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    #[serde(rename = "schema_version")]
    _schema_version: SchemaVersion,
    /// Random identifier of the QuotaLantern run.
    pub run_id: EphemeralId,
    /// Random identifier of the acquisition connection.
    pub source_id: EphemeralId,
    /// Monotonic sequence number within the connection.
    pub seq: SafeUint,
    /// Service the value comes from.
    pub provider: Provider,
    /// Where the adapter runs.
    pub environment: Environment,
    /// When the adapter received the value.
    pub received_at: UtcTimestamp,
    /// When the provider produced the value, if it says so.
    #[serde(deserialize_with = "nullable")]
    pub source_at: Option<UtcTimestamp>,
    #[serde(rename = "provenance")]
    _provenance: ProviderReported,
    /// The measured value or state.
    pub metric: Metric,
}

/// Measured value or state carried by an event.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Metric {
    /// Usage-quota snapshot.
    Quota(Quota),
    /// Context-window snapshot.
    Context(Context),
    /// Cumulative token counter.
    TokenCounter(TokenCounter),
    /// Acquisition state without a value.
    SourceState(SourceState),
}

/// Usage-quota snapshot of one bucket. Snapshots are never summed.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Quota {
    #[serde(rename = "scope")]
    _scope: AccountScope,
    /// Temporary identifier of the account-equivalent scope.
    pub entity_id: EphemeralId,
    /// Temporary identifier of the quota bucket.
    pub bucket_id: EphemeralId,
    /// Share of the bucket already used.
    pub used_percent: Percent,
    /// Window length reported by the provider, if any.
    #[serde(deserialize_with = "nullable")]
    pub window_seconds: Option<WindowSeconds>,
    /// Reset time reported by the provider, if any.
    #[serde(deserialize_with = "nullable")]
    pub resets_at: Option<UtcTimestamp>,
}

/// Context-window snapshot of one session.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Context {
    #[serde(rename = "scope")]
    _scope: SessionScope,
    /// Temporary identifier of the session.
    pub entity_id: EphemeralId,
    /// Share of the context window in use.
    pub used_percent: Percent,
    /// Window size in tokens, if reported.
    #[serde(deserialize_with = "nullable")]
    pub capacity_tokens: Option<PositiveSafeUint>,
}

/// Cumulative counter of one token category in one session.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TokenCounter {
    #[serde(rename = "scope")]
    _scope: SessionScope,
    /// Temporary identifier of the session.
    pub entity_id: EphemeralId,
    /// Changes whenever the provider restarts the counter.
    pub counter_epoch: EphemeralId,
    /// Token category the value counts.
    pub category: TokenCategory,
    /// Cumulative value since the provider's counter start.
    pub value: SafeUint,
}

/// Acquisition state of a source or of one of its metrics.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(try_from = "RawSourceState")]
pub struct SourceState {
    /// What the state applies to.
    pub target: Target,
    /// `bucket_id` or `entity_id` of the metric; `None` means every metric
    /// of the target kind. Always `None` when `target` is [`Target::Source`].
    pub subject_id: Option<EphemeralId>,
    /// The state.
    pub state: State,
    /// Fixed error code; [`ErrorCode::None`] when there is no error.
    pub error_code: ErrorCode,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSourceState {
    target: Target,
    #[serde(deserialize_with = "nullable")]
    subject_id: Option<EphemeralId>,
    state: State,
    error_code: ErrorCode,
}

impl TryFrom<RawSourceState> for SourceState {
    type Error = &'static str;

    fn try_from(raw: RawSourceState) -> Result<Self, Self::Error> {
        if raw.target == Target::Source && raw.subject_id.is_some() {
            return Err("whole-source state must not name a subject");
        }
        Ok(Self {
            target: raw.target,
            subject_id: raw.subject_id,
            state: raw.state,
            error_code: raw.error_code,
        })
    }
}

/// Supported provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    /// Claude Code.
    ClaudeCode,
    /// Codex.
    Codex,
}

/// Where the adapter runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Environment {
    /// macOS, native.
    MacosNative,
    /// Windows, native.
    WindowsNative,
    /// Inside WSL2.
    Wsl2,
}

/// Token category of a counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenCategory {
    /// Provider-reported total.
    Total,
    /// Input tokens.
    Input,
    /// Output tokens.
    Output,
    /// Cache reads.
    CacheRead,
    /// Cache writes.
    CacheWrite,
    /// Reasoning tokens.
    Reasoning,
}

/// What a [`SourceState`] applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Target {
    /// The whole source.
    Source,
    /// Quota metrics.
    Quota,
    /// Context metrics.
    Context,
    /// Token counters.
    TokenCounter,
}

/// Display state (`docs/requirements.md` section 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    /// Not connected or stopped.
    Disabled,
    /// Connecting.
    Connecting,
    /// Connected, waiting for the first value.
    Waiting,
    /// Value available.
    Available,
    /// Value may be outdated.
    Stale,
    /// Not supported in this environment.
    Unsupported,
    /// Re-authentication with the official tool is needed.
    AuthRequired,
    /// Acquisition error.
    Error,
}

/// Fixed error code of a [`SourceState`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// No error.
    None,
    /// Not authenticated.
    NotAuthenticated,
    /// Provider version not supported.
    UnsupportedVersion,
    /// Provider input violated the adapter's expectations.
    InvalidInput,
    /// Transport unavailable.
    TransportUnavailable,
    /// Timed out.
    Timeout,
    /// A privacy gate refused the input.
    PrivacyGateFailed,
    /// A capacity limit was reached.
    CapacityExceeded,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct SchemaVersion;

impl<'de> Deserialize<'de> for SchemaVersion {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match u64::deserialize(deserializer)? {
            SCHEMA_VERSION => Ok(Self),
            _ => Err(de::Error::custom("unsupported schema version")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ProviderReported {
    ProviderReported,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum AccountScope {
    Account,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SessionScope {
    Session,
}

/// Deserializes a required field whose value may be `null`. Unlike a plain
/// `Option` field, a missing key is an error, matching the schema's
/// `required` list.
fn nullable<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::deserialize(deserializer)
}
