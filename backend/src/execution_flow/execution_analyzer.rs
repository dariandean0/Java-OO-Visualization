use super::evaluator::{FlowSignal, Value};
use crate::{analyzer::AnalysisResult, parser::node_text};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tree_sitter::Node;

const MAX_LOOP_ITERATIONS: usize = 100;

/// Maps (class_name, method_name) -> (start_byte, end_byte) of the method body in source
pub type MethodBodyMap = HashMap<(String, String), (usize, usize)>;

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
    MethodEntry {
        class_name: String,
        method_name: String,
    },
    MethodExit {
        class_name: String,
        method_name: String,
        return_value: Option<String>,
    },
    FieldAccess {
        class_name: String,
        field_name: String,
        value: Option<String>,
    },
    FieldMutation {
        class_name: String,
        field_name: String,
        old_value: Option<String>,
        new_value: String,
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
    method_bodies: MethodBodyMap,
    max_call_depth: usize,
    current_call_depth: usize,
    field_values: HashMap<(String, String), String>,
    current_class: Option<String>,
    param_values: HashMap<String, String>,
    local_variables: HashMap<String, String>,
    flow_signal: Option<FlowSignal>,
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
            method_bodies: HashMap::new(),
            max_call_depth: 10,
            current_call_depth: 0,
            field_values: HashMap::new(),
            current_class: None,
            param_values: HashMap::new(),
            local_variables: HashMap::new(),
            flow_signal: None,
        }
    }

    pub fn analyze_execution_flow(&mut self, root_node: &Node, source: &str) -> ExecutionFlow {
        // Build method body map before walking main
        self.method_bodies = Self::build_method_body_map(root_node, source);

        // Split source into lines for reference
        self.source_lines = source.lines().map(|s| s.to_string()).collect();

        // Find the main method first
        // this will be the root of our graph and where everything grows out of from
        if let Some(main_method) = self.find_main_method(root_node, source) {
            let main_method_copy = main_method;
            self.analyze_method_execution(&main_method_copy, source, "main".to_string(), root_node);
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

        Self::find_main_recursive(root_node, source, &mut main_method);

        main_method
    }

    fn find_main_recursive<'a>(node: &Node<'a>, source: &str, main_method: &mut Option<Node<'a>>) {
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
                Self::find_main_recursive(&child, source, main_method);
            }
        }
    }

    fn analyze_method_execution(
        &mut self,
        method_node: &Node,
        source: &str,
        method_name: String,
        root_node: &Node,
    ) {
        self.call_stack.push(method_name.clone());

        // Find the method body
        if let Some(body) = method_node.child_by_field_name("body") {
            self.analyze_block(&body, source, root_node);
        }

        self.call_stack.pop();
    }

    fn analyze_block(&mut self, block_node: &Node, source: &str, root_node: &Node) {
        let mut cursor = block_node.walk();

        for child in block_node.named_children(&mut cursor) {
            self.analyze_statement(&child, source, root_node);
            if self.flow_signal.is_some() {
                break;
            }
        }
    }

    fn analyze_statement(&mut self, stmt_node: &Node, source: &str, root_node: &Node) {
        let line_number = stmt_node.start_position().row + 1;
        let source_line = self.get_source_line(line_number);

        match stmt_node.kind() {
            "local_variable_declaration" => {
                self.analyze_variable_declaration(
                    stmt_node,
                    source,
                    line_number,
                    &source_line,
                    root_node,
                );
            }
            "expression_statement" => {
                self.analyze_expression_statement(
                    stmt_node,
                    source,
                    line_number,
                    &source_line,
                    root_node,
                );
            }
            "if_statement" => {
                self.analyze_if_statement(stmt_node, source, line_number, &source_line, root_node);
            }
            "for_statement" | "while_statement" | "enhanced_for_statement" => {
                self.analyze_loop_statement(
                    stmt_node,
                    source,
                    line_number,
                    &source_line,
                    root_node,
                );
            }
            "do_statement" => {
                self.analyze_do_while_statement(
                    stmt_node,
                    source,
                    line_number,
                    &source_line,
                    root_node,
                );
            }
            "return_statement" => {
                self.analyze_return_statement(
                    stmt_node,
                    source,
                    line_number,
                    &source_line,
                    root_node,
                );
            }
            "block" => {
                self.analyze_block(stmt_node, source, root_node);
            }
            "break_statement" => {
                self.flow_signal = Some(FlowSignal::Break);
            }
            "continue_statement" => {
                self.flow_signal = Some(FlowSignal::Continue);
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
        root_node: &Node,
    ) {
        let mut variable_name = String::new();
        let mut class_name = String::new();
        let mut value_handled = false;

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
                            value_handled = true;
                            let params = self.extract_constructor_parameters(&value_node, source);

                            // Extract actual class name from the creation expression
                            let creation_class =
                                if let Some(type_node) = value_node.child_by_field_name("type") {
                                    node_text(&type_node, source).to_string()
                                } else {
                                    class_name.clone()
                                };

                            self.active_objects
                                .insert(variable_name.clone(), creation_class.clone());
                            self.record_object_creation(&variable_name);

                            // Push constructor onto call stack BEFORE emitting the step
                            let body_range = self.find_method_body(&creation_class, "<init>");
                            let has_body = body_range.is_some()
                                && self.current_call_depth < self.max_call_depth;

                            if has_body {
                                self.current_call_depth += 1;
                                self.call_stack.push(format!("{}.<init>", creation_class));
                            }

                            self.add_execution_step(
                                line_number,
                                source_line,
                                ExecutionAction::ObjectCreation {
                                    variable_name: variable_name.clone(),
                                    class_name: creation_class.clone(),
                                    constructor_params: params.clone(),
                                },
                                format!("Create new {} object: {}", creation_class, variable_name),
                            );

                            // Step into constructor body if available
                            if let Some((start, end)) = body_range
                                && has_body
                                && let Some(body_node) =
                                    root_node.descendant_for_byte_range(start, end)
                            {
                                let old_class = self.current_class.take();
                                self.current_class = Some(creation_class.clone());

                                // Map formal constructor param names to call-site arg values
                                let old_params = std::mem::take(&mut self.param_values);
                                let old_locals = std::mem::take(&mut self.local_variables);
                                if let Some(class) = self
                                    .analysis_result
                                    .classes
                                    .iter()
                                    .find(|c| c.name == creation_class)
                                    && let Some(ctor) = class.constructors.first()
                                {
                                    for (i, formal) in ctor.parameters.iter().enumerate() {
                                        if let Some(arg_val) = params.get(i) {
                                            self.param_values
                                                .insert(formal.name.clone(), arg_val.clone());
                                        }
                                    }
                                }

                                self.analyze_block(&body_node, source, root_node);

                                self.local_variables = old_locals;
                                self.param_values = old_params;
                                self.current_class = old_class;
                            }

                            if has_body {
                                self.call_stack.pop();
                                self.current_call_depth -= 1;
                            }
                        } else if value_node.kind() == "method_invocation" {
                            value_handled = true;
                            self.analyze_method_invocation(
                                &value_node,
                                source,
                                line_number,
                                source_line,
                                root_node,
                            );
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
                        } else {
                            value_handled = true;
                            let raw = node_text(&value_node, source).to_string();
                            let resolved = self.resolve_value(&raw);
                            self.local_variables
                                .insert(variable_name.clone(), resolved.clone());
                            self.add_execution_step(
                                line_number,
                                source_line,
                                ExecutionAction::VariableAssignment {
                                    variable_name: variable_name.clone(),
                                    value_type: class_name.clone(),
                                    value: resolved,
                                },
                                format!("Assign value to variable: {}", variable_name),
                            );
                        }
                    }
                }
                _ => {}
            }
        }

        if !value_handled && !variable_name.is_empty() {
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
        root_node: &Node,
    ) {
        if let Some(expr) = expr_node.child(0) {
            if expr.kind() == "method_invocation" {
                self.analyze_method_invocation(&expr, source, line_number, source_line, root_node);
            } else if expr.kind() == "assignment_expression" {
                self.analyze_assignment(&expr, source, line_number, source_line, root_node);
            }
        }
    }

    fn analyze_method_invocation(
        &mut self,
        method_node: &Node,
        source: &str,
        line_number: usize,
        source_line: &str,
        root_node: &Node,
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

        // Push call stack BEFORE emitting the MethodCall step
        let body_range = self.find_method_body(&target_class, &method_name);
        let has_body = body_range.is_some() && self.current_call_depth < self.max_call_depth;

        if has_body {
            self.current_call_depth += 1;
            self.call_stack
                .push(format!("{}.{}", target_class, method_name));
        }

        self.add_execution_step(
            line_number,
            source_line,
            ExecutionAction::MethodCall {
                caller,
                method_name: method_name.clone(),
                target_class: target_class.clone(),
                parameters: parameters.clone(),
            },
            format!("Call method: {}", method_name),
        );

        if let Some((start, end)) = body_range
            && has_body
            && let Some(body_node) = root_node.descendant_for_byte_range(start, end)
        {
            let old_class = self.current_class.take();
            self.current_class = Some(target_class.clone());

            // Map formal parameter names to call-site argument values
            let resolved_args: Vec<String> = parameters
                .iter()
                .map(|arg| self.resolve_value(arg))
                .collect();

            let old_params = std::mem::take(&mut self.param_values);
            let old_locals = std::mem::take(&mut self.local_variables);
            if let Some(class) = self
                .analysis_result
                .classes
                .iter()
                .find(|c| c.name == target_class)
                && let Some(method) = class.methods.iter().find(|m| m.name == method_name)
            {
                for (i, formal) in method.parameters.iter().enumerate() {
                    if let Some(arg_val) = resolved_args.get(i) {
                        self.param_values
                            .insert(formal.name.clone(), arg_val.clone());
                    }
                }
            }

            self.analyze_block(&body_node, source, root_node);

            self.local_variables = old_locals;
            self.param_values = old_params;
            self.current_class = old_class;
        }

        if has_body {
            self.call_stack.pop();
            self.current_call_depth -= 1;
        }
    }

    fn analyze_assignment(
        &mut self,
        assign_node: &Node,
        source: &str,
        line_number: usize,
        source_line: &str,
        root_node: &Node,
    ) {
        let mut variable_name = String::new();
        let mut value = String::new();

        if let Some(left) = assign_node.child_by_field_name("left") {
            variable_name = node_text(&left, source).to_string();

            // Detect field mutation: this.field = ...
            if left.kind() == "field_access"
                && let Some(object) = left.child_by_field_name("object")
                && node_text(&object, source) == "this"
                && let Some(field) = left.child_by_field_name("field")
            {
                let field_name = node_text(&field, source).to_string();

                if let Some(right) = assign_node.child_by_field_name("right") {
                    value = node_text(&right, source).to_string();
                }

                // Detect compound assignment (+=, -=, etc.)
                let is_compound = assign_node
                    .child(1)
                    .map(|op| {
                        let op_text = node_text(&op, source);
                        op_text != "="
                    })
                    .unwrap_or(false);

                let new_value = if is_compound {
                    self.evaluate_compound_assignment(assign_node, source, &field_name, &value)
                } else {
                    self.resolve_value(&value)
                };

                if let Some(class_name) = self.current_class.clone() {
                    let old_value = self
                        .field_values
                        .get(&(class_name.clone(), field_name.clone()))
                        .cloned();

                    self.field_values
                        .insert((class_name.clone(), field_name.clone()), new_value.clone());

                    self.add_execution_step(
                        line_number,
                        source_line,
                        ExecutionAction::FieldMutation {
                            class_name,
                            field_name: field_name.clone(),
                            old_value,
                            new_value,
                        },
                        format!("Mutate field: this.{}", field_name),
                    );
                    return;
                }
            }
        }

        if let Some(right) = assign_node.child_by_field_name("right") {
            if right.kind() == "object_creation_expression" {
                let params = self.extract_constructor_parameters(&right, source);
                let creation_class = if let Some(type_node) = right.child_by_field_name("type") {
                    node_text(&type_node, source).to_string()
                } else {
                    "unknown".to_string()
                };

                self.clear_field_values_for_class(&creation_class);

                self.active_objects
                    .insert(variable_name.clone(), creation_class.clone());
                self.record_object_creation(&variable_name);

                let body_range = self.find_method_body(&creation_class, "<init>");
                let has_body =
                    body_range.is_some() && self.current_call_depth < self.max_call_depth;

                if has_body {
                    self.current_call_depth += 1;
                    self.call_stack.push(format!("{}.<init>", creation_class));
                }

                self.add_execution_step(
                    line_number,
                    source_line,
                    ExecutionAction::ObjectCreation {
                        variable_name: variable_name.clone(),
                        class_name: creation_class.clone(),
                        constructor_params: params.clone(),
                    },
                    format!("Create new {} object: {}", creation_class, variable_name),
                );

                if let Some((start, end)) = body_range
                    && has_body
                    && let Some(body_node) = root_node.descendant_for_byte_range(start, end)
                {
                    let old_class = self.current_class.take();
                    self.current_class = Some(creation_class.clone());

                    let old_params = std::mem::take(&mut self.param_values);
                    let old_locals = std::mem::take(&mut self.local_variables);
                    if let Some(class) = self
                        .analysis_result
                        .classes
                        .iter()
                        .find(|c| c.name == creation_class)
                        && let Some(ctor) = class.constructors.first()
                    {
                        for (i, formal) in ctor.parameters.iter().enumerate() {
                            if let Some(arg_val) = params.get(i) {
                                self.param_values
                                    .insert(formal.name.clone(), arg_val.clone());
                            }
                        }
                    }

                    self.analyze_block(&body_node, source, root_node);

                    self.local_variables = old_locals;
                    self.param_values = old_params;
                    self.current_class = old_class;
                }

                if has_body {
                    self.call_stack.pop();
                    self.current_call_depth -= 1;
                }
                return;
            }

            value = node_text(&right, source).to_string();

            // Evaluate the RHS expression and track the local variable
            let evaluated = self.evaluate_expression(&right, source);
            if evaluated.is_known() {
                self.local_variables
                    .insert(variable_name.clone(), evaluated.to_storage_string());
            }
        }

        let display_value = self
            .local_variables
            .get(&variable_name)
            .cloned()
            .unwrap_or_else(|| value.clone());

        self.add_execution_step(
            line_number,
            source_line,
            ExecutionAction::VariableAssignment {
                variable_name: variable_name.clone(),
                value_type: "assigned".to_string(),
                value: display_value,
            },
            format!("Assign value to: {}", variable_name),
        );
    }

    fn evaluate_compound_assignment(
        &self,
        assign_node: &Node,
        source: &str,
        field_name: &str,
        rhs_text: &str,
    ) -> String {
        let fallback = node_text(assign_node, source).to_string();

        let op_text = match assign_node.child(1) {
            Some(op) => node_text(&op, source).to_string(),
            None => return fallback,
        };

        let class_name = match &self.current_class {
            Some(c) => c.clone(),
            None => return fallback,
        };

        let left_val: f64 = match self
            .field_values
            .get(&(class_name, field_name.to_string()))
            .and_then(|v| v.parse().ok())
        {
            Some(v) => v,
            None => return fallback,
        };

        let right_val: f64 = self
            .param_values
            .get(rhs_text)
            .or_else(|| self.local_variables.get(rhs_text))
            .and_then(|v| v.parse().ok())
            .or_else(|| rhs_text.parse().ok())
            .unwrap_or(f64::NAN);

        if right_val.is_nan() {
            return fallback;
        }

        let result = match op_text.as_str() {
            "+=" => left_val + right_val,
            "-=" => left_val - right_val,
            "*=" => left_val * right_val,
            "/=" => {
                if right_val == 0.0 {
                    return fallback;
                }
                left_val / right_val
            }
            _ => return fallback,
        };

        if result == result.floor() && result.abs() < 1e15 {
            format!("{:.1}", result)
        } else {
            result.to_string()
        }
    }

    fn analyze_if_statement(
        &mut self,
        if_node: &Node,
        source: &str,
        line_number: usize,
        source_line: &str,
        root_node: &Node,
    ) {
        let mut condition = String::new();
        let mut branch_taken = true; // default: take true branch if we can't evaluate

        if let Some(condition_node) = if_node.child_by_field_name("condition") {
            condition = node_text(&condition_node, source).to_string();
            // Evaluate the condition expression (unwrap parenthesized_expression)
            let eval_node = if condition_node.kind() == "parenthesized_expression" {
                condition_node.child(1).unwrap_or(condition_node)
            } else {
                condition_node
            };
            let value = self.evaluate_expression(&eval_node, source);
            if let Some(b) = value.as_bool() {
                branch_taken = b;
            }
        }

        self.add_execution_step(
            line_number,
            source_line,
            ExecutionAction::ConditionalBranch {
                condition: condition.clone(),
                branch_taken,
            },
            format!("Evaluate condition: {} = {}", condition, branch_taken),
        );

        if branch_taken {
            if let Some(consequence) = if_node.child_by_field_name("consequence") {
                self.analyze_statement(&consequence, source, root_node);
            }
        } else if let Some(alternative) = if_node.child_by_field_name("alternative") {
            self.analyze_statement(&alternative, source, root_node);
        }
    }

    fn analyze_loop_statement(
        &mut self,
        loop_node: &Node,
        source: &str,
        line_number: usize,
        source_line: &str,
        root_node: &Node,
    ) {
        let loop_type = loop_node.kind().to_string();

        match loop_node.kind() {
            "for_statement" => {
                self.analyze_for_statement(loop_node, source, line_number, source_line, root_node);
            }
            "while_statement" => {
                self.analyze_while_statement(
                    loop_node,
                    source,
                    line_number,
                    source_line,
                    root_node,
                );
            }
            "enhanced_for_statement" => {
                // Enhanced for-each: we can't know the collection size, run body once
                let mut condition = String::new();
                if let Some(condition_node) = loop_node.child_by_field_name("value") {
                    condition = node_text(&condition_node, source).to_string();
                }
                self.add_execution_step(
                    line_number,
                    source_line,
                    ExecutionAction::LoopIteration {
                        loop_type,
                        condition: condition.clone(),
                        iteration: 1,
                    },
                    format!("Enter for-each loop over: {}", condition),
                );
                if let Some(body) = loop_node.child_by_field_name("body") {
                    self.analyze_statement(&body, source, root_node);
                }
            }
            _ => {}
        }
    }

    fn analyze_while_statement(
        &mut self,
        loop_node: &Node,
        source: &str,
        line_number: usize,
        source_line: &str,
        root_node: &Node,
    ) {
        let condition_node = loop_node.child_by_field_name("condition");
        let condition_text = condition_node
            .map(|n| node_text(&n, source).to_string())
            .unwrap_or_default();

        let mut iteration = 0;
        loop {
            if iteration >= MAX_LOOP_ITERATIONS {
                break;
            }

            // Evaluate condition
            let cond_val = condition_node.and_then(|n| {
                let inner = if n.kind() == "parenthesized_expression" {
                    n.child(1).unwrap_or(n)
                } else {
                    n
                };
                self.evaluate_expression(&inner, source).as_bool()
            });

            match cond_val {
                Some(false) => break,
                Some(true) => {}
                None => {
                    // Can't evaluate: run body once (fallback)
                    if iteration == 0 {
                        self.add_execution_step(
                            line_number,
                            source_line,
                            ExecutionAction::LoopIteration {
                                loop_type: "while_statement".to_string(),
                                condition: condition_text.clone(),
                                iteration: 1,
                            },
                            format!("Enter loop with condition: {}", condition_text),
                        );
                        if let Some(body) = loop_node.child_by_field_name("body") {
                            self.analyze_statement(&body, source, root_node);
                        }
                    }
                    break;
                }
            }

            iteration += 1;
            self.add_execution_step(
                line_number,
                source_line,
                ExecutionAction::LoopIteration {
                    loop_type: "while_statement".to_string(),
                    condition: condition_text.clone(),
                    iteration,
                },
                format!("Loop iteration {}", iteration),
            );

            if let Some(body) = loop_node.child_by_field_name("body") {
                self.analyze_statement(&body, source, root_node);
            }

            // Check flow signal after body
            match self.flow_signal.take() {
                Some(FlowSignal::Break) => break,
                Some(FlowSignal::Continue) => continue,
                None => {}
            }
        }
    }

    fn analyze_do_while_statement(
        &mut self,
        do_node: &Node,
        source: &str,
        line_number: usize,
        source_line: &str,
        root_node: &Node,
    ) {
        let condition_node = do_node.child_by_field_name("condition");
        let condition_text = condition_node
            .map(|n| node_text(&n, source).to_string())
            .unwrap_or_default();

        let mut iteration = 0;
        loop {
            if iteration >= MAX_LOOP_ITERATIONS {
                break;
            }
            iteration += 1;

            self.add_execution_step(
                line_number,
                source_line,
                ExecutionAction::LoopIteration {
                    loop_type: "do_statement".to_string(),
                    condition: condition_text.clone(),
                    iteration,
                },
                format!("Do-while iteration {}", iteration),
            );

            if let Some(body) = do_node.child_by_field_name("body") {
                self.analyze_statement(&body, source, root_node);
            }

            // Check flow signal after body
            match self.flow_signal.take() {
                Some(FlowSignal::Break) => break,
                Some(FlowSignal::Continue) => {} // continue to condition check
                None => {}
            }

            // Evaluate condition after body execution
            let cond_val = condition_node.and_then(|n| {
                let inner = if n.kind() == "parenthesized_expression" {
                    n.child(1).unwrap_or(n)
                } else {
                    n
                };
                self.evaluate_expression(&inner, source).as_bool()
            });

            match cond_val {
                Some(false) => break,
                Some(true) => {}
                None => break, // can't evaluate: stop after one iteration
            }
        }
    }

    fn analyze_for_statement(
        &mut self,
        for_node: &Node,
        source: &str,
        line_number: usize,
        source_line: &str,
        root_node: &Node,
    ) {
        // Execute init clause
        if let Some(init_node) = for_node.child_by_field_name("init") {
            if init_node.kind() == "local_variable_declaration" {
                let init_line = init_node.start_position().row + 1;
                let init_source = self.get_source_line(init_line);
                self.analyze_variable_declaration(
                    &init_node,
                    source,
                    init_line,
                    &init_source,
                    root_node,
                );
            }
        }

        let condition_node = for_node.child_by_field_name("condition");
        let condition_text = condition_node
            .map(|n| node_text(&n, source).to_string())
            .unwrap_or_default();

        let mut iteration = 0;
        loop {
            if iteration >= MAX_LOOP_ITERATIONS {
                break;
            }

            // Evaluate condition
            let cond_val =
                condition_node.and_then(|n| self.evaluate_expression(&n, source).as_bool());

            match cond_val {
                Some(false) => break,
                Some(true) => {}
                None => {
                    // Fall back to static analysis
                    if let Some(iters) = self.evaluate_for_loop_iterations(for_node, source) {
                        let iters = iters.min(MAX_LOOP_ITERATIONS);
                        for i in 0..iters {
                            self.add_execution_step(
                                line_number,
                                source_line,
                                ExecutionAction::LoopIteration {
                                    loop_type: "for_statement".to_string(),
                                    condition: condition_text.clone(),
                                    iteration: i + 1,
                                },
                                format!("Loop iteration {} of {}", i + 1, iters),
                            );
                            if let Some(body) = for_node.child_by_field_name("body") {
                                self.analyze_statement(&body, source, root_node);
                            }
                        }
                    } else {
                        // Can't determine iterations; run body once
                        self.add_execution_step(
                            line_number,
                            source_line,
                            ExecutionAction::LoopIteration {
                                loop_type: "for_statement".to_string(),
                                condition: condition_text.clone(),
                                iteration: 1,
                            },
                            format!("Enter for loop with condition: {}", condition_text),
                        );
                        if let Some(body) = for_node.child_by_field_name("body") {
                            self.analyze_statement(&body, source, root_node);
                        }
                    }
                    return;
                }
            }

            iteration += 1;
            self.add_execution_step(
                line_number,
                source_line,
                ExecutionAction::LoopIteration {
                    loop_type: "for_statement".to_string(),
                    condition: condition_text.clone(),
                    iteration,
                },
                format!("Loop iteration {}", iteration),
            );

            if let Some(body) = for_node.child_by_field_name("body") {
                self.analyze_statement(&body, source, root_node);
            }

            // Check flow signal
            match self.flow_signal.take() {
                Some(FlowSignal::Break) => break,
                Some(FlowSignal::Continue) => {}
                None => {}
            }

            // Execute update clause (i++, i--, i+=1, etc.)
            if let Some(update_node) = for_node.child_by_field_name("update") {
                self.execute_update_expression(&update_node, source);
            }
        }
    }

    fn evaluate_for_loop_iterations(&self, for_node: &Node, source: &str) -> Option<usize> {
        let init_node = for_node.child_by_field_name("init")?;
        let condition_node = for_node.child_by_field_name("condition")?;

        let init_text = node_text(&init_node, source);
        let cond_text = node_text(&condition_node, source);

        let init_val = self.extract_for_init_value(&init_text)?;

        let (cmp_op, bound_text) = self.extract_for_condition(&cond_text)?;
        let bound_val = self.resolve_numeric(&bound_text)?;

        let iterations = match cmp_op.as_str() {
            "<" => {
                if init_val < bound_val {
                    (bound_val - init_val) as usize
                } else {
                    0
                }
            }
            "<=" => {
                if init_val <= bound_val {
                    (bound_val - init_val + 1.0) as usize
                } else {
                    0
                }
            }
            ">" => {
                if init_val > bound_val {
                    (init_val - bound_val) as usize
                } else {
                    0
                }
            }
            ">=" => {
                if init_val >= bound_val {
                    (init_val - bound_val + 1.0) as usize
                } else {
                    0
                }
            }
            _ => return None,
        };

        Some(iterations)
    }

    fn extract_for_init_value(&self, init_text: &str) -> Option<f64> {
        let parts: Vec<&str> = init_text.split('=').collect();
        if parts.len() == 2 {
            let val_str = parts[1].trim().trim_end_matches(';');
            self.resolve_numeric(val_str)
        } else {
            None
        }
    }

    fn extract_for_condition(&self, cond_text: &str) -> Option<(String, String)> {
        for op in &["<=", ">=", "<", ">"] {
            if let Some(idx) = cond_text.find(op) {
                let rhs = cond_text[idx + op.len()..].trim().to_string();
                return Some((op.to_string(), rhs));
            }
        }
        None
    }

    fn resolve_numeric(&self, text: &str) -> Option<f64> {
        let trimmed = text.trim();
        if let Ok(v) = trimmed.parse::<f64>() {
            return Some(v);
        }
        if let Some(v) = self.param_values.get(trimmed) {
            return v.parse().ok();
        }
        if let Some(v) = self.local_variables.get(trimmed) {
            return v.parse().ok();
        }
        None
    }

    fn analyze_return_statement(
        &mut self,
        return_node: &Node,
        source: &str,
        line_number: usize,
        source_line: &str,
        _root_node: &Node,
    ) {
        let mut return_value = None;

        if let Some(value_node) = return_node.child(1) {
            return_value = Some(node_text(&value_node, source).to_string());

            // Detect field access: return this.field
            if value_node.kind() == "field_access"
                && let Some(object) = value_node.child_by_field_name("object")
                && node_text(&object, source) == "this"
                && let Some(field) = value_node.child_by_field_name("field")
            {
                let field_name = node_text(&field, source).to_string();

                if let Some(class_name) = self.current_class.clone() {
                    let field_value = self
                        .field_values
                        .get(&(class_name.clone(), field_name.clone()))
                        .cloned();

                    self.add_execution_step(
                        line_number,
                        source_line,
                        ExecutionAction::FieldAccess {
                            class_name,
                            field_name: field_name.clone(),
                            value: field_value,
                        },
                        format!("Access field: this.{}", field_name),
                    );
                }
            }
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

    fn clear_field_values_for_class(&mut self, class_name: &str) {
        self.field_values.retain(|(cls, _), _| cls != class_name);
    }

    fn resolve_value(&self, text: &str) -> String {
        if let Some(v) = self.local_variables.get(text) {
            return v.clone();
        }
        if let Some(v) = self.param_values.get(text) {
            return v.clone();
        }
        if text.starts_with("this.") {
            let field_name = &text[5..];
            if let Some(class_name) = &self.current_class {
                if let Some(v) = self
                    .field_values
                    .get(&(class_name.clone(), field_name.to_string()))
                {
                    return v.clone();
                }
            }
        }
        text.to_string()
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
        if object_name == "this" {
            if let Some(class_name) = &self.current_class {
                return class_name.clone();
            }
        }

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

    /// Build a map of (class_name, method_name) -> (start_byte, end_byte) for all
    /// method and constructor bodies found in the AST.
    pub fn build_method_body_map(root: &Node, source: &str) -> MethodBodyMap {
        let mut map = MethodBodyMap::new();
        Self::collect_method_bodies(root, source, &mut map);
        map
    }

    fn collect_method_bodies(node: &Node, source: &str, map: &mut MethodBodyMap) {
        if node.kind() == "class_declaration" {
            let mut class_name = String::new();

            // Extract class name from the `name` child
            if let Some(name_node) = node.child_by_field_name("name") {
                class_name = node_text(&name_node, source).to_string();
            }

            // Find the class_body child and iterate its children
            if let Some(body_node) = node.child_by_field_name("body") {
                let mut cursor = body_node.walk();
                for child in body_node.children(&mut cursor) {
                    match child.kind() {
                        "method_declaration" => {
                            if let Some(name_node) = child.child_by_field_name("name") {
                                let method_name = node_text(&name_node, source).to_string();
                                if let Some(body) = child.child_by_field_name("body") {
                                    map.insert(
                                        (class_name.clone(), method_name),
                                        (body.start_byte(), body.end_byte()),
                                    );
                                }
                            }
                        }
                        "constructor_declaration" => {
                            if let Some(body) = child.child_by_field_name("body") {
                                map.insert(
                                    (class_name.clone(), "<init>".to_string()),
                                    (body.start_byte(), body.end_byte()),
                                );
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // Recurse into children to find nested/sibling class declarations
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            Self::collect_method_bodies(&child, source, map);
        }
    }

    /// Look up the byte range of a method body by class and method name.
    pub fn find_method_body(&self, class_name: &str, method_name: &str) -> Option<(usize, usize)> {
        self.method_bodies
            .get(&(class_name.to_string(), method_name.to_string()))
            .copied()
    }

    /// Evaluate a tree-sitter expression node to a Value.
    /// This is a read-only operation: it does not mutate variable state.
    fn evaluate_expression(&self, node: &Node, source: &str) -> Value {
        match node.kind() {
            "decimal_integer_literal" => {
                let text = node_text(node, source);
                text.parse::<i64>()
                    .map(Value::Int)
                    .unwrap_or(Value::Unknown)
            }
            "hex_integer_literal" | "octal_integer_literal" | "binary_integer_literal" => {
                let text = node_text(node, source);
                // Strip prefix (0x, 0, 0b) and parse
                let stripped = text
                    .trim_start_matches("0x")
                    .trim_start_matches("0X")
                    .trim_start_matches("0b")
                    .trim_start_matches("0B");
                i64::from_str_radix(
                    stripped,
                    match node.kind() {
                        "hex_integer_literal" => 16,
                        "octal_integer_literal" => 8,
                        "binary_integer_literal" => 2,
                        _ => 10,
                    },
                )
                .map(Value::Int)
                .unwrap_or(Value::Unknown)
            }
            "decimal_floating_point_literal" | "hex_floating_point_literal" => {
                let text = node_text(node, source);
                // Strip trailing f/F/d/D
                let cleaned = text.trim_end_matches(['f', 'F', 'd', 'D']);
                cleaned
                    .parse::<f64>()
                    .map(Value::Float)
                    .unwrap_or(Value::Unknown)
            }
            "true" => Value::Bool(true),
            "false" => Value::Bool(false),
            "null_literal" => Value::Null,
            "string_literal" => {
                let text = node_text(node, source);
                // Strip surrounding quotes
                let inner = text.trim_start_matches('"').trim_end_matches('"');
                Value::Str(inner.to_string())
            }
            "character_literal" => {
                let text = node_text(node, source);
                let inner = text.trim_start_matches('\'').trim_end_matches('\'');
                inner
                    .chars()
                    .next()
                    .map(|c| Value::Int(c as i64))
                    .unwrap_or(Value::Unknown)
            }
            "identifier" => {
                let name = node_text(node, source);
                self.resolve_variable_value(name)
            }
            "parenthesized_expression" => {
                // Unwrap parentheses and evaluate inner expression
                if let Some(inner) = node.child(1) {
                    self.evaluate_expression(&inner, source)
                } else {
                    Value::Unknown
                }
            }
            "binary_expression" => {
                let left_node = node.child_by_field_name("left");
                let right_node = node.child_by_field_name("right");
                // Operator is unnamed child at index 1
                let op_node = node.child(1);

                match (left_node, op_node, right_node) {
                    (Some(l), Some(op), Some(r)) => {
                        let left = self.evaluate_expression(&l, source);
                        let op_text = node_text(&op, source);
                        let right = self.evaluate_expression(&r, source);
                        match op_text {
                            "+" => left.add(&right),
                            "-" => left.sub(&right),
                            "*" => left.mul(&right),
                            "/" => left.div(&right),
                            "%" => left.rem(&right),
                            "<" => left.lt(&right),
                            "<=" => left.le(&right),
                            ">" => left.gt(&right),
                            ">=" => left.ge(&right),
                            "==" => left.eq_val(&right),
                            "!=" => left.ne_val(&right),
                            "&&" => left.and(&right),
                            "||" => left.or(&right),
                            _ => Value::Unknown,
                        }
                    }
                    _ => Value::Unknown,
                }
            }
            "unary_expression" => {
                // e.g., !condition, -value
                let mut cursor = node.walk();
                let children: Vec<_> = node.children(&mut cursor).collect();
                if children.len() >= 2 {
                    let op_text = node_text(&children[0], source);
                    let operand = self.evaluate_expression(&children[1], source);
                    match op_text {
                        "!" => operand.not(),
                        "-" => operand.negate(),
                        "+" => operand,
                        _ => Value::Unknown,
                    }
                } else {
                    Value::Unknown
                }
            }
            "field_access" => {
                // Handle this.field
                if let Some(object) = node.child_by_field_name("object")
                    && let Some(field) = node.child_by_field_name("field")
                {
                    if node_text(&object, source) == "this" {
                        let field_name = node_text(&field, source).to_string();
                        if let Some(class_name) = &self.current_class {
                            if let Some(val) =
                                self.field_values.get(&(class_name.clone(), field_name))
                            {
                                return self.parse_stored_value(val);
                            }
                        }
                    }
                }
                Value::Unknown
            }
            "update_expression" => {
                // i++, i--, ++i, --i -- just return the current value
                if let Some(id_node) = node.named_child(0) {
                    if id_node.kind() == "identifier" {
                        return self.resolve_variable_value(node_text(&id_node, source));
                    }
                }
                Value::Unknown
            }
            _ => Value::Unknown,
        }
    }

    /// Look up a variable name in local_variables, param_values, or field_values.
    fn resolve_variable_value(&self, name: &str) -> Value {
        if let Some(v) = self.local_variables.get(name) {
            return self.parse_stored_value(v);
        }
        if let Some(v) = self.param_values.get(name) {
            return self.parse_stored_value(v);
        }
        Value::Unknown
    }

    /// Parse a stored string value into a Value.
    fn parse_stored_value(&self, s: &str) -> Value {
        if s == "true" {
            return Value::Bool(true);
        }
        if s == "false" {
            return Value::Bool(false);
        }
        if s == "null" {
            return Value::Null;
        }
        if let Ok(i) = s.parse::<i64>() {
            return Value::Int(i);
        }
        if let Ok(f) = s.parse::<f64>() {
            return Value::Float(f);
        }
        Value::Unknown
    }

    /// Execute an update expression (i++, i--, ++i, --i) with side effects.
    fn execute_update_expression(&mut self, node: &Node, source: &str) {
        let text = node_text(node, source);
        if let Some(id_node) = node.named_child(0) {
            if id_node.kind() == "identifier" {
                let var_name = node_text(&id_node, source).to_string();
                let current = self.resolve_variable_value(&var_name);
                let new_val = if text.contains("++") {
                    current.add(&Value::Int(1))
                } else if text.contains("--") {
                    current.sub(&Value::Int(1))
                } else {
                    return;
                };
                if new_val.is_known() {
                    self.local_variables
                        .insert(var_name, new_val.to_storage_string());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::JavaAnalyzer;
    use crate::parser::JavaParser;

    #[test]
    fn execution_flow_analysis() {
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

        // Print steps for debugging
        for step in &flow.steps {
            eprintln!(
                "Step {} (line {}): {:?} | call_stack={:?}",
                step.step_number, step.line_number, step.action, step.call_stack
            );
        }

        // Without MethodEntry/MethodExit steps, we expect 11 steps:
        // 1: ObjectCreation(calc) -- call_stack includes Calculator.<init>
        // 2: FieldMutation(value = 0.0)
        // 3: MethodCall(calc.add(5)) -- call_stack includes Calculator.add
        // 4: FieldMutation(value += amount)
        // 5: MethodCall(calc.add(3))
        // 6: FieldMutation(value += amount)
        // 7: MethodCall(calc.getResult()) -- call_stack includes Calculator.getResult
        // 8: FieldAccess(Calculator.value)
        // 9: MethodReturn(Calculator.getResult)
        // 10: VariableAssignment(result = calc.getResult())
        // 11: MethodCall(println)
        assert_eq!(
            flow.steps.len(),
            11,
            "Expected 11 steps without MethodEntry/MethodExit"
        );

        // Step 1: ObjectCreation -- call stack already includes constructor
        assert_eq!(
            flow.steps[0].action,
            ExecutionAction::ObjectCreation {
                variable_name: "calc".into(),
                class_name: "Calculator".into(),
                constructor_params: vec![]
            }
        );
        assert!(
            flow.steps[0]
                .call_stack
                .contains(&"Calculator.<init>".to_string()),
            "ObjectCreation step should already have constructor on call stack"
        );

        // Step 2: FieldMutation for this.value = 0.0
        assert_eq!(
            flow.steps[1].action,
            ExecutionAction::FieldMutation {
                class_name: "Calculator".into(),
                field_name: "value".into(),
                old_value: None,
                new_value: "0.0".into()
            }
        );

        // Step 3: MethodCall for calc.add(5) -- call stack includes Calculator.add
        assert_eq!(
            flow.steps[2].action,
            ExecutionAction::MethodCall {
                caller: Some("calc".into()),
                method_name: "add".into(),
                target_class: "Calculator".into(),
                parameters: vec!["5".into()]
            }
        );
        assert!(
            flow.steps[2]
                .call_stack
                .contains(&"Calculator.add".to_string()),
            "MethodCall step should already have method on call stack"
        );

        // Step 4: FieldMutation for this.value += amount (first call: 0.0 + 5 = 5.0)
        assert_eq!(
            flow.steps[3].action,
            ExecutionAction::FieldMutation {
                class_name: "Calculator".into(),
                field_name: "value".into(),
                old_value: Some("0.0".into()),
                new_value: "5.0".into(),
            }
        );

        // Step 5: MethodCall for calc.add(3)
        assert_eq!(
            flow.steps[4].action,
            ExecutionAction::MethodCall {
                caller: Some("calc".into()),
                method_name: "add".into(),
                target_class: "Calculator".into(),
                parameters: vec!["3".into()]
            }
        );

        // Step 6: FieldMutation for second add call (5.0 + 3 = 8.0)
        assert_eq!(
            flow.steps[5].action,
            ExecutionAction::FieldMutation {
                class_name: "Calculator".into(),
                field_name: "value".into(),
                old_value: Some("5.0".into()),
                new_value: "8.0".into(),
            }
        );

        // Step 7: MethodCall for calc.getResult()
        assert_eq!(
            flow.steps[6].action,
            ExecutionAction::MethodCall {
                caller: Some("calc".into()),
                method_name: "getResult".into(),
                target_class: "Calculator".into(),
                parameters: vec![]
            }
        );

        // Step 8: FieldAccess for this.value in getResult
        if let ExecutionAction::FieldAccess {
            class_name,
            field_name,
            ..
        } = &flow.steps[7].action
        {
            assert_eq!(class_name, "Calculator");
            assert_eq!(field_name, "value");
        } else {
            panic!(
                "Step 8 should be FieldAccess, got {:?}",
                flow.steps[7].action
            );
        }

        // Step 9: MethodReturn from getResult
        if let ExecutionAction::MethodReturn {
            method_name,
            return_value,
        } = &flow.steps[8].action
        {
            assert_eq!(method_name, "Calculator.getResult");
            assert_eq!(return_value, &Some("this.value".to_string()));
        } else {
            panic!(
                "Step 9 should be MethodReturn, got {:?}",
                flow.steps[8].action
            );
        }

        // Step 10: VariableAssignment for result = calc.getResult()
        if let ExecutionAction::VariableAssignment {
            variable_name,
            value,
            ..
        } = &flow.steps[9].action
        {
            assert_eq!(variable_name, "result");
            assert_eq!(value, "calc.getResult()");
        } else {
            panic!(
                "Step 10 should be VariableAssignment, got {:?}",
                flow.steps[9].action
            );
        }

        // Step 11: MethodCall for println (JDK method, NOT stepped into)
        if let ExecutionAction::MethodCall {
            method_name,
            target_class,
            ..
        } = &flow.steps[10].action
        {
            assert_eq!(method_name, "println");
            assert_eq!(target_class, "System.out");
        } else {
            panic!(
                "Step 11 should be MethodCall for println, got {:?}",
                flow.steps[10].action
            );
        }

        // Verify max call stack depth (main + method body = 2)
        assert!(
            flow.max_call_stack_depth >= 2,
            "Max call stack depth should be >= 2, got {}",
            flow.max_call_stack_depth
        );
    }

    #[test]
    fn method_body_map_contains_all_methods() {
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

        let map = ExecutionAnalyzer::build_method_body_map(&root, java_code);

        // Verify all expected entries exist
        assert!(
            map.contains_key(&("Calculator".to_string(), "add".to_string())),
            "Map should contain Calculator.add"
        );
        assert!(
            map.contains_key(&("Calculator".to_string(), "getResult".to_string())),
            "Map should contain Calculator.getResult"
        );
        assert!(
            map.contains_key(&("Calculator".to_string(), "<init>".to_string())),
            "Map should contain Calculator.<init>"
        );
        assert!(
            map.contains_key(&("TestExecution".to_string(), "main".to_string())),
            "Map should contain TestExecution.main"
        );

        // Verify byte ranges are valid (start < end, both > 0)
        for ((class, method), (start, end)) in &map {
            assert!(
                *start > 0,
                "Start byte for {}.{} should be > 0, got {}",
                class,
                method,
                start
            );
            assert!(
                *start < *end,
                "Start byte should be < end byte for {}.{}: {} >= {}",
                class,
                method,
                start,
                end
            );
        }

        // Verify we have exactly 4 entries
        assert_eq!(map.len(), 4, "Map should have exactly 4 entries");
    }
}
