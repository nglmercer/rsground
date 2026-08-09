use std::collections::HashMap;

use operational_transform::OperationSeq;
use serde::Serialize;
use tokio::sync::{Notify, RwLock, RwLockWriteGuard};

use crate::collab::ot::transform_index;
use crate::constants::limits;
use crate::utils::{ArcStr, AsyncInto};

use super::UserOperation;

#[derive(Debug, Default)]
pub struct Document {
    /// State modified by critical sections of the code.
    state: RwLock<DocumentState>,
    /// Used to notify clients of new text operations.
    notify: Notify,
}

#[derive(Debug, Default)]
pub struct DocumentState {
    operations: Vec<UserOperation>,
    text: String,
    pub cursors: HashMap<ArcStr, Vec<(u32, u32)>>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DocumentInfo {
    pub text: String,
    pub revision: usize,
}

impl AsyncInto<DocumentInfo> for &Document {
    async fn async_into(self) -> DocumentInfo {
        let state = self.state.read().await;
        DocumentInfo {
            text: state.text.clone(),
            revision: state.operations.len(),
        }
    }
}

impl Document {
    pub fn new() -> Self {
        Document::default()
    }

    pub fn new_with(text: String) -> Self {
        Document {
            state: DocumentState {
                text,
                ..Default::default()
            }
            .into(),
            ..Default::default()
        }
    }

    pub async fn fork(&self) -> Self {
        Document {
            state: DocumentState {
                text: self.state.read().await.text.clone(),
                ..Default::default()
            }
            .into(),
            ..Default::default()
        }
    }

    pub async fn text(&self) -> String {
        self.state.read().await.text.clone()
    }

    pub async fn revision(&self) -> usize {
        self.state.read().await.operations.len()
    }

    pub async fn state_mut(&self) -> RwLockWriteGuard<'_, DocumentState> {
        self.state.write().await
    }

    pub async fn send_history(&self, start: usize) -> (usize, Option<Vec<UserOperation>>) {
        let operations = {
            let state = self.state.read().await;
            let len = state.operations.len();
            if start < len {
                state.operations[start..].to_owned()
            } else {
                Vec::new()
            }
        };
        let num_ops = operations.len();
        let revision = start + num_ops;

        if num_ops > 0 {
            (revision, Some(operations))
        } else {
            (revision, None)
        }
    }

    /// Add actions to document history.
    /// - Transform desynchorized actions
    /// - Notify to document listeners
    pub async fn compose(
        &self,
        user_id: ArcStr,
        revision: usize,
        mut operation: OperationSeq,
    ) -> Result<(), String> {
        log::info!(
            "edit: id = {}, revision = {}, base_len = {}, target_len = {}",
            user_id,
            revision,
            operation.base_len(),
            operation.target_len()
        );

        let mut state = self.state.write().await;

        let new_text = {
            let len = state.operations.len();
            if revision > len {
                return Err(format!("got revision {revision}, but current is {len}"));
            }
            for history_op in &state.operations[revision..] {
                operation = operation
                    .transform(&history_op.operation)
                    .map_err(|err| err.to_string())?
                    .0;
            }
            if operation.target_len() > limits::MAX_DOCUMENT_BYTES {
                return Err(format!(
                    "target length {} is greater than {} KiB maximum",
                    operation.target_len(),
                    limits::MAX_DOCUMENT_BYTES / 1024,
                ));
            }

            operation
                .apply(&state.text)
                .map_err(|err| err.to_string())?
        };

        for data in state.cursors.values_mut() {
            for (start, end) in data.iter_mut() {
                *start = transform_index(&operation, *start);
                *end = transform_index(&operation, *end);
            }
        }

        state.operations.push(UserOperation { user_id, operation });
        state.text = new_text;
        self.notify.notify_waiters();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Document;
    use crate::constants::limits;
    use operational_transform::OperationSeq;

    fn append(base_len: u32, text: &str) -> OperationSeq {
        let mut operation = OperationSeq::default();
        operation.retain(base_len as u64);
        operation.insert(text);
        operation
    }

    fn prepend(base_len: u32, text: &str) -> OperationSeq {
        let mut operation = OperationSeq::default();
        operation.insert(text);
        operation.retain(base_len as u64);
        operation
    }

    #[tokio::test]
    async fn compose_updates_text_history_and_cursors() {
        let document = Document::new_with("hello".to_owned());
        document
            .state_mut()
            .await
            .cursors
            .insert("client".into(), vec![(1, 4)]);

        document
            .compose("first".into(), 0, append(5, " world"))
            .await
            .unwrap();
        assert_eq!(document.text().await, "hello world");
        assert_eq!(document.revision().await, 1);
        assert_eq!(
            document.state_mut().await.cursors.get("client"),
            Some(&vec![(1, 4)])
        );

        // This operation was authored against revision zero and must be
        // transformed over the append above.
        document
            .compose("second".into(), 0, prepend(5, "X"))
            .await
            .unwrap();
        assert_eq!(document.text().await, "Xhello world");
        assert_eq!(document.revision().await, 2);
        assert_eq!(
            document.state_mut().await.cursors.get("client"),
            Some(&vec![(2, 5)])
        );

        let (revision, history) = document.send_history(1).await;
        let history = history.expect("revision one should have one operation");
        assert_eq!(revision, 2);
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].user_id.as_ref(), "second");

        let (revision, history) = document.send_history(2).await;
        assert_eq!(revision, 2);
        assert!(history.is_none());
    }

    #[tokio::test]
    async fn rejects_future_revisions_and_oversized_documents() {
        let document = Document::new_with("hello".to_owned());
        let error = document
            .compose("client".into(), 1, append(5, "!"))
            .await
            .expect_err("future revisions must be rejected");
        assert!(error.contains("current is 0"));
        assert_eq!(document.revision().await, 0);

        let oversized = Document::new();
        let mut operation = OperationSeq::default();
        operation.insert(&"a".repeat(limits::MAX_DOCUMENT_BYTES + 1));
        let error = oversized
            .compose("client".into(), 0, operation)
            .await
            .expect_err("oversized documents must be rejected");
        assert!(error.contains("greater than"));
        assert_eq!(oversized.text().await, "");
        assert_eq!(oversized.revision().await, 0);
    }

    #[tokio::test]
    async fn fork_copies_text_without_history_or_cursors() {
        let document = Document::new_with("hello".to_owned());
        document
            .state_mut()
            .await
            .cursors
            .insert("client".into(), vec![(0, 1)]);
        document
            .compose("client".into(), 0, append(5, "!"))
            .await
            .unwrap();

        let fork = document.fork().await;
        assert_eq!(fork.text().await, "hello!");
        assert_eq!(fork.revision().await, 0);
        assert!(fork.state_mut().await.cursors.is_empty());
    }
}
