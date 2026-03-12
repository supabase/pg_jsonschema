mod compiled;

use pgrx::*;

use compiled::{JsonSchema, fn_extra_get_or_compile};

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
    match jsonschema::meta::validate(&schema.0) {
        Ok(_) => true,
        Err(err) => {
            notice!("Invalid JSON schema at path: {}", err.instance_path());
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

#[pg_extern(immutable, strict, parallel_safe)]
fn jsonschema_from_json(schema: pgrx::Json) -> JsonSchema {
    JsonSchema::compile(schema.0)
}

#[pg_extern(immutable, strict, parallel_safe)]
fn jsonschema_from_jsonb(schema: pgrx::JsonB) -> JsonSchema {
    JsonSchema::compile(schema.0)
}

pgrx::extension_sql!(
    r#"
    CREATE CAST (json AS jsonschema)
        WITH FUNCTION jsonschema_from_json(json);

    CREATE CAST (jsonb AS jsonschema)
        WITH FUNCTION jsonschema_from_jsonb(jsonb);
    "#,
    name = "jsonschema_casts",
    requires = [jsonschema_from_json, jsonschema_from_jsonb],
);

#[pg_extern(immutable, strict, parallel_safe)]
fn json_matches_compiled_schema(
    schema: JsonSchema,
    instance: Json,
    fcinfo: pg_sys::FunctionCallInfo,
) -> bool {
    let validator = unsafe { fn_extra_get_or_compile(&schema, fcinfo) };
    validator.is_valid(&instance.0)
}

#[pg_extern(immutable, strict, parallel_safe)]
fn jsonb_matches_compiled_schema(
    schema: JsonSchema,
    instance: pgrx::JsonB,
    fcinfo: pg_sys::FunctionCallInfo,
) -> bool {
    let validator = unsafe { fn_extra_get_or_compile(&schema, fcinfo) };
    validator.is_valid(&instance.0)
}

#[pg_extern(
    immutable,
    strict,
    parallel_safe,
    name = "jsonschema_validation_errors_compiled"
)]
fn jsonschema_validation_errors_compiled(
    schema: JsonSchema,
    instance: Json,
    fcinfo: pg_sys::FunctionCallInfo,
) -> Vec<String> {
    let validator = unsafe { fn_extra_get_or_compile(&schema, fcinfo) };
    validator
        .iter_errors(&instance.0)
        .map(|err| err.to_string())
        .collect()
}

#[pg_extern(
    immutable,
    strict,
    parallel_safe,
    name = "jsonb_validation_errors_compiled"
)]
fn jsonschema_validation_errors_compiled_jsonb(
    schema: JsonSchema,
    instance: pgrx::JsonB,
    fcinfo: pg_sys::FunctionCallInfo,
) -> Vec<String> {
    let validator = unsafe { fn_extra_get_or_compile(&schema, fcinfo) };
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

    macro_rules! compiled_schema_tests {
        (json { $($name:ident: $schema:literal, $instance:literal => $expected:literal)* }) => {$(
            #[pg_test]
            fn $name() {
                let result = Spi::get_one::<bool>(concat!(
                    "SELECT json_matches_compiled_schema('", $schema, "'::jsonschema, '", $instance, "'::json)"
                )).unwrap().unwrap();
                assert_eq!(result, $expected);
            }
        )*};
        (jsonb { $($name:ident: $schema:literal, $instance:literal => $expected:literal)* }) => {$(
            #[pg_test]
            fn $name() {
                let result = Spi::get_one::<bool>(concat!(
                    "SELECT jsonb_matches_compiled_schema('", $schema, "'::jsonschema, '", $instance, "'::jsonb)"
                )).unwrap().unwrap();
                assert_eq!(result, $expected);
            }
        )*};
        (errors_json { $($name:ident: $schema:literal, $instance:literal => [$($err:literal),*])* }) => {$(
            #[pg_test]
            fn $name() {
                let errors = Spi::get_one::<Vec<String>>(concat!(
                    "SELECT jsonschema_validation_errors_compiled('", $schema, "'::jsonschema, '",
                    $instance, "'::json)"
                )).unwrap().unwrap();
                assert_eq!(errors, [$($err),*]);
            }
        )*};
        (errors_jsonb { $($name:ident: $schema:literal, $instance:literal => [$($err:literal),*])* }) => {$(
            #[pg_test]
            fn $name() {
                let errors = Spi::get_one::<Vec<String>>(concat!(
                    "SELECT jsonb_validation_errors_compiled('", $schema, "'::jsonschema, '",
                    $instance, "'::jsonb)"
                )).unwrap().unwrap();
                assert_eq!(errors, [$($err),*]);
            }
        )*};
    }

    compiled_schema_tests!(json {
        test_json_matches_compiled_schema: r#"{"type":"string"}"#, r#""hello""# => true
        test_json_rejects_compiled_schema: r#"{"type":"string"}"#, r#"42"#      => false
    });

    compiled_schema_tests!(jsonb {
        test_jsonb_matches_compiled_schema: r#"{"type":"string"}"#, r#""hello""# => true
        test_jsonb_rejects_compiled_schema: r#"{"type":"string"}"#, r#"42"#      => false
    });

    compiled_schema_tests!(errors_json {
        test_validation_errors_compiled_with_error:
            r#"{"maxLength":4}"#, r#""toolong""#
            => [r#""toolong" is longer than 4 characters"#]
    });

    compiled_schema_tests!(errors_jsonb {
        test_validation_errors_compiled_jsonb:
            r#"{"type":"string"}"#, r#"42"#
            => ["42 is not of type \"string\""]
        test_validation_errors_compiled_jsonb_object:
            r#"{"type":"object","required":["name"],"properties":{"name":{"type":"string"}}}"#,
            r#"{"name":42}"#
            => ["42 is not of type \"string\""]
    });

    #[pg_test]
    fn test_jsonschema_cast_from_json() {
        let result =
            Spi::get_one::<bool>(r#"SELECT '{"type":"object"}'::json::jsonschema IS NOT NULL"#)
                .unwrap()
                .unwrap();
        assert!(result);
    }

    #[pg_test]
    fn test_jsonschema_cast_from_jsonb() {
        let result =
            Spi::get_one::<bool>(r#"SELECT '{"type":"object"}'::jsonb::jsonschema IS NOT NULL"#)
                .unwrap()
                .unwrap();
        assert!(result);
    }

    #[pg_test]
    fn test_jsonschema_output_is_canonical() {
        let result = Spi::get_one::<String>(r#"SELECT '{"b":1,"a":2}'::jsonschema::text"#)
            .unwrap()
            .unwrap();
        assert_eq!(result, r#"{"a":2,"b":1}"#);
    }

    #[pg_test]
    fn test_validation_errors_compiled_no_errors() {
        let errors = Spi::get_one::<Vec<String>>(
            r#"SELECT jsonschema_validation_errors_compiled('{"maxLength":4}'::jsonschema, '"foo"'::json)"#,
        )
        .unwrap()
        .unwrap();
        assert!(errors.is_empty());
    }

    #[pg_test]
    fn test_validation_errors_compiled_multiple() {
        let errors = Spi::get_one::<Vec<String>>(
            r#"
            SELECT jsonschema_validation_errors_compiled(
                '{
                    "type":"object",
                    "properties":{
                        "foo":{"type":"string"},
                        "bar":{"type":"number"},
                        "baz":{"type":"boolean"}
                    }
                }'::jsonschema,
                '{"foo":1,"bar":[],"baz":"1"}'::json
            )
            "#,
        )
        .unwrap()
        .unwrap();
        let mut errors = errors;
        errors.sort_unstable();
        assert_eq!(
            errors,
            vec![
                r#""1" is not of type "boolean""#.to_string(),
                r#"1 is not of type "string""#.to_string(),
                r#"[] is not of type "number""#.to_string(),
            ]
        );
    }

    #[pg_test]
    fn test_compiled_schema_reuse_across_calls() {
        let result = Spi::get_one::<i64>(
            r#"
            SELECT count(*)
            FROM generate_series(1, 100) i
            WHERE jsonb_matches_compiled_schema(
                '{"type":"integer","minimum":0}'::jsonschema,
                to_jsonb(i)
            )
        "#,
        )
        .unwrap()
        .unwrap();
        assert_eq!(result, 100);
    }

    #[pg_test]
    fn test_canonical_dedup_same_validation() {
        let r1 = Spi::get_one::<bool>(
            r#"SELECT json_matches_compiled_schema('{"type":"string","maxLength":5}'::jsonschema, '"hi"'::json)"#,
        )
        .unwrap()
        .unwrap();
        let r2 = Spi::get_one::<bool>(
            r#"SELECT json_matches_compiled_schema('{"maxLength":5,"type":"string"}'::jsonschema, '"hi"'::json)"#,
        )
        .unwrap()
        .unwrap();
        assert!(r1);
        assert!(r2);
    }

    #[pg_test]
    fn test_callsite_cache_refresh_on_schema_change() {
        // Two rows with different schemas at the same callsite.
        // The second row triggers the L1 refresh path (fn_extra exists but schema changed).
        let result = Spi::get_one::<i64>(
            r#"
            WITH data(s, v) AS (
                VALUES
                    ('{"type":"string"}'::jsonschema, '"hello"'::jsonb),
                    ('{"type":"integer"}'::jsonschema, '42'::jsonb)
            )
            SELECT count(*) FROM data WHERE jsonb_matches_compiled_schema(s, v)
            "#,
        )
        .unwrap()
        .unwrap();
        assert_eq!(result, 2);
    }

    #[pg_test]
    fn test_check_constraint_with_compiled_schema() {
        Spi::run(
            r#"
            CREATE TEMP TABLE test_compiled_check (
                data jsonb,
                CHECK (jsonb_matches_compiled_schema(
                    '{"type":"object","properties":{"name":{"type":"string"}},"required":["name"]}'::jsonschema,
                    data
                ))
            )
        "#,
        )
        .unwrap();
        Spi::run(r#"INSERT INTO test_compiled_check VALUES ('{"name":"alice"}')"#).unwrap();
    }

    #[pg_test]
    fn test_jsonschema_roundtrip_in_column() {
        Spi::run(
            r#"
            CREATE TEMP TABLE schema_store (id int, s jsonschema);
            INSERT INTO schema_store VALUES
                (1, '{"type":"string","maxLength":5}'::jsonschema),
                (2, '{"type":"integer","minimum":0}'::jsonschema);
        "#,
        )
        .unwrap();

        let ok = Spi::get_one::<bool>(
            r#"SELECT jsonb_matches_compiled_schema(s, '"hi"'::jsonb)
               FROM schema_store WHERE id = 1"#,
        )
        .unwrap()
        .unwrap();
        assert!(ok);

        let not_ok = Spi::get_one::<bool>(
            r#"SELECT jsonb_matches_compiled_schema(s, '"toolong"'::jsonb)
               FROM schema_store WHERE id = 1"#,
        )
        .unwrap()
        .unwrap();
        assert!(!not_ok);

        let ok_int = Spi::get_one::<bool>(
            r#"SELECT jsonb_matches_compiled_schema(s, '42'::jsonb)
               FROM schema_store WHERE id = 2"#,
        )
        .unwrap()
        .unwrap();
        assert!(ok_int);

        let not_ok_int = Spi::get_one::<bool>(
            r#"SELECT jsonb_matches_compiled_schema(s, '-1'::jsonb)
               FROM schema_store WHERE id = 2"#,
        )
        .unwrap()
        .unwrap();
        assert!(!not_ok_int);
    }

    #[pg_test]
    fn test_jsonschema_equality() {
        let result = Spi::get_one::<bool>(
            r#"SELECT '{"type":"string","maxLength":5}'::jsonschema
                    = '{"maxLength":5,"type":"string"}'::jsonschema"#,
        )
        .unwrap()
        .unwrap();
        assert!(result, "canonically equal schemas must be SQL-equal");
    }

    #[pg_test]
    #[should_panic(expected = "invalid JSON: expected ident at line 1 column 2")]
    fn test_invalid_json_cast_to_jsonschema() {
        Spi::run("SELECT 'not valid json'::jsonschema").unwrap();
    }

    #[pg_test]
    fn test_jsonschema_validation_errors_invalid_schema() {
        let errors = crate::jsonschema_validation_errors(
            Json(json!({ "enum": 1 })),
            Json(json!("anything")),
        );
        assert!(!errors.is_empty());
    }

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
    fn test_json_matches_schema_arbitrary_precision() {
        assert!(crate::json_matches_schema(
            Json(json!({ "type": "number", "multipleOf": 0.1 })),
            Json(json!(17.2)),
        ));
        assert!(crate::json_matches_schema(
            Json(json!({ "type": "number", "multipleOf": 0.2 })),
            Json(json!(17.2)),
        ));
        assert!(!crate::json_matches_schema(
            Json(json!({ "type": "number", "multipleOf": 0.3 })),
            Json(json!(17.2)),
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
