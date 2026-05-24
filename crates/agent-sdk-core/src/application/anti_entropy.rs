use serde::{Deserialize, Serialize};

use crate::{
    domain::{AgentError, JournalCursor},
    journal::{JournalRecord, JournalRecordPayload},
    output_delivery::OutputDeliveryRecord,
    replay::journal_cursor_for_seq,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DerivedViewKind {
    EventSubscriptionIndex,
    OutputDedupeIndex,
    OutputSinkRepairCursor,
    TelemetryProjection,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DerivedViewState {
    pub view_id: String,
    pub view_kind: DerivedViewKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_repaired_cursor: Option<JournalCursor>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AntiEntropyRepair {
    pub view_id: String,
    pub view_kind: DerivedViewKind,
    pub repair_from: JournalCursor,
    pub repair_to: JournalCursor,
    pub affected_record_ids: Vec<String>,
    pub repair_reason: String,
    pub host_action_required: bool,
    pub external_side_effect_compensation: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AntiEntropyReport {
    pub latest_journal_cursor: JournalCursor,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub repairs: Vec<AntiEntropyRepair>,
}

#[derive(Clone, Debug, Default)]
pub struct AntiEntropyScanner;

impl AntiEntropyScanner {
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
