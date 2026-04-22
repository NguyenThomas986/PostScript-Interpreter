// tests/integration_test.rs — End-to-end integration tests
//
// These tests exercise the interpreter as a complete system, combining
// multiple features together the way a real PostScript program would.
//
// Unit tests for individual operators live alongside their source files.
// These tests focus on:
//   - Multi-step programs that use several features together
//   - Scoping behavior (dynamic vs lexical)
//   - Recursive procedures
//   - Real-world PostScript patterns

use postscript_interpreter::interpreter::Interpreter;
use postscript_interpreter::types::Value;

// ── Helper ───────────────────────────────────────────────────────────────────

fn run(source: &str) -> Vec<Value> {
    let mut interp = Interpreter::new();
    interp.run(source).expect("interpreter error");
    interp.stack.as_slice().to_vec()
}

fn run_err(source: &str) -> String {
    let mut interp = Interpreter::new();
    interp.run(source).unwrap_err()
}

// ── Stack + Arithmetic ───────────────────────────────────────────────────────────

#[test]
fn test_arithmetic_chain() {
    // (3 + 4) * 2 - 1 = 13
    assert_eq!(run("3 4 add 2 mul 1 sub"), vec![Value::Int(13)]);
}

#[test]
fn test_mixed_int_float_arithmetic() {
    // 5 / 2 = 2.5, then ceiling = 3.0
    assert_eq!(run("5 2 div ceiling"), vec![Value::Float(3.0)]);
}

#[test]
fn test_stack_manipulation_chain() {
    // Push 1 2 3, exch top two → 1 3 2, dup top → 1 3 2 2
    assert_eq!(
        run("1 2 3 exch dup"),
        vec![Value::Int(1), Value::Int(3), Value::Int(2), Value::Int(2)]
    );
}

// ── Dictionary + Procedures ───────────────────────────────────────────────────

#[test]
fn test_define_and_call_procedure() {
    // Define square, call it
    assert_eq!(run("/square { dup mul } def  5 square"), vec![Value::Int(25)]);
}

#[test]
fn test_procedure_using_defined_variable() {
    // Procedure that references a defined variable
    assert_eq!(run("/n 10 def  /addN { n add } def  5 addN"), vec![Value::Int(15)]);
}

#[test]
fn test_nested_procedure_calls() {
    // double calls square — tests procedure calling procedure
    assert_eq!(
        run("/square { dup mul } def  /double { 2 mul } def  3 square double"),
        vec![Value::Int(18)]
    );
}

#[test]
fn test_begin_end_isolates_scope() {
    // Variable defined inside begin/end should not be visible outside
    let mut interp = Interpreter::new();
    interp.run("10 dict begin  /x 42 def  end").unwrap();
    // x should not be defined outside the block
    assert!(interp.run("x").is_err());
}

// ── Control Flow ─────────────────────────────────────────────────────────────

#[test]
fn test_if_with_comparison() {
    // Push 10, check if > 5, if so push 1
    assert_eq!(run("10 5 gt { 1 } if"), vec![Value::Int(1)]);
}

#[test]
fn test_ifelse_branches() {
    assert_eq!(run("3 3 eq { (equal) } { (not equal) } ifelse"),
        vec![Value::Str("equal".to_string())]);
    assert_eq!(run("3 4 eq { (equal) } { (not equal) } ifelse"),
        vec![Value::Str("not equal".to_string())]);
}

#[test]
fn test_for_accumulates_sum() {
    // Sum 1 to 5 = 15
    assert_eq!(run("0  1 1 5 { add } for"), vec![Value::Int(15)]);
}

#[test]
fn test_repeat_builds_stack() {
    assert_eq!(
        run("3 { 7 } repeat"),
        vec![Value::Int(7), Value::Int(7), Value::Int(7)]
    );
}

#[test]
fn test_for_countdown() {
    // 3 down to 1
    assert_eq!(
        run("3 -1 1 { } for"),
        vec![Value::Int(3), Value::Int(2), Value::Int(1)]
    );
}

// ── Boolean Logic ─────────────────────────────────────────────────────────────

#[test]
fn test_boolean_in_ifelse() {
    // true and false → false → takes false branch
    assert_eq!(
        run("true false and { 1 } { 2 } ifelse"),
        vec![Value::Int(2)]
    );
}

#[test]
fn test_not_flips_condition() {
    assert_eq!(run("false not { 99 } if"), vec![Value::Int(99)]);
}

#[test]
fn test_comparison_chain() {
    // 5 >= 3 and 3 >= 1 → both true → and → true
    assert_eq!(run("5 3 ge  3 1 ge  and"), vec![Value::Bool(true)]);
}

// ── Strings ───────────────────────────────────────────────────────────────────

#[test]
fn test_string_length_in_condition() {
    // (hello) length = 5, 5 > 3 → true → push 1
    assert_eq!(run("(hello) length 3 gt { 1 } if"), vec![Value::Int(1)]);
}

#[test]
fn test_getinterval_on_variable() {
    assert_eq!(
        run("/s (PostScript) def  s 0 4 getinterval"),
        vec![Value::Str("Post".to_string())]
    );
}

// ── Recursive procedures ──────────────────────────────────────────────────────

#[test]
fn test_factorial_recursive() {
    // factorial: n! = n * (n-1)!,  base case 0! = 1
    let result = run("
        /factorial {
            dup 0 eq
            { pop 1 }
            { dup 1 sub factorial mul }
            ifelse
        } def
        5 factorial
    ");
    assert_eq!(result, vec![Value::Int(120)]);
}

#[test]
fn test_fibonacci() {
    // fib(7) = 13
    let result = run("
        /fib {
            dup 1 le
            { }
            { dup 1 sub fib  exch 2 sub fib  add }
            ifelse
        } def
        7 fib
    ");
    assert_eq!(result, vec![Value::Int(13)]);
}

// ── Scoping ───────────────────────────────────────────────────────────────────

#[test]
fn test_dynamic_scoping_sees_redefined_variable() {
    // Under dynamic scoping getx should return the LATEST x = 99
    let result = run("
        /x 10 def
        /getx { x } def
        /x 99 def
        getx
    ");
    assert_eq!(result, vec![Value::Int(99)]);
}

#[test]
fn test_lexical_scoping_captures_definition_env() {
    // Under lexical scoping getx should return x = 10 from definition time
    let result = run("
        lexical
        /x 10 def
        /getx { x } def
        /x 99 def
        getx
    ");
    assert_eq!(result, vec![Value::Int(10)]);
}

#[test]
fn test_toggle_back_to_dynamic() {
    let result = run("
        lexical
        dynamic
        /x 10 def
        /getx { x } def
        /x 99 def
        getx
    ");
    assert_eq!(result, vec![Value::Int(99)]);
}

// ── Error handling ────────────────────────────────────────────────────────────

#[test]
fn test_stack_underflow_error() {
    assert!(run_err("pop").contains("underflow") || run_err("pop").contains("underflow"));
}

#[test]
fn test_undefined_name_error() {
    assert!(run_err("undefined_name_xyz").contains("Unknown name"));
}

#[test]
fn test_div_by_zero_error() {
    assert!(run_err("1 0 div").contains("zero"));
}

#[test]
fn test_quit_sentinel() {
    assert_eq!(run_err("quit"), "__quit__");
}