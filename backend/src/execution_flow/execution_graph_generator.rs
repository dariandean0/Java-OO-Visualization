use super::{
    ExecutionFlow,
    execution_analyzer::{ExecutionAction, ExecutionStep},
};
use crate::{analyzer::AnalysisResult, no_flow, repr::RelationshipType};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct ExecutionGraphConfig {
    pub show_call_stack: bool,
    pub show_object_states: bool,
}

impl Default for ExecutionGraphConfig {
    fn default() -> Self {
        ExecutionGraphConfig {
            show_call_stack: true,
            show_object_states: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VisibilityState {
    pub visible_classes: HashSet<String>,
    pub visible_fields: HashSet<(String, String)>,
    pub visible_methods: HashSet<(String, String)>,
    pub runtime_values: HashMap<(String, String), String>,
}

impl Default for VisibilityState {
    fn default() -> Self {
        Self::new()
    }
}

impl VisibilityState {
    pub fn new() -> Self {
        VisibilityState {
            visible_classes: HashSet::new(),
            visible_fields: HashSet::new(),
            visible_methods: HashSet::new(),
            runtime_values: HashMap::new(),
        }
    }

    pub fn update(&mut self, step: &ExecutionStep) {
        match &step.action {
            ExecutionAction::ObjectCreation { class_name, .. } => {
                self.visible_classes.insert(class_name.clone());
            }
            ExecutionAction::MethodCall {
                target_class,
                method_name,
                ..
            } => {
                self.visible_classes.insert(target_class.clone());
                self.visible_methods
                    .insert((target_class.clone(), method_name.clone()));
            }
            ExecutionAction::MethodEntry { .. } | ExecutionAction::MethodExit { .. } => {}
            ExecutionAction::FieldAccess {
                class_name,
                field_name,
                value,
            } => {
                self.visible_fields
                    .insert((class_name.clone(), field_name.clone()));
                if let Some(val) = value {
                    self.runtime_values
                        .insert((class_name.clone(), field_name.clone()), val.clone());
                }
            }
            ExecutionAction::FieldMutation {
                class_name,
                field_name,
                new_value,
                ..
            } => {
                self.visible_fields
                    .insert((class_name.clone(), field_name.clone()));
                self.runtime_values
                    .insert((class_name.clone(), field_name.clone()), new_value.clone());
            }
            ExecutionAction::VariableAssignment { .. }
            | ExecutionAction::MethodReturn { .. }
            | ExecutionAction::ConditionalBranch { .. }
            | ExecutionAction::LoopIteration { .. } => {}
        }
    }

    pub fn build_filtered_analysis(&self, full_analysis: &AnalysisResult) -> AnalysisResult {
        let mut result = full_analysis.clone();

        result
            .classes
            .retain(|c| self.visible_classes.contains(&c.name));

        for class in &mut result.classes {
            let class_name = class.name.clone();

            class.fields.retain(|f| {
                self.visible_fields
                    .contains(&(class_name.clone(), f.name.clone()))
            });

            for field in &mut class.fields {
                if let Some(val) = self
                    .runtime_values
                    .get(&(class_name.clone(), field.name.clone()))
                {
                    field.name = format!("{} = {}", field.name, val);
                }
            }

            class.methods.retain(|m| {
                self.visible_methods
                    .contains(&(class_name.clone(), m.name.clone()))
            });
        }

        result.relationships.retain(|r| match r.kind {
            RelationshipType::MethodCall | RelationshipType::Calls => {
                let from_visible = if let Some((cls, meth)) = r.from.split_once('.') {
                    self.visible_classes.contains(cls)
                        && self
                            .visible_methods
                            .contains(&(cls.to_string(), meth.to_string()))
                } else {
                    self.visible_classes.contains(&r.from)
                };
                let to_visible = if let Some((cls, meth)) = r.to.split_once('.') {
                    self.visible_classes.contains(cls)
                        && self
                            .visible_methods
                            .contains(&(cls.to_string(), meth.to_string()))
                } else {
                    self.visible_classes.contains(&r.to)
                };
                from_visible && to_visible
            }
            _ => self.visible_classes.contains(&r.from) && self.visible_classes.contains(&r.to),
        });

        result
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionGraphStep {
    pub step_number: usize,
    pub description: String,
    pub dot_code: String,
    pub execution_state: ExecutionState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionState {
    pub active_method: String,
    pub call_stack_depth: usize,
    pub objects_created: usize,
    pub method_calls_made: usize,
}

pub struct ExecutionGraphGenerator {
    config: ExecutionGraphConfig,
}

impl Default for ExecutionGraphGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutionGraphGenerator {
    pub fn new() -> Self {
        ExecutionGraphGenerator {
            config: ExecutionGraphConfig::default(),
        }
    }

    pub fn with_config(config: ExecutionGraphConfig) -> Self {
        ExecutionGraphGenerator { config }
    }

    pub fn generate_execution_graphs(
        &self,
        flow: &ExecutionFlow,
        analysis: &AnalysisResult,
    ) -> Vec<ExecutionGraphStep> {
        let mut graphs = Vec::new();
        let mut visibility_state = VisibilityState::new();
        let mut cumulative_steps = Vec::new();

        for (i, step) in flow.steps.iter().enumerate() {
            // Update visibility state
            visibility_state.update(step);
            cumulative_steps.push(step.clone());

            // Build filtered analysis for class diagram
            let filtered = visibility_state.build_filtered_analysis(analysis);

            // Generate class diagram DOT body using no_flow generator
            let no_flow_gen = no_flow::GraphGenerator::new();
            let class_diagram_content = no_flow_gen.generate_dot_body(&filtered);

            // Build composed DOT
            let dot_code =
                self.build_composed_dot(i + 1, &class_diagram_content, &cumulative_steps, step);

            let execution_state = self.calculate_execution_state(&cumulative_steps);

            graphs.push(ExecutionGraphStep {
                step_number: i + 1,
                description: step.description.clone(),
                dot_code,
                execution_state,
            });
        }

        graphs
    }

    fn build_composed_dot(
        &self,
        step_number: usize,
        class_diagram_content: &str,
        steps: &[ExecutionStep],
        current_step: &ExecutionStep,
    ) -> String {
        let mut dot = String::new();

        dot.push_str(&format!("digraph ExecutionStep_{} {{\n", step_number));
        dot.push_str("    rankdir=TB;\n");
        dot.push_str("    fontname=\"Arial\";\n");
        dot.push_str("    node [fontname=\"Arial\"];\n");
        dot.push_str("    edge [fontname=\"Arial\", fontsize=10];\n");
        dot.push_str("    compound=true;\n\n");
        dot.push_str(&format!(
            "    label=\"Step {} | {}\";\n",
            step_number,
            self.escape_label(&current_step.source_line)
        ));
        dot.push_str("    labelloc=top;\n");
        dot.push_str("    fontsize=16;\n\n");

        if !class_diagram_content.trim().is_empty() {
            let highlighted = self.apply_highlights(class_diagram_content, current_step);
            dot.push_str(&highlighted);
            dot.push('\n');
        }

        // Supplementary panels
        if self.config.show_call_stack && !steps.is_empty() {
            dot.push_str(&self.generate_call_stack_subgraph(steps.last().unwrap()));
        }

        if self.config.show_object_states {
            dot.push_str(&self.generate_object_state_subgraph(steps));
        }

        dot.push_str("}\n");
        dot
    }

    fn apply_highlights(&self, dot_content: &str, current_step: &ExecutionStep) -> String {
        let mut highlight_classes: HashSet<String> = HashSet::new();
        let mut highlight_fields: HashSet<String> = HashSet::new();

        for entry in &current_step.call_stack {
            if let Some((class, _method)) = entry.split_once('.') {
                highlight_classes.insert(class.to_string());
            }
        }

        match &current_step.action {
            ExecutionAction::FieldAccess {
                class_name,
                field_name,
                ..
            }
            | ExecutionAction::FieldMutation {
                class_name,
                field_name,
                ..
            } => {
                highlight_classes.insert(class_name.clone());
                highlight_fields.insert(field_name.clone());
            }
            _ => {}
        }

        let mut result = dot_content.to_string();

        for class_name in &highlight_classes {
            let node_prefix = format!("\"{}_class\"", class_name);
            let lines: Vec<&str> = result.lines().collect();
            let mut new_lines = Vec::new();
            for line in lines {
                if line.contains(&node_prefix) && line.contains("shape=") {
                    let modified = line.replace("fillcolor=white", "fillcolor=lightyellow");
                    new_lines.push(modified);
                } else {
                    new_lines.push(line.to_string());
                }
            }
            result = new_lines.join("\n");
        }

        for field_name in &highlight_fields {
            result = result.replace(
                &format!("BGCOLOR=\"lightyellow\">{field_name}"),
                &format!("BGCOLOR=\"gold\">{field_name}"),
            );
        }

        result
    }

    fn generate_call_stack_subgraph(&self, current_step: &ExecutionStep) -> String {
        let mut subgraph = String::new();

        subgraph.push_str("    subgraph cluster_callstack {\n");
        subgraph.push_str("        label=\"Call Stack\";\n");
        subgraph.push_str("        style=filled;\n");
        subgraph.push_str("        fillcolor=lightblue;\n");

        if current_step.call_stack.is_empty() {
            subgraph.push_str("        empty_stack [label=\"(empty)\", style=dashed];\n");
        } else {
            for (i, method) in current_step.call_stack.iter().enumerate() {
                let node_id = format!("stack_{}", i);
                let style = if i == current_step.call_stack.len() - 1 {
                    // yellow for last call in call stack
                    "filled, fillcolor=yellow"
                } else {
                    "filled, fillcolor=white"
                };

                subgraph.push_str(&format!(
                    "        {} [label=\"{}\", style=\"{}\"];\n",
                    node_id,
                    self.escape_label(method),
                    style
                ));

                if i > 0 {
                    subgraph.push_str(&format!(
                        "        stack_{} -> {} [style=dashed];\n",
                        i - 1,
                        node_id
                    ));
                }
            }
        }

        subgraph.push_str("    }\n\n");
        subgraph
    }

    /// Java primitive types that should render as value boxes, not object references.
    const PRIMITIVE_TYPES: &'static [&'static str] = &[
        "int", "long", "short", "byte", "float", "double", "char", "boolean",
    ];

    fn is_primitive_type(type_name: &str) -> bool {
        Self::PRIMITIVE_TYPES.contains(&type_name)
    }

    /// Render active objects and primitives using memory diagram notation:
    /// - Primitive variable = labeled box containing its value
    /// - Object variable = box with reference arrow to class ellipse
    /// - Class ellipse shows field names and their current runtime values
    fn generate_object_state_subgraph(&self, steps: &[ExecutionStep]) -> String {
        let mut subgraph = String::new();

        // Collect object variables: var_name -> class_name
        let mut active_objects: HashMap<String, String> = HashMap::new();
        // Collect primitive variables: var_name -> (type, current_value)
        let mut primitives: HashMap<String, (String, String)> = HashMap::new();
        // Collect per-object field values: (var_name, field_name) -> value
        let mut object_fields: HashMap<String, Vec<(String, String)>> = HashMap::new();

        for step in steps {
            match &step.action {
                ExecutionAction::ObjectCreation {
                    variable_name,
                    class_name,
                    ..
                } => {
                    active_objects.insert(variable_name.clone(), class_name.clone());
                }
                ExecutionAction::VariableAssignment {
                    variable_name,
                    value_type,
                    value,
                } => {
                    // Only track primitive declarations in main scope (not "declared" or "assigned")
                    if Self::is_primitive_type(value_type) && value != "declared" {
                        primitives
                            .insert(variable_name.clone(), (value_type.clone(), value.clone()));
                    }
                }
                ExecutionAction::FieldMutation {
                    class_name,
                    field_name,
                    new_value,
                    ..
                } => {
                    // Find which variable(s) hold this class and update their field table.
                    // Multiple variables of the same class each get their own field state,
                    // but we only know the class name here, so update all instances.
                    for (var_name, cls) in &active_objects {
                        if cls == class_name {
                            let fields = object_fields.entry(var_name.clone()).or_default();
                            // Update existing or insert new
                            if let Some(entry) = fields.iter_mut().find(|(n, _)| n == field_name) {
                                entry.1 = new_value.clone();
                            } else {
                                fields.push((field_name.clone(), new_value.clone()));
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        if active_objects.is_empty() && primitives.is_empty() {
            return subgraph;
        }

        subgraph.push_str("    subgraph cluster_objects {\n");
        subgraph.push_str("        label=\"Active Objects\";\n");
        subgraph.push_str("        style=filled;\n");
        subgraph.push_str("        fillcolor=\"#f0f0f0\";\n");

        // Render object variables
        for (var_name, class_name) in &active_objects {
            let var_id = format!("var_{}", self.sanitize_name(var_name));
            let obj_id = format!("obj_{}", self.sanitize_name(var_name));

            // Variable reference box
            subgraph.push_str(&format!(
                "        {var_id} [label=<\
                <TABLE BORDER=\"0\" CELLBORDER=\"0\" CELLSPACING=\"0\" CELLPADDING=\"2\">\
                <TR><TD><FONT POINT-SIZE=\"10\">{class_name} {var_name}</FONT></TD></TR>\
                <TR><TD BORDER=\"1\" WIDTH=\"60\" HEIGHT=\"20\"> </TD></TR>\
                </TABLE>>, shape=none];\n",
                var_name = self.escape_html(var_name),
                class_name = self.escape_html(class_name),
            ));

            // Object ellipse with class name and field values
            let fields = object_fields.get(var_name);
            let field_rows = fields
                .map(|flds| {
                    flds.iter()
                        .map(|(name, val)| {
                            format!(
                                "<TR><TD ALIGN=\"LEFT\" BORDER=\"1\" BGCOLOR=\"lightyellow\">{} = {}</TD></TR>",
                                self.escape_html(name),
                                self.escape_html(val),
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("")
                })
                .unwrap_or_default();

            subgraph.push_str(&format!(
                "        {obj_id} [label=<\
                <TABLE BORDER=\"0\" CELLBORDER=\"0\" CELLSPACING=\"0\" CELLPADDING=\"4\">\
                <TR><TD><B>{class_name}</B></TD></TR>\
                {field_rows}\
                </TABLE>>, shape=ellipse, style=filled, fillcolor=white];\n",
                class_name = self.escape_html(class_name),
            ));

            subgraph.push_str(&format!(
                "        {var_id} -> {obj_id} [arrowhead=normal];\n",
            ));
        }

        // Render primitive variables as value boxes
        for (var_name, (type_name, value)) in &primitives {
            let var_id = format!("prim_{}", self.sanitize_name(var_name));

            subgraph.push_str(&format!(
                "        {var_id} [label=<\
                <TABLE BORDER=\"0\" CELLBORDER=\"0\" CELLSPACING=\"0\" CELLPADDING=\"2\">\
                <TR><TD><FONT POINT-SIZE=\"10\">{type_name} {var_name}</FONT></TD></TR>\
                <TR><TD BORDER=\"1\" WIDTH=\"60\" HEIGHT=\"25\" BGCOLOR=\"white\">{value}</TD></TR>\
                </TABLE>>, shape=none];\n",
                var_name = self.escape_html(var_name),
                type_name = self.escape_html(type_name),
                value = self.escape_html(value),
            ));
        }

        subgraph.push_str("    }\n\n");

        subgraph
    }

    fn calculate_execution_state(&self, steps: &[ExecutionStep]) -> ExecutionState {
        let mut objects_created = 0;
        let mut method_calls_made = 0;
        let mut max_stack_depth = 0;
        let mut current_method = "main".to_string();

        for step in steps {
            match &step.action {
                ExecutionAction::ObjectCreation { .. } => objects_created += 1,
                ExecutionAction::MethodCall { .. } => method_calls_made += 1,
                _ => {}
            }

            max_stack_depth = max_stack_depth.max(step.call_stack.len());

            if let Some(method) = step.call_stack.last() {
                current_method = method.clone();
            }
        }

        ExecutionState {
            active_method: current_method,
            call_stack_depth: max_stack_depth,
            objects_created,
            method_calls_made,
        }
    }

    fn sanitize_name(&self, name: &str) -> String {
        name.replace([' ', '.', '<', '>', '[', ']', '(', ')', '-', '+'], "_")
    }

    fn escape_label(&self, text: &str) -> String {
        text.replace('\"', "\\\"")
            .replace('{', "\\{")
            .replace('}', "\\}")
            .replace('|', "\\|")
    }

    fn escape_html(&self, text: &str) -> String {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    }
}

#[cfg(test)]
mod generator_tests {
    use super::super::execution_analyzer::{ExecutionAction, ExecutionStep};
    use super::*;
    use crate::analyzer::AnalysisResult;
    use crate::repr::{JavaClass, JavaField, JavaMethod, Relationship, RelationshipType};

    #[test]
    fn execution_graph_generation() {
        let steps = vec![ExecutionStep {
            step_number: 1,
            line_number: 3,
            source_line: "Calculator calc = new Calculator();".to_string(),
            action: ExecutionAction::ObjectCreation {
                variable_name: "calc".to_string(),
                class_name: "Calculator".to_string(),
                constructor_params: vec![],
            },
            call_stack: vec!["main".to_string()],
            active_objects: vec!["calc".to_string()],
            description: "Create Calculator object".to_string(),
        }];

        let flow = ExecutionFlow {
            steps,
            call_graph: HashMap::new(),
            object_lifecycle: HashMap::new(),
            max_call_stack_depth: 1,
        };

        let analysis = AnalysisResult {
            classes: vec![JavaClass {
                name: "Calculator".to_string(),
                visibility: "public".to_string(),
                is_abstract: false,
                is_interface: false,
                extends: None,
                implements: vec![],
                fields: vec![],
                methods: vec![],
                constructors: vec![],
            }],
            relationships: vec![],
            object_registry: HashMap::new(),
            type_inference: HashMap::new(),
        };

        let generator = ExecutionGraphGenerator::new();
        let graphs = generator.generate_execution_graphs(&flow, &analysis);

        assert_eq!(graphs.len(), 1);
        assert!(graphs[0].dot_code.contains("ExecutionStep_1"));
        assert!(graphs[0].dot_code.contains("Calculator"));
    }

    // -- Helper builders for VisibilityState tests --

    fn make_step(action: ExecutionAction) -> ExecutionStep {
        ExecutionStep {
            step_number: 1,
            line_number: 1,
            source_line: String::new(),
            action,
            call_stack: vec!["main".to_string()],
            active_objects: vec![],
            description: String::new(),
        }
    }

    fn sample_analysis() -> AnalysisResult {
        AnalysisResult {
            classes: vec![
                JavaClass {
                    name: "Calculator".to_string(),
                    visibility: "public".to_string(),
                    is_abstract: false,
                    is_interface: false,
                    extends: None,
                    implements: vec![],
                    fields: vec![JavaField {
                        name: "value".to_string(),
                        field_type: "double".to_string(),
                        visibility: "private".to_string(),
                        is_static: false,
                        is_final: false,
                    }],
                    methods: vec![JavaMethod {
                        name: "add".to_string(),
                        return_type: "void".to_string(),
                        visibility: "public".to_string(),
                        is_static: false,
                        is_abstract: false,
                        parameters: vec![],
                        calls: vec![],
                    }],
                    constructors: vec![],
                },
                JavaClass {
                    name: "Printer".to_string(),
                    visibility: "public".to_string(),
                    is_abstract: false,
                    is_interface: false,
                    extends: None,
                    implements: vec![],
                    fields: vec![],
                    methods: vec![JavaMethod {
                        name: "print".to_string(),
                        return_type: "void".to_string(),
                        visibility: "public".to_string(),
                        is_static: false,
                        is_abstract: false,
                        parameters: vec![],
                        calls: vec![],
                    }],
                    constructors: vec![],
                },
            ],
            relationships: vec![Relationship {
                from: "Calculator".to_string(),
                to: "Printer".to_string(),
                kind: RelationshipType::Uses,
            }],
            object_registry: HashMap::new(),
            type_inference: HashMap::new(),
        }
    }

    // -- VisibilityState tests --

    #[test]
    fn object_creation_makes_class_visible() {
        let mut state = VisibilityState::new();
        let step = make_step(ExecutionAction::ObjectCreation {
            variable_name: "calc".to_string(),
            class_name: "Calculator".to_string(),
            constructor_params: vec![],
        });

        state.update(&step);

        assert!(state.visible_classes.contains("Calculator"));
    }

    #[test]
    fn method_call_makes_method_visible() {
        let mut state = VisibilityState::new();
        let step = make_step(ExecutionAction::MethodCall {
            caller: Some("calc".to_string()),
            method_name: "add".to_string(),
            target_class: "Calculator".to_string(),
            parameters: vec!["5".to_string()],
        });

        state.update(&step);

        assert!(state.visible_classes.contains("Calculator"));
        assert!(
            state
                .visible_methods
                .contains(&("Calculator".to_string(), "add".to_string()))
        );
    }

    #[test]
    fn field_mutation_adds_runtime_value() {
        let mut state = VisibilityState::new();
        let step = make_step(ExecutionAction::FieldMutation {
            class_name: "Calculator".to_string(),
            field_name: "value".to_string(),
            old_value: None,
            new_value: "5.0".to_string(),
        });

        state.update(&step);

        assert!(
            state
                .visible_fields
                .contains(&("Calculator".to_string(), "value".to_string()))
        );
        assert_eq!(
            state
                .runtime_values
                .get(&("Calculator".to_string(), "value".to_string())),
            Some(&"5.0".to_string())
        );
    }

    #[test]
    fn method_entry_exit_are_noop_on_visibility() {
        let mut state = VisibilityState::new();

        let enter = make_step(ExecutionAction::MethodEntry {
            class_name: "Calculator".to_string(),
            method_name: "add".to_string(),
        });
        state.update(&enter);

        let exit = make_step(ExecutionAction::MethodExit {
            class_name: "Calculator".to_string(),
            method_name: "add".to_string(),
            return_value: None,
        });
        state.update(&exit);

        assert!(state.visible_classes.is_empty());
        assert!(state.visible_fields.is_empty());
        assert!(state.visible_methods.is_empty());
    }

    #[test]
    fn build_filtered_analysis_returns_only_visible_elements() {
        let mut state = VisibilityState::new();
        let analysis = sample_analysis();

        // Make only Calculator visible with its add method and value field
        state.visible_classes.insert("Calculator".to_string());
        state
            .visible_methods
            .insert(("Calculator".to_string(), "add".to_string()));
        state
            .visible_fields
            .insert(("Calculator".to_string(), "value".to_string()));
        state.runtime_values.insert(
            ("Calculator".to_string(), "value".to_string()),
            "5.0".to_string(),
        );

        let filtered = state.build_filtered_analysis(&analysis);

        // Only Calculator class should remain
        assert_eq!(filtered.classes.len(), 1);
        assert_eq!(filtered.classes[0].name, "Calculator");

        // Only the "add" method should remain
        assert_eq!(filtered.classes[0].methods.len(), 1);
        assert_eq!(filtered.classes[0].methods[0].name, "add");

        // Only the "value" field, with runtime value appended
        assert_eq!(filtered.classes[0].fields.len(), 1);
        assert_eq!(filtered.classes[0].fields[0].name, "value = 5.0");

        // Relationship between Calculator and Printer is gone (Printer not visible)
        assert!(filtered.relationships.is_empty());
    }

    #[test]
    fn relationships_visible_only_when_both_endpoints_visible() {
        let mut state = VisibilityState::new();
        let analysis = sample_analysis();

        // Only one endpoint visible -> filtered analysis has no relationships
        let step1 = make_step(ExecutionAction::ObjectCreation {
            variable_name: "calc".to_string(),
            class_name: "Calculator".to_string(),
            constructor_params: vec![],
        });
        state.update(&step1);
        let filtered = state.build_filtered_analysis(&analysis);
        assert!(
            filtered.relationships.is_empty(),
            "Relationship should not appear with only one endpoint"
        );

        // Both endpoints visible -> relationship appears in filtered analysis
        let step2 = make_step(ExecutionAction::ObjectCreation {
            variable_name: "p".to_string(),
            class_name: "Printer".to_string(),
            constructor_params: vec![],
        });
        state.update(&step2);
        let filtered = state.build_filtered_analysis(&analysis);
        assert_eq!(filtered.relationships.len(), 1);
        assert_eq!(filtered.relationships[0].from, "Calculator");
        assert_eq!(filtered.relationships[0].to, "Printer");
    }

    #[test]
    fn primitive_variable_rendered_as_value_box() {
        let steps = vec![ExecutionStep {
            step_number: 1,
            line_number: 3,
            source_line: "int num = 5;".to_string(),
            action: ExecutionAction::VariableAssignment {
                variable_name: "num".to_string(),
                value_type: "int".to_string(),
                value: "5".to_string(),
            },
            call_stack: vec!["main".to_string()],
            active_objects: vec![],
            description: "Assign value to variable: num".to_string(),
        }];

        let generator = ExecutionGraphGenerator::new();
        let subgraph = generator.generate_object_state_subgraph(&steps);

        // Should contain the primitive variable box
        assert!(
            subgraph.contains("prim_num"),
            "Subgraph should contain primitive node for num, got:\n{}",
            subgraph
        );
        assert!(
            subgraph.contains("int num"),
            "Subgraph should label with type and name, got:\n{}",
            subgraph
        );
        assert!(
            subgraph.contains(">5<"),
            "Subgraph should display value 5 inside the box, got:\n{}",
            subgraph
        );
        // Should NOT create an object circle or arrow
        assert!(
            !subgraph.contains("obj_num"),
            "Primitive should not have an object circle"
        );
        assert!(
            !subgraph.contains("-> "),
            "Primitive should not have a reference arrow"
        );
    }

    #[test]
    fn object_shows_field_values_on_circle() {
        let steps = vec![
            ExecutionStep {
                step_number: 1,
                line_number: 5,
                source_line: "Dog casper = new Dog(\"arf\", 5);".to_string(),
                action: ExecutionAction::ObjectCreation {
                    variable_name: "casper".to_string(),
                    class_name: "Dog".to_string(),
                    constructor_params: vec!["\"arf\"".to_string(), "5".to_string()],
                },
                call_stack: vec!["main".to_string()],
                active_objects: vec!["casper".to_string()],
                description: "Create Dog".to_string(),
            },
            ExecutionStep {
                step_number: 2,
                line_number: 6,
                source_line: "this.talk = a;".to_string(),
                action: ExecutionAction::FieldMutation {
                    class_name: "Dog".to_string(),
                    field_name: "talk".to_string(),
                    old_value: None,
                    new_value: "\"arf\"".to_string(),
                },
                call_stack: vec!["main".to_string(), "Dog.<init>".to_string()],
                active_objects: vec!["casper".to_string()],
                description: "Mutate field".to_string(),
            },
            ExecutionStep {
                step_number: 3,
                line_number: 7,
                source_line: "this.age = anAge;".to_string(),
                action: ExecutionAction::FieldMutation {
                    class_name: "Dog".to_string(),
                    field_name: "age".to_string(),
                    old_value: None,
                    new_value: "5".to_string(),
                },
                call_stack: vec!["main".to_string(), "Dog.<init>".to_string()],
                active_objects: vec!["casper".to_string()],
                description: "Mutate field".to_string(),
            },
        ];

        let generator = ExecutionGraphGenerator::new();
        let subgraph = generator.generate_object_state_subgraph(&steps);

        // Object variable reference box
        assert!(
            subgraph.contains("var_casper"),
            "Should have variable reference for casper"
        );
        // Object circle
        assert!(
            subgraph.contains("obj_casper"),
            "Should have object node for casper"
        );
        // Field values shown on the object
        assert!(
            subgraph.contains("talk = \"arf\""),
            "Object should show talk field with value, got:\n{}",
            subgraph
        );
        assert!(
            subgraph.contains("age = 5"),
            "Object should show age field with value, got:\n{}",
            subgraph
        );
        // Reference arrow
        assert!(
            subgraph.contains("var_casper -> obj_casper"),
            "Should have reference arrow from var to object"
        );
    }

    #[test]
    fn mixed_primitives_and_objects() {
        let steps = vec![
            ExecutionStep {
                step_number: 1,
                line_number: 3,
                source_line: "int num = 5;".to_string(),
                action: ExecutionAction::VariableAssignment {
                    variable_name: "num".to_string(),
                    value_type: "int".to_string(),
                    value: "5".to_string(),
                },
                call_stack: vec!["main".to_string()],
                active_objects: vec![],
                description: "Assign".to_string(),
            },
            ExecutionStep {
                step_number: 2,
                line_number: 4,
                source_line: "Dog fido = new Dog();".to_string(),
                action: ExecutionAction::ObjectCreation {
                    variable_name: "fido".to_string(),
                    class_name: "Dog".to_string(),
                    constructor_params: vec![],
                },
                call_stack: vec!["main".to_string()],
                active_objects: vec!["fido".to_string()],
                description: "Create Dog".to_string(),
            },
        ];

        let generator = ExecutionGraphGenerator::new();
        let subgraph = generator.generate_object_state_subgraph(&steps);

        // Both should be present
        assert!(subgraph.contains("prim_num"), "Should have primitive num");
        assert!(subgraph.contains("var_fido"), "Should have object var fido");
        assert!(
            subgraph.contains("obj_fido"),
            "Should have object circle fido"
        );
        // Primitive should not be confused with object
        assert!(
            !subgraph.contains("var_num"),
            "num is a primitive, should use prim_ prefix"
        );
    }
}
