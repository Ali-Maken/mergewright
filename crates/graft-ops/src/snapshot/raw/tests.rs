use super::converted::converting_paths;

fn triple(path: &str, attribute: &str, value: &str) -> Vec<u8> {
    format!("{path}\0{attribute}\0{value}\0").into_bytes()
}

fn converting(records: &[Vec<u8>], autocrlf: bool) -> Vec<String> {
    converting_paths(&records.concat(), autocrlf)
        .expect("the records parse")
        .into_iter()
        .map(|path| String::from_utf8(path).expect("utf-8"))
        .collect()
}

#[test]
fn nothing_set_and_autocrlf_off_converts_nothing() {
    let records = [
        triple("a", "text", "unspecified"),
        triple("a", "filter", "unspecified"),
    ];
    assert!(converting(&records, false).is_empty());
}

#[test]
fn autocrlf_converts_unless_text_is_unset() {
    let records = [
        triple("a", "text", "unspecified"),
        triple("bin", "text", "unset"),
    ];
    assert_eq!(converting(&records, true), vec!["a"]);
}

#[test]
fn any_converting_attribute_set_converts_even_without_autocrlf() {
    let records = [
        triple("f", "filter", "lfs"),
        triple("e", "eol", "crlf"),
        triple("t", "text", "auto"),
        triple("i", "ident", "set"),
        triple("w", "working-tree-encoding", "UTF-16"),
        triple("n", "text", "unset"),
    ];
    assert_eq!(converting(&records, false), vec!["e", "f", "i", "t", "w"]);
}

#[test]
fn a_filter_on_a_binary_file_still_converts() {
    let records = [triple("b", "text", "unset"), triple("b", "filter", "lfs")];
    assert_eq!(converting(&records, true), vec!["b"]);
}

#[test]
fn a_torn_triple_is_an_error() {
    assert!(converting_paths(b"a\0text\0", false).is_err());
}
