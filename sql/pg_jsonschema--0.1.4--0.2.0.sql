-- src/lib.rs:33
-- pg_jsonschema::jsonschema_validation_errors
CREATE  FUNCTION "jsonschema_validation_errors"(
	"schema" json, /* pgrx::datum::json::Json */
	"instance" json /* pgrx::datum::json::Json */
) RETURNS TEXT[] /* alloc::vec::Vec<alloc::string::String> */
IMMUTABLE STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'jsonschema_validation_errors_wrapper';

-- src/lib.rs:16
-- pg_jsonschema::jsonschema_is_valid
CREATE  FUNCTION "jsonschema_is_valid"(
	"schema" json /* pgrx::datum::json::Json */
) RETURNS bool /* bool */
IMMUTABLE STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'jsonschema_is_valid_wrapper';
