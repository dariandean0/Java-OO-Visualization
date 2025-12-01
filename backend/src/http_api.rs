use serde::{Deserialize, Serialize};

use crate::{Diagram, Mistake, compare_from_code_and_student};

#[derive(Debug, Deserialize)]
pub struct CompareRequest {
    pub source_code: String,
    pub student_diagram: Diagram,
}

#[derive(Debug, Serialize)]
pub struct CompareResponse {
    pub mistakes: Vec<Mistake>,
}

pub fn handle_compare(req: CompareRequest) -> CompareResponse {
    let mistakes = compare_from_code_and_student(&req.source_code, &req.student_diagram);
    CompareResponse { mistakes }
}
