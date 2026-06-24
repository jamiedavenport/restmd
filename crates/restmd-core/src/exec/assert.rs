//! Evaluating `assert` directives against a response.

use serde_json::Value as Json;
use serde_json_path::JsonPath;

use super::capture::stringify;
use super::report::AssertionResult;
use super::transport::HttpResponse;
use crate::model::{AssertOp, Assertion, CompareOp, Value};

/// Evaluate one assertion into a pass/fail result.
pub(crate) fn eval_assertion(assertion: &Assertion, response: &HttpResponse) -> AssertionResult {
    match assertion {
        Assertion::Status { op, code } => {
            let actual = response.status;
            let passed = compare_ord(actual, *op, *code);
            AssertionResult {
                description: format!("status {} {code}", op_str(*op)),
                passed,
                detail: (!passed).then(|| format!("actual status was {actual}")),
            }
        }
        Assertion::Body { path, op } => eval_body(path, op, response),
    }
}

fn eval_body(path: &str, op: &AssertOp, response: &HttpResponse) -> AssertionResult {
    let description = describe_body(path, op);
    let fail = |detail: String| AssertionResult {
        description: description.clone(),
        passed: false,
        detail: Some(detail),
    };
    let pass = || AssertionResult {
        description: description.clone(),
        passed: true,
        detail: None,
    };

    let json: Json = match serde_json::from_slice(&response.body) {
        Ok(v) => v,
        Err(_) => return fail("response body is not JSON".to_string()),
    };
    let query = match JsonPath::parse(path) {
        Ok(q) => q,
        Err(e) => return fail(format!("invalid JSONPath: {e}")),
    };
    let nodes = query.query(&json);

    match op {
        AssertOp::Exists => {
            if nodes.is_empty() {
                fail(format!("`{path}` matched nothing"))
            } else {
                pass()
            }
        }
        AssertOp::Matches(re) => {
            let Some(node) = nodes.first() else {
                return fail(format!("`{path}` matched nothing"));
            };
            match regex::Regex::new(re) {
                Ok(re) => {
                    let actual = stringify(node);
                    if re.is_match(&actual) {
                        pass()
                    } else {
                        fail(format!("`{actual}` does not match /{re}/"))
                    }
                }
                Err(e) => fail(format!("invalid regex: {e}")),
            }
        }
        AssertOp::Compare(cmp, expected) => {
            let Some(node) = nodes.first() else {
                return fail(format!("`{path}` matched nothing"));
            };
            match compare_json(node, *cmp, expected) {
                Ok(true) => pass(),
                Ok(false) => fail(format!("actual value was {}", stringify(node))),
                Err(reason) => fail(reason),
            }
        }
    }
}

/// Compare a JSON node against an expected literal. Ordering operators require
/// both sides to be numbers; `==`/`!=` work across types.
fn compare_json(node: &Json, op: CompareOp, expected: &Value) -> Result<bool, String> {
    match op {
        CompareOp::Eq => Ok(json_eq(node, expected)),
        CompareOp::Ne => Ok(!json_eq(node, expected)),
        CompareOp::Lt | CompareOp::Gt | CompareOp::Le | CompareOp::Ge => {
            let (Some(a), Value::Number(b)) = (node.as_f64(), expected) else {
                return Err("ordering comparison needs numbers on both sides".to_string());
            };
            Ok(compare_ord(a, op, *b))
        }
    }
}

fn json_eq(node: &Json, expected: &Value) -> bool {
    match (node, expected) {
        (Json::String(a), Value::String(b)) => a == b,
        (Json::Bool(a), Value::Bool(b)) => a == b,
        (Json::Null, Value::Null) => true,
        (n, Value::Number(b)) => n.as_f64() == Some(*b),
        _ => false,
    }
}

fn compare_ord<T: PartialOrd>(a: T, op: CompareOp, b: T) -> bool {
    match op {
        CompareOp::Eq => a == b,
        CompareOp::Ne => a != b,
        CompareOp::Lt => a < b,
        CompareOp::Gt => a > b,
        CompareOp::Le => a <= b,
        CompareOp::Ge => a >= b,
    }
}

fn op_str(op: CompareOp) -> &'static str {
    match op {
        CompareOp::Eq => "==",
        CompareOp::Ne => "!=",
        CompareOp::Lt => "<",
        CompareOp::Gt => ">",
        CompareOp::Le => "<=",
        CompareOp::Ge => ">=",
    }
}

fn describe_body(path: &str, op: &AssertOp) -> String {
    match op {
        AssertOp::Exists => format!("{path} exists"),
        AssertOp::Matches(re) => format!("{path} matches /{re}/"),
        AssertOp::Compare(cmp, expected) => format!("{path} {} {expected:?}", op_str(*cmp)),
    }
}
