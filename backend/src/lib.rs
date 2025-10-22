mod analyzer;
mod execution_analyzer;
mod execution_graph_generator;
mod graph_generator;
mod integration_tests;
mod parser;
mod visualizer;

pub use crate::analyzer::JavaAnalyzer;
pub use crate::execution_analyzer::{ExecutionAnalyzer, ExecutionFlow};
pub use crate::execution_graph_generator::{ExecutionGraphConfig, ExecutionGraphGenerator};
pub use crate::parser::JavaParser;
pub use crate::visualizer::{visualize_java_code, ExecutionVisualizationResult, JavaVisualizer};
