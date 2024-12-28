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
    match jsonschema::meta::try_validate(&schema.0) {
        Ok(Ok(_)) => true,
        Ok(Err(err)) => {
            notice!("Invalid JSON schema at path: {}", err.instance_path);
            false
        }
        Err(err) => {
            notice!("{err}");
            false
        }
    }
}

#[pg_extern(immutable, strict, parallel_safe)]
fn jsonschema_validation_errors(schema: Json, instance: Json) -> Vec<String> {
    let validator = match jsonschema::validator_for(&schema.0) {
        Ok(v) => v,
        Err(err) => return vec![err.to_string()],
    };
    validator
        .iter_errors(&instance.0)
        .map(|err| err.to_string())
        .collect()
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
    fn test_jsonschema_unknown_specification() {
        assert!(!crate::jsonschema_is_valid(Json(json!({
            "$schema": "invalid-uri", "type": "string"
        }))));
    }

    #[pg_test]
    fn test_jsonschema_validation_errors_none() {
        let errors = crate::jsonschema_validation_errors(
            Json(json!({ "maxLength": 4 })),
            Json(json!("foo")),
        );
        assert!(errors.is_empty());
    }

    #[pg_test]
    fn test_jsonschema_validation_erros_one() {
        let errors = crate::jsonschema_validation_errors(
            Json(json!({ "maxLength": 4 })),
            Json(json!("123456789")),
        );
        assert!(errors.len() == 1);
        assert!(errors[0] == *"\"123456789\" is longer than 4 characters");
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
        assert!(errors[0] == *"[] is not of type \"number\"");
        assert!(errors[1] == *"\"1\" is not of type \"boolean\"");
        assert!(errors[2] == *"1 is not of type \"string\"");
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
