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
fn and_or_short_circuit() {
    let result = execute("false and print(\"should not print\")").unwrap();
    assert_eq_value(result, Value::Bool(false));

    let result = execute("true or print(\"should not print\")").unwrap();
    assert_eq_value(result, Value::Bool(true));

    let result = execute("true and 42").unwrap();
    assert_eq_value(result, Value::Num(42.0));

    let result = execute("false or \"hello\"").unwrap();
    assert_eq_value(result, Value::Str("hello".to_string()));
}

#[test]
fn and_or_precedence() {
    let result = execute("true and false or true").unwrap();
    assert_eq_value(result, Value::Bool(true));

    let result = execute("false or true and false").unwrap();
    assert_eq_value(result, Value::Bool(false));
}

#[test]
fn labelled_loop_break() {
    let result = execute("'outer loop { 'inner loop { break 'outer 42; }; };").unwrap();
    assert_eq_value(result, Value::Num(42.0));

    let result =
        execute("mut x = 0; 'outer loop { x = x + 1; loop { break 'outer; }; }; x").unwrap();
    assert_eq_value(result, Value::Num(1.0));
}

#[test]
fn pipe_placeholder() {
    let result = execute("42 |> _ + 1").unwrap();
    assert_eq_value(result, Value::Num(43.0));

    let result = execute("\"hello\" |> _ + \" world\"").unwrap();
    assert_eq_value(result, Value::Str("hello world".to_string()));

    let result = execute("10 |> _ * 2 |> _ + 5").unwrap();
    assert_eq_value(result, Value::Num(25.0));
}

#[test]
fn arrow_functions() {
    let result = execute("let add = |a, b| a + b; add(3, 4)").unwrap();
    assert_eq_value(result, Value::Num(7.0));

    let result = execute("let double = |x| x * 2; double(5)").unwrap();
    assert_eq_value(result, Value::Num(10.0));

    let result = execute("let inc = || { let x = 1; x + 1; }; inc()").unwrap();
    assert_eq_value(result, Value::Num(2.0));
}

#[test]
fn mutable_capture() {
    let result = execute("mut x = 0; let f = fn() [mut x] { x = x + 1; }; f(); x").unwrap();
    assert_eq_value(result, Value::Num(1.0));

    let result = execute("mut x = 10; let f = fn() [mut x] { x = x * 2; }; f(); f(); x").unwrap();
    assert_eq_value(result, Value::Num(40.0));
}

#[test]
fn undefined_var_suggestion() {
    let result = execute("let xyz = 42; xyx");
    match result {
        Err(Error::Compile { msg, .. }) => {
            assert!(msg.contains("undefined variable"));
            assert!(msg.contains("did you mean"));
            assert!(msg.contains("xyz"));
        }
        _ => panic!("expected compile error"),
    }
}

#[test]
fn basic_arithmetic() {
    assert_eq_value(execute("1 + 2").unwrap(), Value::Num(3.0));
    assert_eq_value(execute("5 - 3").unwrap(), Value::Num(2.0));
    assert_eq_value(execute("4 * 5").unwrap(), Value::Num(20.0));
    assert_eq_value(execute("10 / 2").unwrap(), Value::Num(5.0));
    assert_eq_value(execute("-5").unwrap(), Value::Num(-5.0));
}

#[test]
fn comparison() {
    assert_eq_value(execute("3 < 5").unwrap(), Value::Bool(true));
    assert_eq_value(execute("3 > 5").unwrap(), Value::Bool(false));
    assert_eq_value(execute("3 <= 3").unwrap(), Value::Bool(true));
    assert_eq_value(execute("5 >= 3").unwrap(), Value::Bool(true));
    assert_eq_value(execute("3 == 3").unwrap(), Value::Bool(true));
    assert_eq_value(execute("3 != 5").unwrap(), Value::Bool(true));
}

#[test]
fn if_expression() {
    assert_eq_value(
        execute("if (true) { 1 } else { 2 }").unwrap(),
        Value::Num(1.0),
    );
    assert_eq_value(
        execute("if (false) { 1 } else { 2 }").unwrap(),
        Value::Num(2.0),
    );
    assert_eq_value(execute("if (null) { 1 }").unwrap(), Value::Null);
}

#[test]
fn loops() {
    let result = execute("mut i = 0; loop { i = i + 1; if (i >= 5) { break; }; }; i").unwrap();
    assert_eq_value(result, Value::Num(5.0));

    let result = execute("loop { break 10; }").unwrap();
    assert_eq_value(result, Value::Num(10.0));
}

#[test]
fn functions() {
    let result = execute("fn add(a, b) { a + b; }; add(2, 3)").unwrap();
    assert_eq_value(result, Value::Num(5.0));

    let result = execute("fn mul(a, b) { return a * b; }; mul(4, 5)").unwrap();
    assert_eq_value(result, Value::Num(20.0));
}

#[test]
fn closures() {
    let result = execute("let x = 10; let f = fn(y) { x + y; }; f(5)").unwrap();
    assert_eq_value(result, Value::Num(15.0));
}

#[test]
fn string_concat() {
    assert_eq_value(
        execute("\"hello\" + \" world\"").unwrap(),
        Value::Str("hello world".to_string()),
    );
}

#[test]
fn truthiness() {
    assert_eq_value(execute("null or 1").unwrap(), Value::Num(1.0));
    assert_eq_value(execute("false or 2").unwrap(), Value::Num(2.0));
    assert_eq_value(execute("0 and 3").unwrap(), Value::Num(3.0));
    assert_eq_value(execute("\"\" and 4").unwrap(), Value::Num(4.0));
}

#[test]
fn division_by_zero() {
    let result = execute("1 / 0");
    match result {
        Err(Error::Runtime { msg, .. }) => {
            assert!(msg.contains("division by zero"));
        }
        _ => panic!("expected runtime error"),
    }
}

#[test]
fn comprehensive_example() {
    // ── 1. Variables & Arithmetic ──
    let result = execute(
        r#"
        let a = 10;
        let b = 20;
        a + b
    "#,
    )
    .unwrap();
    assert_eq_value(result, Value::Num(30.0));

    // ── 2. String concat & comparison ──
    let result = execute(
        r#"
        let greeting = "Hello";
        let name = "World";
        let msg = greeting + " " + name;
        if (msg == "Hello World") { 42 } else { 0 }
    "#,
    )
    .unwrap();
    assert_eq_value(result, Value::Num(42.0));

    // ── 3. Mutable counter with loop ──
    let result = execute(
        r#"
        mut sum = 0;
        mut n = 1;
        loop {
            if (n > 10) { break; };
            sum = sum + n;
            n = n + 1;
        };
        sum
    "#,
    )
    .unwrap();
    assert_eq_value(result, Value::Num(55.0)); // 1+2+...+10 = 55

    // ── 4. Named function ──
    let result = execute(
        r#"
        fn factorial(n) {
            if (n <= 1) { return 1; };
            return n * factorial(n - 1);
        };
        factorial(5)
    "#,
    )
    .unwrap();
    assert_eq_value(result, Value::Num(120.0));

    // ── 5. Arrow function + pipe ──
    let result = execute(
        r#"
        let square = |x| x * x;
        let inc = |x| x + 1;
        5 |> square |> inc
    "#,
    )
    .unwrap();
    assert_eq_value(result, Value::Num(26.0)); // inc(square(5)) = 5*5+1 = 26

    // ── 6. Pipe with placeholder ──
    let result = execute(
        r#"
        "hello" |> _ + " world" |> _ + "!"
    "#,
    )
    .unwrap();
    assert_eq_value(result, Value::Str("hello world!".to_string()));

    // ── 7. Closure capturing outer variable ──
    let result = execute(
        r#"
        let base = 100;
        let adder = fn(x) { base + x; };
        adder(23)
    "#,
    )
    .unwrap();
    assert_eq_value(result, Value::Num(123.0));

    // ── 8. Mutable closure capture ──
    let result = execute(
        r#"
        mut counter = 0;
        let increment = fn() [mut counter] { counter = counter + 1; };
        increment();
        increment();
        increment();
        counter
    "#,
    )
    .unwrap();
    assert_eq_value(result, Value::Num(3.0));

    // ── 9. Labelled loop break with value ──
    let result = execute(
        r#"
        'outer loop {
            mut tries = 0;
            loop {
                tries = tries + 1;
                if (tries >= 3) { break 'outer tries; };
            };
        }
    "#,
    )
    .unwrap();
    assert_eq_value(result, Value::Num(3.0));

    // ── 10. Logical short-circuit ──
    let result = execute(
        r#"
        let x = 10;
        (x > 5) and (x < 20) and (x != 0)
    "#,
    )
    .unwrap();
    assert_eq_value(result, Value::Bool(true));

    // ── 11. Truthiness: 0 & "" are truthy, null/false are falsy ──
    let result = execute(
        r#"
        let a = null or 1;
        let b = false or 2;
        let c = 0 and 3;
        let d = "" and 4;
        a + b + c + d
    "#,
    )
    .unwrap();
    assert_eq_value(result, Value::Num(10.0)); // null→1, false→2, 0→3, ""→4

    // ── 12. Error: immutable reassignment ──
    let result = execute("let pi = 3.14; pi = 3.0;");
    match result {
        Err(Error::Compile { msg, .. }) => {
            assert!(msg.contains("cannot assign to immutable variable"));
        }
        _ => panic!("expected compile error"),
    }
}

#[test]
fn immutability_error() {
    let result = execute("let x = 0; x = 1;");
    match result {
        Err(Error::Compile { msg, .. }) => {
            assert!(msg.contains("cannot assign to immutable variable"));
            assert!(msg.contains("x"));
        }
        _ => panic!("expected compile error"),
    }

    let result = execute("let x = 0; x = x + 1;");
    match result {
        Err(Error::Compile { msg, .. }) => {
            assert!(msg.contains("cannot assign to immutable variable"));
        }
        _ => panic!("expected compile error"),
    }
}
