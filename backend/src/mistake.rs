use serde::Serialize;

/// The type of mistake the student made in their diagram.
#[derive(Debug, Serialize)]
pub enum MistakeKind {
    MissingClass,
    ExtraClass,
    MissingRelationship,
    ExtraRelationship,
    WrongRelationshipType,
    MissingMethod,                // <-- added
}

/// A single mistake we can show to the user.
#[derive(Debug, Serialize)]
pub struct Mistake {
    pub kind: MistakeKind,
    pub message: String,
    pub related_elements: Vec<String>,
}

/// Helper constructors for mistakes.
/// These make it easy to generate mistakes cleanly in compare.rs.
impl Mistake {

    pub fn missing_class(name: &str) -> Self {
        Self {
            kind: MistakeKind::MissingClass,
            message: format!("Class '{}' is missing from your diagram.", name),
            related_elements: vec![name.to_string()],
        }
    }

    pub fn extra_class(name: &str) -> Self {
        Self {
            kind: MistakeKind::ExtraClass,
            message: format!("Class '{}' appears in your diagram but not in the code.", name),
            related_elements: vec![name.to_string()],
        }
    }

    pub fn missing_relationship(from: &str, to: &str) -> Self {
        Self {
            kind: MistakeKind::MissingRelationship,
            message: format!("Relationship from '{}' to '{}' is missing.", from, to),
            related_elements: vec![from.to_string(), to.to_string()],
        }
    }

    pub fn extra_relationship(from: &str, to: &str) -> Self {
        Self {
            kind: MistakeKind::ExtraRelationship,
            message: format!("Relationship from '{}' to '{}' should not exist.", from, to),
            related_elements: vec![from.to_string(), to.to_string()],
        }
    }

    pub fn wrong_relationship_type(from: &str, to: &str) -> Self {
        Self {
            kind: MistakeKind::WrongRelationshipType,
            message: format!("Relationship between '{}' and '{}' has the wrong type.", from, to),
            related_elements: vec![from.to_string(), to.to_string()],
        }
    }

    /// NEW — MissingMethod mistake
    pub fn missing_method(class_name: &str, method_name: &str) -> Self {
        Self {
            kind: MistakeKind::MissingMethod,
            message: format!("Method '{}' is missing from class '{}'.", method_name, class_name),
            related_elements: vec![class_name.to_string(), method_name.to_string()],
        }
    }
}

