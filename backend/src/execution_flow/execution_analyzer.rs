use crate::analyzer::AnalysisResult;
use crate::parser::node_text;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tree_sitter::Node;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutionStep {
    pub step_number: usize,
    pub line_number: usize,
    pub source_line: String,
    pub action: ExecutionAction,
    pub call_stack: Vec<String>,
    pub active_objects: Vec<String>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExecutionAction {
    MethodCall {
        caller: Option<String>,
        method_name: String,
        target_class: String,
        parameters: Vec<String>,
    },
    ObjectCreation {
        variable_name: String,
        class_name: String,
        constructor_params: Vec<String>,
    },
    VariableAssignment {
        variable_name: String,
        value_type: String,
        value: String,
    },
    MethodReturn {
        method_name: String,
        return_value: Option<String>,
    },
    ConditionalBranch {
        condition: String,
        branch_taken: bool,
    },
    LoopIteration {
        loop_type: String,
        condition: String,
        iteration: usize,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionFlow {
    pub steps: Vec<ExecutionStep>,
    pub call_graph: HashMap<String, Vec<String>>,
    pub object_lifecycle: HashMap<String, Vec<usize>>, // object_name -> [creation_step, usage_steps...]
    pub max_call_stack_depth: usize,
}

pub struct ExecutionAnalyzer {
    analysis_result: AnalysisResult,
    current_step: usize,
    call_stack: Vec<String>,
    active_objects: HashMap<String, String>, // variable_name -> class_name
    steps: Vec<ExecutionStep>,
    call_graph: HashMap<String, Vec<String>>,
    object_lifecycle: HashMap<String, Vec<usize>>,
    source_lines: Vec<String>,
    enhanced_object_tracking: bool,
}

impl ExecutionAnalyzer {
    pub fn new(analysis_result: AnalysisResult) -> Self {
        ExecutionAnalyzer {
            analysis_result,
            current_step: 0,
            call_stack: Vec::new(),
            active_objects: HashMap::new(),
            steps: Vec::new(),
            call_graph: HashMap::new(),
            object_lifecycle: HashMap::new(),
            source_lines: Vec::new(),
            enhanced_object_tracking: true,
        }
    }

    pub fn analyze_execution_flow(&mut self, root_node: &Node, source: &str) -> ExecutionFlow {
        // Split source into lines for reference
        self.source_lines = source.lines().map(|s| s.to_string()).collect();

        // Find the main method first
        // this will be the root of our graph and where everything grows out of from
        if let Some(main_method) = self.find_main_method(root_node, source) {
            let main_method_copy = main_method;
            self.analyze_method_execution(&main_method_copy, source, "main".to_string());
        }

        ExecutionFlow {
            steps: self.steps.clone(),
            call_graph: self.call_graph.clone(),
            object_lifecycle: self.object_lifecycle.clone(),
            max_call_stack_depth: self
                .steps
                .iter()
                .map(|s| s.call_stack.len())
                .max()
                .unwrap_or(0),
        }
    }

    fn find_main_method<'a>(&self, root_node: &Node<'a>, source: &str) -> Option<Node<'a>> {
        let mut main_method = None;

        self.find_main_recursive(root_node, source, &mut main_method);

        main_method
    }

    fn find_main_recursive<'a>(
        &self,
        node: &Node<'a>,
        source: &str,
        main_method: &mut Option<Node<'a>>,
    ) {
        if node.kind() == "method_declaration" {
            let method_text = node_text(node, source);
            if method_text.contains("main") && method_text.contains("static") {
                *main_method = Some(*node);
                return;
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if main_method.is_none() {
                self.find_main_recursive(&child, source, main_method);
            }
        }
    }

    fn analyze_method_execution(&mut self, method_node: &Node, source: &str, method_name: String) {
        self.call_stack.push(method_name.clone());

        // Find the method body
        if let Some(body) = method_node.child_by_field_name("body") {
            self.analyze_block(&body, source);
        }

        self.call_stack.pop();
    }

    fn analyze_block(&mut self, block_node: &Node, source: &str) {
        let mut cursor = block_node.walk();

        for child in block_node.children(&mut cursor) {
            self.analyze_statement(&child, source);
        }
    }

    fn analyze_statement(&mut self, stmt_node: &Node, source: &str) {
        let line_number = stmt_node.start_position().row + 1;
        let source_line = self.get_source_line(line_number);

        match stmt_node.kind() {
            "local_variable_declaration" => {
                self.analyze_variable_declaration(stmt_node, source, line_number, &source_line);
            }
            "expression_statement" => {
                self.analyze_expression_statement(stmt_node, source, line_number, &source_line);
            }
            "if_statement" => {
                self.analyze_if_statement(stmt_node, source, line_number, &source_line);
            }
            "for_statement" | "while_statement" | "enhanced_for_statement" => {
                self.analyze_loop_statement(stmt_node, source, line_number, &source_line);
            }
            "return_statement" => {
                self.analyze_return_statement(stmt_node, source, line_number, &source_line);
            }
            "block" => {
                self.analyze_block(stmt_node, source);
            }
            _ => {
                // Handle other statement types
                if !stmt_node.children(&mut stmt_node.walk()).any(|_| true) {
                    // Only log leaf nodes to avoid duplication
                    self.add_execution_step(
                        line_number,
                        &source_line,
                        ExecutionAction::VariableAssignment {
                            variable_name: "unknown".to_string(),
                            value_type: "statement".to_string(),
                            value: node_text(stmt_node, source).to_string(),
                        },
                        format!("Execute statement: {}", stmt_node.kind()),
                    );
                }
            }
        }
    }

    fn analyze_variable_declaration(
        &mut self,
        decl_node: &Node,
        source: &str,
        line_number: usize,
        source_line: &str,
    ) {
        let mut variable_name = String::new();
        let mut class_name = String::new();
        let mut is_object_creation = false;

        // Extract variable name and type
        let mut cursor = decl_node.walk();
        for child in decl_node.children(&mut cursor) {
            match child.kind() {
                "type" => {
                    class_name = self.extract_type_name(&child, source);
                }
                "variable_declarator" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        variable_name = node_text(&name_node, source).to_string();
                    }

                    // Check if there's an object creation
                    if let Some(value_node) = child.child_by_field_name("value") {
                        if value_node.kind() == "object_creation_expression" {
                            is_object_creation = true;
                            let params = self.extract_constructor_parameters(&value_node, source);

                            self.active_objects
                                .insert(variable_name.clone(), class_name.clone());
                            self.record_object_creation(&variable_name);

                            self.add_execution_step(
                                line_number,
                                source_line,
                                ExecutionAction::ObjectCreation {
                                    variable_name: variable_name.clone(),
                                    class_name: class_name.clone(),
                                    constructor_params: params,
                                },
                                format!("Create new {}object: {}", class_name, variable_name),
                            );
                        } else {
                            // Regular variable assignment
                            let value = node_text(&value_node, source).to_string();
                            self.add_execution_step(
                                line_number,
                                source_line,
                                ExecutionAction::VariableAssignment {
                                    variable_name: variable_name.clone(),
                                    value_type: class_name.clone(),
                                    value,
                                },
                                format!("Assign value to variable: {}", variable_name),
                            );
                        }
                    }
                }
                _ => {}
            }
        }

        if !is_object_creation && !variable_name.is_empty() {
            self.add_execution_step(
                line_number,
                source_line,
                ExecutionAction::VariableAssignment {
                    variable_name: variable_name.clone(),
                    value_type: class_name,
                    value: "declared".to_string(),
                },
                format!("Declare variable: {}", variable_name),
            );
        }
    }

    fn analyze_expression_statement(
        &mut self,
        expr_node: &Node,
        source: &str,
        line_number: usize,
        source_line: &str,
    ) {
        if let Some(expr) = expr_node.child(0) {
            if expr.kind() == "method_invocation" {
                self.analyze_method_invocation(&expr, source, line_number, source_line);
            } else if expr.kind() == "assignment_expression" {
                self.analyze_assignment(&expr, source, line_number, source_line);
            }
        }
    }

    fn analyze_method_invocation(
        &mut self,
        method_node: &Node,
        source: &str,
        line_number: usize,
        source_line: &str,
    ) {
        let mut method_name = String::new();
        let mut caller = None;
        let mut target_class = "unknown".to_string();
        let mut parameters = Vec::new();

        // Extract method name
        if let Some(name_node) = method_node.child_by_field_name("name") {
            method_name = node_text(&name_node, source).to_string();
        }

        // Extract caller object
        if let Some(object_node) = method_node.child_by_field_name("object") {
            let caller_name = node_text(&object_node, source).to_string();
            caller = Some(caller_name.clone());

            // Enhanced object class resolution
            target_class = self.resolve_object_class_enhanced(&caller_name);
        }

        // Extract parameters
        if let Some(args_node) = method_node.child_by_field_name("arguments") {
            parameters = self.extract_method_arguments(&args_node, source);
        }

        // Record the method call in call graph
        let caller_method = self
            .call_stack
            .last()
            .unwrap_or(&"unknown".to_string())
            .clone();
        let called_method = format!("{}.{}", target_class, method_name);

        self.call_graph
            .entry(caller_method)
            .or_default()
            .push(called_method.clone());

        // Record object usage
        if let Some(caller_name) = &caller {
            self.record_object_usage(caller_name);
        }

        self.add_execution_step(
            line_number,
            source_line,
            ExecutionAction::MethodCall {
                caller,
                method_name: method_name.clone(),
                target_class,
                parameters,
            },
            format!("Call method: {}", method_name),
        );
    }

    fn analyze_assignment(
        &mut self,
        assign_node: &Node,
        source: &str,
        line_number: usize,
        source_line: &str,
    ) {
        let mut variable_name = String::new();
        let mut value = String::new();

        if let Some(left) = assign_node.child_by_field_name("left") {
            variable_name = node_text(&left, source).to_string();
        }

        if let Some(right) = assign_node.child_by_field_name("right") {
            value = node_text(&right, source).to_string();
        }

        self.add_execution_step(
            line_number,
            source_line,
            ExecutionAction::VariableAssignment {
                variable_name: variable_name.clone(),
                value_type: "assigned".to_string(),
                value,
            },
            format!("Assign value to: {}", variable_name),
        );
    }

    fn analyze_if_statement(
        &mut self,
        if_node: &Node,
        source: &str,
        line_number: usize,
        source_line: &str,
    ) {
        let mut condition = String::new();

        if let Some(condition_node) = if_node.child_by_field_name("condition") {
            condition = node_text(&condition_node, source).to_string();
        }

        // For simplicity, assume we take the true branch
        self.add_execution_step(
            line_number,
            source_line,
            ExecutionAction::ConditionalBranch {
                condition: condition.clone(),
                branch_taken: true,
            },
            format!("Evaluate condition: {}", condition),
        );

        // Analyze the consequence (if body)
        if let Some(consequence) = if_node.child_by_field_name("consequence") {
            self.analyze_statement(&consequence, source);
        }
    }

    fn analyze_loop_statement(
        &mut self,
        loop_node: &Node,
        source: &str,
        line_number: usize,
        source_line: &str,
    ) {
        let loop_type = loop_node.kind().to_string();
        let mut condition = String::new();

        // Extract condition based on loop type
        if let Some(condition_node) = loop_node.child_by_field_name("condition") {
            condition = node_text(&condition_node, source).to_string();
        }

        // For simplicity, simulate one iteration
        self.add_execution_step(
            line_number,
            source_line,
            ExecutionAction::LoopIteration {
                loop_type,
                condition: condition.clone(),
                iteration: 1,
            },
            format!("Enter loop with condition: {}", condition),
        );

        // Analyze loop body
        if let Some(body) = loop_node.child_by_field_name("body") {
            self.analyze_statement(&body, source);
        }
    }

    fn analyze_return_statement(
        &mut self,
        return_node: &Node,
        source: &str,
        line_number: usize,
        source_line: &str,
    ) {
        let mut return_value = None;

        if let Some(value_node) = return_node.child(1) {
            return_value = Some(node_text(&value_node, source).to_string());
        }

        let method_name = self
            .call_stack
            .last()
            .unwrap_or(&"unknown".to_string())
            .clone();

        self.add_execution_step(
            line_number,
            source_line,
            ExecutionAction::MethodReturn {
                method_name,
                return_value,
            },
            "Return from method".to_string(),
        );
    }

    fn add_execution_step(
        &mut self,
        line_number: usize,
        source_line: &str,
        action: ExecutionAction,
        description: String,
    ) {
        self.current_step += 1;

        let step = ExecutionStep {
            step_number: self.current_step,
            line_number,
            source_line: source_line.to_string(),
            action,
            call_stack: self.call_stack.clone(),
            active_objects: self.active_objects.keys().cloned().collect(),
            description,
        };

        self.steps.push(step);
    }

    fn extract_type_name(&self, type_node: &Node, source: &str) -> String {
        node_text(type_node, source).to_string()
    }

    fn extract_constructor_parameters(&self, creation_node: &Node, source: &str) -> Vec<String> {
        let mut params = Vec::new();

        if let Some(args_node) = creation_node.child_by_field_name("arguments") {
            params = self.extract_method_arguments(&args_node, source);
        }

        params
    }

    fn extract_method_arguments(&self, args_node: &Node, source: &str) -> Vec<String> {
        let mut arguments = Vec::new();
        let mut cursor = args_node.walk();

        for child in args_node.children(&mut cursor) {
            if child.kind() != "(" && child.kind() != ")" && child.kind() != "," {
                arguments.push(node_text(&child, source).to_string());
            }
        }

        arguments
    }

    fn record_object_creation(&mut self, object_name: &str) {
        self.object_lifecycle
            .entry(object_name.to_string())
            .or_default()
            .push(self.current_step);
    }

    fn record_object_usage(&mut self, object_name: &str) {
        if let Some(lifecycle) = self.object_lifecycle.get_mut(object_name) {
            lifecycle.push(self.current_step);
        }
    }

    fn get_source_line(&self, line_number: usize) -> String {
        if line_number > 0 && line_number <= self.source_lines.len() {
            self.source_lines[line_number - 1].trim().to_string()
        } else {
            String::new()
        }
    }

    fn resolve_object_class_enhanced(&self, object_name: &str) -> String {
        // First check active objects (runtime)
        if let Some(class_name) = self.active_objects.get(object_name) {
            return class_name.clone();
        }

        if self.enhanced_object_tracking {
            // Check static analysis type inference
            if let Some(class_name) = self.analysis_result.type_inference.get(object_name) {
                return class_name.clone();
            }

            // Check object registry
            if let Some(object_info) = self.analysis_result.object_registry.get(object_name) {
                return object_info.class_name.clone();
            }
        }

        // Check if it's a static class name (starts with uppercase)
        if object_name
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false)
        {
            return object_name.to_string();
        }

        "unknown".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::JavaAnalyzer;
    use crate::parser::JavaParser;

    #[test]
    fn test_execution_flow_analysis() {
        let java_code = r#"
            public class TestExecution {
                public static void main(String[] args) {
                    Calculator calc = new Calculator();
                    calc.add(5);
                    calc.add(3);
                    double result = calc.getResult();
                    System.out.println(result);
                }
            }

            public class Calculator {
                private double value;

                public Calculator() {
                    this.value = 0.0;
                }

                public void add(double amount) {
                    this.value += amount;
                }

                public double getResult() {
                    return this.value;
                }
            }
        "#;

        let mut parser = JavaParser::new().unwrap();
        let tree = parser.parse(java_code).unwrap();
        let root = parser.get_root_node(&tree);

        let mut analyzer = JavaAnalyzer::new();
        let analysis = analyzer.analyze(&root, java_code);

        let mut exec_analyzer = ExecutionAnalyzer::new(analysis);
        let flow = exec_analyzer.analyze_execution_flow(&root, java_code);

        assert_eq!(flow.steps.len(), 8);
        assert_eq!(
            flow.steps,
            vec![
                ExecutionStep {
                    step_number: 1,
                    line_number: 3,
                    source_line: "public static void main(String[] args) {".into(),
                    action: ExecutionAction::VariableAssignment {
                        variable_name: "unknown".into(),
                        value_type: "statement".into(),
                        value: "{".into()
                    },
                    call_stack: vec!["main".into()],
                    active_objects: vec![],
                    description: "Execute statement: {".into()
                },
                ExecutionStep {
                    step_number: 2,
                    line_number: 4,
                    source_line: "Calculator calc = new Calculator();".into(),
                    action: ExecutionAction::ObjectCreation {
                        variable_name: "calc".into(),
                        class_name: "".into(),
                        constructor_params: vec![]
                    },
                    call_stack: vec!["main".into()],
                    active_objects: vec!["calc".into()],
                    description: "Create new object: calc".into()
                },
                ExecutionStep {
                    step_number: 3,
                    line_number: 5,
                    source_line: "calc.add(5);".into(),
                    action: ExecutionAction::MethodCall {
                        caller: Some("calc".into()),
                        method_name: "add".into(),
                        target_class: "".into(),
                        parameters: vec!["5".into()]
                    },
                    call_stack: vec!["main".into()],
                    active_objects: vec!["calc".into()],
                    description: "Call method: add".into()
                },
                ExecutionStep {
                    step_number: 4,
                    line_number: 6,
                    source_line: "calc.add(3);".into(),
                    action: ExecutionAction::MethodCall {
                        caller: Some("calc".into()),
                        method_name: "add".into(),
                        target_class: "".into(),
                        parameters: vec!["3".into()]
                    },
                    call_stack: vec!["main".into()],
                    active_objects: vec!["calc".into()],
                    description: "Call method: add".into()
                },
                ExecutionStep {
                    step_number: 5,
                    line_number: 7,
                    source_line: "double result = calc.getResult();".into(),
                    action: ExecutionAction::VariableAssignment {
                        variable_name: "result".into(),
                        value_type: "".into(),
                        value: "calc.getResult()".into()
                    },
                    call_stack: vec!["main".into()],
                    active_objects: vec!["calc".into()],
                    description: "Assign value to variable: result".into()
                },
                ExecutionStep {
                    step_number: 6,
                    line_number: 7,
                    source_line: "double result = calc.getResult();".into(),
                    action: ExecutionAction::VariableAssignment {
                        variable_name: "result".into(),
                        value_type: "".into(),
                        value: "declared".into()
                    },
                    call_stack: vec!["main".into()],
                    active_objects: vec!["calc".into()],
                    description: "Declare variable: result".into()
                },
                ExecutionStep {
                    step_number: 7,
                    line_number: 8,
                    source_line: "System.out.println(result);".into(),
                    action: ExecutionAction::MethodCall {
                        caller: Some("System.out".into()),
                        method_name: "println".into(),
                        target_class: "System.out".into(),
                        parameters: vec!["result".into()]
                    },
                    call_stack: vec!["main".into()],
                    active_objects: vec!["calc".into()],
                    description: "Call method: println".into()
                },
                ExecutionStep {
                    step_number: 8,
                    line_number: 9,
                    source_line: "}".into(),
                    action: ExecutionAction::VariableAssignment {
                        variable_name: "unknown".into(),
                        value_type: "statement".into(),
                        value: "}".into()
                    },
                    call_stack: vec!["main".into()],
                    active_objects: vec!["calc".into()],
                    description: "Execute statement: }".into()
                }
            ]
        );
    }
}
