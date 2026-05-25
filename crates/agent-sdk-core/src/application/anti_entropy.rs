//! Application-layer coordination over core primitives. Use these services to lower
//! helpers, drive runs, validate output, coordinate tools, approvals, delivery,
//! isolation, telemetry, and feature layers. Methods in this layer may call
//! configured ports, mutate in-memory stores, append journals, or publish events as
//! documented. This file contains the anti entropy portion of that contract.
//!
use serde::{Deserialize, Serialize};

use crate::{
    domain::{AgentError, JournalCursor},
    journal::{JournalRecord, JournalRecordPayload},
    output_delivery::OutputDeliveryRecord,
    replay::journal_cursor_for_seq,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite derived view kind cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum DerivedViewKind {
    /// Use this variant when the contract needs to represent event subscription index; selecting it has no side effect by itself.
    EventSubscriptionIndex,
    /// Use this variant when the contract needs to represent output dedupe index; selecting it has no side effect by itself.
    OutputDedupeIndex,
    /// Use this variant when the contract needs to represent output sink repair cursor; selecting it has no side effect by itself.
    OutputSinkRepairCursor,
    /// Use this variant when the contract needs to represent telemetry projection; selecting it has no side effect by itself.
    TelemetryProjection,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Holds derived view state application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct DerivedViewState {
    /// Stable view id used for typed lineage, lookup, or dedupe.
    pub view_id: String,
    /// Kind discriminator for view kind.
    /// Use it to route finite match arms without parsing display text.
    pub view_kind: DerivedViewKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Last journal cursor this derived view successfully reconciled.
    /// Anti-entropy scans use it to avoid replaying already-repaired derived-view gaps.
    pub last_repaired_cursor: Option<JournalCursor>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Holds anti entropy repair application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct AntiEntropyRepair {
    /// Stable view id used for typed lineage, lookup, or dedupe.
    pub view_id: String,
    /// Kind discriminator for view kind.
    /// Use it to route finite match arms without parsing display text.
    pub view_kind: DerivedViewKind,
    /// First journal cursor included in the derived-view gap.
    /// Repair logic uses this as the lower bound for replay or reconciliation evidence.
    pub repair_from: JournalCursor,
    /// Last journal cursor included in the derived-view gap.
    /// Repair logic stores this as the cursor reached after the derived view is reconciled.
    pub repair_to: JournalCursor,
    /// Identifiers used to select or correlate affected record values.
    /// Use them for typed lookup, filtering, or lineage instead of stringly typed matching.
    pub affected_record_ids: Vec<String>,
    /// Human-readable reason the anti-entropy scan queued this repair.
    /// Use it for diagnostics and host action prompts, not as a machine policy discriminator.
    pub repair_reason: String,
    /// Whether host action required is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub host_action_required: bool,
    /// Whether recovery must account for an external side effect that may already have
    /// happened.
    /// Repair code uses this to choose compensation or reconciliation instead of blindly
    /// retrying the effect.
    pub external_side_effect_compensation: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Holds anti entropy report application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct AntiEntropyReport {
    /// Cursor identifying a replay, export, or subscription position.
    /// Use it to resume without widening the original scope.
    pub latest_journal_cursor: JournalCursor,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Repairs queued by the scan for stale or inconsistent derived views.
    /// Applying these records mutates only the derived-view state unless a host action is explicitly
    /// requested.
    pub repairs: Vec<AntiEntropyRepair>,
}

#[derive(Clone, Debug, Default)]
/// Holds anti entropy scanner application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct AntiEntropyScanner;

impl AntiEntropyScanner {
    /// Returns derived view derived from the supplied state.
    /// This uses only local coordinator state and performs no hidden host work.
    pub fn derived_view(
        &self,
        view_id: impl Into<String>,
        last_repaired_cursor: Option<JournalCursor>,
    ) -> DerivedViewState {
        DerivedViewState {
            view_id: view_id.into(),
            view_kind: DerivedViewKind::OutputSinkRepairCursor,
            last_repaired_cursor,
        }
    }

    /// Operates on in-memory or journal-derived application::anti_entropy
    /// state for diagnostics and repair evidence. It does not create a second
    /// run loop or product workflow owner.
    pub fn scan(
        &self,
        records: &[JournalRecord],
        views: &[DerivedViewState],
    ) -> Result<AntiEntropyReport, AgentError> {
        let latest_seq = records
            .iter()
            .map(|record| record.journal_seq)
            .max()
            .unwrap_or(0);
        let mut repairs = Vec::new();
        for view in views {
            if let Some(repair) = self.scan_view(records, view)? {
                repairs.push(repair);
            }
        }
        Ok(AntiEntropyReport {
            latest_journal_cursor: journal_cursor_for_seq(latest_seq),
            repairs,
        })
    }

    /// Returns repair internal view derived from the supplied state.
    /// This uses only local coordinator state and performs no hidden host work.
    pub fn repair_internal_view(
        &self,
        view: &mut DerivedViewState,
        repair: &AntiEntropyRepair,
    ) -> Result<(), AgentError> {
        if view.view_id != repair.view_id || view.view_kind != repair.view_kind {
            return Err(AgentError::contract_violation(
                "anti-entropy repair does not target this derived view",
            ));
        }
        view.last_repaired_cursor = Some(repair.repair_to.clone());
        Ok(())
    }

    fn scan_view(
        &self,
        records: &[JournalRecord],
        view: &DerivedViewState,
    ) -> Result<Option<AntiEntropyRepair>, AgentError> {
        let last_seq = view
            .last_repaired_cursor
            .as_ref()
            .map(journal_cursor_seq)
            .unwrap_or(0);
        let relevant = records
            .iter()
            .filter(|record| record.journal_seq > last_seq)
            .filter(|record| relevant_to_view(record, &view.view_kind))
            .collect::<Vec<_>>();
        if relevant.is_empty() {
            return Ok(None);
        }

        let repair_from = relevant
            .iter()
            .map(|record| record.journal_seq)
            .min()
            .map(journal_cursor_for_seq)
            .expect("nonempty relevant records");
        let repair_to = relevant
            .iter()
            .map(|record| record.journal_seq)
            .max()
            .map(journal_cursor_for_seq)
            .expect("nonempty relevant records");
        let host_action_required = relevant.iter().any(|record| {
            matches!(
                &record.payload,
                JournalRecordPayload::OutputDelivery(OutputDeliveryRecord::Reconciliation(_))
            )
        });

        Ok(Some(AntiEntropyRepair {
            view_id: view.view_id.clone(),
            view_kind: view.view_kind.clone(),
            repair_from,
            repair_to,
            affected_record_ids: relevant
                .iter()
                .map(|record| record.record_id.clone())
                .collect(),
            repair_reason: match view.view_kind {
                DerivedViewKind::OutputSinkRepairCursor if host_action_required => {
                    "output delivery reconciliation requires sink-scoped repair cursor".to_string()
                }
                DerivedViewKind::OutputSinkRepairCursor => {
                    "output delivery derived view is behind journal".to_string()
                }
                _ => "derived view is behind journal".to_string(),
            },
            host_action_required,
            external_side_effect_compensation: false,
        }))
    }
}

fn relevant_to_view(record: &JournalRecord, kind: &DerivedViewKind) -> bool {
    match kind {
        DerivedViewKind::OutputSinkRepairCursor => matches!(
            &record.payload,
            JournalRecordPayload::OutputDelivery(OutputDeliveryRecord::Intent(_))
                | JournalRecordPayload::OutputDelivery(OutputDeliveryRecord::Reconciliation(_))
        ),
        DerivedViewKind::OutputDedupeIndex => matches!(
            &record.payload,
            JournalRecordPayload::OutputDelivery(OutputDeliveryRecord::Dedupe(_))
                | JournalRecordPayload::OutputDelivery(OutputDeliveryRecord::Result(_))
        ),
        DerivedViewKind::EventSubscriptionIndex | DerivedViewKind::TelemetryProjection => true,
    }
}

fn journal_cursor_seq(cursor: &JournalCursor) -> u64 {
    cursor
        .as_str()
        .rsplit_once('.')
        .and_then(|(_, seq)| seq.parse::<u64>().ok())
        .unwrap_or(0)
}
