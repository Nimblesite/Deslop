//! Function info — name extraction for symbol catalog.

pub struct FunctionInfo {
    pub name: String,
    pub arity: u8,
}

pub fn function_names(functions: &[FunctionInfo]) -> Vec<&str> {
    let names: Vec<&str> = functions.iter().map(|func| func.name.as_str()).collect();
    names
}

pub fn snake_case_names(functions: &[FunctionInfo]) -> Vec<String> {
    let snake: Vec<String> = functions
        .iter()
        .map(|func| func.name.to_lowercase())
        .collect();
    snake
}
