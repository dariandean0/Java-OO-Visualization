use std::collections::HashMap;

use crate::{Diagram, Mistake, analyze_mistakes};
use crate::model::{Class, Method, Relationship, RelationshipKind};
use regex::Regex;

/// Build a "correct" Diagram from the source code.
///
/// Right now this is a lightweight, comparer-local parser:
/// - Finds class names
/// - For each class, finds method names inside it
///
/// It does NOT touch or depend on the real GraphGenerator / tree-sitter parser.
/// That keeps your part isolated from the main parser work.
fn diagram_from_code(source: &str) -> Diagram {
    // Matches: "class Animal" or "abstract class Animal"
    let class_re =
        Regex::new(r"\b(?:abstract\s+)?class\s+([A-Za-z_][A-Za-z0-9_]*)").unwrap();

    // Very simple Java-like method pattern:
    // [visibility] [static] [return_type] methodName(
    //
    // Examples it will match:
    //   public void makeSound(
    //   private int getAge(
    //   protected static String foo(
    let method_re = Regex::new(
        r"\b(?:public|private|protected)?\s*(?:static\s+)?[A-Za-z0-9_<>\[\]]+\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(",
    )
    .unwrap();

    // Map from class name -> list of methods
    let mut classes: HashMap<String, Vec<Method>> = HashMap::new();
    let mut current_class: Option<String> = None;

    for line in source.lines() {
        // 1) Detect class declarations
        if let Some(cap) = class_re.captures(line) {
            let class_name = cap[1].to_string();
            classes.entry(class_name.clone()).or_insert_with(Vec::new);
            current_class = Some(class_name);
            continue;
        }

        // 2) If we're "inside" a class, look for method signatures
        if let Some(ref class_name) = current_class {
            if let Some(cap) = method_re.captures(line) {
                let method_name = cap[1].to_string();
                if let Some(methods) = classes.get_mut(class_name) {
                    methods.push(Method { name: method_name });
                }
            }
        }
    }

    // Convert HashMap<String, Vec<Method>> into our Diagram model
    let class_vec: Vec<Class> = classes
        .into_iter()
        .map(|(name, methods)| Class { name, methods })
        .collect();

    Diagram {
        classes: class_vec,
        // Relationships will be added later when you decide how to parse them
        relationships: Vec::new(),
    }
}

/// Public function used by CLI / HTTP: compare code vs student diagram.
pub fn compare_from_code_and_student(source_code: &str, student: &Diagram) -> Vec<Mistake> {
    let correct = diagram_from_code(source_code);
    analyze_mistakes(&correct, student)
}


