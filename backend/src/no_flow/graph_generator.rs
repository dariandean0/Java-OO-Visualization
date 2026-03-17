use crate::{
    analyzer::AnalysisResult,
    repr::{JavaClass, JavaField, JavaMethod, Relationship, RelationshipType},
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

        dot.push_str(&self.generate_dot_body(analysis));

        dot.push_str("}\n");
        dot
    }

    pub(crate) fn generate_dot_body(&self, analysis: &AnalysisResult) -> String {
        let mut body = String::new();

        for class in &analysis.classes {
            body.push_str(&self.generate_class_node(class));
            body.push('\n');
        }

        if self.config.include_relationships {
            body.push_str(&self.generate_inter_class_relationships(analysis));
        }

        body
    }

    /// Generate a memory-diagram-style node for a class.
    ///
    /// Per the Memory Diagram Guide:
    ///   - Objects (class instances) are represented as circles.
    ///   - Interfaces / static-only classes are represented as diamonds.
    ///   - Private members go inside the shape; public members go on the border.
    ///   - Fields are small rectangles; methods are short lines.
    ///
    /// In DOT we approximate this with HTML labels:
    ///   - Classes use `shape=circle` (ellipse for readability).
    ///   - Interfaces use `shape=diamond`.
    ///   - The HTML label arranges members with visual separation
    ///     between "interior" (private) and "border" (public) sections.
    pub(crate) fn generate_class_node(&self, class: &JavaClass) -> String {
        let mut out = String::new();
        let safe = class.name.replace('.', "_");

        let fields: Vec<&JavaField> = if self.config.show_fields {
            class
                .fields
                .iter()
                .filter(|f| self.config.show_private_members || f.visibility != "private")
                .collect()
        } else {
            vec![]
        };

        let methods: Vec<&JavaMethod> = if self.config.show_methods {
            class
                .methods
                .iter()
                .filter(|m| self.config.show_private_members || m.visibility != "private")
                .collect()
        } else {
            vec![]
        };

        let private_fields: Vec<&&JavaField> = fields
            .iter()
            .filter(|f| f.visibility == "private")
            .collect();
        let public_fields: Vec<&&JavaField> = fields
            .iter()
            .filter(|f| f.visibility != "private")
            .collect();
        let private_methods: Vec<&&JavaMethod> = methods
            .iter()
            .filter(|m| m.visibility == "private")
            .collect();
        let public_methods: Vec<&&JavaMethod> = methods
            .iter()
            .filter(|m| m.visibility != "private")
            .collect();

        let shape = if class.is_interface {
            "diamond"
        } else {
            "circle"
        };

        let label = self.build_html_label(
            class,
            &private_fields,
            &public_fields,
            &private_methods,
            &public_methods,
        );

        let peripheries = if class.fields.iter().all(|f| f.is_final) && !class.fields.is_empty() {
            ", peripheries=2"
        } else {
            ""
        };

        out.push_str(&format!(
            "    \"{safe}_class\" [shape={shape}, label=<{label}>, style=filled, fillcolor=white{peripheries}];\n",
        ));

        out
    }

    /// Build an HTML label that mirrors the memory diagram layout.
    ///
    /// Layout (top-to-bottom inside the shape):
    ///   1. Class name (bold, top)
    ///   2. Public methods  — on the "border" (separated by a line)
    ///   3. Public fields   — on the "border" (separated by a line)
    ///   4. Private fields  — in the "interior"
    ///   5. Private methods — in the "interior"
    fn build_html_label(
        &self,
        class: &JavaClass,
        private_fields: &[&&JavaField],
        public_fields: &[&&JavaField],
        private_methods: &[&&JavaMethod],
        public_methods: &[&&JavaMethod],
    ) -> String {
        let mut html = String::new();

        html.push_str("<TABLE BORDER=\"0\" CELLBORDER=\"0\" CELLSPACING=\"2\" CELLPADDING=\"4\">");

        let class_label = self.get_class_label(class);
        html.push_str(&format!(
            "<TR><TD><B><FONT POINT-SIZE=\"14\">{}</FONT></B></TD></TR>",
            Self::escape_html(&class_label)
        ));

        for method in public_methods {
            let label = self.format_method(method);
            html.push_str(&format!(
                "<TR><TD><U>{}</U></TD></TR>",
                Self::escape_html(&label)
            ));
        }

        for field in public_fields {
            let label = self.format_field_for_diagram(field);
            html.push_str(&format!(
                "<TR><TD BORDER=\"1\" BGCOLOR=\"lightyellow\">{}</TD></TR>",
                Self::escape_html(&label)
            ));
        }

        let has_public = !public_methods.is_empty() || !public_fields.is_empty();
        let has_private = !private_methods.is_empty() || !private_fields.is_empty();
        if has_public && has_private {
            html.push_str("<HR/>");
        }

        for field in private_fields {
            let label = self.format_field_for_diagram(field);
            html.push_str(&format!(
                "<TR><TD BORDER=\"1\" BGCOLOR=\"lightyellow\">{}</TD></TR>",
                Self::escape_html(&label)
            ));
        }

        for method in private_methods {
            let label = self.format_method(method);
            html.push_str(&format!(
                "<TR><TD><U>{}</U></TD></TR>",
                Self::escape_html(&label)
            ));
        }

        html.push_str("</TABLE>");
        html
    }

    pub(crate) fn get_class_label(&self, class: &JavaClass) -> String {
        if class.is_interface {
            format!("{} (interface)", class.name)
        } else if class.is_abstract {
            format!("{} (abstract)", class.name)
        } else {
            class.name.clone()
        }
    }

    pub(crate) fn generate_inter_class_relationships(&self, analysis: &AnalysisResult) -> String {
        let mut relationships = String::new();

        for relationship in &analysis.relationships {
            match relationship.kind {
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
                RelationshipType::Calls if self.config.show_method_calls => {
                    relationships.push_str(&self.generate_method_call_relationship(relationship));
                }
                RelationshipType::MethodCall if self.config.show_method_calls => {
                    relationships.push_str(&self.generate_method_call_relationship(relationship));
                }
                _ => {}
            }
        }

        relationships
    }

    fn generate_method_call_relationship(&self, relationship: &Relationship) -> String {
        let (from_class, from_method) = relationship
            .from
            .split_once('.')
            .unwrap_or((&relationship.from, ""));
        let (to_class, to_method) = relationship
            .to
            .split_once('.')
            .unwrap_or((&relationship.to, ""));

        let from_node = format!("{}_class", from_class);
        let to_node = format!("{}_class", to_class);

        let label = if !from_method.is_empty() && !to_method.is_empty() {
            format!("{} -> {}", from_method, to_method)
        } else if !to_method.is_empty() {
            to_method.to_string()
        } else {
            String::new()
        };

        format!(
            "    \"{}\" -> \"{}\" [arrowhead=normal, style=solid, color=blue, label=\"{}\"];\n",
            from_node, to_node, label
        )
    }

    /// Format a field label for the memory diagram.
    /// Shows: type name  (e.g. "int age" or "String name")
    /// Omits visibility prefix since position (inside vs border) conveys that.
    fn format_field_for_diagram(&self, field: &JavaField) -> String {
        let static_modifier = if field.is_static { "static " } else { "" };
        let final_modifier = if field.is_final { "final " } else { "" };

        let type_str = if self.config.show_field_types {
            format!("{} ", field.field_type)
        } else {
            String::new()
        };

        format!(
            "{}{}{}{}",
            static_modifier, final_modifier, type_str, field.name
        )
    }

    pub(crate) fn format_method(&self, method: &JavaMethod) -> String {
        let vis_prefix = if method.visibility == "package" || method.visibility.is_empty() {
            String::new()
        } else {
            format!("{} ", method.visibility)
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
            vis_prefix, static_modifier, abstract_modifier, method.name, params, method.return_type
        )
    }

    fn escape_html(text: &str) -> String {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    }
}
