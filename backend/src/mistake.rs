use serde::Serialize;

/// The type of mistake the student made in their diagram.
#[derive(Debug, Serialize)]
pub enum MistakeKind {
    /// Student is missing a class
    MissingClass,

    /// Student has added an EXTRA class
    ExtraClass,

    /// Student forgot to add a relationship
    MissingRelationship,

    /// Student added an EXTRA relationship
    ExtraRelationship,

    /// Student added a relationship, but it's the wrong kind
    WrongRelationshipType,

    /// Student forgot to add a method
    MissingMethod,
}

/// A single mistake we can show to the user.
#[derive(Debug, Serialize)]
pub struct Mistake {
    /// Enum that represents the Mistake that was made
    pub kind: MistakeKind,

    /// String that is presented to the user in the UI to
    /// explain the mistake
    pub message: String,

    /// Elements that are related to the mistake
    pub related_elements: Vec<String>,
}

/// Helper constructors for mistakes.
/// These make it easy to generate mistakes cleanly in compare.rs.
impl Mistake {
    /// Create a user-facing message about a Missing Class
    pub fn missing_class(name: &str) -> Self {
        Self {
            kind: MistakeKind::MissingClass,
            message: format!("Class '{}' is missing from your diagram.", name),
            related_elements: vec![name.to_string()],
        }
    }

    /// Create a user-facing message about an Extra Class
    pub fn extra_class(name: &str) -> Self {
        Self {
            kind: MistakeKind::ExtraClass,
            message: format!(
                "Class '{}' appears in your diagram but not in the code.",
                name
            ),
            related_elements: vec![name.to_string()],
        }
    }

    /// Create a user-facing message about a Missing Relationship
    pub fn missing_relationship(from: &str, to: &str) -> Self {
        Self {
            kind: MistakeKind::MissingRelationship,
            message: format!("Relationship from '{}' to '{}' is missing.", from, to),
            related_elements: vec![from.to_string(), to.to_string()],
        }
    }

    /// Create a user-facing message about an extra relationship
    pub fn extra_relationship(from: &str, to: &str) -> Self {
        Self {
            kind: MistakeKind::ExtraRelationship,
            message: format!("Relationship from '{}' to '{}' should not exist.", from, to),
            related_elements: vec![from.to_string(), to.to_string()],
        }
    }

    /// Create a user-facing message about a wrong relationship type
    pub fn wrong_relationship_type(from: &str, to: &str) -> Self {
        Self {
            kind: MistakeKind::WrongRelationshipType,
            message: format!(
                "Relationship between '{}' and '{}' has the wrong type.",
                from, to
            ),
            related_elements: vec![from.to_string(), to.to_string()],
        }
    }

    /// Create a user-facing message about a Missing Method
    pub fn missing_method(class_name: &str, method_name: &str) -> Self {
        Self {
            kind: MistakeKind::MissingMethod,
            message: format!(
                "Method '{}' is missing from class '{}'.",
                method_name, class_name
            ),
            related_elements: vec![class_name.to_string(), method_name.to_string()],
        }
    }
}
