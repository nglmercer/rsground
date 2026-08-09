//! Helper methods for working with operational transformation.

use operational_transform::{Operation, OperationSeq};

/// Return the new index of a position in the string.
pub fn transform_index(operation: &OperationSeq, position: u32) -> u32 {
    let mut index = position as i32;
    let mut new_index = index;
    for op in operation.ops() {
        match op {
            &Operation::Retain(n) => index -= n as i32,
            Operation::Insert(s) => new_index += bytecount::num_chars(s.as_bytes()) as i32,
            &Operation::Delete(n) => {
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

#[cfg(test)]
mod tests {
    use super::transform_index;
    use operational_transform::OperationSeq;

    #[test]
    fn insertion_shifts_positions_by_unicode_character_count() {
        let mut operation = OperationSeq::default();
        operation.insert("🌎");
        operation.retain(4);

        assert_eq!(transform_index(&operation, 0), 1);
        assert_eq!(transform_index(&operation, 2), 3);
        assert_eq!(transform_index(&operation, 4), 5);
    }

    #[test]
    fn deletion_collapses_positions_inside_deleted_range() {
        let mut operation = OperationSeq::default();
        operation.retain(2);
        operation.delete(2);
        operation.retain(2);

        assert_eq!(transform_index(&operation, 1), 1);
        assert_eq!(transform_index(&operation, 2), 2);
        assert_eq!(transform_index(&operation, 3), 2);
        assert_eq!(transform_index(&operation, 6), 4);
    }
}
