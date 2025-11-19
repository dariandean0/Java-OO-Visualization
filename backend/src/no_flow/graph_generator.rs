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
    pub show_method_calls: bool,
    pub cluster_fields: bool,
    pub cluster_methods: bool,
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
            show_method_calls: true,
            cluster_fields: true,
            cluster_methods: true,
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
        dot.push_str("    fontname=\"Arial\";\n");
        dot.push_str("    node [fontname=\"Arial\"];\n");
        dot.push_str("    edge [fontname=\"Arial\", fontsize=10];\n\n");

        // Generate class subgraphs
        for class in &analysis.classes {
            dot.push_str(&self.generate_class_subgraph(class, analysis));
            dot.push('\n');
        }

        // Generate inter-class relationships
        if self.config.include_relationships {
            dot.push_str(&self.generate_inter_class_relationships(analysis));
        }

        dot.push_str("}\n");
        dot
    }

    fn generate_class_subgraph(&self, class: &JavaClass, analysis: &AnalysisResult) -> String {
        let mut subgraph = String::new();
        let safe_class_name = class.name.replace('.', "_");

        // Start subgraph
        subgraph.push_str(&format!("    subgraph cluster_{} {{\n", safe_class_name));
        subgraph.push_str(&format!(
            "        label=\"{}\";\n",
            self.get_class_label(class)
        ));
        subgraph.push_str("        style=filled;\n");
        subgraph.push_str("        color=lightgrey;\n");
        subgraph.push_str("        node [shape=box, style=filled, fillcolor=white];\n\n");

        // Add class header node
        subgraph.push_str(&format!(
            "        \"{}_class\" [label=\"{}\", shape=ellipse, style=filled, fillcolor=lightblue];\n",
            safe_class_name, self.get_class_label(class)
        ));

        // Add field nodes
        if self.config.show_fields && !class.fields.is_empty() {
            if self.config.cluster_fields {
                subgraph.push_str(&format!(
                    "        subgraph cluster_{}_fields {{\n",
                    safe_class_name
                ));
                subgraph.push_str("            label=\"Fields\";\n");
                subgraph.push_str("            style=dashed;\n");
            }

            for field in &class.fields {
                if !self.config.show_private_members && field.visibility == "private" {
                    continue;
                }
                subgraph.push_str(&self.generate_field_node(field, &safe_class_name));
            }

            if self.config.cluster_fields {
                subgraph.push_str("        }\n");
            }
        }

        // Add method nodes
        if self.config.show_methods && !class.methods.is_empty() {
            if self.config.cluster_methods {
                subgraph.push_str(&format!(
                    "        subgraph cluster_{}_methods {{\n",
                    safe_class_name
                ));
                subgraph.push_str("            label=\"Methods\";\n");
                subgraph.push_str("            style=dashed;\n");
            }

            for method in &class.methods {
                if !self.config.show_private_members && method.visibility == "private" {
                    continue;
                }
                subgraph.push_str(&self.generate_method_node(method, &safe_class_name));
            }

            if self.config.cluster_methods {
                subgraph.push_str("        }\n");
            }
        }

        // Add constructor nodes (commented out - not needed for visualization)
        // if self.config.show_constructors && !class.constructors.is_empty() {
        //     for constructor in &class.constructors {
        //         if !self.config.show_private_members && constructor.visibility == "private" {
        //             continue;
        //         }
        //         subgraph.push_str(&self.generate_constructor_node(constructor, &safe_class_name));
        //     }
        // }

        // Add internal connections (methods accessing fields)
        subgraph.push_str(&self.generate_internal_connections(class, &safe_class_name));

        subgraph.push_str("    }\n");
        subgraph
    }

    fn get_class_label(&self, class: &JavaClass) -> String {
        if class.is_interface {
            format!("{} (interface)", class.name)
        } else if class.is_abstract {
            format!("{} (abstract)", class.name)
        } else {
            class.name.clone()
        }
    }

    fn generate_field_node(&self, field: &JavaField, class_name: &str) -> String {
        let field_id = format!("{}_{}", class_name, field.name);
        let label = self.format_field(field);

        format!(
            "        \"{}\" [label=\"{}\", shape=note, style=filled, fillcolor=lightyellow];\n",
            field_id, label
        )
    }

    fn generate_method_node(&self, method: &JavaMethod, class_name: &str) -> String {
        let method_id = format!("{}_{}", class_name, method.name);
        let label = self.format_method(method);

        let color = match method.visibility.as_str() {
            "public" => "lightgreen",
            "private" => "lightcoral",
            "protected" => "lightorange",
            _ => "lightgray",
        };

        format!(
            "        \"{}\" [label=\"{}\", shape=component, style=filled, fillcolor={}];\n",
            method_id, label, color
        )
    }

    fn generate_constructor_node(&self, constructor: &JavaMethod, class_name: &str) -> String {
        let constructor_id = format!("{}_constructor", class_name);
        let label = self.format_constructor(constructor);

        format!(
            "        \"{}\" [label=\"{}\", shape=house, style=filled, fillcolor=lightcyan];\n",
            constructor_id, label
        )
    }

    fn generate_internal_connections(&self, class: &JavaClass, class_name: &str) -> String {
        let mut connections = String::new();

        // Connect class to its fields and methods
        for field in &class.fields {
            if !self.config.show_private_members && field.visibility == "private" {
                continue;
            }
            let field_id = format!("{}_{}", class_name, field.name);
            connections.push_str(&format!(
                "        \"{}_class\" -> \"{}\" [style=dashed, arrowhead=none];\n",
                class_name, field_id
            ));
        }

        for method in &class.methods {
            if !self.config.show_private_members && method.visibility == "private" {
                continue;
            }
            let method_id = format!("{}_{}", class_name, method.name);
            connections.push_str(&format!(
                "        \"{}_class\" -> \"{}\" [style=dashed, arrowhead=none];\n",
                class_name, method_id
            ));
        }

        // Constructors removed - not needed for visualization

        connections
    }

    fn generate_inter_class_relationships(&self, analysis: &AnalysisResult) -> String {
        let mut relationships = String::new();

        for relationship in &analysis.relationships {
            match relationship.relationship_type {
                RelationshipType::Extends => {
                    relationships.push_str(&format!(
                        "    \"{}_class\" -> \"{}_class\" [arrowhead=empty, style=solid, label=extends];\n",
                        relationship.from.replace('.', "_"),
                        relationship.to.replace('.', "_")
                    ));
                }
                RelationshipType::Implements => {
                    relationships.push_str(&format!(
                        "    \"{}_class\" -> \"{}_class\" [arrowhead=empty, style=dashed, label=implements];\n",
                        relationship.from.replace('.', "_"),
                        relationship.to.replace('.', "_")
                    ));
                }
                RelationshipType::Calls => {
                    if self.config.show_method_calls {
                        relationships
                            .push_str(&self.generate_legacy_method_call_relationship(relationship));
                    }
                }
                RelationshipType::MethodCall => {
                    if self.config.show_method_calls {
                        relationships
                            .push_str(&self.generate_method_call_relationship(relationship));
                    }
                }
                _ => {}
            }
        }

        relationships
    }

    fn generate_method_call_relationship(&self, relationship: &Relationship) -> String {
        // For method calls, connect specific method nodes directly
        let from_clean = relationship.from.replace('.', "_");
        let to_clean = relationship.to.replace('.', "_");

        // Create direct method-to-method connections
        format!(
            "    \"{}\" -> \"{}\" [arrowhead=normal, style=solid, color=blue];\n",
            from_clean, to_clean
        )
    }

    fn generate_legacy_method_call_relationship(&self, relationship: &Relationship) -> String {
        // For legacy method calls (class-to-class)
        let from_clean = relationship.from.replace('.', "_");
        let to_clean = relationship.to.replace('.', "_");

        format!(
            "    \"{}\" -> \"{}\" [arrowhead=normal, style=solid, color=blue];\n",
            from_clean, to_clean
        )
    }

    fn format_field(&self, field: &JavaField) -> String {
        let visibility = if field.visibility == "package" {
            ""
        } else {
            &field.visibility
        };
        let static_modifier = if field.is_static { "static " } else { "" };
        let final_modifier = if field.is_final { "final " } else { "" };

        let type_str = if self.config.show_field_types {
            format!("{} ", field.field_type)
        } else {
            String::new()
        };

        format!(
            "{}{}{}{}{}",
            visibility, static_modifier, final_modifier, type_str, field.name
        )
    }

    fn format_method(&self, method: &JavaMethod) -> String {
        let visibility = if method.visibility == "package" {
            ""
        } else {
            &method.visibility
        };
        let static_modifier = if method.is_static { "static " } else { "" };
        let abstract_modifier = if method.is_abstract { "abstract " } else { "" };

        let params = if self.config.show_method_parameters {
            let param_strs: Vec<String> = method
                .parameters
                .iter()
                .map(|p| {
                    if self.config.show_field_types {
                        format!("{} {}", p.param_type, p.name)
                    } else {
                        p.name.clone()
                    }
                })
                .collect();
            format!("({})", param_strs.join(", "))
        } else {
            "()".to_string()
        };

        format!(
            "{}{}{}{}{}: {}",
            visibility, static_modifier, abstract_modifier, method.name, params, method.return_type
        )
    }

    fn format_constructor(&self, constructor: &JavaMethod) -> String {
        let visibility = if constructor.visibility == "package" {
            ""
        } else {
            &constructor.visibility
        };

        let params = if self.config.show_method_parameters {
            let param_strs: Vec<String> = constructor
                .parameters
                .iter()
                .map(|p| {
                    if self.config.show_field_types {
                        format!("{} {}", p.param_type, p.name)
                    } else {
                        p.name.clone()
                    }
                })
                .collect();
            format!("({})", param_strs.join(", "))
        } else {
            "()".to_string()
        };

        format!("{}{}", visibility, params)
    }

    fn escape_label(&self, label: &str) -> String {
        label.replace('"', "\\\"")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::{JavaClass, JavaField, JavaMethod};

    #[test]
    fn test_simple_dot_generation() {
        let class = JavaClass {
            name: "TestClass".to_string(),
            visibility: "public".to_string(),
            is_abstract: false,
            is_interface: false,
            extends: None,
            implements: Vec::new(),
            fields: vec![],
            methods: vec![],
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
        assert!(dot.contains("subgraph cluster_TestClass"));
        assert!(dot.contains("TestClass_class"));
    }

    #[test]
    fn test_class_with_fields_and_methods() {
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
                parameters: vec![],
                calls: vec![],
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

        assert!(dot.contains("TestClass_name"));
        assert!(dot.contains("TestClass_getName"));
        assert!(dot.contains("shape=note"));
        assert!(dot.contains("shape=component"));
    }
}
