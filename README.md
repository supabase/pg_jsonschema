# pg_jsonschema

<p>
<a href=""><img src="https://img.shields.io/badge/postgresql-12+-blue.svg" alt="PostgreSQL version" height="18"></a>
<a href="https://github.com/supabase/pg_jsonschema/blob/master/LICENSE"><img src="https://img.shields.io/pypi/l/markdown-subtemplate.svg" alt="License" height="18"></a>

</p>

---

**Source Code**: <a href="https://github.com/supabase/pg_jsonschema" target="_blank">https://github.com/supabase/pg_jsonschema</a>

---

## Summary

`pg_jsonschema` is a PostgreSQL extension adding support for [JSON schema](https://json-schema.org/) validation on `json` and `jsonb` data types.


## API
SQL functions:

```sql
-- Validates a json *instance* against a *schema*
json_matches_schema(schema json, instance json) returns bool
```
and 
```sql
-- Validates a jsonb *instance* against a *schema*
jsonb_matches_schema(schema json, instance jsonb) returns bool
```

## Usage
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


## Installation

Requires:
- [pgx](https://github.com/tcdi/pgx)


```shell
cargo pgx run
```

which drops into a psql prompt.
```psql
psql (13.6)
Type "help" for help.

pg_jsonschema=# create extension pg_jsonschema;
CREATE EXTENSION

pg_jsonschema=# select json_matches_schema('{"type": "object"}', '{}');
 json_matches_schema 
---------------------
 t
(1 row)
```

for more complete installation guidelines see the [pgx](https://github.com/tcdi/pgx) docs.
