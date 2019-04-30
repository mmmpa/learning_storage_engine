use std::fmt::Debug;

mod most_simple;
mod hash_index;
mod hash_index_compaction;
mod log_structured_merged_tree;

trait StringError<T> {
    fn str_err(self, str: &str) -> Result<T, String>;
}

impl<T, E: Debug> StringError<T> for Result<T, E> {
    fn str_err(self, str: &str) -> Result<T, String> {
        match self {
            Ok(t) => Ok(t),
            Err(e) => Err(format!("{}: {:?}", str, e))
        }
    }
}
