pub mod analyzer;
pub mod execution_flow;
pub mod no_flow;
pub mod parser;
pub mod visualizer;

pub mod api;
pub mod compare;
pub mod http_api;
pub mod mistake;
pub mod model;

#[cfg(test)]
mod tests;

use analyzer::JavaAnalyzer;
pub use api::compare_from_code_and_student;
pub use compare::analyze_mistakes;
pub use http_api::{CompareRequest, CompareResponse, handle_compare};
pub use mistake::{Mistake, MistakeKind};
pub use model::{Class, Diagram, Relationship, RelationshipKind};
use no_flow::GraphGenerator;
use parser::JavaParser;
//use wasm_bindgen::prelude::*;
pub fn execution_flow_gen(java_code: &str) -> Vec<String> {
    let mut visualizer = visualizer::JavaVisualizer::new().unwrap();
    let result = visualizer.analyze_execution_flow(java_code).unwrap();
    result
        .execution_graphs
        .into_iter()
        .map(|g| g.dot_code)
        .collect()
}

pub fn no_flow_gen(java_code: &str) -> String {
    let mut parser = JavaParser::new().unwrap();
    let tree = parser.parse(java_code).unwrap();
    let root = parser.get_root_node(&tree);

    let mut analyzer = JavaAnalyzer::new();
    let analysis = analyzer.analyze(&root, java_code);

    let generator = GraphGenerator::new();
    generator.generate_dot(&analysis)
}

/*
// WASM-compatible exports
#[wasm_bindgen]
pub fn wasm_execution_flow_gen(java_code: &str) -> String {
    match execution_flow_gen(java_code) {
        vec => serde_json::to_string(&vec).unwrap_or_else(|e| format!("Error serializing: {}", e)),
    }
}

#[wasm_bindgen]
pub fn wasm_no_flow_gen(java_code: &str) -> String {
    no_flow_gen(java_code)
}

#[wasm_bindgen]
pub fn wasm_visualize_java_code(java_code: &str) -> String {
    match visualizer::visualize_java_code(java_code) {
        Ok(result) => result,
        Err(e) => format!("Error: {}", e),
    }
}
*/

//emscripten compatible exports
use std::ffi::CString;
use std::os::raw::c_char;
use std::str;

fn to_c_string(s: String) -> *mut c_char {
    CString::new(s).unwrap().into_raw()
}

#[unsafe(no_mangle)]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn wasm_execution_flow_gen(ptr: *const c_char) -> *mut c_char {
    let c_str = unsafe { std::ffi::CStr::from_ptr(ptr) };
    let java_code = c_str.to_str().unwrap_or("");
    let vec = execution_flow_gen(java_code);
    let json = serde_json::to_string(&vec).unwrap_or_else(|e| format!("Error serializing: {}", e));
    to_c_string(json)
}

#[unsafe(no_mangle)]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn wasm_no_flow_gen(ptr: *const c_char) -> *mut c_char {
    let c_str = unsafe { std::ffi::CStr::from_ptr(ptr) };
    let java_code = c_str.to_str().unwrap_or("");
    let result = no_flow_gen(java_code);
    to_c_string(result)
}

#[unsafe(no_mangle)]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn wasm_visualize_java_code(ptr: *const c_char) -> *mut c_char {
    //let java_code = ptr_to_str(ptr, len);

    let c_str = unsafe { std::ffi::CStr::from_ptr(ptr) };
    let java_code = c_str.to_str().unwrap_or("");
    eprintln!("Received Java code: {:?}", java_code);
    let result = match visualizer::visualize_java_code(java_code) {
        Ok(output) => output,
        Err(e) => format!("Error: {}", e),
    };
    to_c_string(result)
}
