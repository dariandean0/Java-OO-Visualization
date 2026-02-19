use std::collections::HashMap;

use crate::mistake::{Mistake, MistakeKind};
use crate::model::{Diagram, Relationship};

/// Compare the correct diagram and the student drawn diagran.
/// and return a list of mistakes found in the student diagram.
pub fn analyze_mistakes(correct: &Diagram, student: &Diagram) -> Vec<Mistake> {
    let mut mistakes = Vec::new();

    compare_classes(correct, student, &mut mistakes);
    compare_relationships(correct, student, &mut mistakes);
    compare_methods(correct, student, &mut mistakes);
    mistakes
}

fn compare_classes(correct: &Diagram, student: &Diagram, mistakes: &mut Vec<Mistake>) {
    let correct_names: HashMap<_, _> = correct
        .classes
        .iter()
        .map(|c| (c.name.clone(), ()))
        .collect();

    let student_names: HashMap<_, _> = student
        .classes
        .iter()
        .map(|c| (c.name.clone(), ()))
        .collect();

    for name in correct_names.keys() {
        if !student_names.contains_key(name) {
            mistakes.push(Mistake {
                kind: MistakeKind::MissingClass,
                message: format!("Class '{}' is missing from your diagram.", name),
                related_elements: vec![name.clone()],
            });
        }
    }

    for name in student_names.keys() {
        if !correct_names.contains_key(name) {
            mistakes.push(Mistake {
                kind: MistakeKind::ExtraClass,
                message: format!(
                    "Class '{}' is not present in the code but appears in your diagram.",
                    name
                ),
                related_elements: vec![name.clone()],
            });
        }
    }
}

fn key(rel: &Relationship) -> (String, String) {
    (rel.from.clone(), rel.to.clone())
}

fn compare_relationships(correct: &Diagram, student: &Diagram, mistakes: &mut Vec<Mistake>) {
    let correct_map: HashMap<_, _> = correct.relationships.iter().map(|r| (key(r), r)).collect();

    let student_map: HashMap<_, _> = student.relationships.iter().map(|r| (key(r), r)).collect();

    for (k, rel) in &correct_map {
        if !student_map.contains_key(k) {
            mistakes.push(Mistake {
                kind: MistakeKind::MissingRelationship,
                message: format!(
                    "Relationship from '{}' to '{}' is missing.",
                    rel.from, rel.to
                ),
                related_elements: vec![rel.from.clone(), rel.to.clone()],
            });
        }
    }

    for (k, rel) in &student_map {
        if !correct_map.contains_key(k) {
            mistakes.push(Mistake {
                kind: MistakeKind::ExtraRelationship,
                message: format!(
                    "Extra relationship from '{}' to '{}' appears in your diagram.",
                    rel.from, rel.to
                ),
                related_elements: vec![rel.from.clone(), rel.to.clone()],
            });
        }
    }

    for (k, correct_rel) in &correct_map {
        if let Some(student_rel) = student_map.get(k)
            && correct_rel.kind != student_rel.kind
        {
            mistakes.push(Mistake {
                kind: MistakeKind::WrongRelationshipType,
                message: format!(
                    "Relationship between '{}' and '{}' should be {:?}, not {:?}.",
                    correct_rel.from, correct_rel.to, correct_rel.kind, student_rel.kind
                ),
                related_elements: vec![correct_rel.from.clone(), correct_rel.to.clone()],
            });
        }
    }
}
fn compare_methods(correct: &Diagram, student: &Diagram, mistakes: &mut Vec<Mistake>) {
    for correct_class in &correct.classes {
        // Find the student class with the same name
        let student_class = student
            .classes
            .iter()
            .find(|c| c.name == correct_class.name);

        let student_methods = match student_class {
            Some(c) => c.methods.iter().map(|m| m.name.clone()).collect(),
            None => Vec::<String>::new(),
        };

        for method in &correct_class.methods {
            if !student_methods.contains(&method.name) {
                mistakes.push(Mistake::missing_method(&correct_class.name, &method.name));
            }
        }
    }
}
