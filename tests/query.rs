use esql::{from, select, wh, wh_in, Args, Query, Type};

#[test]
fn simple_query() {
    let query = select("a")
        + select("b,c")
        + from("foobar")
        + wh("foo = 'bar'")
        + wh(("bar = ?", 1))
        + (wh(("d = ?", 10)) | wh(("e != ?", 20)));

    assert_query(
        query,
        "SELECT a,b,c FROM foobar WHERE foo = 'bar' AND bar = ? AND (d = ? OR e != ?)",
        [1, 10, 20],
    );
}

#[test]
fn query_concatenation() {
    let or_conditions = wh(("c = ?", 3)) | wh("d = 4") | wh(("e = ?", 4));
    let query = select("*") + from("test") + wh(("a = ?", 1)) + wh(("b = ?", 2)) + or_conditions;

    assert_query(
        query,
        "SELECT * FROM test WHERE a = ? AND b = ? AND (c = ? OR d = 4 OR e = ?)",
        [1, 2, 3, 4],
    );
}

#[test]
fn query_where_in() {
    let query = select("id")
        + select("email")
        + select("countries.name AS country")
        + from("users")
        + "JOIN countries ON countries.id = users.country_id"
        + wh_in("users.id", [10, 20, 30, 40, 50, 60, 70, 80, 90, 100]);

    assert_query(
        query,
        "SELECT id,email,countries.name AS country FROM users JOIN countries ON countries.id = users.country_id WHERE users.id IN (?,?,?,?,?,?,?,?,?,?)",
        [10, 20, 30, 40, 50, 60, 70, 80, 90, 100],
    );

    let query = select("*") + from("contacts") + wh_in("contacts.id", [] as [u32; 0]);

    assert_query(query, "SELECT * FROM contacts WHERE 1=0", [] as [u32; 0]);
}

fn assert_query<'a>(
    query: Query<'a>,
    expected_query: &str,
    expected_args: impl IntoIterator<Item = impl Into<Type<'a>>>,
) {
    assert_eq!(
        query.build().unwrap(),
        (
            expected_query.to_string(),
            Args(expected_args.into_iter().map(Into::into).collect()),
        )
    )
}
