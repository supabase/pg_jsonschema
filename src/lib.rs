use pgx::*;

pg_module_magic!();

#[pg_extern(immutable, strict)]
fn json_matches_schema(schema: Json, instance: Json) -> bool {
    jsonschema::is_valid(&schema.0, &instance.0)
}

#[pg_extern(immutable, strict)]
fn jsonb_matches_schema(schema: Json, instance: JsonB) -> bool {
    jsonschema::is_valid(&schema.0, &instance.0)
}

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use pgx::*;
    use serde_json::json;

    #[pg_test]
    fn test_json_matches_schema_rs() {
        // Test from Rust
        assert!(crate::json_matches_schema(
            Json(json!({"maxLength": 5})),
            Json(json!("foo")),
        ));
    }

    #[pg_test]
    fn test_jsonb_matches_schema_spi() {
        // Test from SQL
        let result = Spi::get_one::<bool>(
            r#"
            select jsonb_matches_schema('{"maxLength": 5}', '"foo"')
        "#,
        )
        .expect("error?");
        assert!(result);
    }

    #[pg_test]
    fn test_json_matches_schema_spi_fail_wrong_type() {
        // Test from SQL
        let result = Spi::get_one::<bool>(
            r#"
            select json_matches_schema('{"type": "object"}', '1')
        "#,
        )
        .expect("error?");
        assert_eq!(result, false);
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
