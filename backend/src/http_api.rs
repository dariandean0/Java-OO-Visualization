use serde::{Deserialize, Serialize};

use crate::{api::compare_from_code_and_student, mistake::Mistake, repr::Diagram};

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
