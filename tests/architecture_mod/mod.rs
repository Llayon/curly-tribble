mod utils;
mod main_guards;

// Реэкспорт тестов для того, чтобы они запускались как один бинарь architecture_mod
pub use main_guards::*;
