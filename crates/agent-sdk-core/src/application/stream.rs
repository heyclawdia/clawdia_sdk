use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{
    domain::AgentError,
    stream_records::{
        RedactedMatch, RepeatPolicy, StreamDelta, StreamIntervention, StreamMatcher, StreamRule,
        StreamRuleRepeatStateSnapshot,
    },
};

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct StreamRuleEngineState {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub seen_match_keys: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct StreamRuleEngine {
    rules: Vec<StreamRule>,
    buffers: BTreeMap<String, String>,
    seen_match_keys: BTreeSet<String>,
}

impl StreamRuleEngine {
    pub fn new(rules: Vec<StreamRule>) -> Result<Self, AgentError> {
        for rule in &rules {
            rule.validate()?;
        }
        Ok(Self {
            rules,
            buffers: BTreeMap::new(),
            seen_match_keys: BTreeSet::new(),
        })
    }

    pub fn restore(
        rules: Vec<StreamRule>,
        state: StreamRuleEngineState,
    ) -> Result<Self, AgentError> {
        let mut engine = Self::new(rules)?;
        engine.seen_match_keys = state.seen_match_keys.into_iter().collect();
        Ok(engine)
    }

    pub fn rules(&self) -> &[StreamRule] {
        &self.rules
    }

    pub fn snapshot_state(&self) -> StreamRuleEngineState {
        StreamRuleEngineState {
            seen_match_keys: self.seen_match_keys.iter().cloned().collect(),
        }
    }

    pub fn repeat_state_for(&self, rule: &StreamRule) -> StreamRuleRepeatStateSnapshot {
        let prefix = format!("{}:", rule.id.as_str());
        StreamRuleRepeatStateSnapshot {
            seen_match_keys: self
                .seen_match_keys
                .iter()
                .filter(|key| key.starts_with(&prefix))
                .cloned()
                .collect(),
        }
    }

    pub fn observe_delta(
        &mut self,
        delta: StreamDelta,
    ) -> Result<Vec<StreamIntervention>, AgentError> {
        if !delta.channel.is_policy_visible() {
            return Ok(Vec::new());
        }

        let mut interventions = Vec::new();
        let rules = self.rules.clone();
        for rule in &rules {
            if !rule
                .channels
                .iter()
                .any(|selector| selector.matches(&delta))
            {
                continue;
            }

            let match_text = match &rule.matcher {
                StreamMatcher::Marker { marker_id, .. } => {
                    if delta.marker_id.as_ref() == Some(marker_id) {
                        Some(delta.redacted_summary.as_str())
                    } else {
                        None
                    }
                }
                StreamMatcher::Literal { .. } | StreamMatcher::Regex { .. } => delta.matcher_text(),
                StreamMatcher::HostMatcher { .. } => None,
            };

            let Some(chunk_text) = match_text else {
                continue;
            };

            let buffer_key = buffer_key(rule, &delta);
            let buffer = self.buffers.entry(buffer_key).or_default();
            buffer.push_str(chunk_text);
            truncate_utf8_suffix(buffer, rule.matcher.window_bytes() as usize);

            let Some((start, end)) = find_match(&rule.matcher, buffer)? else {
                continue;
            };
            let matched_text = &buffer[start..end];
            let redacted = RedactedMatch::from_text(rule, &delta, matched_text);
            let repeat_key = repeat_key(rule, &delta, &redacted);
            if !matches!(rule.repeat, RepeatPolicy::Always)
                && !self.seen_match_keys.insert(repeat_key)
            {
                continue;
            }

            interventions.push(StreamIntervention::proposed(rule, redacted));
        }
        Ok(interventions)
    }
}

fn buffer_key(rule: &StreamRule, delta: &StreamDelta) -> String {
    format!(
        "{}:{:?}:{:?}:{:?}:{:?}",
        rule.id.as_str(),
        delta.channel,
        delta.direction,
        delta.attempt_id.as_ref().map(|id| id.as_str().to_string()),
        delta
            .realtime_session_id
            .as_ref()
            .map(|id| id.as_str().to_string())
    )
}

fn repeat_key(rule: &StreamRule, delta: &StreamDelta, redacted: &RedactedMatch) -> String {
    match rule.repeat {
        RepeatPolicy::Always => format!(
            "{}:always:{}:{}",
            rule.id.as_str(),
            redacted.text_hash,
            delta.cursor.chunk_sequence
        ),
        RepeatPolicy::OncePerRun => format!("{}:run:{}", rule.id.as_str(), delta.run_id.as_str()),
        RepeatPolicy::OncePerTurn => format!(
            "{}:turn:{}",
            rule.id.as_str(),
            delta
                .turn_id
                .as_ref()
                .map(|id| id.as_str())
                .unwrap_or(delta.run_id.as_str())
        ),
        RepeatPolicy::OncePerAttemptAndSpan => format!(
            "{}:attempt:{:?}:{:?}:{}:{}:{}",
            rule.id.as_str(),
            delta.attempt_id.as_ref().map(|id| id.as_str().to_string()),
            delta
                .realtime_session_id
                .as_ref()
                .map(|id| id.as_str().to_string()),
            delta.channel.as_contract_name(),
            redacted.text_hash,
            redacted.cursor.chunk_sequence
        ),
    }
}

fn find_match(matcher: &StreamMatcher, buffer: &str) -> Result<Option<(usize, usize)>, AgentError> {
    match matcher {
        StreamMatcher::Literal {
            text,
            case_sensitive,
            ..
        } => {
            if *case_sensitive {
                Ok(buffer.find(text).map(|start| (start, start + text.len())))
            } else {
                let haystack = buffer.to_lowercase();
                let needle = text.to_lowercase();
                Ok(haystack
                    .find(&needle)
                    .map(|start| (start, start + needle.len())))
            }
        }
        StreamMatcher::Regex { pattern, .. } => safe_regex_find(pattern, buffer),
        StreamMatcher::Marker { .. } => Ok(Some((0, buffer.len()))),
        StreamMatcher::HostMatcher { .. } => Ok(None),
    }
}

fn safe_regex_find(pattern: &str, buffer: &str) -> Result<Option<(usize, usize)>, AgentError> {
    crate::stream_records::validate_safe_regex(pattern)?;

    if let Some(match_range) = find_char_class_repetition(pattern, buffer) {
        return Ok(Some(match_range));
    }
    if let Some(match_range) = find_digit_repetition(pattern, buffer) {
        return Ok(Some(match_range));
    }
    if pattern.contains(".*") {
        return Ok(find_ordered_parts(pattern, buffer));
    }

    let literal = unescape_regex_literal(pattern);
    Ok(buffer
        .find(&literal)
        .map(|start| (start, start + literal.len())))
}

fn find_char_class_repetition(pattern: &str, buffer: &str) -> Option<(usize, usize)> {
    let class_start = pattern.find('[')?;
    let class_end = pattern[class_start..].find(']')? + class_start;
    let quantifier = &pattern[class_end + 1..];
    let min = if let Some(open) = quantifier.find('{') {
        let close = quantifier[open + 1..].find('}')? + open + 1;
        quantifier[open + 1..close]
            .trim_end_matches(',')
            .parse::<usize>()
            .ok()?
    } else {
        return None;
    };
    let prefix = unescape_regex_literal(&pattern[..class_start]);
    let suffix = "";
    let start = buffer.find(&prefix)?;
    let mut index = start + prefix.len();
    let mut count = 0;
    for character in buffer[index..].chars() {
        if character.is_ascii_alphanumeric() {
            index += character.len_utf8();
            count += 1;
        } else {
            break;
        }
    }
    if count >= min && buffer[index..].starts_with(suffix) {
        Some((start, index + suffix.len()))
    } else {
        None
    }
}

fn find_digit_repetition(pattern: &str, buffer: &str) -> Option<(usize, usize)> {
    let marker = "\\d+";
    let digit_start = pattern.find(marker)?;
    let prefix = unescape_regex_literal(&pattern[..digit_start]);
    let suffix = unescape_regex_literal(&pattern[digit_start + marker.len()..]);
    let start = buffer.find(&prefix)?;
    let mut index = start + prefix.len();
    let mut count = 0;
    for character in buffer[index..].chars() {
        if character.is_ascii_digit() {
            index += character.len_utf8();
            count += 1;
        } else {
            break;
        }
    }
    if count > 0 && buffer[index..].starts_with(&suffix) {
        Some((start, index + suffix.len()))
    } else {
        None
    }
}

fn find_ordered_parts(pattern: &str, buffer: &str) -> Option<(usize, usize)> {
    let parts = pattern
        .split(".*")
        .map(unescape_regex_literal)
        .collect::<Vec<_>>();
    let first = parts.first()?;
    let mut start = buffer.find(first)?;
    let mut cursor = start;
    for part in &parts {
        if part.is_empty() {
            continue;
        }
        let relative = buffer[cursor..].find(part)?;
        cursor += relative + part.len();
    }
    if first.is_empty() {
        start = 0;
    }
    Some((start, cursor))
}

fn unescape_regex_literal(pattern: &str) -> String {
    pattern
        .replace("\\.", ".")
        .replace("\\-", "-")
        .replace("\\_", "_")
}

fn truncate_utf8_suffix(buffer: &mut String, max_bytes: usize) {
    if max_bytes == 0 || buffer.len() <= max_bytes {
        return;
    }
    let mut start = buffer.len() - max_bytes;
    while !buffer.is_char_boundary(start) {
        start += 1;
    }
    buffer.replace_range(..start, "");
}
