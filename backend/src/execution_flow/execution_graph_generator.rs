use super::ExecutionFlow;
use super::execution_analyzer::{ExecutionAction, ExecutionStep};
use crate::analyzer::{AnalysisResult, RelationshipType};
use crate::no_flow;
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

        result.relationships.retain(|r| match r.relationship_type {
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
        let mut highlight_ids: HashSet<String> = HashSet::new();

        for entry in &current_step.call_stack {
            if let Some((class, method)) = entry.split_once('.') {
                highlight_ids.insert(format!("{}_{}", class, method));
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
                highlight_ids.insert(format!("{}_{}", class_name, field_name));
            }
            _ => {}
        }

        let mut result = dot_content.to_string();

        for element_id in &highlight_ids {
            let field_prefix = format!("\"{}", element_id);
            let lines: Vec<&str> = result.lines().collect();
            let mut new_lines = Vec::new();
            for line in lines {
                if line.contains(&field_prefix) && (line.contains(" [") || line.contains(" = ")) {
                    let modified = line
                        .replace("fillcolor=lightyellow", "fillcolor=gold")
                        .replace("fillcolor=lightgreen", "fillcolor=lime");
                    new_lines.push(modified);
                } else {
                    new_lines.push(line.to_string());
                }
            }
            result = new_lines.join("\n");
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

    fn generate_object_state_subgraph(&self, steps: &[ExecutionStep]) -> String {
        let mut subgraph = String::new();
        let mut active_objects = HashMap::new();

        // Collect all objects that have been created
        for step in steps {
            if let ExecutionAction::ObjectCreation {
                variable_name,
                class_name,
                ..
            } = &step.action
            {
                active_objects.insert(variable_name.clone(), class_name.clone());
            }
        }

        if !active_objects.is_empty() {
            subgraph.push_str("    subgraph cluster_objects {\n");
            subgraph.push_str("        label=\"Active Objects\";\n");
            subgraph.push_str("        style=filled;\n");
            subgraph.push_str("        fillcolor=lightgreen;\n");

            for (var_name, class_name) in &active_objects {
                let node_id = format!("obj_{}", self.sanitize_name(var_name));
                subgraph.push_str(&format!(
                    "        {} [label=\"{}\\n({})\", shape=ellipse, style=\"filled\", fillcolor=white];\n",
                    node_id,
                    self.escape_label(var_name),
                    self.escape_label(class_name)
                ));
            }

            subgraph.push_str("    }\n\n");
        }

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
}

#[cfg(test)]
mod generator_tests {
    use super::super::execution_analyzer::{ExecutionAction, ExecutionStep};
    use super::*;
    use crate::analyzer::{
        AnalysisResult, JavaClass, JavaField, JavaMethod, Relationship, RelationshipType,
    };

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
                relationship_type: RelationshipType::Uses,
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
}
