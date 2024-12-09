use difflib::sequencematcher::SequenceMatcher;
use serde_json::{Map, Value};

use crate::colorize::colorize_to_array;

/// Auxiliary structure to encapsulate data about the structural difference
/// of two JSON files.
#[allow(clippy::module_name_repetitions)]
pub struct JsonDiff {
    /// Quantifies the difference between two JSON files.
    ///
    /// If `0.`: the two JSON files are entirely different one from the other.
    /// If `100.`: the two JSON files are identical.
    pub score: f64,
    /// The JSON structural difference of two JSON files.
    ///
    /// If `None`: the two JSON files are identical.
    pub diff: Option<Value>,
}

struct BestMatch {
    score: f64,
    key: String,
    index_distance: usize,
}

impl BestMatch {
    fn new(score: f64, key: String, index_distance: usize) -> Self {
        Self {
            score,
            key,
            index_distance,
        }
    }
}

impl JsonDiff {
    /// Finds the JSON structural difference of two JSON files.
    #[must_use]
    pub fn diff(json1: &Value, json2: &Value, keys_only: bool) -> Self {
        Self::diff_with_score(json1, json2, keys_only)
    }

    /// Finds the JSON structural difference of two JSON files and
    /// returns it as a formatted string.
    #[must_use]
    pub fn diff_string(json1: &Value, json2: &Value, keys_only: bool) -> Option<String> {
        let Self { score: _, diff } = Self::diff(json1, json2, keys_only);
        diff.map(|value| colorize_to_array(&value).join("\n") + "\n")
    }

    fn object_diff(obj1: &Map<String, Value>, obj2: &Map<String, Value>, keys_only: bool) -> Self {
        let mut result = Map::new();
        let mut score = 0.;

        for (key, value1) in obj1 {
            if !obj2.contains_key(key) {
                let key_deleted = format!("{key}__deleted");
                result.insert(key_deleted, value1.clone());
                score -= 30.;
            }
        }

        for (key, value2) in obj2 {
            if !obj1.contains_key(key) {
                let key_added = format!("{key}__added");
                result.insert(key_added, value2.clone());
                score -= 30.;
            }
        }

        for (key, value1) in obj1 {
            if let Some(value2) = obj2.get(key) {
                score += 20.;
                let Self {
                    score: subscore,
                    diff: change,
                } = Self::diff_with_score(value1, value2, keys_only);
                if let Some(change) = change {
                    result.insert(key.clone(), change);
                }
                score += (subscore / 5.).clamp(-10., 20.);
            }
        }

        if result.is_empty() {
            #[allow(clippy::cast_precision_loss)]
            Self {
                score: 100. * (obj1.len() as f64).max(0.5),
                diff: None,
            }
        } else {
            let output = json!(result);
            Self {
                score: score.max(0.),
                diff: Some(output),
            }
        }
    }

    fn check_type(item1: &Value, item2: &Value) -> bool {
        item1.is_null() == item2.is_null()
            || item1.is_boolean() == item2.is_boolean()
            || item1.is_number() == item2.is_number()
            || item1.is_string() == item2.is_string()
            || item1.is_array() == item2.is_array()
            || item1.is_object() == item2.is_object()
    }

    fn find_matching_object(
        item: &Value,
        index: usize,
        fuzzy_originals: &Map<String, Value>,
    ) -> Option<BestMatch> {
        let mut best_match: Option<BestMatch> = None;

        for (match_index, (key, candidate)) in fuzzy_originals.into_iter().enumerate() {
            if key != "__next" {
                let index_distance = (match_index).wrapping_sub(index);
                if Self::check_type(item, candidate) {
                    let Self { score, diff: _ } = Self::diff(item, candidate, false);
                    if best_match.as_ref().map_or(true, |v| score > v.score)
                        || (best_match
                            .as_ref()
                            .map_or(true, |v| (score - v.score).abs() < f64::EPSILON)
                            && best_match
                                .as_ref()
                                .map_or(true, |v| index_distance < v.index_distance))
                    {
                        best_match = Some(BestMatch::new(score, key.clone(), index_distance));
                    }
                }
            }
        }

        best_match
    }

    fn scalarize(
        array: &[Value],
        scalar_values: &mut Map<String, Value>,
        originals: &mut Map<String, Value>,
        fuzzy_originals: Option<&Map<String, Value>>,
    ) -> Vec<String> {
        let mut output_array: Vec<String> = Vec::new();
        for (index, item) in array.iter().enumerate() {
            let mut value = if let Value::Object(_) = item {
                None
            } else {
                let key = item.to_string();
                scalar_values.insert(key.clone(), item.clone());
                Some(key)
            };

            if let Some(fuzzy_originals) = fuzzy_originals {
                if let Some(best_match) = Self::find_matching_object(item, index, fuzzy_originals) {
                    if best_match.score > 40. && !originals.contains_key(&best_match.key) {
                        originals.insert(best_match.key.clone(), item.to_owned());
                        value = Some(best_match.key);
                    }
                }
            }

            if value.is_none() {
                let original = originals.get_mut("__next").unwrap();
                let proxy = "__$!SCALAR".to_owned() + &(original).to_string();

                *original = json!(original.as_u64().unwrap() + 1);
                originals.insert(proxy.clone(), item.to_owned());
                value = Some(proxy);
            }

            let final_value = value.unwrap();
            output_array.push(final_value);
        }
        output_array
    }

    fn is_scalarized(key: &str, originals: &Map<String, Value>) -> bool {
        originals.contains_key(key)
    }

    fn get_scalar(key: &str, scalar_values: &Map<String, Value>) -> Value {
        scalar_values.get(key).unwrap().clone()
    }

    fn descalarize(
        key: &str,
        scalar_values: &Map<String, Value>,
        originals: &Map<String, Value>,
    ) -> Value {
        if let Some(val) = originals.get(key) {
            val.clone()
        } else {
            Self::get_scalar(key, scalar_values)
        }
    }

    #[allow(clippy::too_many_lines)]
    fn array_diff(array1: &[Value], array2: &[Value], keys_only: bool) -> Self {
        let mut originals1 = Map::new();
        let mut scalar_values1 = Map::new();
        originals1.insert("__next".to_owned(), json!(1));
        let seq1: Vec<String> = Self::scalarize(array1, &mut scalar_values1, &mut originals1, None);

        let mut originals2 = Map::new();
        let mut scalar_values2 = Map::new();
        let originals1_value = originals1.get("__next").unwrap();
        originals2.insert("__next".to_owned(), json!(originals1_value));
        let seq2: Vec<String> = Self::scalarize(
            array2,
            &mut scalar_values2,
            &mut originals2,
            Some(&originals1),
        );

        let opcodes = SequenceMatcher::new(&seq1, &seq2).get_opcodes();

        let mut result: Vec<Value> = Vec::new();
        let mut score: f64 = 0.;
        let mut all_equal = true;

        for opcode in &opcodes {
            if !(opcode.tag == "equal" || (keys_only && opcode.tag == "replace")) {
                all_equal = false;
            }

            match opcode.tag.as_str() {
                "equal" => {
                    for key in seq1.iter().take(opcode.first_end).skip(opcode.first_start) {
                        let is_scalarized1 = Self::is_scalarized(key, &originals1);
                        assert!(!is_scalarized1 || (Self::is_scalarized(key, &originals2)),
                            "Internal bug: the items associated to the key {key} are different in the two dictionaries"
                        );
                        if is_scalarized1 {
                            let item1 = Self::descalarize(key, &scalar_values1, &originals1);
                            let item2 = Self::descalarize(key, &scalar_values2, &originals2);
                            let Self {
                                score: _,
                                diff: change,
                            } = Self::diff(&item1, &item2, keys_only);
                            if let Some(change) = change {
                                result.push(json!([json!('~'), change]));
                                all_equal = false;
                            } else {
                                result.push(json!([json!(' ')]));
                            }
                        } else {
                            result
                                .push(json!([json!(' '), Self::get_scalar(key, &scalar_values1)]));
                        }
                        score += 10.;
                    }
                }
                "delete" => {
                    for key in seq1.iter().take(opcode.first_end).skip(opcode.first_start) {
                        result.push(json!([
                            json!('-'),
                            Self::descalarize(key, &scalar_values1, &originals1)
                        ]));
                        score -= 5.;
                    }
                }
                "insert" => {
                    for key in seq2
                        .iter()
                        .take(opcode.second_end)
                        .skip(opcode.second_start)
                    {
                        result.push(json!([
                            json!('+'),
                            Self::descalarize(key, &scalar_values2, &originals2)
                        ]));
                        score -= 5.;
                    }
                }
                "replace" => {
                    if keys_only {
                        for (key1, key2) in seq1
                            .iter()
                            .take(opcode.first_end)
                            .skip(opcode.first_start)
                            .zip(
                                seq2.iter()
                                    .take(
                                        opcode.first_end - opcode.first_start + opcode.second_start,
                                    )
                                    .skip(opcode.second_start),
                            )
                        {
                            let Self {
                                score: _,
                                diff: change,
                            } = Self::diff(
                                &Self::descalarize(key1, &scalar_values1, &originals1),
                                &Self::descalarize(key2, &scalar_values2, &originals2),
                                keys_only,
                            );
                            if let Some(change) = change {
                                result.push(json!([json!('~'), change]));
                                all_equal = false;
                            } else {
                                result.push(json!(' '));
                            }
                        }
                    } else {
                        for key in seq1.iter().take(opcode.first_end).skip(opcode.first_start) {
                            result.push(json!([
                                json!('-'),
                                Self::descalarize(key, &scalar_values1, &originals1)
                            ]));
                            score -= 5.;
                        }
                        for key in seq2
                            .iter()
                            .take(opcode.second_end)
                            .skip(opcode.second_start)
                        {
                            result.push(json!([
                                json!('+'),
                                Self::descalarize(key, &scalar_values2, &originals2)
                            ]));
                            score -= 5.;
                        }
                    }
                }
                _ => all_equal = true,
            }
        }

        if all_equal || opcodes.is_empty() {
            Self {
                score: 100.,
                diff: None,
            }
        } else {
            Self {
                score: score.max(0.),
                diff: Some(json!(result)),
            }
        }
    }

    fn diff_with_score(json1: &Value, json2: &Value, keys_only: bool) -> Self {
        if let (Value::Object(obj1), Value::Object(obj2)) = (json1, json2) {
            return Self::object_diff(obj1, obj2, keys_only);
        }
        if let (Value::Array(array1), Value::Array(array2)) = (json1, json2) {
            return Self::array_diff(array1, array2, keys_only);
        }

        if !keys_only && json1 != json2 {
            Self {
                score: 0.,
                diff: Some(json!({ "__old": json1, "__new": json2 })),
            }
        } else {
            Self {
                score: 100.,
                diff: None,
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use std::error::Error;
    use std::fs::File;
    use std::io::BufReader;
    use std::path::Path;

    use super::JsonDiff;

    #[test]
    fn test_scalar() {
        assert_eq!(JsonDiff::diff(&json!(42), &json!(42), false).diff, None);
        assert_eq!(
            JsonDiff::diff(&json!("foo"), &json!("foo"), false).diff,
            None
        );
        assert_eq!(
            JsonDiff::diff(&json!(42), &json!(10), false).diff,
            Some(json!({"__old": 42, "__new": 10 }))
        );
    }

    #[test]
    fn test_objects() {
        assert_eq!(JsonDiff::diff(&json!({}), &json!({}), false).diff, None);

        assert_eq!(
            JsonDiff::diff(
                &json!({"foo": 42, "bar": 10 }),
                &json!({"foo": 42, "bar": 10 }),
                false
            )
            .diff,
            None
        );

        assert_eq!(
            JsonDiff::diff(
                &json!({"foo": 42, "bar": {"bbbar": 10, "bbboz": 11 }}),
                &json!({"foo": 42, "bar": {"bbbar": 10, "bbboz": 11 }}),
                false
            )
            .diff,
            None
        );

        assert_eq!(
            JsonDiff::diff(&json!({"foo": 42, "bar": 10 }), &json!({"bar": 10 }), false).diff,
            Some(json!({"foo__deleted": 42 }))
        );

        assert_eq!(
            JsonDiff::diff(&json!({"bar": 10 }), &json!({"foo": 42, "bar": 10 }), false).diff,
            Some(json!({"foo__added": 42 }))
        );

        assert_eq!(
            JsonDiff::diff(&json!({"foo": 42 }), &json!({"foo": 10 }), false).diff,
            Some(json!({"foo": {"__old": 42, "__new": 10 } }))
        );

        assert_eq!(
            JsonDiff::diff(
                &json!({"foo": 42, "bar": {"bbbar": 10, "bbboz": 11 }}),
                &json!({"foo": 42, "bar": {"bbbar": 12 }}),
                false
            )
            .diff,
            Some(json!(
                {
                  "bar": {
                           "bbboz__deleted": 11,
                           "bbbar": {"__old": 10, "__new": 12 }
                         }
                }
            ))
        );
    }

    #[test]
    fn test_array_of_scalars() {
        assert_eq!(
            JsonDiff::diff(&json!([10, 20, 30]), &json!([10, 20, 30]), false).diff,
            None
        );

        assert_eq!(
            JsonDiff::diff(&json!([10, 20, 30]), &json!([10, 30]), false).diff,
            Some(json!([[' ', 10], ['-', 20], [' ', 30]]))
        );

        assert_eq!(
            JsonDiff::diff(&json!([10, 30]), &json!([10, 20, 30]), false).diff,
            Some(json!([[' ', 10], ['+', 20], [' ', 30]]))
        );

        assert_eq!(
            JsonDiff::diff(&json!([10, 20]), &json!([10, 20, 30]), false).diff,
            Some(json!([[' ', 10], [' ', 20], ['+', 30]]))
        );
    }

    #[test]
    fn test_array_of_objects() {
        assert_eq!(
            JsonDiff::diff(
                &json!([{"foo": 10 }, {"foo": 20 }, {"foo": 30 }]),
                &json!([{"foo": 10 }, {"foo": 20 }, {"foo": 30 }]),
                false
            )
            .diff,
            None
        );

        assert_eq!(JsonDiff::diff(&json!([{}]), &json!([{}]), false).diff, None);

        assert_eq!(JsonDiff::diff(&json!([[]]), &json!([[]]), false).diff, None);

        assert_eq!(
            JsonDiff::diff(&json!([1, null, null]), &json!([1, null, null]), false).diff,
            None
        );

        assert_eq!(
            JsonDiff::diff(
                &json!([{"a": 1, "b": 2 }, {"a": 1, "b": 2 }]),
                &json!([{"a": 1, "b": 2 }, {"a": 1, "b": 2 }]),
                false
            )
            .diff,
            None
        );

        assert_eq!(
            JsonDiff::diff(
                &json!([{"foo": 10 }, {"foo": 20 }, {"foo": 30 }]),
                &json!([{"foo": 10 }, {"foo": 30 }]),
                false
            )
            .diff,
            Some(json!([[' '], ['-', { "foo": 20 }], [' ']]))
        );

        assert_eq!(
            JsonDiff::diff(
                &json!([{"foo": 10 }, {"foo": 30 }]),
                &json!([{"foo": 10 }, {"foo": 20 }, {"foo": 30 }]),
                false
            )
            .diff,
            Some(json!([[' '], ['+', {"foo": 20 }], [' ']]))
        );

        assert_eq!(
            JsonDiff::diff(
                &json!(
                    [
                      {"name": "Foo", "a": 3, "b": 1 },
                      { "foo": 10 }
                    ]
                ),
                &json!(
                    [
                      {"name": "Foo", "a": 3, "b": 1 },
                      {"name": "Foo", "a": 3, "b": 1, "c": 1 },
                      {"foo": 10 }
                    ]
                ),
                false
            )
            .diff,
            Some(json!(
               [
                 [' '],
                 ['+', {"name": "Foo", "a": 3, "b": 1, "c": 1 }],
                 [' ']
               ]
            ))
        );

        assert_eq!(
            JsonDiff::diff(
                &json!(
                    [
                      {"foo": 10, "bar": {"bbbar": 10, "bbboz": 11 } },
                      {"foo": 20, "bar": {"bbbar": 50, "bbboz": 25 } },
                      {"foo": 30, "bar": {"bbbar": 92, "bbboz": 34 } }
                    ]
                ),
                &json!(
                    [
                      {"foo": 10, "bar": {"bbbar": 10, "bbboz": 11 } },
                      {"foo": 21, "bar": {"bbbar": 50, "bbboz": 25 } },
                      {"foo": 30, "bar": {"bbbar": 92, "bbboz": 34 } }
                    ]
                ),
                false
            )
            .diff,
            Some(json!(
               [
                 [' '],
                 ['~', {"foo": { "__old": 20, "__new": 21 } }],
                 [' ']
               ]
            ))
        );
    }

    #[test]
    fn test_scalar_keys() {
        assert_eq!(JsonDiff::diff(&json!(42), &json!(42), true).diff, None);

        assert_eq!(
            JsonDiff::diff(&json!("foo"), &json!("foo"), true).diff,
            None
        );

        assert_eq!(JsonDiff::diff(&json!(42), &json!(10), true).diff, None);
    }

    #[test]
    fn test_objects_keys() {
        assert_eq!(JsonDiff::diff(&json!({}), &json!({}), true).diff, None);

        assert_eq!(
            JsonDiff::diff(
                &json!({"foo": 42, "bar": 10 }),
                &json!({"foo": 42, "bar": 10 }),
                true
            )
            .diff,
            None
        );

        assert_eq!(
            JsonDiff::diff(
                &json!({"foo": 42, "bar": {"bbbar": 10, "bbboz": 11 } }),
                &json!({"foo": 42, "bar": {"bbbar": 10, "bbboz": 11 } }),
                true
            )
            .diff,
            None
        );

        assert_eq!(
            JsonDiff::diff(&json!({"foo": 42, "bar": 10 }), &json!({"bar": 10 }), true).diff,
            Some(json!({"foo__deleted": 42 }))
        );

        assert_eq!(
            JsonDiff::diff(&json!({"bar": 10 }), &json!({"foo": 42, "bar": 10 }), true).diff,
            Some(json!({"foo__added": 42 }))
        );

        assert_eq!(
            JsonDiff::diff(&json!({"foo": 42 }), &json!({"foo": 10 }), true).diff,
            None
        );

        assert_eq!(
            JsonDiff::diff(
                &json!({"foo": 42, "bar": {"bbbar": 10 }}),
                &json!({"foo": 42, "bar": {"bbbar": 12 }}),
                true
            )
            .diff,
            None
        );

        assert_eq!(
            JsonDiff::diff(
                &json!({"foo": 42, "bar": {"bbbar": 10, "bbboz": 11 } }),
                &json!({"foo": 42, "bar": {"bbbar": 12 } }),
                true
            )
            .diff,
            Some(json!({"bar": {"bbboz__deleted": 11 } }))
        );
    }

    #[test]
    fn test_array_of_scalars_keys() {
        assert_eq!(
            JsonDiff::diff(&json!([10, 20, 30]), &json!([10, 20, 30]), true).diff,
            None
        );

        assert_eq!(
            JsonDiff::diff(&json!([10, 20, 30]), &json!([10, 42, 30]), true).diff,
            None
        );

        assert_eq!(
            JsonDiff::diff(&json!([10, 20, 30]), &json!([10, 30]), true).diff,
            Some(json!([[' ', 10], ['-', 20], [' ', 30]]))
        );

        assert_eq!(
            JsonDiff::diff(&json!([10, 30]), &json!([10, 20, 30]), true).diff,
            Some(json!([[' ', 10], ['+', 20], [' ', 30]]))
        );

        assert_eq!(
            JsonDiff::diff(&json!([10, 20]), &json!([10, 20, 30]), true).diff,
            Some(json!([[' ', 10], [' ', 20], ['+', 30]]))
        );
    }

    #[test]
    fn test_array_of_objects_keys() {
        assert_eq!(
            JsonDiff::diff(
                &json!([{"foo": 10, "foo": 20, "foo": 30}]),
                &json!([{"foo": 10, "foo": 20, "foo": 30}]),
                true
            )
            .diff,
            None
        );

        assert_eq!(JsonDiff::diff(&json!([{}]), &json!([{}]), true).diff, None);

        assert_eq!(JsonDiff::diff(&json!([[]]), &json!([[]]), true).diff, None);

        assert_eq!(
            JsonDiff::diff(
                &json!([{"a": 1, "b": 2 }, {"a": 1, "b": 2 }]),
                &json!([{"a": 1, "b": 2 }, {"a": 1, "b": 2 }]),
                true
            )
            .diff,
            None
        );

        assert_eq!(
            JsonDiff::diff(
                &json!([{"foo": 10 }, {"foo": 20 }, {"foo": 30 }]),
                &json!([{"foo": 10 }, {"foo": 30 }]),
                true
            )
            .diff,
            Some(json!([[' '], ['-', {"foo": 20 }], [' ']]))
        );

        assert_eq!(
            JsonDiff::diff(
                &json!([{"foo": 10 }, {"foo": 30 }]),
                &json!([{"foo": 10 }, {"foo": 20 }, {"foo": 30 }]),
                true
            )
            .diff,
            Some(json!([[' '], ['+', {"foo": 20 }], [' ']]))
        );

        assert_eq!(
            JsonDiff::diff(
                &json!(
                    [
                      {"foo": 10, "bar": {"bbbar": 10, "bbboz": 11 } },
                      {"foo": 20, "bar": {"bbbar": 50, "bbboz": 25 } },
                      {"foo": 30, "bar": {"bbbar": 92, "bbboz": 34 } }
                    ]
                ),
                &json!(
                    [
                      {"foo": 10, "bar": {"bbbar": 10, "bbboz": 11 } },
                      {"foo": 21, "bar": {"bbbar": 50, "bbboz": 25 } },
                      {"foo": 30, "bar": {"bbbar": 92, "bbboz": 34 } }
                    ]
                ),
                true
            )
            .diff,
            None
        );
    }

    #[test]
    fn test_diff_string() {
        fn read_json_file(filename: &str) -> Result<serde_json::Value, Box<dyn Error>> {
            // Get path
            let path = Path::new(filename);

            // Open the file in read-only mode with buffer.
            let file = File::open(path)?;
            let reader = BufReader::new(file);

            // Read the JSON contents of the file as an instance of `Value`.
            let value = serde_json::from_reader(reader)?;

            // Return the `Value`.
            Ok(value)
        }

        let json1 = read_json_file("data/a.json").unwrap();
        let json2 = read_json_file("data/b.json").unwrap();

        assert_eq!(
            JsonDiff::diff_string(&json1, &json2, false).unwrap(),
            std::fs::read_to_string("data/result.jsdiff")
                .unwrap()
                .replace("\r\n", "\n")
        );

        assert_eq!(JsonDiff::diff_string(&json1, &json1, false), None);
    }
}
