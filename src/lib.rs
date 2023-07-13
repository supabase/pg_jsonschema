use pgrx::*;

pg_module_magic!();

#[pg_extern(immutable, strict)]
fn json_matches_schema(schema: Json, instance: Json) -> bool {
    if jsonschema::is_valid(&schema.0, &instance.0) {
        true
    } else {
        // "jsonschema::is_valid(...)" already checks the validaity of JSON schema, and panics if it is invalid.
        // Hence, we just unwrap the schema here.
        let compiled = jsonschema::JSONSchema::compile(&schema.0).unwrap();
        let err = compiled.validate(&instance.0).unwrap_err();
        let _ = err
            .into_iter()
            .for_each(|e| notice!("Invalid instance {} at {}", e.instance, e.instance_path));

        false
    }
}

#[pg_extern(immutable, strict)]
fn jsonb_matches_schema(schema: Json, instance: JsonB) -> bool {
    if jsonschema::is_valid(&schema.0, &instance.0) {
        true
    } else {
        // "jsonschema::is_valid(...)" already checks the validaity of JSON schema, and panics if it is invalid.
        // Hence, we just unwrap the schema here.
        let compiled = jsonschema::JSONSchema::compile(&schema.0).unwrap();
        let err = compiled.validate(&instance.0).unwrap_err();
        let _ = err
            .into_iter()
            .for_each(|e| notice!("Invalid instance {} at {}", e.instance, e.instance_path));

        false
    }
}

#[pg_extern(immutable, strict)]
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
