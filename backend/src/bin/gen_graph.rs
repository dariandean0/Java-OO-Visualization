use backend::*;
use std::io::Write;

fn main() {
    let files = std::env::args()
        .skip(1)
        .map(std::fs::read_to_string)
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

    let graph = no_flow_gen(&java_code);
    let mut file = std::fs::File::create_new("graph.dot").expect("unable to create file");
    file.write_all(graph.as_bytes()).expect("unable to write");
    file.flush().expect("unable to flush");
}
