use crate::analyzer::{
    AnalysisResult, JavaClass, JavaField, JavaMethod, Relationship, RelationshipType,
};

pub struct GraphGenerator {
    config: GraphConfig,
}

#[derive(Debug, Clone)]
pub struct GraphConfig {
    pub show_fields: bool,
    pub show_methods: bool,
    pub show_constructors: bool,
    pub show_method_parameters: bool,
    pub show_field_types: bool,
    pub show_private_members: bool,
    pub include_relationships: bool,
}

impl Default for GraphConfig {
    fn default() -> Self {
        GraphConfig {
            show_fields: true,
            show_methods: true,
            show_constructors: true,
            show_method_parameters: true,
            show_field_types: true,
            show_private_members: true,
            include_relationships: true,
        }
    }
}

impl Default for GraphGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphGenerator {
    pub fn new() -> Self {
        GraphGenerator {
            config: GraphConfig::default(),
        }
    }

    pub fn with_config(config: GraphConfig) -> Self {
        GraphGenerator { config }
    }

    pub fn generate_dot(&self, analysis: &AnalysisResult) -> String {
        let mut dot = String::new();

        dot.push_str("digraph JavaClasses {\n");
        dot.push_str("    rankdir=TB;\n");
        dot.push_str("    node [shape=record, fontname=\"Arial\"];\n");
        dot.push_str("    edge [fontname=\"Arial\", fontsize=10];\n\n");

        // Generate class nodes
        for class in &analysis.classes {
            dot.push_str(&self.generate_class_node(class));
            dot.push('\n');
        }

        // Generate relationships
        if self.config.include_relationships {
            for relationship in &analysis.relationships {
                dot.push_str(&self.generate_relationship_edge(relationship));
                dot.push('\n');
            }
        }

        dot.push_str("}\n");
        dot
    }

    fn generate_class_node(&self, class: &JavaClass) -> String {
        let mut label = String::new();

        // Class header
        let class_header = if class.is_interface {
            format!("{} (interface)", class.name)
        } else if class.is_abstract {
            format!("{} (abstract)", class.name)
        } else {
            class.name.clone()
        };

        label.push_str(&self.escape_label(&class_header));

        // Fields section
        if self.config.show_fields && !class.fields.is_empty() {
            label.push('|');
            let mut field_parts = Vec::new();

            for field in &class.fields {
                if !self.config.show_private_members && field.visibility == "private" {
                    continue;
                }
                field_parts.push(self.format_field(field));
            }

            if !field_parts.is_empty() {
                label.push_str(&field_parts.join("\\l"));
                label.push_str("\\l");
            }
        }

        // Methods section
        if self.config.show_methods
            && (!class.methods.is_empty()
                || (self.config.show_constructors && !class.constructors.is_empty()))
        {
            label.push('|');
            let mut method_parts = Vec::new();

            // Add constructors
            if self.config.show_constructors {
                for constructor in &class.constructors {
                    if !self.config.show_private_members && constructor.visibility == "private" {
                        continue;
                    }
                    method_parts.push(self.format_method(constructor, true));
                }
            }

            // Add methods
            for method in &class.methods {
                if !self.config.show_private_members && method.visibility == "private" {
                    continue;
                }
                method_parts.push(self.format_method(method, false));
            }

            if !method_parts.is_empty() {
                label.push_str(&method_parts.join("\\l"));
                label.push_str("\\l");
            }
        }

        // Generate the node with styling
        let node_style = self.get_node_style(class);
        format!(
            "    {} [label=\"{}\", {}];",
            self.sanitize_name(&class.name),
            label,
            node_style
        )
    }

    fn generate_class_with_fields(&self, class: &JavaClass) -> String {
        let mut label = String::new();

        // Class header
        let class_header = if class.is_interface {
            format!("{} (interface)", class.name)
        } else if class.is_abstract {
            format!("{} (abstract)", class.name)
        } else {
            class.name.clone()
        };

        label.push_str(&self.escape_label(&class_header));

        // Fields section only
        if self.config.show_fields && !class.fields.is_empty() {
            label.push('|');
            let mut field_parts = Vec::new();

            for field in &class.fields {
                if !self.config.show_private_members && field.visibility == "private" {
                    continue;
                }
                field_parts.push(self.format_field(field));
            }

            if !field_parts.is_empty() {
                label.push_str(&field_parts.join("\\l"));
                label.push_str("\\l");
            }
        }

        let node_style = self.get_node_style(class);
        format!(
            "    {} [label=\"{}\", {}];",
            self.sanitize_name(&class.name),
            label,
            node_style
        )
    }

    fn format_field(&self, field: &JavaField) -> String {
        let visibility_symbol = self.get_visibility_symbol(&field.visibility);
        let modifiers = self.get_field_modifiers(field);

        if self.config.show_field_types {
            format!(
                "{}{} {}: {}",
                visibility_symbol, modifiers, field.name, field.field_type
            )
        } else {
            format!("{}{} {}", visibility_symbol, modifiers, field.name)
        }
    }

    fn format_method(&self, method: &JavaMethod, is_constructor: bool) -> String {
        let visibility_symbol = self.get_visibility_symbol(&method.visibility);
        let modifiers = self.get_method_modifiers(method);

        let params = if self.config.show_method_parameters {
            let param_strings: Vec<String> = method
                .parameters
                .iter()
                .map(|p| format!("{}: {}", p.name, p.param_type))
                .collect();
            format!("({})", param_strings.join(", "))
        } else {
            "()".to_string()
        };

        if is_constructor {
            format!(
                "{}{}{}{}",
                visibility_symbol, modifiers, method.name, params
            )
        } else {
            format!(
                "{}{}{}{}: {}",
                visibility_symbol, modifiers, method.name, params, method.return_type
            )
        }
    }

    fn get_visibility_symbol(&self, visibility: &str) -> &str {
        match visibility {
            "public" => "+ ",
            "private" => "- ",
            "protected" => "# ",
            _ => "~ ", // package-private
        }
    }

    fn get_field_modifiers(&self, field: &JavaField) -> String {
        let mut modifiers = String::new();
        if field.is_static {
            modifiers.push_str("(static) ");
        }
        if field.is_final {
            modifiers.push_str("(final) ");
        }
        modifiers
    }

    fn get_method_modifiers(&self, method: &JavaMethod) -> String {
        let mut modifiers = String::new();
        if method.is_static {
            modifiers.push_str("(static) ");
        }
        if method.is_abstract {
            modifiers.push_str("(abstract) ");
        }
        modifiers
    }

    fn get_node_style(&self, class: &JavaClass) -> String {
        if class.is_interface {
            "fillcolor=lightblue, style=filled".to_string()
        } else if class.is_abstract {
            "fillcolor=lightyellow, style=filled".to_string()
        } else {
            "fillcolor=lightgreen, style=filled".to_string()
        }
    }

    fn generate_relationship_edge(&self, relationship: &Relationship) -> String {
        let (arrow_style, label, color) = match relationship.relationship_type {
            RelationshipType::Extends => ("arrowhead=empty", "extends", "blue"),
            RelationshipType::Implements => {
                ("arrowhead=empty, style=dashed", "implements", "green")
            }
            RelationshipType::Uses => ("arrowhead=open", "uses", "gray"),
            RelationshipType::Calls => ("arrowhead=open, style=dashed", "calls", "orange"),
            RelationshipType::Contains => ("arrowhead=diamond", "contains", "purple"),
        };

        format!(
            "    {} -> {} [{}, label=\"{}\", color={}];",
            self.sanitize_name(&relationship.from),
            self.sanitize_name(&relationship.to),
            arrow_style,
            label,
            color
        )
    }

    fn sanitize_name(&self, name: &str) -> String {
        name.replace([' ', '.', '<', '>', '[', ']'], "_")
    }

    fn escape_label(&self, text: &str) -> String {
        text.replace('\"', "\\\"")
            .replace('{', "\\{")
            .replace('}', "\\}")
            .replace('|', "\\|")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::{JavaClass, JavaField, JavaMethod, JavaParameter};

    #[test]
    fn test_simple_dot_generation() {
        let class = JavaClass {
            name: "TestClass".to_string(),
            visibility: "public".to_string(),
            is_abstract: false,
            is_interface: false,
            extends: None,
            implements: Vec::new(),
            fields: vec![JavaField {
                name: "name".to_string(),
                field_type: "String".to_string(),
                visibility: "private".to_string(),
                is_static: false,
                is_final: false,
            }],
            methods: vec![JavaMethod {
                name: "getName".to_string(),
                return_type: "String".to_string(),
                visibility: "public".to_string(),
                is_static: false,
                is_abstract: false,
                parameters: Vec::new(),
                calls: Vec::new(),
            }],
            constructors: Vec::new(),
        };

        let analysis = AnalysisResult {
            classes: vec![class],
            relationships: Vec::new(),
            object_registry: std::collections::HashMap::new(),
            type_inference: std::collections::HashMap::new(),
        };

        let generator = GraphGenerator::new();
        let dot = generator.generate_dot(&analysis);

        assert!(dot.contains("digraph JavaClasses"));
        assert!(dot.contains("TestClass"));
        assert!(dot.contains("name: String"));
        assert!(dot.contains("getName(): String"));
    }
}
