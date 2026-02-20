use serde::{Deserialize, Serialize};

/// A high-level representation of an OO diagram.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagram {
    pub classes: Vec<Class>,
    pub relationships: Vec<Relationship>,
}

/// A method belonging to a class.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Method {
    pub name: String,
}

/// A single class in the diagram.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Class {
    pub name: String,

    // If the frontend does not provide methods (it won't), Serde will treat it as []
    #[serde(default)]
    pub methods: Vec<Method>,
}

/// The kind of relationship between classes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RelationshipKind {
    Association,
    Aggregation,
    Composition,
    Inheritance,
}

/// A relationship from one class to another.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub from: String,
    pub to: String,
    pub kind: RelationshipKind,
}
