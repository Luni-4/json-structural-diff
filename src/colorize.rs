use regex::Regex;
use serde_json::Value;

fn subcolorize<F>(key: Option<&str>, diff: &Value, output: &mut F, color: &str, indent: &str)
where
    F: FnMut(&str, &str),
{
    let prefix = if let Some(key) = key {
        format!("{key}: ")
    } else {
        String::new()
    };
    let subindent = &format!("{indent}  ");

    match diff {
        Value::Object(obj) => {
            if obj.len() == 2 && obj.contains_key("__old") && obj.contains_key("__new") {
                let old = obj.get("__old").unwrap();
                let new = obj.get("__new").unwrap();
                subcolorize(key, old, output, "-", indent);
                subcolorize(key, new, output, "+", indent);
            } else {
                output(color, &format!("{indent}{prefix}{{"));
                let re_delete = Regex::new(r"^(.*)__deleted$").unwrap();
                let re_added = Regex::new(r"^(.*)__added$").unwrap();
                for (subkey, subvalue) in obj {
                    if let Some(caps) = re_delete.captures(subkey) {
                        subcolorize(
                            Some(caps.get(1).unwrap().as_str()),
                            subvalue,
                            output,
                            "-",
                            subindent,
                        );
                        continue;
                    }
                    if let Some(caps) = re_added.captures(subkey) {
                        subcolorize(
                            Some(caps.get(1).unwrap().as_str()),
                            subvalue,
                            output,
                            "+",
                            subindent,
                        );
                        continue;
                    }
                    subcolorize(Some(subkey), subvalue, output, color, subindent);
                }
                output(color, &format!("{indent}}}"));
            }
        }
        Value::Array(array) => {
            output(color, &format!("{indent}{prefix}["));

            let mut looks_like_diff = true;
            for item in array {
                looks_like_diff = if let Value::Array(arr) = item {
                    if !(arr.len() == 2
                        || (arr.len() == 1
                            && (arr[0].is_string() && arr[0].as_str().unwrap() == " ")))
                    {
                        false
                    } else if let Value::String(str1) = &arr[0] {
                        str1.len() == 1 && ([" ", "-", "+", "~"].contains(&str1.as_str()))
                    } else {
                        false
                    }
                } else {
                    false
                };
            }

            if looks_like_diff {
                for item in array {
                    if let Value::Array(subitem) = item {
                        let op = subitem[0].as_str().unwrap();
                        let subvalue = &subitem.get(1);
                        if op == " " && subvalue.is_none() {
                            output(" ", &format!("{subindent}..."));
                        } else {
                            assert!(([" ", "-", "+", "~"].contains(&op)), "Unexpected op '{}'", op);
                            let subvalue = subvalue.unwrap();
                            let color = if op == "~" { " " } else { op };
                            subcolorize(None, subvalue, output, color, subindent);
                        }
                    }
                }
            } else {
                for subvalue in array {
                    subcolorize(None, subvalue, output, color, subindent);
                }
            }

            output(color, &format!("{indent}]"));
        }
        _ => output(color, &(indent.to_owned() + &prefix + &diff.to_string())),
    }
}

/// Returns the JSON structural difference formatted as a `Vec<String>`.
///
/// If `None`, there is no JSON structural difference to be formatted.
#[must_use] pub fn colorize_to_array(diff: &Value) -> Vec<String> {
    let mut output: Vec<String> = Vec::new();

    let mut output_func = |color: &str, line: &str| {
        output.push(format!("{color}{line}"));
    };

    subcolorize(None, diff, &mut output_func, " ", "");

    output
}

/// Returns the JSON structural difference formatted as a `String`.
///
/// If `None`, there is no JSON structural difference to be formatted.
#[cfg(feature = "colorize")]
pub fn colorize(diff: &Value, is_color: bool) -> String {
    use console::Style;

    let mut output: Vec<String> = Vec::new();

    let mut output_func = |color: &str, line: &str| {
        let color_line = format!("{}{}", color, line);
        let str_output = if is_color {
            match color {
                "+" => format!("{}", Style::new().green().apply_to(color_line)),
                "-" => format!("{}", Style::new().red().apply_to(color_line)),
                _ => color_line,
            }
        } else {
            color_line
        };
        output.push(str_output + "\n");
    };

    subcolorize(None, diff, &mut output_func, " ", "");

    output.join("")
}

#[cfg(test)]
mod tests {

    use super::colorize_to_array;

    #[test]
    fn test_colorize_to_array() {
        assert_eq!(colorize_to_array(&json!(42)), &[" 42"]);

        assert_eq!(colorize_to_array(&json!(null)), &[" null"]);

        assert_eq!(colorize_to_array(&json!(false)), &[" false"]);

        assert_eq!(
            colorize_to_array(&json!({"__old": 42, "__new": 10 })),
            &["-42", "+10"]
        );

        assert_eq!(
            colorize_to_array(&json!({"__old": false, "__new": null })),
            &["-false", "+null"]
        );

        assert_eq!(
            colorize_to_array(&json!({"foo__deleted": 42 })),
            &[" {", "-  foo: 42", " }"]
        );

        assert_eq!(
            colorize_to_array(&json!({"foo__added": 42 })),
            &[" {", "+  foo: 42", " }"]
        );

        assert_eq!(
            colorize_to_array(&json!({ "foo__added": null })),
            &[" {", "+  foo: null", " }"]
        );

        assert_eq!(
            colorize_to_array(&json!({ "foo__added": false })),
            &[" {", "+  foo: false", " }"]
        );

        assert_eq!(
            colorize_to_array(&json!({"foo__added": {"bar": 42 } })),
            &[" {", "+  foo: {", "+    bar: 42", "+  }", " }"]
        );

        assert_eq!(
            colorize_to_array(&json!({"foo": {"__old": 42, "__new": 10 } })),
            &[" {", "-  foo: 42", "+  foo: 10", " }"]
        );

        assert_eq!(
            colorize_to_array(&json!([[' ', 10], ['+', 20], [' ', 30]])),
            &[" [", "   10", "+  20", "   30", " ]"]
        );

        assert_eq!(
            colorize_to_array(&json!([[' ', 10], ['-', 20], [' ', 30]])),
            &[" [", "   10", "-  20", "   30", " ]"]
        );

        assert_eq!(
            colorize_to_array(&json!([ [" "], ["~", {"foo__added": 42}], [" "] ])),
            &[
                " [",
                "   ...",
                "   {",
                "+    foo: 42",
                "   }",
                "   ...",
                " ]"
            ],
        );
    }

    #[test]
    #[cfg(feature = "colorize")]
    fn test_colorize_no_colors() {
        use super::colorize;
        assert_eq!(
            colorize(&json!({"foo": {"__old": 42, "__new": 10 } }), false),
            " {\n-  foo: 42\n+  foo: 10\n }\n"
        );
    }
}
