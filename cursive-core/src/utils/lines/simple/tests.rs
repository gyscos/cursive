#[test]
fn test_prefix() {
    use super::prefix;

    assert_eq!(prefix(" abra ".split(' '), 5, " ").length, 5);
    assert_eq!(prefix("abra a".split(' '), 5, " ").length, 4);
    assert_eq!(prefix("a a br".split(' '), 5, " ").length, 3);
}

#[test]
fn test_lines() {
    use super::make_lines;

    let content = "This is a line.\n\nThis is a second line.";
    let rows = make_lines(content, 30);

    assert_eq!(rows.len(), 3);
}
