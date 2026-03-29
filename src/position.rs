#[derive(Debug, Clone)]
pub struct Position {
    pub index: usize,
    pub line: usize,
    pub column: usize,
    pub file_name: String,
    pub file_text: String,
}

impl Position {
    pub fn new(index: usize, line: usize, column: usize, file_name: &str, file_text: &str) -> Self {
        Self {
            index,
            line,
            column,
            file_name: file_name.to_string(),
            file_text: file_text.to_string(),
        }
    }

    pub fn advance(&mut self, current_char: Option<char>) {
        self.index += 1;
        self.column += 1;

        if current_char == Some('\n') {
            self.line += 1;
            self.column = 0;
        }
    }

    // Return a copy of the position
    pub fn copy(&self) -> Self {
        self.clone()
    }
}
