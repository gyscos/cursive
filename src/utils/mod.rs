//! Toolbox to make text layout easier.

mod reader;
pub mod lines;

pub use self::reader::ProgressReader;

#[cfg(test)]
mod tests {
    use utils;

    #[test]
    fn test_prefix() {
        assert_eq!(utils::prefix(" abra ".split(' '), 5, " ").length, 5);
        assert_eq!(utils::prefix("abra a".split(' '), 5, " ").length, 4);
        assert_eq!(utils::prefix("a a br".split(' '), 5, " ").length, 3);
    }
}
