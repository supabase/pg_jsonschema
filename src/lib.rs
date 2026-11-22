mod compiled;
mod raw;

use jsonschema::{
    Validator,
    json::{JsonText, Jsonb, jsonb::take_pending_error},
};
use pgrx::*;

use compiled::{JsonSchema, SchemaArg, fn_extra_get_or_compile};
use raw::{RawJson, RawJsonb};

pg_module_magic!();

#[pg_extern(immutable, strict, parallel_safe)]
fn json_matches_schema(schema: Json, instance: RawJson) -> bool {
    let document = instance.parse();
    jsonschema::options_for::<JsonText>()
        .build(&schema.0)
        .expect("Invalid schema")
        .is_valid(document.root())
}

#[pg_extern(immutable, strict, parallel_safe)]
fn jsonb_matches_schema(schema: Json, instance: RawJsonb) -> bool {
    jsonschema::options_for::<Jsonb>()
        .build(&schema.0)
        .unwrap_or_else(|err| error!("invalid JSON schema: {err}"))
        .is_valid(Jsonb::root(instance.as_bytes()))
}

// Reporting a `jsonb` instance rebuilds it as a `serde_json::Value`, which stops at a fixed depth
// and records why instead.
fn jsonb_errors(validator: &Validator<Jsonb>, instance: &RawJsonb) -> Vec<String> {
    let errors = validator
        .iter_errors(Jsonb::root(instance.as_bytes()))
        .map(|err| err.to_string())
        .collect();
    if let Some(message) = take_pending_error() {
        error!("{message}");
    }
    errors
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
fn jsonschema_validation_errors(schema: Json, instance: RawJson) -> Vec<String> {
    let document = instance.parse();
    let validator = match jsonschema::options_for::<JsonText>().build(&schema.0) {
        Ok(v) => v,
        Err(err) => return vec![err.to_string()],
    };
    validator
        .iter_errors(document.root())
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

#[pg_extern(immutable, strict, parallel_safe, requires = [JsonSchema])]
fn json_matches_compiled_schema(
    schema: SchemaArg,
    instance: RawJson,
    fcinfo: pg_sys::FunctionCallInfo,
) -> bool {
    let document = instance.parse();
    let validator = unsafe { fn_extra_get_or_compile::<JsonText>(&schema, fcinfo) };
    validator.is_valid(document.root())
}

#[pg_extern(immutable, strict, parallel_safe, requires = [JsonSchema])]
fn jsonb_matches_compiled_schema(
    schema: SchemaArg,
    instance: RawJsonb,
    fcinfo: pg_sys::FunctionCallInfo,
) -> bool {
    let validator = unsafe { fn_extra_get_or_compile::<Jsonb>(&schema, fcinfo) };
    validator.is_valid(Jsonb::root(instance.as_bytes()))
}

#[pg_extern(immutable, strict, parallel_safe, requires = [JsonSchema])]
fn json_validation_errors_compiled(
    schema: SchemaArg,
    instance: RawJson,
    fcinfo: pg_sys::FunctionCallInfo,
) -> Vec<String> {
    let document = instance.parse();
    let validator = unsafe { fn_extra_get_or_compile::<JsonText>(&schema, fcinfo) };
    validator
        .iter_errors(document.root())
        .map(|err| err.to_string())
        .collect()
}

#[pg_extern(immutable, strict, parallel_safe, requires = [JsonSchema])]
fn jsonb_validation_errors_compiled(
    schema: SchemaArg,
    instance: RawJsonb,
    fcinfo: pg_sys::FunctionCallInfo,
) -> Vec<String> {
    let validator = unsafe { fn_extra_get_or_compile::<Jsonb>(&schema, fcinfo) };
    jsonb_errors(&validator, &instance)
}

#[pg_schema]
#[cfg(any(test, feature = "pg_test"))]
mod tests {
    use pgrx::*;
    use serde_json::json;

    fn call_json_matches_compiled_schema(schema: &str, instance: &str, expected: bool) {
        let result = Spi::get_one::<bool>(&format!(
            "select json_matches_compiled_schema('{schema}'::jsonschema, '{instance}'::json)"
        ))
        .unwrap()
        .unwrap();
        assert_eq!(result, expected);
    }

    fn call_jsonb_matches_compiled_schema(schema: &str, instance: &str, expected: bool) {
        let result = Spi::get_one::<bool>(&format!(
            "select jsonb_matches_compiled_schema('{schema}'::jsonschema, '{instance}'::jsonb)"
        ))
        .unwrap()
        .unwrap();
        assert_eq!(result, expected);
    }

    fn call_json_validation_errors_compiled(
        schema: &str,
        instance: &str,
        expected_errors: &[&str],
    ) {
        let errors = Spi::get_one::<Vec<String>>(&format!(
            "select json_validation_errors_compiled('{schema}'::jsonschema, '{instance}'::json)"
        ))
        .unwrap()
        .unwrap();
        assert_eq!(errors, expected_errors);
    }

    fn call_json_matches_schema(schema: &str, instance: &str) -> bool {
        Spi::get_one::<bool>(&format!(
            "SELECT json_matches_schema('{schema}', '{instance}')"
        ))
        .unwrap()
        .unwrap()
    }

    fn call_jsonschema_validation_errors(schema: &str, instance: &str) -> Vec<String> {
        Spi::get_one::<Vec<String>>(&format!(
            "SELECT jsonschema_validation_errors('{schema}', '{instance}')"
        ))
        .unwrap()
        .unwrap()
    }

    fn call_jsonb_validation_errors_compiled(
        schema: &str,
        instance: &str,
        expected_errors: &[&str],
    ) {
        let errors = Spi::get_one::<Vec<String>>(&format!(
            "select jsonb_validation_errors_compiled('{schema}'::jsonschema, '{instance}'::jsonb)"
        ))
        .unwrap()
        .unwrap();
        assert_eq!(errors, expected_errors);
    }

    #[pg_test]
    fn test_json_matches_compiled_schema() {
        call_json_matches_compiled_schema(r#"{"type":"string"}"#, r#""hello""#, true);
    }

    #[pg_test]
    fn test_json_rejects_compiled_schema() {
        call_json_matches_compiled_schema(r#"{"type":"string"}"#, r#"42"#, false);
    }

    #[pg_test]
    fn test_jsonb_matches_compiled_schema() {
        call_jsonb_matches_compiled_schema(r#"{"type":"string"}"#, r#""hello""#, true);
    }

    #[pg_test]
    fn test_jsonb_rejects_compiled_schema() {
        call_jsonb_matches_compiled_schema(r#"{"type":"string"}"#, r#"42"#, false);
    }

    #[pg_test]
    fn test_validation_errors_compiled_with_error() {
        call_json_validation_errors_compiled(
            r#"{"maxLength":4}"#,
            r#""toolong""#,
            &[r#""toolong" is longer than 4 characters"#],
        );
    }

    #[pg_test]
    fn test_validation_errors_compiled_jsonb() {
        call_jsonb_validation_errors_compiled(
            r#"{"type":"string"}"#,
            r#"42"#,
            &["42 is not of type \"string\""],
        );
    }

    #[pg_test]
    fn test_validation_errors_compiled_jsonb_object() {
        call_jsonb_validation_errors_compiled(
            r#"{"type":"object","required":["name"],"properties":{"name":{"type":"string"}}}"#,
            r#"{"name":42}"#,
            &["42 is not of type \"string\""],
        );
    }

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
            r#"SELECT json_validation_errors_compiled('{"maxLength":4}'::jsonschema, '"foo"'::json)"#,
        )
        .unwrap()
        .unwrap();
        assert!(errors.is_empty());
    }

    // Errors for object members follow the document's key order.
    #[pg_test]
    fn test_validation_errors_compiled_multiple() {
        let errors = Spi::get_one::<Vec<String>>(
            r#"
            SELECT json_validation_errors_compiled(
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
        assert_eq!(
            errors,
            [
                r#"1 is not of type "string""#,
                r#"[] is not of type "number""#,
                r#""1" is not of type "boolean""#,
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

    // Called through an operator, `fn_expr` is an `OpExpr`, not a `FuncExpr`.
    #[pg_test]
    fn test_callsite_cache_operator() {
        Spi::run(
            r#"
            CREATE OPERATOR @@@ (FUNCTION = jsonb_matches_compiled_schema, LEFTARG = jsonschema, RIGHTARG = jsonb);
            CREATE TABLE t AS SELECT '{"type":"integer"}'::jsonschema AS s, '42'::jsonb AS v;
            "#,
        )
        .unwrap();
        let result = Spi::get_one::<bool>("SELECT s @@@ v FROM t")
            .unwrap()
            .unwrap();
        assert!(result);
    }

    // A PL/pgSQL variable is a stable argument, yet the callsite outlives its changes.
    #[pg_test]
    fn test_callsite_cache_plpgsql_variable_schema() {
        Spi::run(
            r#"
            DO $$
            DECLARE
                t text;
                results bool[] := '{}';
            BEGIN
                FOREACH t IN ARRAY ARRAY['{"type":"string"}', '{"type":"integer"}'] LOOP
                    DECLARE s jsonschema := t::jsonschema;
                    BEGIN
                        results := results || jsonb_matches_compiled_schema(s, '42');
                    END;
                END LOOP;
                IF results <> ARRAY[false, true] THEN
                    RAISE EXCEPTION 'unexpected results: %', results;
                END IF;
            END
            $$;
            "#,
        )
        .unwrap();
    }

    // Schemas from a column arrive as TOAST pointers; each row's pointer picks its own schema.
    #[pg_test]
    fn test_callsite_cache_toasted_schema_column() {
        Spi::run(
            r#"
            CREATE TEMP TABLE big_schemas(s jsonschema);
            ALTER TABLE big_schemas ALTER COLUMN s SET STORAGE EXTERNAL;
            INSERT INTO big_schemas VALUES
                (('{"type":"string","description":"' || repeat('x', 100000) || '"}')::jsonschema),
                (('{"type":"integer","description":"' || repeat('y', 100000) || '"}')::jsonschema);
            "#,
        )
        .unwrap();
        let result = Spi::get_one::<i64>(
            r#"
            SELECT count(*)
            FROM big_schemas, (VALUES ('"a"'::jsonb), ('1'::jsonb), ('"b"'::jsonb)) AS v(doc)
            WHERE jsonb_matches_compiled_schema(s, doc)
            "#,
        )
        .unwrap()
        .unwrap();
        assert_eq!(result, 3);
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
        let errors = call_jsonschema_validation_errors(r#"{"enum": 1}"#, r#""anything""#);
        assert!(!errors.is_empty());
    }

    #[pg_test]
    #[should_panic(expected = "Invalid schema: ")]
    fn test_json_matches_schema_invalid_schema() {
        Spi::run(r#"SELECT json_matches_schema('{"type": "obj"}', '1')"#).unwrap();
    }

    #[pg_test]
    fn test_json_matches_schema_rs() {
        assert!(call_json_matches_schema(r#"{"maxLength": 5}"#, r#""foo""#));
    }

    #[pg_test]
    fn test_json_not_matches_schema_rs() {
        assert!(!call_json_matches_schema(
            r#"{"maxLength": 5}"#,
            r#""foobar""#
        ));
    }

    #[pg_test]
    fn test_json_matches_schema_arbitrary_precision() {
        assert!(call_json_matches_schema(
            r#"{"type": "number", "multipleOf": 0.1}"#,
            "17.2"
        ));
        assert!(call_json_matches_schema(
            r#"{"type": "number", "multipleOf": 0.2}"#,
            "17.2"
        ));
        assert!(!call_json_matches_schema(
            r#"{"type": "number", "multipleOf": 0.3}"#,
            "17.2"
        ));
    }

    #[pg_test]
    fn test_jsonb_matches_schema_rs() {
        let result = Spi::get_one::<bool>(
            r#"SELECT jsonb_matches_schema('{"maxLength": 5}', '"foo"'::jsonb)"#,
        )
        .unwrap()
        .unwrap();
        assert!(result);
    }

    #[pg_test]
    fn test_jsonb_not_matches_schema_rs() {
        let result = Spi::get_one::<bool>(
            r#"SELECT jsonb_matches_schema('{"maxLength": 5}', '"foobar"'::jsonb)"#,
        )
        .unwrap()
        .unwrap();
        assert!(!result);
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
        let errors = call_jsonschema_validation_errors(r#"{"maxLength": 4}"#, r#""foo""#);
        assert!(errors.is_empty());
    }

    #[pg_test]
    fn test_jsonschema_validation_erros_one() {
        let errors = call_jsonschema_validation_errors(r#"{"maxLength": 4}"#, r#""123456789""#);
        assert_eq!(errors, [r#""123456789" is longer than 4 characters"#]);
    }

    // Errors for object members follow the document's key order.
    #[pg_test]
    fn test_jsonschema_validation_errors_multiple() {
        let errors = call_jsonschema_validation_errors(
            r#"{
                "type": "object",
                "properties": {
                    "foo": {"type": "string"},
                    "bar": {"type": "number"},
                    "baz": {"type": "boolean"},
                    "additionalProperties": false
                }
            }"#,
            r#"{"foo": 1, "bar": [], "baz": "1"}"#,
        );
        assert_eq!(
            errors,
            [
                r#"1 is not of type "string""#,
                r#"[] is not of type "number""#,
                r#""1" is not of type "boolean""#,
            ]
        );
    }
    // Each function reads the instance in its own way; all must give the same verdict.
    fn assert_representations_agree(schema: &str, instance: &str, expected: bool) {
        for function in [
            "json_matches_schema('{s}', '{i}'::json)",
            "jsonb_matches_schema('{s}', '{i}'::jsonb)",
            "json_matches_compiled_schema('{s}'::jsonschema, '{i}'::json)",
            "jsonb_matches_compiled_schema('{s}'::jsonschema, '{i}'::jsonb)",
        ] {
            let query = format!(
                "SELECT {}",
                function.replace("{s}", schema).replace("{i}", instance)
            );
            let result = Spi::get_one::<bool>(&query).unwrap().unwrap();
            assert_eq!(result, expected, "{query}");
        }
    }

    #[pg_test]
    fn test_agree_string() {
        assert_representations_agree(r#"{"type":"string"}"#, r#""hello""#, true);
    }

    #[pg_test]
    fn test_agree_string_invalid() {
        assert_representations_agree(r#"{"type":"string"}"#, r#"42"#, false);
    }

    #[pg_test]
    fn test_agree_integer() {
        assert_representations_agree(r#"{"type":"integer"}"#, r#"-17"#, true);
    }

    #[pg_test]
    fn test_agree_integer_fraction() {
        assert_representations_agree(r#"{"type":"integer"}"#, r#"1.5"#, false);
    }

    #[pg_test]
    fn test_agree_number_large() {
        assert_representations_agree(
            r#"{"type":"integer"}"#,
            r#"123456789012345678901234567890"#,
            true,
        );
    }

    #[pg_test]
    fn test_agree_multiple_of() {
        assert_representations_agree(r#"{"multipleOf":0.1}"#, r#"17.2"#, true);
    }

    #[pg_test]
    fn test_agree_multiple_of_invalid() {
        assert_representations_agree(r#"{"multipleOf":0.3}"#, r#"17.2"#, false);
    }

    #[pg_test]
    fn test_agree_boolean() {
        assert_representations_agree(r#"{"type":"boolean"}"#, r#"false"#, true);
    }

    #[pg_test]
    fn test_agree_null() {
        assert_representations_agree(r#"{"type":"null"}"#, r#"null"#, true);
    }

    #[pg_test]
    fn test_agree_empty_array() {
        assert_representations_agree(r#"{"type":"array","maxItems":0}"#, r#"[]"#, true);
    }

    #[pg_test]
    fn test_agree_empty_object() {
        assert_representations_agree(r#"{"type":"object","maxProperties":0}"#, r#"{}"#, true);
    }

    #[pg_test]
    fn test_agree_unique_items() {
        assert_representations_agree(r#"{"uniqueItems":true}"#, r#"[1, 1.0]"#, false);
    }

    #[pg_test]
    fn test_agree_const() {
        assert_representations_agree(
            r#"{"const":{"a":[1,{"b":null}]}}"#,
            r#"{"a":[1.0,{"b":null}]}"#,
            true,
        );
    }

    #[pg_test]
    fn test_agree_astral_length() {
        assert_representations_agree(r#"{"maxLength":1}"#, r#""😀😀""#, false);
    }

    // `json` keeps every occurrence of a key, `jsonb` the last one.
    #[pg_test]
    fn test_agree_repeated_key() {
        assert_representations_agree(
            r#"{"properties":{"a":{"type":"string"}}}"#,
            r#"{"a":1,"a":"x"}"#,
            true,
        );
    }

    #[pg_test]
    fn test_agree_nested_invalid() {
        assert_representations_agree(
            r#"{"properties":{"items":{"items":{"required":["id"]}}}}"#,
            r#"{"items":[{"id":1},{"name":"x"}]}"#,
            false,
        );
    }

    #[pg_test]
    fn test_validation_errors_compiled_jsonb_many() {
        call_jsonb_validation_errors_compiled(
            r#"{"type":"object","required":["name"],"properties":{"age":{"type":"number"},"tags":{"uniqueItems":true}},"additionalProperties":false}"#,
            r#"{"age":"x","tags":[1,1],"extra":{"deep":[1]}}"#,
            &[
                "\"name\" is a required property",
                "\"x\" is not of type \"number\"",
                "[1,1] has non-unique elements",
                "Additional properties are not allowed ('extra' was unexpected)",
            ],
        );
    }

    // The large document is stored out of line, so it reaches the function as a TOAST pointer;
    // the small one is stored with a 1-byte header.
    #[pg_test]
    fn test_stored_documents() {
        Spi::run(
            r#"
            CREATE TEMP TABLE stored(id int, doc jsonb, jdoc json);
            ALTER TABLE stored
                ALTER COLUMN doc SET STORAGE EXTERNAL,
                ALTER COLUMN jdoc SET STORAGE EXTERNAL;
            INSERT INTO stored
            SELECT 1, large, large::json
            FROM (
                SELECT jsonb_build_object('data', string_agg(md5(i::text), '')) AS large
                FROM generate_series(1, 1000) i
            ) documents;
            INSERT INTO stored VALUES (2, '{"data":"x"}', '{"data":"x"}');
            "#,
        )
        .unwrap();
        let toast_size = Spi::get_one::<i64>(
            "SELECT pg_relation_size(reltoastrelid) FROM pg_class WHERE oid = 'stored'::regclass",
        )
        .unwrap()
        .unwrap();
        assert!(toast_size > 0);
        let schema = r#"{"properties":{"data":{"type":"string","minLength":32000}}}"#;
        for function in [
            "json_matches_schema('{s}', jdoc)",
            "jsonb_matches_schema('{s}', doc)",
            "json_matches_compiled_schema('{s}'::jsonschema, jdoc)",
            "jsonb_matches_compiled_schema('{s}'::jsonschema, doc)",
        ] {
            let query = format!(
                "SELECT array_agg({} ORDER BY id) FROM stored",
                function.replace("{s}", schema)
            );
            let results = Spi::get_one::<Vec<bool>>(&query).unwrap().unwrap();
            assert_eq!(results, [true, false], "{query}");
        }
    }

    // `json` stores text that `serde_json` rejects; validating it reports the parse error.
    #[pg_test]
    #[should_panic(
        expected = r#"datum must refer to a valid json varlena: Error("recursion limit exceeded", line: 1, column: 128)"#
    )]
    fn test_json_matches_compiled_schema_deep_instance() {
        Spi::run(
            r#"SELECT json_matches_compiled_schema('{"type":"array"}'::jsonschema, (repeat('[', 200) || repeat(']', 200))::json)"#,
        )
        .unwrap();
    }

    #[pg_test]
    #[should_panic(
        expected = r#"datum must refer to a valid json varlena: Error("lone leading surrogate in hex escape", line: 1, column: 7)"#
    )]
    fn test_json_validation_errors_compiled_lone_surrogate() {
        Spi::run(
            r#"SELECT json_validation_errors_compiled('{"type":"number"}'::jsonschema, '"\udc00"'::json)"#,
        )
        .unwrap();
    }

    // Reporting an instance nested this deep is refused rather than overflowing the stack.
    #[pg_test]
    #[should_panic(expected = "maximum nesting depth")]
    fn test_jsonb_validation_errors_deep_instance() {
        Spi::run(
            r#"SELECT jsonb_validation_errors_compiled('{"type":"string"}'::jsonschema, (repeat('[', 200) || repeat(']', 200))::jsonb)"#,
        )
        .unwrap();
    }

    #[pg_test]
    fn test_jsonb_matches_deep_instance() {
        let result = Spi::get_one::<bool>(
            r#"SELECT jsonb_matches_compiled_schema('{"type":"array"}'::jsonschema, (repeat('[', 5000) || repeat(']', 5000))::jsonb)"#,
        )
        .unwrap()
        .unwrap();
        assert!(result);
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
