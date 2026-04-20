use serde::{Deserialize, Serialize};

/// A high-level representation of an OO diagram.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagram {
    /// Classes rendered in the diagram
    pub classes: Vec<JavaClass>,

    /// Edges between those classes (extends, implements, uses, ...)
    pub relationships: Vec<Relationship>,
}

/// A Class in the Java Programming language
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JavaClass {
    /// Name of the Java Class
    pub name: String,

    /// Visibility of the class
    /// Possible values: "public", "private", "protected", and ""
    pub visibility: String,

    /// Is the class abstract?
    pub is_abstract: bool,

    /// Is the class an interface?
    pub is_interface: bool,

    /// What subclasses does this class extend?
    pub extends: Option<String>,

    /// List of interfaces the class implements
    pub implements: Vec<String>,

    /// What fields does this class contain?
    pub fields: Vec<JavaField>,

    /// What methods does this class contain? (non constructor)
    pub methods: Vec<JavaMethod>,

    /// What constructor methods does the class have?
    pub constructors: Vec<JavaMethod>,
}

/// A Field in a [`JavaClass`]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaField {
    /// Name of the field
    pub name: String,

    /// Type of the field
    pub field_type: String,

    /// Visibility: "public", "private", "protected", and ""
    pub visibility: String,

    /// Is the field a static field
    pub is_static: bool,

    /// Is the field final?
    pub is_final: bool,
}

/// A Method defined in a [`JavaClass`]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JavaMethod {
    /// Name of the method
    pub name: String,

    /// What type does the method return
    pub return_type: String,

    /// Visibility: "public", "private", "protected", and ""
    pub visibility: String,

    /// Stores whether or not the method is static
    pub is_static: bool,

    /// Stores wheter or not the method is abstract
    pub is_abstract: bool,

    /// What are the parameters that the method needs to be called with?
    pub parameters: Vec<JavaParameter>,

    /// What other methods does this method invoke?
    pub calls: Vec<MethodCall>,
}

/// A parameter of a [`JavaMethod`]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaParameter {
    /// What is the name of the parameter?
    pub name: String,

    /// What object type is the parameter
    pub param_type: String,
}

/// The invocation of a [`JavaMethod`]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodCall {
    /// What method is calling this method
    pub caller_method: String,

    /// What class is the caller_method in?
    pub caller_class: String,

    /// The name of the method being called
    pub method_name: String,

    /// The name of the class the target method is in
    pub target_class: String,

    /// Is it a static call?
    pub is_static_call: bool,
}

/// Runtime-style information about a declared object reference,
/// collected from variable declarations and parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectInfo {
    /// Name of the variable
    pub variable_name: String,

    /// Object class/type
    pub class_name: String,

    /// Where is the line declared?
    pub declared_at_line: usize,

    /// Is this parameter?
    /// Similar to [`JavaParameter`]
    pub is_parameter: bool,
}

/// A directed relationship between two [`JavaClass`]es.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    /// Class A
    pub from: String,
    /// Class B
    pub to: String,
    /// What relationship does Class A have to Class B
    pub kind: RelationshipType,
}

/// How Class A relates to Class B in a [`Relationship`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RelationshipType {
    /// Class A extends class B
    Extends,
    /// Class A implements interface B
    Implements,
    /// Class A uses class B (e.g. as a field or local type)
    Uses,
    /// Class A's method calls a method on class B
    Calls,
    /// Class A contains an instance of class B
    Contains,
    /// Class A invokes a specific method on class B
    MethodCall,
}
