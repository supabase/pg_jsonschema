use pgrx::*;

pg_module_magic!();

#[pg_extern(immutable, strict, parallel_safe)]
fn json_matches_schema(schema: Json, instance: Json) -> bool {
    jsonschema::is_valid(&schema.0, &instance.0)
}

#[pg_extern(immutable, strict, parallel_safe)]
fn jsonb_matches_schema(schema: Json, instance: JsonB) -> bool {
    jsonschema::is_valid(&schema.0, &instance.0)
}

#[pg_extern(immutable, strict, parallel_safe)]
fn jsonschema_is_valid(schema: Json) -> bool {
    match jsonschema::JSONSchema::compile(&schema.0) {
        Ok(_) => true,
        Err(e) => {
            // Only call notice! for a non empty instance_path
            if e.instance_path.last().is_some() {
                notice!(
                    "Invalid JSON schema at path: {}",
                    e.instance_path.to_string()
                );
            }
            false
        }
    }
}

#[pg_extern(immutable, strict, parallel_safe)]
fn jsonschema_validation_errors(schema: Json, instance: Json) -> Vec<String> {
    let schema = match jsonschema::JSONSchema::compile(&schema.0) {
        Ok(s) => s,
        Err(e) => return vec![e.to_string()],
    };
    let errors = match schema.validate(&instance.0) {
        Ok(_) => vec![],
        Err(e) => e.into_iter().map(|e| e.to_string()).collect(),
    };
    errors
}

#[pg_schema]
#[cfg(any(test, feature = "pg_test"))]
mod tests {
    use pgrx::*;
    use serde_json::json;

    #[pg_test]
    fn test_json_matches_schema_rs() {
        let max_length: i32 = 5;
        assert!(crate::json_matches_schema(
            Json(json!({ "maxLength": max_length })),
            Json(json!("foo")),
        ));
    }

    #[pg_test]
    fn test_json_not_matches_schema_rs() {
        let max_length: i32 = 5;
        assert!(!crate::json_matches_schema(
            Json(json!({ "maxLength": max_length })),
            Json(json!("foobar")),
        ));
    }

    #[pg_test]
    fn test_jsonb_matches_schema_rs() {
        let max_length: i32 = 5;
        assert!(crate::jsonb_matches_schema(
            Json(json!({ "maxLength": max_length })),
            JsonB(json!("foo")),
        ));
    }

    #[pg_test]
    fn test_jsonb_not_matches_schema_rs() {
        let max_length: i32 = 5;
        assert!(!crate::jsonb_matches_schema(
            Json(json!({ "maxLength": max_length })),
            JsonB(json!("foobar")),
        ));
    }

    #[pg_test]
    fn test_json_matches_schema_spi() {
        let result = Spi::get_one::<bool>(
            r#"
            select json_matches_schema('{"type": "object"}', '{}')
        "#,
        )
        .unwrap()
        .unwrap();
        assert!(result);
    }

    #[pg_test]
    fn test_json_not_matches_schema_spi() {
        let result = Spi::get_one::<bool>(
            r#"
            select json_matches_schema('{"type": "object"}', '1')
        "#,
        )
        .unwrap()
        .unwrap();
        assert!(!result);
    }

    #[pg_test]
    fn test_jsonb_matches_schema_spi() {
        let result = Spi::get_one::<bool>(
            r#"
            select jsonb_matches_schema('{"type": "object"}', '{}')
        "#,
        )
        .unwrap()
        .unwrap();
        assert!(result);
    }

    #[pg_test]
    fn test_jsonb_not_matches_schema_spi() {
        let result = Spi::get_one::<bool>(
            r#"
            select jsonb_matches_schema('{"type": "object"}', '1')
        "#,
        )
        .unwrap()
        .unwrap();
        assert!(!result);
    }

    #[pg_test]
    fn test_jsonschema_is_valid() {
        assert!(crate::jsonschema_is_valid(Json(json!({
            "type": "object"
        }))));
    }

    #[pg_test]
    fn test_jsonschema_is_not_valid() {
        assert!(!crate::jsonschema_is_valid(Json(json!({
            "type": "obj"
        }))));
    }

    #[pg_test]
    fn test_jsonschema_validation_errors_none() {
        let errors = crate::jsonschema_validation_errors(
            Json(json!({ "maxLength": 4 })),
            Json(json!("foo")),
        );
        assert!(errors.len() == 0);
    }

    #[pg_test]
    fn test_jsonschema_validation_erros_one() {
        let errors = crate::jsonschema_validation_errors(
            Json(json!({ "maxLength": 4 })),
            Json(json!("123456789")),
        );
        assert!(errors.len() == 1);
        assert!(errors[0] == "\"123456789\" is longer than 4 characters".to_string());
    }

    #[pg_test]
    fn test_jsonschema_validation_errors_multiple() {
        let errors = crate::jsonschema_validation_errors(
            Json(json!(
            {
                "type": "object",
                "properties": {
                    "foo": {
                        "type": "string"
                    },
                    "bar": {
                        "type": "number"
                    },
                    "baz": {
                        "type": "boolean"
                    },
                    "additionalProperties": false,
                }
            })),
            Json(json!({"foo": 1, "bar": [], "baz": "1"})),
        );

        assert!(errors.len() == 3);
        assert!(errors[0] == "[] is not of type \"number\"".to_string());
        assert!(errors[1] == "\"1\" is not of type \"boolean\"".to_string());
        assert!(errors[2] == "1 is not of type \"string\"".to_string());
    }
}

#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {
        // perform one-off initialization when the pg_test framework starts
    }

    pub fn postgresql_conf_options() -> Vec<&'static str> {
        // return any postgresql.conf settings that are required for your tests
        vec![]
    }
}
