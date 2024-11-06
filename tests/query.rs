use esql::{expr, in_expr, query, ArgFormat, Query, Type};

#[test]
fn simple_query() {
    let q = query("SELECT a,b,c FROM foobar")
        .wh("foo = 'bar'")
        .and(("bar = ?", 1))
        .and(expr(("d = ?", 10)).or(("e != ?", 20)));

    assert_query(
        q,
        "SELECT a,b,c FROM foobar WHERE foo = 'bar' AND bar = ? AND (d = ? OR e != ?)",
        [1, 10, 20],
    );
}

#[test]
fn query_concatenation() {
    let or_conditions = expr(("c = ?", 3)).or("d = 4").or(("e = ?", 4));

    let q = query("SELECT * FROM test")
        .wh(("a = ?", 1))
        .and(("b = ?", 2))
        .and(or_conditions);

    assert_query(
        q,
        "SELECT * FROM test WHERE a = ? AND b = ? AND (c = ? OR d = 4 OR e = ?)",
        [1, 2, 3, 4],
    );
}

#[test]
fn query_where_in() {
    let q = (query("SELECT id, email, countries.name AS country")
        + "FROM users"
        + "JOIN countries ON countries.id = users.country_id")
        .wh(in_expr(
            "users.id",
            [10, 20, 30, 40, 50, 60, 70, 80, 90, 100],
        ));

    assert_query(
        q,
        "SELECT id, email, countries.name AS country FROM users JOIN countries ON countries.id = users.country_id WHERE users.id IN (?,?,?,?,?,?,?,?,?,?)",
        [10, 20, 30, 40, 50, 60, 70, 80, 90, 100],
    );

    let q = query("SELECT * FROM contacts").wh(in_expr("contacts.id", [] as [u32; 0]));

    assert_query(q, "SELECT * FROM contacts WHERE 1=0", [] as [u32; 0]);
}

fn assert_query<'a, S>(
    query: Query<'a, S>,
    expected_query: &str,
    expected_args: impl IntoIterator<Item = impl Into<Type<'a>>>,
) {
    assert_eq!(
        query.build(ArgFormat::QuestionMark),
        (
            expected_query.to_string(),
            expected_args.into_iter().map(Into::into).collect(),
        )
    )
}
