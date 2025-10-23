mod analyzer;
mod execution_flow;
mod no_flow;
mod parser;
mod visualizer;

use no_flow::GraphGenerator;
use parser::JavaParser;

use analyzer::JavaAnalyzer;
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
