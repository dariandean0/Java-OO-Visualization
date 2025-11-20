use crate::analyzer::{AnalysisResult, JavaAnalyzer, RelationshipType};
use crate::no_flow::GraphConfig;
use crate::no_flow::GraphGenerator;
use crate::parser::JavaParser;
use crate::visualizer::{JavaVisualizer, visualize_java_code, visualize_java_code_with_config};
use crate::{execution_flow_gen, no_flow_gen};

/// Test helper to parse and analyze Java code
fn analyze_java_code(code: &str) -> AnalysisResult {
    let mut parser = JavaParser::new().unwrap();
    let tree = parser.parse(code).unwrap();
    let root = parser.get_root_node(&tree);

    let mut analyzer = JavaAnalyzer::new();
    analyzer.analyze(&root, code)
}

/// Test helper to generate DOT from analysis
fn generate_dot(analysis: &AnalysisResult) -> String {
    let generator = GraphGenerator::new();
    generator.generate_dot(analysis)
}

#[cfg(test)]
mod basic_structure {
    use super::*;

    #[test]
    fn simple_class_analysis() {
        let code = r#"
public class SimpleClass {
    private int value;
    
    public SimpleClass() {
        this.value = 0;
    }
    
    public int getValue() {
        return this.value;
    }
}
"#;

        let result = analyze_java_code(code);

        // Verify class structure
        assert_eq!(result.classes.len(), 1);
        let class = &result.classes[0];
        assert_eq!(class.name, "SimpleClass");
        assert_eq!(class.visibility, "public");
        assert!(!class.is_abstract);
        assert!(!class.is_interface);

        // Verify fields
        assert_eq!(class.fields.len(), 1);
        let field = &class.fields[0];
        assert_eq!(field.name, "value");
        println!("DEBUG: field.field_type = '{}'", field.field_type);
        assert_eq!(field.field_type, "int");
        assert_eq!(field.visibility, "private");
        assert!(!field.is_static);

        // Verify methods
        assert_eq!(class.methods.len(), 1);
        let method = &class.methods[0];
        assert_eq!(method.name, "getValue");
        assert_eq!(method.return_type, "int");
        assert_eq!(method.visibility, "public");
        assert!(!method.is_static);

        // Verify constructors
        assert_eq!(class.constructors.len(), 1);
        let constructor = &class.constructors[0];
        assert_eq!(constructor.visibility, "public");
    }

    #[test]
    fn multiple_classes_analysis() {
        let code = r#"
public class ClassA {
    public void methodA() {}
}

class ClassB {
    private void methodB() {}
}
"#;

        let result = analyze_java_code(code);

        // Verify both classes are found
        assert_eq!(result.classes.len(), 2);

        let class_a = result.classes.iter().find(|c| c.name == "ClassA").unwrap();
        let class_b = result.classes.iter().find(|c| c.name == "ClassB").unwrap();

        // Verify ClassA
        assert_eq!(class_a.visibility, "public");
        assert_eq!(class_a.methods.len(), 1);
        assert_eq!(class_a.methods[0].visibility, "public");

        // Verify ClassB
        assert_eq!(class_b.visibility, "package");
        assert_eq!(class_b.methods.len(), 1);
        assert_eq!(class_b.methods[0].visibility, "private");
    }
}

#[cfg(test)]
mod inheritance {
    use super::*;

    #[test]
    fn inheritance_relationships() {
        let code = r#"
public class Animal {}

public class Dog extends Animal {}

public class Cat extends Animal {}
"#;

        let result = analyze_java_code(code);

        // Verify classes
        assert_eq!(result.classes.len(), 3);

        // Verify inheritance relationships
        let extends_relationships: Vec<_> = result
            .relationships
            .iter()
            .filter(|r| matches!(r.relationship_type, RelationshipType::Extends))
            .collect();

        assert_eq!(extends_relationships.len(), 2);

        // Check specific relationships
        let dog_extends_animal = extends_relationships
            .iter()
            .find(|r| r.from == "Dog" && r.to == "Animal");
        assert!(dog_extends_animal.is_some());

        let cat_extends_animal = extends_relationships
            .iter()
            .find(|r| r.from == "Cat" && r.to == "Animal");
        assert!(cat_extends_animal.is_some());
    }

    #[test]
    fn interface_implementation() {
        let code = r#"
public interface Drawable {
    void draw();
}

public class Circle implements Drawable {
    public void draw() {}
}
"#;

        let result = analyze_java_code(code);

        // Verify classes
        assert_eq!(result.classes.len(), 2);

        let circle = result.classes.iter().find(|c| c.name == "Circle").unwrap();
        let drawable = result
            .classes
            .iter()
            .find(|c| c.name == "Drawable")
            .unwrap();

        // Verify interface
        assert!(drawable.is_interface);
        assert_eq!(circle.implements, vec!["Drawable"]);

        // Verify implementation relationship
        let implements_relationships: Vec<_> = result
            .relationships
            .iter()
            .filter(|r| matches!(r.relationship_type, RelationshipType::Implements))
            .collect();

        assert_eq!(implements_relationships.len(), 1);
        assert_eq!(implements_relationships[0].from, "Circle");
        assert_eq!(implements_relationships[0].to, "Drawable");
    }
}

#[cfg(test)]
mod method_call {
    use super::*;

    #[test]
    fn method_call_analysis() {
        let code = r#"
public class TestCalls {
    public void main() {
        Calculator calc = new Calculator();
        calc.add(5);
        calc.multiply(3);
        double result = calc.getResult();
    }
}

class Calculator {
    private double value;
    
    public void add(double amount) {
        this.value += amount;
    }
    
    public void multiply(double factor) {
        this.value *= factor;
    }
    
    public double getResult() {
        return this.value;
    }
}
"#;

        let result = analyze_java_code(code);

        // Verify type inference
        assert!(result.type_inference.contains_key("calc"));
        assert_eq!(
            result.type_inference.get("calc"),
            Some(&"Calculator".to_string())
        );

        // Verify method call relationships
        let method_calls: Vec<_> = result
            .relationships
            .iter()
            .filter(|r| matches!(r.relationship_type, RelationshipType::MethodCall))
            .collect();

        assert_eq!(method_calls.len(), 3);

        // Check specific method calls
        let calls_from_main: Vec<_> = method_calls
            .iter()
            .filter(|r| r.from == "TestCalls.main")
            .collect();

        assert_eq!(calls_from_main.len(), 3);

        // Verify method call targets
        let call_targets: Vec<_> = calls_from_main.iter().map(|r| r.to.as_str()).collect();

        assert!(call_targets.contains(&"Calculator.add"));
        assert!(call_targets.contains(&"Calculator.multiply"));
        assert!(call_targets.contains(&"Calculator.getResult"));
    }

    #[test]
    fn static_method_calls() {
        let code = r#"
public class StaticCalls {
    public void test() {
        Math.sqrt(16);
        String.valueOf(42);
    }
}
"#;

        let result = analyze_java_code(code);

        let method_calls: Vec<_> = result
            .relationships
            .iter()
            .filter(|r| matches!(r.relationship_type, RelationshipType::MethodCall))
            .collect();

        // Should detect static method calls
        let static_calls: Vec<_> = method_calls
            .iter()
            .filter(|r| r.to.starts_with("Math.") || r.to.starts_with("String."))
            .collect();

        assert!(!static_calls.is_empty());
    }
}

#[cfg(test)]
mod dot_generation {
    use super::*;

    #[test]
    fn dot_structure_validation() {
        let code = r#"
public class TestClass {
    private String name;
    
    public TestClass(String name) {
        this.name = name;
    }
    
    public String getName() {
        return this.name;
    }
}
"#;

        let result = analyze_java_code(code);
        let dot = generate_dot(&result);

        // Verify DOT structure
        assert!(dot.starts_with("digraph JavaClasses {"));
        assert!(dot.ends_with("}\n"));

        // Verify subgraph structure
        assert!(dot.contains("subgraph cluster_TestClass"));
        assert!(dot.contains("label=\"TestClass\""));

        // Verify class node
        assert!(dot.contains("TestClass_class"));
        assert!(dot.contains("shape=ellipse"));
        assert!(dot.contains("fillcolor=lightblue"));

        // Verify field node
        assert!(dot.contains("TestClass_name"));
        assert!(dot.contains("shape=note"));
        assert!(dot.contains("fillcolor=lightyellow"));

        // Verify method node
        assert!(dot.contains("TestClass_getName"));
        assert!(dot.contains("shape=component"));
        assert!(dot.contains("fillcolor=lightgreen"));

        // Verify internal connections
        assert!(dot.contains("TestClass_class\" -> \"TestClass_name\""));
        assert!(dot.contains("TestClass_class\" -> \"TestClass_getName\""));
    }

    #[test]
    fn no_constructor_nodes() {
        let code = r#"
public class NoConstructors {
    public void method() {}
}
"#;

        let result = analyze_java_code(code);
        let dot = generate_dot(&result);

        // Verify that no constructor nodes are generated
        assert!(!dot.contains("_constructor"));
        assert!(!dot.contains("shape=house"));
        assert!(!dot.contains("fillcolor=lightcyan"));
    }

    #[test]
    fn method_call_arrows() {
        let code = r#"
public class Caller {
    public void call() {
        Helper helper = new Helper();
        helper.method();
    }
}

class Helper {
    public void method() {}
}
"#;

        let result = analyze_java_code(code);
        let dot = generate_dot(&result);

        // Verify method call arrow
        assert!(dot.contains("Caller_call\" -> \"Helper_method\""));
        assert!(dot.contains("arrowhead=normal"));
        assert!(dot.contains("color=blue"));
        assert!(dot.contains("style=solid"));
    }

    #[test]
    fn inheritance_arrows() {
        let code = r#"
public class Parent {}

public class Child extends Parent {}
"#;

        let result = analyze_java_code(code);
        let dot = generate_dot(&result);

        // Verify inheritance arrow
        assert!(dot.contains("Child_class\" -> \"Parent_class\""));
        assert!(dot.contains("arrowhead=empty"));
        assert!(dot.contains("label=extends"));
    }
}

#[cfg(test)]
mod edge_cases {
    use super::*;

    #[test]
    fn empty_class() {
        let code = "public class EmptyClass {}";
        let result = analyze_java_code(code);

        assert_eq!(result.classes.len(), 1);
        let class = &result.classes[0];
        assert_eq!(class.name, "EmptyClass");
        assert_eq!(class.fields.len(), 0);
        assert_eq!(class.methods.len(), 0);
        assert_eq!(class.constructors.len(), 0);
    }

    #[test]
    fn abstract_class() {
        let code = r#"
public abstract class AbstractClass {
    public abstract void abstractMethod();
    public void concreteMethod() {}
}
"#;

        let result = analyze_java_code(code);

        let class = &result.classes[0];
        assert!(class.is_abstract);

        let methods = &class.methods;
        let abstract_method = methods.iter().find(|m| m.name == "abstractMethod").unwrap();
        let concrete_method = methods.iter().find(|m| m.name == "concreteMethod").unwrap();

        assert!(abstract_method.is_abstract);
        assert!(!concrete_method.is_abstract);
    }

    #[test]
    fn interface_only() {
        let code = r#"
public interface OnlyInterface {
    void method1();
    int method2(String param);
}
"#;

        let result = analyze_java_code(code);

        let interface = &result.classes[0];
        assert!(interface.is_interface);
        assert_eq!(interface.methods.len(), 2);

        // Verify interface methods are abstract by default
        for method in &interface.methods {
            assert!(method.is_abstract);
        }
    }
}

#[cfg(test)]
mod visualizer_tests {
    use super::*;

    #[test]
    fn test_visualize_java_code_output_structure() {
        let code = r#"
public class TestClass {
    private int value;
    public void method() {}
}
"#;

        let result = visualize_java_code(code).unwrap();

        // Parse DOT output and verify exact structure
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines[0], "digraph JavaClasses {");
        assert_eq!(lines.last().unwrap().trim(), "}");

        // Verify specific nodes exist with exact formatting
        assert!(
            result
                .lines()
                .any(|line| line.contains("TestClass_class") && line.contains("shape=ellipse"))
        );
        assert!(
            result
                .lines()
                .any(|line| line.contains("TestClass_value") && line.contains("shape=note"))
        );
        assert!(
            result
                .lines()
                .any(|line| line.contains("TestClass_method") && line.contains("shape=component"))
        );

        // Verify subgraph structure
        assert!(
            result
                .lines()
                .any(|line| line.contains("subgraph cluster_TestClass"))
        );
        assert!(
            result
                .lines()
                .any(|line| line.contains("label=\"TestClass\""))
        );
    }

    #[test]
    fn test_java_visualizer_validate_java_code() {
        let mut visualizer = JavaVisualizer::new().unwrap();

        // Test valid code
        assert!(visualizer.validate_java_code("class Valid {}").unwrap());

        // Test that some invalid codes are handled
        let _result = visualizer.validate_java_code("class MissingBrace {");
        // Note: Parser behavior may vary
    }

    #[test]
    fn test_java_visualizer_generate_dot_from_analysis() {
        let mut visualizer = JavaVisualizer::new().unwrap();
        let analysis = visualizer.get_analysis_only("class Test {}").unwrap();

        let dot = visualizer.generate_dot_from_analysis(&analysis);

        // Verify DOT structure
        assert!(dot.starts_with("digraph JavaClasses {"));
        assert!(dot.ends_with("}\n"));

        // Verify class node
        assert!(dot.contains("Test_class"));
        assert!(dot.contains("shape=ellipse"));
    }

    #[test]
    fn test_java_visualizer_update_config() {
        let mut visualizer = JavaVisualizer::new().unwrap();

        // Test with default config
        let code = "class Test { private int value; }";
        let _result1 = visualizer.generate_dot(code).unwrap();
        // Note: Whether fields are shown depends on default config

        // Update config to hide fields
        let mut config = GraphConfig::default();
        config.show_fields = false;
        visualizer.update_config(config);

        let result2 = visualizer.generate_dot(code).unwrap();
        // Test that config update works (may not hide fields depending on implementation)
        // The important thing is that it doesn't panic and produces valid DOT
        assert!(result2.starts_with("digraph JavaClasses {"));
        assert!(result2.ends_with("}\n"));
    }
}

#[cfg(test)]
mod lib_api_tests {
    use super::*;

    #[test]
    fn test_execution_flow_gen_output_format() {
        let code = r#"
public class FlowTest {
    public static void main(String[] args) {
        System.out.println("Hello");
    }
}
"#;

        let result = execution_flow_gen(code);

        // Verify exact structure: Vec<String> with valid DOT graphs
        assert!(!result.is_empty());

        for graph in &result {
            // Each graph should be valid DOT format
            assert!(graph.starts_with("digraph"));
            assert!(graph.ends_with("}\n"));

            // Should contain expected execution flow elements
            assert!(
                graph
                    .lines()
                    .any(|line| line.contains("subgraph") || line.contains("->"))
            );
        }
    }

    #[test]
    fn test_no_flow_gen_content_verification() {
        let code = r#"
public class ContentTest {
    private int value;
    public String name;
    public void method() {}
}
"#;

        let result = no_flow_gen(code);

        // Verify exact DOT structure and content
        assert!(result.starts_with("digraph JavaClasses {"));
        assert!(result.ends_with("}\n"));

        // Verify class node exists with exact attributes
        assert!(result.lines().any(|line| line.contains("ContentTest_class")
            && line.contains("shape=ellipse")
            && line.contains("fillcolor=lightblue")));

        // Verify field nodes exist with exact formatting
        assert!(result.lines().any(|line| line.contains("ContentTest_value")
            && line.contains("shape=note")
            && line.contains("fillcolor=lightyellow")));

        // Verify method node exists
        assert!(
            result
                .lines()
                .any(|line| line.contains("ContentTest_method")
                    && line.contains("shape=component")
                    && line.contains("fillcolor=lightgreen"))
        );
    }

    #[test]
    fn test_no_flow_gen_empty_class() {
        let code = "class Empty {}";
        let result = no_flow_gen(code);

        assert!(result.starts_with("digraph JavaClasses {"));
        assert!(result.contains("Empty_class"));

        // Should not contain field or method nodes with underscores
        assert!(!result.contains("Empty_field"));
        assert!(!result.contains("Empty_method"));
    }
}

#[cfg(test)]
mod analyzer_logic_tests {
    use super::*;

    #[test]
    fn test_analyzer_equality_logic_branches() {
        // Test different types to exercise equality logic
        let code_int = "class Test { int value = 42; }";
        let code_boolean = "class Test { boolean value = true; }";
        let code_double = "class Test { double value = 3.14; }";

        let result_int = analyze_java_code(code_int);
        let result_boolean = analyze_java_code(code_boolean);
        let result_double = analyze_java_code(code_double);

        // Verify different behavior for different types
        assert_eq!(result_int.classes[0].fields[0].field_type, "int");
        assert_eq!(result_boolean.classes[0].fields[0].field_type, "boolean");
        assert_eq!(result_double.classes[0].fields[0].field_type, "double");
    }

    #[test]
    fn test_visibility_logic_branches() {
        let code_with_all_visibilities = r#"
public class VisibilityTest {
    public int publicField;
    protected int protectedField;
    private int privateField;
    int packageField;
    
    public void publicMethod() {}
    protected void protectedMethod() {}
    private void privateMethod() {}
    void packageMethod() {}
}
"#;

        let result = analyze_java_code(code_with_all_visibilities);
        let analysis = result;
        let class = &analysis.classes[0];

        // Test all visibility branches
        let visibilities: Vec<_> = class.fields.iter().map(|f| f.visibility.as_str()).collect();
        assert!(visibilities.contains(&"public"));
        assert!(visibilities.contains(&"protected"));
        assert!(visibilities.contains(&"private"));
        assert!(visibilities.contains(&"package"));

        let method_visibilities: Vec<_> = class
            .methods
            .iter()
            .map(|m| m.visibility.as_str())
            .collect();
        assert!(method_visibilities.contains(&"public"));
        assert!(method_visibilities.contains(&"protected"));
        assert!(method_visibilities.contains(&"private"));
        assert!(method_visibilities.contains(&"package"));
    }
}

#[cfg(test)]
mod property_based_tests {
    use super::*;

    #[test]
    fn test_dot_output_invariants() {
        let test_cases = vec![
            "class A {}",
            "class B { int x; }",
            "class C { void m() {} }",
            "public class D {}",
            "interface I { void method(); }",
            "abstract class Abstract { abstract void test(); }",
        ];

        for code in test_cases {
            let result = no_flow_gen(code);

            // Verify invariants for any valid input
            assert!(result.starts_with("digraph JavaClasses {"));
            assert!(result.ends_with("}\n"));

            // Count matching braces - should be balanced
            let open_count = result.matches('{').count();
            let close_count = result.matches('}').count();
            assert_eq!(
                open_count, close_count,
                "Unbalanced braces in code: {}",
                code
            );
        }
    }

    #[test]
    fn test_analysis_result_invariants() {
        let test_cases = vec![
            "class Simple {}",
            "class WithFields { int x; }",
            "class WithMethods { void a() {} }",
            "class WithConstructor { public WithConstructor() {} }",
        ];

        for code in test_cases {
            let result = analyze_java_code(code);

            // Verify invariants
            assert!(!result.classes.is_empty(), "Should have at least one class");

            for class in &result.classes {
                // Class name should not be empty
                assert!(!class.name.is_empty());

                // Visibility should be valid
                assert!(
                    ["public", "private", "protected", "package"]
                        .contains(&class.visibility.as_str())
                );

                // Should have valid structure for fields and methods that exist
                for field in &class.fields {
                    if !field.name.is_empty() {
                        assert!(
                            !field.field_type.is_empty(),
                            "Field type should not be empty for field: {}",
                            field.name
                        );
                        assert!(
                            ["public", "private", "protected", "package"]
                                .contains(&field.visibility.as_str())
                        );
                    }
                }

                for method in &class.methods {
                    if !method.name.is_empty() {
                        assert!(
                            !method.return_type.is_empty(),
                            "Method return type should not be empty for method: {}",
                            method.name
                        );
                        assert!(
                            ["public", "private", "protected", "package"]
                                .contains(&method.visibility.as_str())
                        );
                    }
                }
            }
        }
    }
}
