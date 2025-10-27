mod analyzer;
mod execution_flow;
mod no_flow;
mod parser;
mod visualizer;

use no_flow::GraphGenerator;
use parser::JavaParser;
use analyzer::JavaAnalyzer;
//use wasm_bindgen::prelude::*;
pub fn execution_flow_gen(java_code: &str) -> Vec<String> {
    use execution_flow::{ExecutionAnalyzer, ExecutionGraphGenerator};

    let mut parser = JavaParser::new().unwrap();
    let tree = parser.parse(&java_code).unwrap();
    let root = parser.get_root_node(&tree);

    let mut analyzer = JavaAnalyzer::new();
    let analysis = analyzer.analyze(&root, &java_code);

    let mut exec_analyzer = ExecutionAnalyzer::new(analysis);
    let flow = exec_analyzer.analyze_execution_flow(&root, &java_code);

    let generator = ExecutionGraphGenerator::new();
    let graphs = generator.generate_execution_graphs(&flow);

    graphs.into_iter().map(|g| g.dot_code).collect()
}

pub fn no_flow_gen(java_code: &str) -> String {
    let mut parser = JavaParser::new().unwrap();
    let tree = parser.parse(&java_code).unwrap();
    let root = parser.get_root_node(&tree);

    let mut analyzer = JavaAnalyzer::new();
    let analysis = analyzer.analyze(&root, &java_code);

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
use std::slice;
use std::str;
use crate::{execution_flow_gen, no_flow_gen, visualizer};

fn ptr_to_str<'a>(ptr: *const u8, len: usize) -> &'a str {
    let bytes = unsafe { slice::from_raw_parts(ptr, len) };
    str::from_utf8(bytes).unwrap_or("")
}
fn to_c_string(s: String) -> *mut c_char {
    CString::new(s).unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn wasm_execution_flow_gen(ptr: *const u8, len: usize) -> *mut c_char {
    let java_code = ptr_to_str(ptr, len);
    let vec = execution_flow_gen(java_code);
    let json = serde_json::to_string(&vec).unwrap_or_else(|e| format!("Error serializing: {}", e));
    to_c_string(json);
}

#[no_mangle]
pub extern "C" fn wasm_no_flow_gen(ptr: *const u8, len: usize) -> *mut c_char {
    let java_code = ptr_to_str(ptr, len);
    let result = no_flow_gen(java_code);
    to_c_string(result)
}

#[no_mangle]
pub extern "C" fn wasm_visualize_java_code(ptr: *const u8, len: usize) -> *mut c_char {
    let java_code = ptr_to_str(ptr, len);
    let result = match visualizer::visualize_java_code(java_code) {
        Ok(output) => output,
        Err(e) => format!("Error: {}", e),
    };
    to_c_string(result)
}

