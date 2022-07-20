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

#[pg_schema]
#[cfg(any(test, feature = "pg_test"))]
mod tests {
    use pgx::*;
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
        .unwrap();
        assert_eq!(result, false);
    }

    #[pg_test]
    fn test_json_not_matches_schema_spi() {
        let result = Spi::get_one::<bool>(
            r#"
            select json_matches_schema('{"type": "object"}', '1')
        "#,
        )
        .unwrap();
        assert_eq!(result, false);
    }

    #[pg_test]
    fn test_jsonb_matches_schema_spi() {
        let result = Spi::get_one::<bool>(
            r#"
            select json_matches_schema('{"type": "object"}', '{}')
        "#,
        )
        .unwrap();
        assert_eq!(result, false);
    }

    #[pg_test]
    fn test_jsonb_not_matches_schema_spi() {
        let result = Spi::get_one::<bool>(
            r#"
            select json_matches_schema('{"type": "object"}', '1')
        "#,
        )
        .unwrap();
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
