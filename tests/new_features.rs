use qwqlang::{error::Error, execute, vm::Value};

fn assert_eq_value(a: Value, b: Value) {
    match (a, b) {
        (Value::Null, Value::Null) => {}
        (Value::Bool(x), Value::Bool(y)) => assert_eq!(x, y),
        (Value::Num(x), Value::Num(y)) => assert_eq!(x, y),
        (Value::Str(x), Value::Str(y)) => assert_eq!(x, y),
        _ => panic!("values not equal"),
    }
}

#[test]
fn array_literal() {
    let result = execute("[1, 2, 3]").unwrap();
    match result {
        Value::Arr(a) => {
            assert_eq!(a.len(), 3);
            assert_eq_value(a[0].clone(), Value::Num(1.0));
            assert_eq_value(a[1].clone(), Value::Num(2.0));
            assert_eq_value(a[2].clone(), Value::Num(3.0));
        }
        _ => panic!("expected array"),
    }

    let result = execute("[]").unwrap();
    match result {
        Value::Arr(a) => assert_eq!(a.len(), 0),
        _ => panic!("expected empty array"),
    }
}

#[test]
fn subscript_access() {
    let result = execute("[10, 20, 30][0]").unwrap();
    assert_eq_value(result, Value::Num(10.0));

    let result = execute("[10, 20, 30][2]").unwrap();
    assert_eq_value(result, Value::Num(30.0));

    let result = execute("[10, 20, 30][5]").unwrap();
    assert_eq_value(result, Value::Null);
}

#[test]
fn dict_literal() {
    let result = execute(r#"{"a" = 1, "b" = 2}"#).unwrap();
    match result {
        Value::Dict(d) => {
            assert_eq!(d.len(), 2);
        }
        _ => panic!("expected dict"),
    }
}

#[test]
fn dict_subscript() {
    let result = execute(r#"let d = {"x" = 42}; d["x"]"#).unwrap();
    assert_eq_value(result, Value::Num(42.0));
}

#[test]
fn string_subscript() {
    let result = execute(r#""hello"[0]"#).unwrap();
    assert_eq_value(result, Value::Str("h".to_string()));

    let result = execute(r#""hello"[4]"#).unwrap();
    assert_eq_value(result, Value::Str("o".to_string()));
}

#[test]
fn len_builtin() {
    let result = execute("[1, 2, 3] |> len()").unwrap();
    assert_eq_value(result, Value::Num(3.0));

    let result = execute(r#""hello" |> len()"#).unwrap();
    assert_eq_value(result, Value::Num(5.0));
}

#[test]
fn while_loop() {
    let result = execute("mut x = 0; while (x < 5) { x = x + 1; }; x").unwrap();
    assert_eq_value(result, Value::Num(5.0));
}

#[test]
fn for_loop_basic() {
    let result = execute("mut sum = 0; for i in [1, 2, 3] { sum = sum + i; }; sum").unwrap();
    assert_eq_value(result, Value::Num(6.0));
}

#[test]
fn comprehensive_new_features() {
    let result = execute(
        r#"
        mut arr = [10, 20, 30];
        arr[0] = 100;
        let d = {"name" = "qwq", "value" = 42};
        mut total = 0;
        for item in [1, 2, 3, 4] {
            total = total + item;
        };
        total
        "#,
    )
    .unwrap();
    assert_eq_value(result, Value::Num(10.0));
}
