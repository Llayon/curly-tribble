use std::fs;

pub struct CodeSniffer {
    pub raw: String,
    pub clean: String,
}

impl CodeSniffer {
    pub fn new(path: &str) -> Self {
        let raw = fs::read_to_string(path).expect(&format!("Critical Error: Could not read file at {}", path));
        
        // Очищаем код для анализа
        let clean = raw.lines()
            .map(|line| line.split("//").next().unwrap_or("").trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
            
        Self { raw, clean }
    }

    pub fn contains_call(&self, pattern: &str) -> bool {
        self.clean.contains(pattern)
    }

    pub fn count_occurrences(&self, pattern: &str) -> usize {
        self.clean.split(pattern).count() - 1
    }
}

pub fn is_data_only(clean_code: &str) -> bool {
    // Если в файле нет ни одной функции (fn), он считается файлом данных/модулей
    !clean_code.contains("fn ") && !clean_code.contains("pub fn ")
}
