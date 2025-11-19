use crate::parser::{node_text, walk_tree};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tree_sitter::Node;

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub caller_object: Option<String>,
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
pub struct AnalysisResult {
    pub classes: Vec<JavaClass>,
    pub relationships: Vec<Relationship>,
    pub object_registry: HashMap<String, ObjectInfo>,
    pub type_inference: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub from: String,
    pub to: String,
    pub relationship_type: RelationshipType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationshipType {
    Extends,
    Implements,
    Uses,
    Calls,
    Contains,
}

pub struct JavaAnalyzer {
    current_class: Option<JavaClass>,
    classes: Vec<JavaClass>,
    relationships: Vec<Relationship>,
    object_registry: HashMap<String, ObjectInfo>,
    type_inference: HashMap<String, String>,
    current_method: Option<String>,
}

impl Default for JavaAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl JavaAnalyzer {
    pub fn new() -> Self {
        JavaAnalyzer {
            current_class: None,
            classes: Vec::new(),
            relationships: Vec::new(),
            object_registry: HashMap::new(),
            type_inference: HashMap::new(),
            current_method: None,
        }
    }

    pub fn analyze(&mut self, root_node: &Node, source: &str) -> AnalysisResult {
        self.classes.clear();
        self.relationships.clear();
        self.object_registry.clear();
        self.type_inference.clear();
        self.current_class = None;
        self.current_method = None;

        walk_tree(root_node, source, 0, &mut |node, source, _depth| {
            self.process_node(node, source);
        });

        if let Some(class) = self.current_class.take() {
            self.classes.push(class);
        }

        AnalysisResult {
            classes: self.classes.clone(),
            relationships: self.relationships.clone(),
            object_registry: self.object_registry.clone(),
            type_inference: self.type_inference.clone(),
        }
    }

    fn process_node(&mut self, node: &Node, source: &str) {
        match node.kind() {
            "class_declaration" => self.process_class_declaration(node, source),
            "interface_declaration" => self.process_interface_declaration(node, source),
            "field_declaration" => self.process_field_declaration(node, source),
            "method_declaration" => self.process_method_declaration(node, source),
            "constructor_declaration" => self.process_constructor_declaration(node, source),
            "method_invocation" => self.process_method_invocation(node, source),
            "local_variable_declaration" => self.process_local_variable_declaration(node, source),
            "assignment_expression" => self.process_assignment_expression(node, source),
            _ => {}
        }
    }

    fn process_class_declaration(&mut self, node: &Node, source: &str) {
        if let Some(current_class) = self.current_class.take() {
            self.classes.push(current_class);
        }

        let mut class = JavaClass {
            name: String::new(),
            visibility: "package".to_string(),
            is_abstract: false,
            is_interface: false,
            extends: None,
            implements: Vec::new(),
            fields: Vec::new(),
            methods: Vec::new(),
            constructors: Vec::new(),
        };

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "modifiers" => {
                    class.visibility = self.extract_visibility(&child, source);
                    class.is_abstract = self.has_modifier(&child, source, "abstract");
                }
                "identifier" => {
                    class.name = node_text(&child, source).to_string();
                }
                "superclass" => {
                    if let Some(extends_class) = self.extract_extends(&child, source) {
                        class.extends = Some(extends_class.clone());
                        self.relationships.push(Relationship {
                            from: class.name.clone(),
                            to: extends_class,
                            relationship_type: RelationshipType::Extends,
                        });
                    }
                }
                "super_interfaces" => {
                    let interfaces = self.extract_implements(&child, source);
                    for interface in &interfaces {
                        self.relationships.push(Relationship {
                            from: class.name.clone(),
                            to: interface.clone(),
                            relationship_type: RelationshipType::Implements,
                        });
                    }
                    class.implements = interfaces;
                }
                _ => {}
            }
        }

        self.current_class = Some(class);
    }

    fn process_interface_declaration(&mut self, node: &Node, source: &str) {
        if let Some(current_class) = self.current_class.take() {
            self.classes.push(current_class);
        }

        let mut class = JavaClass {
            name: String::new(),
            visibility: "public".to_string(),
            is_abstract: false,
            is_interface: true,
            extends: None,
            implements: Vec::new(),
            fields: Vec::new(),
            methods: Vec::new(),
            constructors: Vec::new(),
        };

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" {
                class.name = node_text(&child, source).to_string();
                break;
            }
        }

        self.current_class = Some(class);
    }

    fn process_field_declaration(&mut self, node: &Node, source: &str) {
        let field = self.extract_field(node, source);
        if let Some(ref mut class) = self.current_class {
            class.fields.push(field);
        }
    }

    fn process_method_declaration(&mut self, node: &Node, source: &str) {
        if let Some(name_node) = node.child_by_field_name("name") {
            self.current_method = Some(node_text(&name_node, source).to_string());
        }

        let method = self.extract_method(node, source);
        if let Some(ref mut class) = self.current_class {
            class.methods.push(method);
        }

        self.current_method = None;
    }

    fn process_constructor_declaration(&mut self, node: &Node, source: &str) {
        let constructor = self.extract_constructor(node, source);
        if let Some(ref mut class) = self.current_class {
            class.constructors.push(constructor);
        }
    }

    fn process_method_invocation(&mut self, node: &Node, source: &str) {
        // Track method calls for relationship analysis
        if let Some(ref class) = self.current_class {
            let method_call = self.extract_enhanced_method_call(node, source);
            if !method_call.method_name.is_empty() {
                self.relationships.push(Relationship {
                    from: class.name.clone(),
                    to: method_call.target_class.clone(),
                    relationship_type: RelationshipType::Calls,
                });
            }
        }
    }

    fn extract_field(&self, node: &Node, source: &str) -> JavaField {
        let mut field = JavaField {
            name: String::new(),
            field_type: String::new(),
            visibility: "package".to_string(),
            is_static: false,
            is_final: false,
        };

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "modifiers" => {
                    field.visibility = self.extract_visibility(&child, source);
                    field.is_static = self.has_modifier(&child, source, "static");
                    field.is_final = self.has_modifier(&child, source, "final");
                }
                "type" => {
                    field.field_type = self.extract_type(&child, source);
                }
                "variable_declarator" => {
                    if let Some(identifier) = child.child_by_field_name("name") {
                        field.name = node_text(&identifier, source).to_string();
                    }
                }
                _ => {}
            }
        }

        field
    }

    fn extract_method(&self, node: &Node, source: &str) -> JavaMethod {
        let mut method = JavaMethod {
            name: String::new(),
            return_type: "void".to_string(),
            visibility: "package".to_string(),
            is_static: false,
            is_abstract: false,
            parameters: Vec::new(),
            calls: Vec::new(),
        };

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "modifiers" => {
                    method.visibility = self.extract_visibility(&child, source);
                    method.is_static = self.has_modifier(&child, source, "static");
                    method.is_abstract = self.has_modifier(&child, source, "abstract");
                }
                "type" => {
                    method.return_type = self.extract_type(&child, source);
                }
                "identifier" => {
                    method.name = node_text(&child, source).to_string();
                }
                "formal_parameters" => {
                    method.parameters = self.extract_parameters(&child, source);
                }
                "block" => {
                    method.calls = self.extract_method_calls_from_block(&child, source);
                }
                _ => {}
            }
        }

        method
    }

    fn extract_constructor(&self, node: &Node, source: &str) -> JavaMethod {
        let mut constructor = JavaMethod {
            name: String::new(),
            return_type: String::new(),
            visibility: "package".to_string(),
            is_static: false,
            is_abstract: false,
            parameters: Vec::new(),
            calls: Vec::new(),
        };

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "modifiers" => {
                    constructor.visibility = self.extract_visibility(&child, source);
                }
                "identifier" => {
                    constructor.name = node_text(&child, source).to_string();
                }
                "formal_parameters" => {
                    constructor.parameters = self.extract_parameters(&child, source);
                }
                _ => {}
            }
        }

        constructor
    }

    fn extract_visibility(&self, modifiers_node: &Node, source: &str) -> String {
        let mut cursor = modifiers_node.walk();
        for child in modifiers_node.children(&mut cursor) {
            let modifier = node_text(&child, source);
            match modifier {
                "public" | "private" | "protected" => return modifier.to_string(),
                _ => {}
            }
        }
        "package".to_string()
    }

    fn has_modifier(&self, modifiers_node: &Node, source: &str, target_modifier: &str) -> bool {
        let mut cursor = modifiers_node.walk();
        for child in modifiers_node.children(&mut cursor) {
            if node_text(&child, source) == target_modifier {
                return true;
            }
        }
        false
    }

    fn extract_type(&self, type_node: &Node, source: &str) -> String {
        node_text(type_node, source).to_string()
    }

    fn extract_extends(&self, superclass_node: &Node, source: &str) -> Option<String> {
        let mut cursor = superclass_node.walk();
        for child in superclass_node.children(&mut cursor) {
            if child.kind() == "type_identifier" {
                return Some(node_text(&child, source).to_string());
            }
        }
        None
    }

    fn extract_implements(&self, interfaces_node: &Node, source: &str) -> Vec<String> {
        let mut interfaces = Vec::new();
        let mut cursor = interfaces_node.walk();
        for child in interfaces_node.children(&mut cursor).skip(1) {
            interfaces.push(node_text(&child, source).to_string());
        }
        interfaces
    }

    fn extract_parameters(&self, params_node: &Node, source: &str) -> Vec<JavaParameter> {
        let mut parameters = Vec::new();
        let mut cursor = params_node.walk();
        for child in params_node.children(&mut cursor) {
            if child.kind() == "formal_parameter"
                && let Some(param) = self.extract_parameter(&child, source)
            {
                parameters.push(param);
            }
        }
        parameters
    }

    fn extract_parameter(&self, param_node: &Node, source: &str) -> Option<JavaParameter> {
        let mut param_type = String::new();
        let mut param_name = String::new();

        let mut cursor = param_node.walk();
        for child in param_node.children(&mut cursor) {
            match child.kind() {
                "type" => param_type = self.extract_type(&child, source),
                "identifier" => param_name = node_text(&child, source).to_string(),
                _ => {}
            }
        }

        if !param_name.is_empty() && !param_type.is_empty() {
            Some(JavaParameter {
                name: param_name,
                param_type,
            })
        } else {
            None
        }
    }

    fn process_local_variable_declaration(&mut self, node: &Node, source: &str) {
        let mut variable_name = String::new();
        let mut type_name = String::new();
        let line_number = node.start_position().row + 1;

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "type" => {
                    type_name = self.extract_type(&child, source);
                }
                "variable_declarator" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        variable_name = node_text(&name_node, source).to_string();
                    }

                    // Check if this is an object creation
                    if let Some(value_node) = child.child_by_field_name("value")
                        && value_node.kind() == "object_creation_expression"
                            && let Some(class_node) = value_node.child_by_field_name("type") {
                                type_name = self.extract_type(&class_node, source);
                            }
                }
                _ => {}
            }
        }

        if !variable_name.is_empty() && !type_name.is_empty() {
            self.type_inference
                .insert(variable_name.clone(), type_name.clone());

            let object_info = ObjectInfo {
                variable_name: variable_name.clone(),
                class_name: type_name,
                declared_at_line: line_number,
                is_parameter: false,
            };

            self.object_registry.insert(variable_name, object_info);
        }
    }

    fn process_assignment_expression(&mut self, node: &Node, source: &str) {
        if let Some(left) = node.child_by_field_name("left")
            && let Some(right) = node.child_by_field_name("right") {
                let variable_name = node_text(&left, source).to_string();

                // If the right side is an object creation, update type inference
                if right.kind() == "object_creation_expression" {
                    if let Some(type_node) = right.child_by_field_name("type") {
                        let type_name = self.extract_type(&type_node, source);
                        self.type_inference
                            .insert(variable_name.clone(), type_name.clone());

                        let line_number = node.start_position().row + 1;
                        let object_info = ObjectInfo {
                            variable_name: variable_name.clone(),
                            class_name: type_name,
                            declared_at_line: line_number,
                            is_parameter: false,
                        };

                        self.object_registry.insert(variable_name, object_info);
                    }
                }
                // If the right side is a method call, try to infer return type
                else if right.kind() == "method_invocation" {
                    let method_call = self.extract_enhanced_method_call(&right, source);
                    // Try to find the method in our classes to get return type
                    if let Some(return_type) = self
                        .get_method_return_type(&method_call.target_class, &method_call.method_name)
                    {
                        self.type_inference
                            .insert(variable_name.clone(), return_type);
                    }
                }
            }
    }

    fn extract_enhanced_method_call(&self, node: &Node, source: &str) -> MethodCall {
        let mut method_call = MethodCall {
            caller_object: None,
            method_name: String::new(),
            target_class: "unknown".to_string(),
            is_static_call: false,
        };

        // Extract method name
        if let Some(name_node) = node.child_by_field_name("name") {
            method_call.method_name = node_text(&name_node, source).to_string();
        }

        // Extract caller object
        if let Some(object_node) = node.child_by_field_name("object") {
            let object_name = node_text(&object_node, source).to_string();
            method_call.caller_object = Some(object_name.clone());

            // Try to resolve the object's class
            if let Some(class_name) = self.resolve_object_class(&object_name) {
                method_call.target_class = class_name;
            } else {
                // Check if it's a static class call (starts with uppercase)
                if object_name
                    .chars()
                    .next()
                    .map(|c| c.is_uppercase())
                    .unwrap_or(false)
                {
                    method_call.target_class = object_name.clone();
                    method_call.is_static_call = true;
                } else {
                    // For unresolved objects, use the object name as a fallback
                    method_call.target_class = object_name.clone();
                }
            }
        }

        method_call
    }

    fn resolve_object_class(&self, object_name: &str) -> Option<String> {
        // First check type inference
        if let Some(class_name) = self.type_inference.get(object_name) {
            return Some(class_name.clone());
        }

        // Then check object registry
        if let Some(object_info) = self.object_registry.get(object_name) {
            return Some(object_info.class_name.clone());
        }

        // Check if it's a field in current class
        if let Some(ref class) = self.current_class {
            for field in &class.fields {
                if field.name == object_name {
                    return Some(field.field_type.clone());
                }
            }
        }

        None
    }

    fn get_method_return_type(&self, class_name: &str, method_name: &str) -> Option<String> {
        for class in &self.classes {
            if class.name == class_name {
                for method in &class.methods {
                    if method.name == method_name {
                        return Some(method.return_type.clone());
                    }
                }
                // Check constructors too
                for constructor in &class.constructors {
                    if constructor.name == method_name {
                        return Some(class_name.to_string()); // Constructor "returns" the class type
                    }
                }
            }
        }
        None
    }

    fn extract_method_calls_from_block(&self, block_node: &Node, source: &str) -> Vec<MethodCall> {
        let mut calls = Vec::new();

        let mut cursor = block_node.walk();
        for child in block_node.children(&mut cursor) {
            if child.kind() == "expression_statement"
                && let Some(expr) = child.child(0)
                    && expr.kind() == "method_invocation" {
                        calls.push(self.extract_enhanced_method_call(&expr, source));
                    }
        }

        calls
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::JavaParser;

    #[test]
    fn test_implements_extends() {
        let mut parser = JavaParser::new().unwrap();
        let code = r#"
public interface Trainable {}

public abstract class Animal {}

public class Dog extends Animal implements Trainable {}
"#;

        let tree = parser.parse(code).unwrap();
        let root = parser.get_root_node(&tree);

        let mut analyzer = JavaAnalyzer::new();
        let result = analyzer.analyze(&root, code);

        assert_eq!(result.classes[2].implements, vec!["Trainable"]);
        assert_eq!(result.classes[2].extends, Some("Animal".to_string()));
    }
}
