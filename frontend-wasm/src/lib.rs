use operational_transform::OperationSeq;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// Set a panic listener to display better error messages.
#[wasm_bindgen]
pub fn set_panic_hook() {
    console_error_panic_hook::set_once();
}

/// This is an wrapper around `operational_transform::OperationSeq`, which is
/// necessary for Wasm compatibility through `wasm-bindgen`.
#[wasm_bindgen]
#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OpSeq(OperationSeq);

/// This is a pair of `OpSeq` structs, which is needed to handle some return
/// values from `wasm-bindgen`.
#[wasm_bindgen]
#[derive(Default, Clone, Debug, PartialEq)]
pub struct OpSeqPair(OpSeq, OpSeq);

impl OpSeq {
    /// Transforms two operations A and B that happened concurrently and produces
    /// two operations A' and B' (in an array) such that
    ///     `apply(apply(S, A), B') = apply(apply(S, B), A')`.
    /// This function is the heart of OT.
    ///
    /// Unlike `OpSeq::transform`, this function returns a raw tuple, which is
    /// more efficient but cannot be exported by `wasm-bindgen`.
    ///
    /// # Error
    ///
    /// Returns `None` if the operations cannot be transformed due to
    /// length conflicts.
    pub fn transform_raw(&self, other: &OpSeq) -> Option<(OpSeq, OpSeq)> {
        let (a, b) = self.0.transform(&other.0).ok()?;
        Some((Self(a), Self(b)))
    }
}

#[wasm_bindgen]
impl OpSeq {
    /// Creates a default empty `OpSeq`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a store for operatations which does not need to allocate  until
    /// `capacity` operations have been stored inside.
    pub fn with_capacity(capacity: usize) -> Self {
        Self(OperationSeq::with_capacity(capacity))
    }

    /// Merges the operation with `other` into one operation while preserving
    /// the changes of both. Or, in other words, for each input string S and a
    /// pair of consecutive operations A and B.
    ///     `apply(apply(S, A), B) = apply(S, compose(A, B))`
    /// must hold.
    ///
    /// # Error
    ///
    /// Returns `None` if the operations are not composable due to length
    /// conflicts.
    pub fn compose(&self, other: &OpSeq) -> Option<OpSeq> {
        self.0.compose(&other.0).ok().map(Self)
    }

    /// Deletes `n` characters at the current cursor position.
    pub fn delete(&mut self, n: u32) {
        self.0.delete(n as u64)
    }

    /// Inserts a `s` at the current cursor position.
    pub fn insert(&mut self, s: &str) {
        self.0.insert(s)
    }

    /// Moves the cursor `n` characters forwards.
    pub fn retain(&mut self, n: u32) {
        self.0.retain(n as u64)
    }

    /// Transforms two operations A and B that happened concurrently and produces
    /// two operations A' and B' (in an array) such that
    ///     `apply(apply(S, A), B') = apply(apply(S, B), A')`.
    /// This function is the heart of OT.
    ///
    /// # Error
    ///
    /// Returns `None` if the operations cannot be transformed due to
    /// length conflicts.
    pub fn transform(&self, other: &OpSeq) -> Option<OpSeqPair> {
        let (a, b) = self.0.transform(&other.0).ok()?;
        Some(OpSeqPair(Self(a), Self(b)))
    }

    /// Applies an operation to a string, returning a new string.
    ///
    /// # Error
    ///
    /// Returns an error if the operation cannot be applied due to length
    /// conflicts.
    pub fn apply(&self, s: &str) -> Option<String> {
        self.0.apply(s).ok()
    }

    /// Computes the inverse of an operation. The inverse of an operation is the
    /// operation that reverts the effects of the operation, e.g. when you have
    /// an operation 'insert("hello "); skip(6);' then the inverse is
    /// 'delete("hello "); skip(6);'. The inverse should be used for
    /// implementing undo.
    pub fn invert(&self, s: &str) -> Self {
        Self(self.0.invert(s))
    }

    /// Checks if this operation has no effect.
    pub fn is_noop(&self) -> bool {
        self.0.is_noop()
    }

    /// Returns the length of a string these operations can be applied to
    pub fn base_len(&self) -> usize {
        self.0.base_len()
    }

    /// Returns the length of the resulting string after the operations have
    /// been applied.
    pub fn target_len(&self) -> usize {
        self.0.target_len()
    }

    /// Return the new index of a position in the string.
    pub fn transform_index(&self, position: u32) -> u32 {
        let mut index = position as i32;
        let mut new_index = index;
        for op in self.0.ops() {
            use operational_transform::Operation::*;
            match op {
                &Retain(n) => index -= n as i32,
                Insert(s) => new_index += bytecount::num_chars(s.as_bytes()) as i32,
                &Delete(n) => {
                    new_index -= std::cmp::min(index, n as i32);
                    index -= n as i32;
                }
            }
            if index < 0 {
                break;
            }
        }
        new_index as u32
    }

    /// Attempts to deserialize an `OpSeq` from a JSON string.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<OpSeq> {
        serde_json::from_str(s).ok()
    }

    /// Converts this object to a JSON string.
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).expect("json serialization failure")
    }
}

#[wasm_bindgen]
impl OpSeqPair {
    /// Returns the first element of the pair.
    pub fn first(&self) -> OpSeq {
        self.0.clone()
    }

    /// Returns the second element of the pair.
    pub fn second(&self) -> OpSeq {
        self.1.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::OpSeq;

    #[test]
    fn applies_unicode_insertions_and_deletions() {
        let mut operation = OpSeq::new();
        operation.retain(5);
        operation.insert(" brave");
        operation.retain(6);

        assert_eq!(operation.base_len(), 11);
        assert_eq!(operation.target_len(), 17);
        assert_eq!(
            operation.apply("hello world"),
            Some("hello brave world".to_owned())
        );

        let mut delete = OpSeq::new();
        delete.retain(5);
        delete.delete(6);
        assert_eq!(delete.apply("hello world"), Some("hello".to_owned()));
    }

    #[test]
    fn inverse_reverts_an_operation() {
        let mut operation = OpSeq::new();
        operation.retain(5);
        operation.insert(" brave");
        operation.retain(6);

        let changed = operation.apply("hello world").unwrap();
        let inverse = operation.invert("hello world");

        assert_eq!(inverse.apply(&changed), Some("hello world".to_owned()));
    }

    #[test]
    fn composes_consecutive_operations() {
        let mut first = OpSeq::new();
        first.insert("hello");

        let mut second = OpSeq::new();
        second.retain(5);
        second.insert(" world");

        let composed = first.compose(&second).unwrap();
        assert_eq!(composed.apply(""), Some("hello world".to_owned()));

        let mut incompatible = OpSeq::new();
        incompatible.retain(1);
        assert!(first.compose(&incompatible).is_none());
    }

    #[test]
    fn transforms_concurrent_operations_in_both_orders() {
        let mut local = OpSeq::new();
        local.retain(5);
        local.insert("!");

        let mut remote = OpSeq::new();
        remote.retain(5);
        remote.insert("?");

        let pair = local.transform(&remote).unwrap();
        let local_then_remote = pair.second().apply(&local.apply("hello").unwrap()).unwrap();
        let remote_then_local = pair.first().apply(&remote.apply("hello").unwrap()).unwrap();

        assert_eq!(local_then_remote, remote_then_local);
        assert_eq!(
            local.transform_raw(&remote).unwrap(),
            (pair.first(), pair.second())
        );

        let mut incompatible = OpSeq::new();
        incompatible.retain(4);
        assert!(local.transform(&incompatible).is_none());
    }

    #[test]
    fn transforms_indexes_for_insertions_and_deletions() {
        let mut insertion = OpSeq::new();
        insertion.insert("🌎");
        insertion.retain(4);

        assert_eq!(insertion.transform_index(0), 1);
        assert_eq!(insertion.transform_index(2), 3);
        assert_eq!(insertion.transform_index(4), 5);

        let mut deletion = OpSeq::new();
        deletion.retain(2);
        deletion.delete(2);
        deletion.retain(2);

        assert_eq!(deletion.transform_index(1), 1);
        assert_eq!(deletion.transform_index(2), 2);
        assert_eq!(deletion.transform_index(3), 2);
        assert_eq!(deletion.transform_index(6), 4);
    }

    #[test]
    fn serializes_and_deserializes_operations() {
        let mut operation = OpSeq::with_capacity(3);
        operation.retain(5);
        operation.insert(" brave");
        operation.retain(6);

        let serialized = operation.to_string();
        assert_eq!(serialized, r#"[5," brave",6]"#);
        assert_eq!(OpSeq::from_str(&serialized), Some(operation.clone()));
        assert!(OpSeq::from_str("not json").is_none());
        assert!(OpSeq::from_str(r#"{"invalid":true}"#).is_none());
    }

    #[test]
    fn reports_noop_and_length_metadata() {
        let operation = OpSeq::new();
        assert!(operation.is_noop());
        assert_eq!(operation.base_len(), 0);
        assert_eq!(operation.target_len(), 0);

        let mut retain = OpSeq::new();
        retain.retain(3);
        assert!(retain.is_noop());
        assert_eq!(retain.base_len(), 3);
        assert_eq!(retain.target_len(), 3);

        let mut changed = OpSeq::new();
        changed.insert("x");
        assert!(!changed.is_noop());
    }
}
