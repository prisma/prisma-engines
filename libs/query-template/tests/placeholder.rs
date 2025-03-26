use query_template::PlaceholderFormat;

#[test]
fn placeholder_formatting_numbered() {
    let mut sql = String::new();
    let mut n = 1;

    let numbered = PlaceholderFormat {
        prefix: "$P",
        has_numbering: true,
    };

    numbered.write(&mut sql, &mut n).unwrap();
    assert_eq!(sql, "$P1");

    sql.push(',');

    numbered.write(&mut sql, &mut n).unwrap();
    assert_eq!(sql, "$P1,$P2");

    assert_eq!(n, 3);
}

#[test]
fn placeholder_formatting_unnumbered() {
    let mut sql = String::new();
    let mut n = 1;

    let unnumbered = PlaceholderFormat {
        prefix: "?",
        has_numbering: false,
    };

    unnumbered.write(&mut sql, &mut n).unwrap();
    assert_eq!(sql, "?");

    sql.push(',');

    unnumbered.write(&mut sql, &mut n).unwrap();
    assert_eq!(sql, "?,?");

    assert_eq!(n, 1);
}
