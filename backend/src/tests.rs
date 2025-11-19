use crate::analyzer::{AnalysisResult, JavaAnalyzer, RelationshipType};
use crate::no_flow::GraphGenerator;
use crate::parser::JavaParser;

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
