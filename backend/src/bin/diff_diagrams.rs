use std::collections::HashSet;
use std::fs;
use std::process::Command;


fn main() {
   
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: cargo run --bin diff_diagrams <file1.dot> <file2.dot>");
        return;
    }

    let dot1 = fs::read_to_string(&args[1]).expect("Failed to read first DOT file");
    let dot2 = fs::read_to_string(&args[2]).expect("Failed to read second DOT file");

    let set1: HashSet<_> = dot1.lines().map(|l| l.trim().to_string()).collect();
    let set2: HashSet<_> = dot2.lines().map(|l| l.trim().to_string()).collect();

    let mut output = String::from("digraph diff {\n");

 
    for line in &set1 {
        if line.contains("->") {
            let formatted = line.trim_end_matches(';').to_string();
            if !set2.contains(line) {
                output.push_str(&format!("{} [color=gray];\n", formatted));
            } else {
                output.push_str(&format!("{} [color=black];\n", formatted));
            }
        }
    }

    for line in &set2 {
        if line.contains("->") {
            let formatted = line.trim_end_matches(';').to_string();
            if !set1.contains(line) {
                output.push_str(&format!("{} [color=red];\n", formatted));
            }
        }
    }

    output.push_str("}\n");

    fs::write("diff.dot", &output).expect("Failed to write diff.dot");

 
    let result = Command::new("dot")
        .args(["-Tpng", "diff.dot", "-o", "diff.png"])
        .status();

    match result {
        Ok(status) if status.success() => {
            println!("Generated diff.png");
        }
        Ok(_) | Err(_) => {
            eprintln!("Failed to generate diff.png");
        }
    }
}

