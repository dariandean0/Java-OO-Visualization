pub mod analyzer;
pub mod execution_flow;
pub mod no_flow;
pub mod parser;
pub mod visualizer;

pub mod api;
pub mod compare;
pub mod mistake;
pub mod repr;

#[cfg(test)]
mod tests;

use analyzer::JavaAnalyzer;
use no_flow::GraphGenerator;
use parser::JavaParser;

//use wasm_bindgen::prelude::*;
pub fn execution_flow_gen(java_code: &str) -> Vec<String> {
    let mut visualizer = match visualizer::JavaVisualizer::new() {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    let result = match visualizer.analyze_execution_flow(java_code) {
        Ok(r) => r,
        Err(_) => return vec![],
    };
    result
        .execution_graphs
        .into_iter()
        .map(|g| g.dot_code)
        .collect()
}

pub fn no_flow_gen(java_code: &str) -> String {
    let mut parser = match JavaParser::new() {
        Ok(p) => p,
        Err(e) => return format!("Error: {}", e),
    };
    let tree = match parser.parse(java_code) {
        Ok(t) => t,
        Err(e) => return format!("Error: {}", e),
    };
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

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        execution_flow_gen(java_code)
    }));

    let json = match result {
        Ok(vec) => {
            serde_json::to_string(&vec).unwrap_or_else(|e| format!("Error serializing: {}", e))
        }
        Err(_) => "[]".to_string(),
    };
    to_c_string(json)
}

#[unsafe(no_mangle)]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn wasm_no_flow_gen(ptr: *const c_char) -> *mut c_char {
    let c_str = unsafe { std::ffi::CStr::from_ptr(ptr) };
    let java_code = c_str.to_str().unwrap_or("");

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        no_flow_gen(java_code)
    }));

    let output = match result {
        Ok(s) => s,
        Err(_) => "digraph JavaClasses { }".to_string(),
    };
    to_c_string(output)
}

#[unsafe(no_mangle)]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn wasm_visualize_java_code(ptr: *const c_char) -> *mut c_char {
    let c_str = unsafe { std::ffi::CStr::from_ptr(ptr) };
    let java_code = c_str.to_str().unwrap_or("");

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        match visualizer::visualize_java_code(java_code) {
            Ok(output) => output,
            Err(e) => format!("Error: {}", e),
        }
    }));

    let output = match result {
        Ok(s) => s,
        Err(_) => "digraph JavaClasses { }".to_string(),
    };
    to_c_string(output)
}
