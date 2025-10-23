use super::{
    execution_analyzer::ExecutionAction, execution_analyzer::ExecutionStep, ExecutionFlow,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ExecutionGraphConfig {
    pub show_line_numbers: bool,
    pub show_call_stack: bool,
    pub show_object_states: bool,
    pub show_parameters: bool,
    pub max_steps_per_graph: Option<usize>,
    pub highlight_current_step: bool,
}

impl Default for ExecutionGraphConfig {
    fn default() -> Self {
        ExecutionGraphConfig {
            show_line_numbers: true,
            show_call_stack: true,
            show_object_states: true,
            show_parameters: true,
            max_steps_per_graph: Some(10),
            highlight_current_step: true,
        }
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

impl ExecutionGraphGenerator {
    pub fn new() -> Self {
        ExecutionGraphGenerator {
            config: ExecutionGraphConfig::default(),
        }
    }

    pub fn with_config(config: ExecutionGraphConfig) -> Self {
        ExecutionGraphGenerator { config }
    }

    pub fn generate_execution_graphs(&self, flow: &ExecutionFlow) -> Vec<ExecutionGraphStep> {
        let mut graphs = Vec::new();
        let mut cumulative_steps = Vec::new();

        for (i, step) in flow.steps.iter().enumerate() {
            cumulative_steps.push(step.clone());

            // Generate a graph for this step
            let dot_code = self.generate_step_graph(&cumulative_steps, i + 1);
            let execution_state = self.calculate_execution_state(&cumulative_steps);

            graphs.push(ExecutionGraphStep {
                step_number: i + 1,
                description: step.description.clone(),
                dot_code,
                execution_state,
            });

            // If we have a max steps limit, keep only recent steps for next iteration
            if let Some(max_steps) = self.config.max_steps_per_graph {
                if cumulative_steps.len() > max_steps {
                    cumulative_steps = cumulative_steps.into_iter().skip(1).collect();
                }
            }
        }

        graphs
    }

    fn generate_step_graph(&self, steps: &[ExecutionStep], current_step: usize) -> String {
        let mut dot = String::new();

        dot.push_str(&format!("digraph ExecutionFlow_{} {{\n", current_step));
        dot.push_str("    rankdir=TB;\n");
        dot.push_str("    node [shape=box, fontname=\"Arial\"];\n");
        dot.push_str("    edge [fontname=\"Arial\", fontsize=10];\n");
        dot.push_str("    compound=true;\n\n");

        // Add title
        dot.push_str(&format!(
            "    label=\"Execution Flow - Step {}\";\n",
            current_step
        ));
        dot.push_str("    labelloc=top;\n");
        dot.push_str("    fontsize=16;\n\n");

        // Create call stack visualization
        if self.config.show_call_stack && !steps.is_empty() {
            dot.push_str(&self.generate_call_stack_subgraph(&steps.last().unwrap()));
        }

        // Create object state visualization
        if self.config.show_object_states {
            dot.push_str(&self.generate_object_state_subgraph(steps));
        }

        // Create execution timeline
        dot.push_str(&self.generate_execution_timeline(steps, current_step));

        // Create connections between steps
        if steps.len() > 1 {
            dot.push_str(&self.generate_step_connections(steps));
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

    fn generate_execution_timeline(&self, steps: &[ExecutionStep], current_step: usize) -> String {
        let mut timeline = String::new();

        timeline.push_str("    subgraph cluster_timeline {\n");
        timeline.push_str("        label=\"Execution Timeline\";\n");
        timeline.push_str("        style=filled;\n");
        timeline.push_str("        fillcolor=lightyellow;\n");
        timeline.push_str("        rankdir=LR;\n");

        let start_idx = if steps.len() > 5 { steps.len() - 5 } else { 0 };

        for (_i, step) in steps.iter().enumerate().skip(start_idx) {
            let node_id = format!("step_{}", step.step_number);
            let is_current = step.step_number == current_step;

            let style = if is_current && self.config.highlight_current_step {
                "filled, fillcolor=orange, color=red, penwidth=3"
            } else {
                "filled, fillcolor=white"
            };

            let label = self.format_step_label(step);

            timeline.push_str(&format!(
                "        {} [label=\"{}\", style=\"{}\", shape=box];\n",
                node_id, label, style
            ));
        }

        // Connect timeline steps
        for i in start_idx..(steps.len() - 1) {
            let from_step = steps[i].step_number;
            let to_step = steps[i + 1].step_number;
            timeline.push_str(&format!(
                "        step_{} -> step_{} [color=gray];\n",
                from_step, to_step
            ));
        }

        timeline.push_str("    }\n\n");
        timeline
    }

    fn generate_step_connections(&self, steps: &[ExecutionStep]) -> String {
        let mut connections = String::new();

        // Connect method calls to objects
        for step in steps {
            match &step.action {
                ExecutionAction::MethodCall {
                    caller,
                    method_name,
                    ..
                } => {
                    if let Some(caller_name) = caller {
                        let obj_id = format!("obj_{}", self.sanitize_name(caller_name));
                        let step_id = format!("step_{}", step.step_number);
                        connections.push_str(&format!(
                            "    {} -> {} [label=\"{}\", color=blue, style=dashed];\n",
                            obj_id,
                            step_id,
                            self.escape_label(method_name)
                        ));
                    }
                }
                _ => {}
            }
        }

        connections
    }

    fn format_step_label(&self, step: &ExecutionStep) -> String {
        let mut label = String::new();

        if self.config.show_line_numbers {
            label.push_str(&format!("L{}\\n", step.line_number));
        }

        match &step.action {
            ExecutionAction::ObjectCreation {
                variable_name,
                class_name,
                ..
            } => {
                label.push_str(&format!("new {}\\n{}", class_name, variable_name));
            }
            ExecutionAction::MethodCall {
                method_name,
                parameters,
                ..
            } => {
                if self.config.show_parameters && !parameters.is_empty() {
                    let params = parameters.join(", ");
                    label.push_str(&format!("{}({})\\n", method_name, params));
                } else {
                    label.push_str(&format!("{}()\\n", method_name));
                }
            }
            ExecutionAction::VariableAssignment {
                variable_name,
                value,
                ..
            } => {
                label.push_str(&format!("{} = {}\\n", variable_name, value));
            }
            ExecutionAction::MethodReturn { return_value, .. } => {
                if let Some(val) = return_value {
                    label.push_str(&format!("return {}\\n", val));
                } else {
                    label.push_str("return\\n");
                }
            }
            ExecutionAction::ConditionalBranch {
                condition,
                branch_taken,
            } => {
                label.push_str(&format!(
                    "if ({})\\n{}",
                    condition,
                    if *branch_taken { "true" } else { "false" }
                ));
            }
            ExecutionAction::LoopIteration {
                loop_type,
                iteration,
                ..
            } => {
                label.push_str(&format!("{} #{}\\n", loop_type, iteration));
            }
        }

        // Remove trailing newline and escape
        if label.ends_with("\\n") {
            label.truncate(label.len() - 2);
        }

        self.escape_label(&label)
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
mod tests {
    use super::*;
    use crate::execution_analyzer::{ExecutionAction, ExecutionStep};

    #[test]
    fn test_execution_graph_generation() {
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
        assert!(graphs[0].dot_code.contains("ExecutionFlow_1"));
        assert!(graphs[0].dot_code.contains("Calculator"));
    }
}
