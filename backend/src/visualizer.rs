use crate::analyzer::{AnalysisResult, JavaAnalyzer};
use crate::graph_generator::{GraphConfig, GraphGenerator};
use crate::parser::JavaParser;
use crate::execution_analyzer::{ExecutionAnalyzer, ExecutionFlow};
use crate::execution_graph_generator::{ExecutionGraphGenerator, ExecutionGraphConfig, ExecutionGraphStep};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualizationResult {
    pub dot_code: String,
    pub analysis: AnalysisResult,
    pub step_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionVisualizationResult {
    pub execution_flow: ExecutionFlow,
    pub static_analysis: AnalysisResult,
    pub execution_graphs: Vec<ExecutionGraphStep>,
}

pub struct JavaVisualizer {
    parser: JavaParser,
    analyzer: JavaAnalyzer,
    graph_generator: GraphGenerator,
}

impl JavaVisualizer {
    pub fn new() -> Result<Self> {
        let parser = JavaParser::new().context("Failed to create JavaParser")?;
        let analyzer = JavaAnalyzer::new();
        let graph_generator = GraphGenerator::new();

        Ok(JavaVisualizer {
            parser,
            analyzer,
            graph_generator,
        })
    }

    pub fn with_config(config: GraphConfig) -> Result<Self> {
        let parser = JavaParser::new().context("Parsing Error")?;
        let analyzer = JavaAnalyzer::new();
        let graph_generator = GraphGenerator::with_config(config);

        Ok(JavaVisualizer {
            parser,
            analyzer,
            graph_generator,
        })
    }

    #[wasm_bindgen]
    pub fn generate_dot(&mut self, java_code: &str) -> Result<String> {
        let result = self.analyze_and_generate(java_code)?;
        Ok(result.dot_code)
    }

    pub fn analyze_and_generate(&mut self, java_code: &str) -> Result<VisualizationResult> {
        // Parse the Java code
        let tree = self.parser.parse(java_code).context("ParseError")?;

        let root_node = self.parser.get_root_node(&tree);

        // Analyze the AST
        let analysis = self.analyzer.analyze(&root_node, java_code);

        // Generate the dot code
        let dot_code = self.graph_generator.generate_dot(&analysis);

        Ok(VisualizationResult {
            dot_code,
            analysis,
            step_count: 1,
        })
    }

    pub fn get_analysis_only(&mut self, java_code: &str) -> Result<AnalysisResult> {
        let tree = self
            .parser
            .parse(java_code)
            .context("Visualization Error")?;

        let root_node = self.parser.get_root_node(&tree);
        let analysis = self.analyzer.analyze(&root_node, java_code);

        Ok(analysis)
    }

    pub fn generate_dot_from_analysis(&self, analysis: &AnalysisResult) -> String {
        self.graph_generator.generate_dot(analysis)
    }

    pub fn update_config(&mut self, config: GraphConfig) {
        self.graph_generator = GraphGenerator::with_config(config);
    }

    pub fn validate_java_code(&mut self, java_code: &str) -> Result<bool> {
        self.parser
            .parse(java_code)
            .context("Parse Error")
            .map(|_| true)
    }

    /// Analyze execution flow starting from main method
    pub fn analyze_execution_flow(&mut self, java_code: &str) -> Result<ExecutionVisualizationResult> {
        // First do static analysis
        let tree = self.parser.parse(java_code).context("Failed to parse Java code")?;
        let root_node = self.parser.get_root_node(&tree);
        let static_analysis = self.analyzer.analyze(&root_node, java_code);

        // Then do execution flow analysis
        let mut execution_analyzer = ExecutionAnalyzer::new(static_analysis.clone());
        let execution_flow = execution_analyzer.analyze_execution_flow(&root_node, java_code);

        // Generate step-by-step execution graphs
        let graph_generator = ExecutionGraphGenerator::new();
        let execution_graphs = graph_generator.generate_execution_graphs(&execution_flow);

        Ok(ExecutionVisualizationResult {
            execution_flow,
            static_analysis,
            execution_graphs,
        })
    }

    /// Analyze execution flow with custom configuration
    pub fn analyze_execution_flow_with_config(
        &mut self,
        java_code: &str,
        execution_config: ExecutionGraphConfig,
    ) -> Result<ExecutionVisualizationResult> {
        // First do static analysis
        let tree = self.parser.parse(java_code).context("Failed to parse Java code")?;
        let root_node = self.parser.get_root_node(&tree);
        let static_analysis = self.analyzer.analyze(&root_node, java_code);

        // Then do execution flow analysis
        let mut execution_analyzer = ExecutionAnalyzer::new(static_analysis.clone());
        let execution_flow = execution_analyzer.analyze_execution_flow(&root_node, java_code);

        // Generate step-by-step execution graphs with custom config
        let graph_generator = ExecutionGraphGenerator::with_config(execution_config);
        let execution_graphs = graph_generator.generate_execution_graphs(&execution_flow);

        Ok(ExecutionVisualizationResult {
            execution_flow,
            static_analysis,
            execution_graphs,
        })
    }

    /// Generate only execution flow without graphs (for performance)
    pub fn get_execution_flow_only(&mut self, java_code: &str) -> Result<ExecutionFlow> {
        let tree = self.parser.parse(java_code).context("Failed to parse Java code")?;
        let root_node = self.parser.get_root_node(&tree);
        let static_analysis = self.analyzer.analyze(&root_node, java_code);

        let mut execution_analyzer = ExecutionAnalyzer::new(static_analysis);
        Ok(execution_analyzer.analyze_execution_flow(&root_node, java_code))
    }
}

#[allow(dead_code)]
pub fn visualize_java_code(java_code: &str) -> Result<String> {
    let mut visualizer = JavaVisualizer::new()?;
    visualizer.generate_dot(java_code)
}

#[allow(dead_code)]
pub fn visualize_java_code_with_config(java_code: &str, config: GraphConfig) -> Result<String> {
    let mut visualizer = JavaVisualizer::with_config(config)?;
    visualizer.generate_dot(java_code)
}

#[allow(dead_code)]
pub fn analyze_java_code(java_code: &str) -> Result<AnalysisResult> {
    let mut visualizer = JavaVisualizer::new()?;
    visualizer.get_analysis_only(java_code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_visualization() {
        let java_code = r#"
            public class TestClass {
                private String name;

                public TestClass(String name) {
                    this.name = name;
                }

                public String getName() {
                    return name;
                }
            }
        "#;

        let result = visualize_java_code(java_code);
        assert!(result.is_ok());

        let dot_code = result.unwrap();
        assert!(dot_code.contains("digraph JavaClasses"));
        assert!(dot_code.contains("TestClass"));
    }

    #[test]
    fn test_inheritance_visualization() {
        let java_code = r#"
            public abstract class Animal {
                protected String name;
                public abstract void makeSound();
            }

            public class Dog extends Animal {
                public void makeSound() {
                    System.out.println("Woof!");
                }
            }
        "#;

        let mut visualizer = JavaVisualizer::new().unwrap();
        let result = visualizer.analyze_and_generate(java_code);

        assert!(result.is_ok());
        let vis_result = result.unwrap();

        assert_eq!(vis_result.analysis.classes.len(), 2);
        assert!(!vis_result.analysis.relationships.is_empty());
        assert!(vis_result.dot_code.contains("Animal"));
        assert!(vis_result.dot_code.contains("Dog"));
    }

    #[test]
    fn test_interface_visualization() {
        let java_code = r#"
            public interface Drawable {
                void draw();
            }

            public class Circle implements Drawable {
                private double radius;

                public void draw() {
                    // Implementation
                }
            }
        "#;

        let result = visualize_java_code(java_code);
        assert!(result.is_ok());

        let dot_code = result.unwrap();
        assert!(dot_code.contains("interface"));
        assert!(dot_code.contains("Circle"));
        assert!(dot_code.contains("Drawable"));
    }

    #[test]
    fn test_custom_config() {
        let java_code = r#"
            public class TestClass {
                private String name;
                public String getName() { return name; }
            }
        "#;

        let config = GraphConfig {
            show_fields: false,
            show_methods: true,
            show_private_members: false,
            ..Default::default()
        };

        let result = visualize_java_code_with_config(java_code, config);
        assert!(result.is_ok());
    }
}
