use std::io::Write;

use backend::*;

fn main() {
    let files = std::env::args()
        .skip(1)
        .map(|x| std::fs::read_to_string(x))
        .collect::<Result<Vec<String>, _>>()
        .expect("no work");
    let java_code = files
        .into_iter()
        .reduce(|mut old, new| {
            old += "\n";
            old += &new;
            old
        })
        .expect("no work concat");

    let mut parser = JavaParser::new().unwrap();
    let tree = parser.parse(&java_code).unwrap();
    let root = parser.get_root_node(&tree);

    let mut analyzer = JavaAnalyzer::new();
    let analysis = analyzer.analyze(&root, &java_code);

    let mut exec_analyzer = ExecutionAnalyzer::new(analysis);
    let flow = exec_analyzer.analyze_execution_flow(&root, &java_code);

    let generator = ExecutionGraphGenerator::new();
    let graphs = generator.generate_execution_graphs(&flow);

    for (i, graph) in graphs.into_iter().enumerate() {
        let mut file =
            std::fs::File::create_new(format!("graph_{}.dot", i)).expect("unable to create file");
        file.write_all(graph.dot_code.as_bytes())
            .expect("unable to write");
        file.flush().expect("unable to flush");
    }
}
