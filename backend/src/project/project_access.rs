use serde::{Deserialize, Serialize};

use crate::ws::messages::ServerMessageError;

#[derive(Serialize, Deserialize, Debug, Default, Clone, Copy, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AccessLevel {
    #[default]
    Queue,
    ReadOnly,
    Editor,
}

impl AccessLevel {
    pub fn can_read(&self) -> bool {
        matches!(self, Self::ReadOnly | Self::Editor)
    }

    #[expect(dead_code)]
    pub fn is_readonly(&self) -> bool {
        matches!(self, Self::ReadOnly)
    }

    #[expect(dead_code)]
    pub fn is_editor(&self) -> bool {
        matches!(self, Self::Editor)
    }

    pub fn need_read(&self) -> Result<(), ServerMessageError> {
        self.can_read()
            .then_some(())
            .ok_or(ServerMessageError::NotAccessible)
    }

    pub fn need_editor(&self) -> Result<(), ServerMessageError> {
        match self {
            Self::Queue => Err(ServerMessageError::NotAccessible),
            Self::ReadOnly => Err(ServerMessageError::ReadonlyPermission),
            Self::Editor => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AccessLevel;
    use crate::ws::messages::ServerMessageError;

    #[test]
    fn access_levels_have_expected_permissions() {
        assert!(!AccessLevel::Queue.can_read());
        assert!(AccessLevel::ReadOnly.can_read());
        assert!(AccessLevel::Editor.can_read());

        assert!(!AccessLevel::Queue.is_readonly());
        assert!(AccessLevel::ReadOnly.is_readonly());
        assert!(!AccessLevel::Editor.is_readonly());

        assert!(!AccessLevel::Queue.is_editor());
        assert!(!AccessLevel::ReadOnly.is_editor());
        assert!(AccessLevel::Editor.is_editor());
    }

    #[test]
    fn read_and_edit_guards_return_specific_errors() {
        assert!(matches!(
            AccessLevel::Queue.need_read(),
            Err(ServerMessageError::NotAccessible)
        ));
        assert!(AccessLevel::ReadOnly.need_read().is_ok());
        assert!(AccessLevel::Editor.need_read().is_ok());

        assert!(matches!(
            AccessLevel::Queue.need_editor(),
            Err(ServerMessageError::NotAccessible)
        ));
        assert!(matches!(
            AccessLevel::ReadOnly.need_editor(),
            Err(ServerMessageError::ReadonlyPermission)
        ));
        assert!(AccessLevel::Editor.need_editor().is_ok());
    }
}
