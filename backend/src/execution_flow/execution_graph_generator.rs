use super::{
    ExecutionFlow,
    execution_analyzer::{ExecutionAction, ExecutionStep},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Controls what is rendered into each step's DOT document.
#[derive(Debug, Clone)]
pub struct ExecutionGraphConfig {
    /// Render the call-stack side panel
    pub show_call_stack: bool,
    /// Render the active-object panel with field values
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

/// One step of the animated execution trace, ready to render.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionGraphStep {
    /// 1-based step index matching the source `ExecutionStep`
    pub step_number: usize,
    /// Human-readable caption for this step
    pub description: String,
    /// Standalone DOT document for this step
    pub dot_code: String,
    /// Aggregate counters describing execution up through this step
    pub execution_state: ExecutionState,
}

/// Summary counters describing the program at a particular step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionState {
    /// Innermost method currently executing
    pub active_method: String,
    /// Number of frames on the call stack
    pub call_stack_depth: usize,
    /// Objects instantiated so far in the trace
    pub objects_created: usize,
    /// Method invocations observed so far in the trace
    pub method_calls_made: usize,
}

/// Turns an [`ExecutionFlow`] into a sequence of per-step DOT documents
/// suitable for step-by-step playback in the frontend.
pub struct ExecutionGraphGenerator {
    config: ExecutionGraphConfig,
}

impl Default for ExecutionGraphGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutionGraphGenerator {
    /// Create a new [`ExecutionGraphGenerator`] with the default config.
    pub fn new() -> Self {
        ExecutionGraphGenerator {
            config: ExecutionGraphConfig::default(),
        }
    }

    /// Create a new [`ExecutionGraphGenerator`] with a custom [`ExecutionGraphConfig`].
    pub fn with_config(config: ExecutionGraphConfig) -> Self {
        ExecutionGraphGenerator { config }
    }

    /// Produce one [`ExecutionGraphStep`] per step in `flow`. Each step's DOT
    /// document shows the cumulative state up to and including that step.
    pub fn generate_execution_graphs(&self, flow: &ExecutionFlow) -> Vec<ExecutionGraphStep> {
        let mut graphs = Vec::new();
        let mut cumulative_steps = Vec::new();

        for (i, step) in flow.steps.iter().enumerate() {
            cumulative_steps.push(step.clone());

            let dot_code = self.build_composed_dot(i + 1, &cumulative_steps, step);
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

        // Active Objects is now the primary panel.
        if self.config.show_object_states {
            dot.push_str(&self.generate_object_state_subgraph(steps));
        }

        // Call stack is shown as a secondary panel to the side.
        if self.config.show_call_stack && !steps.is_empty() {
            dot.push_str(&self.generate_call_stack_subgraph(steps.last().unwrap()));
        }

        dot.push_str("}\n");
        dot
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
        // Track which variable was most recently created per class_name
        let mut last_created: HashMap<String, String> = HashMap::new();

        for step in steps {
            match &step.action {
                ExecutionAction::ObjectCreation {
                    variable_name,
                    class_name,
                    ..
                } => {
                    active_objects.insert(variable_name.clone(), class_name.clone());
                    // Track which instance was most recently created per class
                    last_created.insert(class_name.clone(), variable_name.clone());
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
                    // Attribute mutation to the most recently created instance of this class.
                    // Falls back to updating all instances if no creation was tracked.
                    let target_var = last_created.get(class_name).cloned();
                    for (var_name, cls) in &active_objects {
                        if cls != class_name {
                            continue;
                        }
                        if let Some(ref target) = target_var {
                            if var_name != target {
                                continue;
                            }
                        }
                        let fields = object_fields.entry(var_name.clone()).or_default();
                        if let Some(entry) = fields.iter_mut().find(|(n, _)| n == field_name) {
                            entry.1 = new_value.clone();
                        } else {
                            fields.push((field_name.clone(), new_value.clone()));
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

        let generator = ExecutionGraphGenerator::new();
        let graphs = generator.generate_execution_graphs(&flow);

        assert_eq!(graphs.len(), 1);
        assert!(graphs[0].dot_code.contains("ExecutionStep_1"));
        assert!(graphs[0].dot_code.contains("Calculator"));
    }

    // -- End-to-end integration tests --
    // These go through the full pipeline: Java source -> analyze -> generate DOT.
    // Hand-crafted step tests (which bypass the analyzer) can mask real bugs such
    // as an empty `value_type`, so only full-pipeline tests live here.

    fn run_full_pipeline(java_code: &str) -> Vec<String> {
        crate::execution_flow_gen(java_code)
    }
    #[test]
    fn e2e_primitive_int_renders_as_box_with_value() {
        // Reproduces screenshot bug: int num = 7 was missing from the visualization
        // because value_type was "" instead of "int".
        let java = r#"
public class App {
    public static void main(String[] args) {
        int num = 7;
    }
}
        "#;

        let dots = run_full_pipeline(java);
        assert!(
            !dots.is_empty(),
            "Pipeline should produce at least one step"
        );

        let last_dot = dots.last().unwrap();
        assert!(
            last_dot.contains("prim_num"),
            "Final step should contain primitive node for num. DOT:\n{}",
            last_dot
        );
        assert!(
            last_dot.contains(">7<"),
            "Final step should show value 7 inside primitive box. DOT:\n{}",
            last_dot
        );
        assert!(
            last_dot.contains("int num"),
            "Final step should label box with type int. DOT:\n{}",
            last_dot
        );
    }

    #[test]
    fn e2e_two_instances_have_separate_field_values() {
        // Reproduces screenshot bug: both casper and harvey showed harvey's values
        // because FieldMutation has no instance info and we updated all instances.
        let java = r#"
public class Dog {
    String talk;
    int age;

    public Dog(String a, int anAge) {
        this.talk = a;
        this.age = anAge;
    }

    public static void main(String[] args) {
        Dog casper = new Dog("arf", 5);
        Dog harvey = new Dog("ruff", 10);
    }
}
        "#;

        let dots = run_full_pipeline(java);
        let last_dot = dots.last().expect("should have steps");

        // Extract only the casper object definition
        let casper_def = last_dot
            .lines()
            .find(|l| l.contains("obj_casper ["))
            .expect("should have obj_casper definition");
        let harvey_def = last_dot
            .lines()
            .find(|l| l.contains("obj_harvey ["))
            .expect("should have obj_harvey definition");

        // casper must show its own values
        assert!(
            casper_def.contains("arf"),
            "casper should show 'arf', got:\n{}",
            casper_def
        );
        assert!(
            casper_def.contains("= 5"),
            "casper should show age = 5, got:\n{}",
            casper_def
        );
        assert!(
            !casper_def.contains("ruff"),
            "casper should NOT show harvey's value 'ruff', got:\n{}",
            casper_def
        );
        assert!(
            !casper_def.contains("= 10"),
            "casper should NOT show harvey's age 10, got:\n{}",
            casper_def
        );

        // harvey must show its own values
        assert!(
            harvey_def.contains("ruff"),
            "harvey should show 'ruff', got:\n{}",
            harvey_def
        );
        assert!(
            harvey_def.contains("= 10"),
            "harvey should show age = 10, got:\n{}",
            harvey_def
        );
    }

    #[test]
    fn e2e_constructor_field_uses_resolved_param_value() {
        // Reproduces the original constructor bug: this.talk = a must show 'arf'
        // in the DOT output, not the literal parameter name 'a'.
        let java = r#"
public class Dog {
    String talk;
    int age;

    public Dog(String a, int anAge) {
        this.talk = a;
        this.age = anAge;
    }

    public static void main(String[] args) {
        Dog casper = new Dog("arf", 5);
    }
}
        "#;

        let dots = run_full_pipeline(java);
        let last_dot = dots.last().expect("should have steps");

        // Strip the step title label (which echoes the source line) before checking
        // for unresolved param names — those would naturally appear in the source.
        let dot_no_title: String = last_dot
            .lines()
            .filter(|l| !l.starts_with("    label="))
            .collect::<Vec<_>>()
            .join("\n");

        // The field values must be resolved, not literal param names
        assert!(
            dot_no_title.contains("talk = \"arf\""),
            "DOT must contain resolved talk value, got:\n{}",
            dot_no_title
        );
        assert!(
            dot_no_title.contains("age = 5"),
            "DOT must contain resolved age value, got:\n{}",
            dot_no_title
        );
        // Field name = param name patterns must not appear anywhere in the rendered nodes
        assert!(
            !dot_no_title.contains("talk = a<") && !dot_no_title.contains("talk = a\""),
            "DOT must NOT contain unresolved param name 'a' for talk, got:\n{}",
            dot_no_title
        );
        assert!(
            !dot_no_title.contains("age = anAge"),
            "DOT must NOT contain unresolved param name 'anAge', got:\n{}",
            dot_no_title
        );
    }

    #[test]
    fn e2e_full_screenshot_scenario() {
        // The complete scenario from the user's screenshot:
        // two Dog instances + one primitive
        let java = r#"
public class Dog {
    String talk;
    int age;

    public Dog(String a, int anAge) {
        this.talk = a;
        this.age = anAge;
    }

    public static void main(String[] args) {
        Dog casper = new Dog("arf", 5);
        Dog harvey = new Dog("ruff", 10);
        int num = 7;
    }
}
        "#;

        let dots = run_full_pipeline(java);
        let last_dot = dots.last().expect("should have steps");

        let dot_no_title: String = last_dot
            .lines()
            .filter(|l| !l.starts_with("    label="))
            .collect::<Vec<_>>()
            .join("\n");

        // All three requirements from the screenshot must be met:
        // 1. Both Dog instances visible with correct per-instance values
        assert!(
            dot_no_title.contains("obj_casper") && dot_no_title.contains("obj_harvey"),
            "Both Dog instances should be visible. DOT:\n{}",
            dot_no_title
        );
        // 2. Primitive num visible with value 7
        assert!(
            dot_no_title.contains("prim_num") && dot_no_title.contains(">7<"),
            "Primitive num with value 7 should be visible. DOT:\n{}",
            dot_no_title
        );
        // 3. Class diagram circle must not show literal param names
        assert!(
            !dot_no_title.contains("String talk = a<") && !dot_no_title.contains("int age = anAge"),
            "Class diagram must not show literal param names. DOT:\n{}",
            dot_no_title
        );
    }
}
