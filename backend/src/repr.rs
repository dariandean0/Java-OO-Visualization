use serde::{Deserialize, Serialize};

/// A high-level representation of an OO diagram.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagram {
    pub classes: Vec<JavaClass>,
    pub relationships: Vec<Relationship>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JavaClass {
    pub name: String,
    pub visibility: String,
    pub is_abstract: bool,
    pub is_interface: bool,
    pub extends: Option<String>,
    pub implements: Vec<String>,
    pub fields: Vec<JavaField>,
    pub methods: Vec<JavaMethod>,
    pub constructors: Vec<JavaMethod>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaField {
    pub name: String,
    pub field_type: String,
    pub visibility: String,
    pub is_static: bool,
    pub is_final: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JavaMethod {
    pub name: String,
    pub return_type: String,
    pub visibility: String,
    pub is_static: bool,
    pub is_abstract: bool,
    pub parameters: Vec<JavaParameter>,
    pub calls: Vec<MethodCall>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaParameter {
    pub name: String,
    pub param_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodCall {
    pub caller_method: String,
    pub caller_class: String,
    pub method_name: String,
    pub target_class: String,
    pub is_static_call: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectInfo {
    pub variable_name: String,
    pub class_name: String,
    pub declared_at_line: usize,
    pub is_parameter: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    /// Class A
    pub from: String,
    /// Class B
    pub to: String,
    /// What relationship does Class A have to Class B
    pub kind: RelationshipType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RelationshipType {
    /// Class A extends class B
    Extends,
    /// Class A implements interface B
    Implements,
    Uses,
    Calls,
    Contains,
    MethodCall,
}
