# pg_jsonschema

<p>
<a href=""><img src="https://img.shields.io/badge/postgresql-12+-blue.svg" alt="PostgreSQL version" height="18"></a>
<a href="https://github.com/supabase/pg_jsonschema/blob/master/LICENSE"><img src="https://img.shields.io/pypi/l/markdown-subtemplate.svg" alt="License" height="18"></a>

</p>

---

**Source Code**: <a href="https://github.com/supabase/pg_jsonschema" target="_blank">https://github.com/supabase/pg_jsonschema</a>

---

`pg_jsonschema` is a PostgreSQL extension adding support for performant [JSON schema](https://json-schema.org/) validation.

It exposes two functions:

```sql
json_matches_schema(schema json, instance json) returns bool
```
and 
```sql
jsonb_matches_schema(schema json, instance json) returns bool
```

Those functions can be used to constrain `json` and `jsonb` columns to conform to a schema.

For example:
```sql
create extension pg_jsonschema;

create table customer(
    id serial primary key,
    ...
    metadata json,

    check (
        json_matches_schema(
            '{
                "type": "object",
                "properties": {
                    "tags": {
                        "type": "array",
                        "items": {
                            "type": "string",
                            "maxLength": 16
                        }
                    }
                }
            }',
            metadata
        )
    )
);

-- Example: Valid Payload
insert into customer(metadata)
values ('{"tags": ["vip", "darkmode-ui"]}');
-- Result:
--   INSERT 0 1

-- Example: Invalid Payload
insert into customer(metadata)
values ('{"tags": [1, 3]}');
-- Result:
--   ERROR:  new row for relation "customer" violates check constraint "customer_metadata_check"
--   DETAIL:  Failing row contains (2, {"tags": [1, 3]}).
```


