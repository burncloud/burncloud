# model_capabilities schema reference

The runtime table represented by this crate is `model_capabilities`.

```json
{
  "table_name": "model_capabilities",
  "columns": [
    {"name": "id", "type": "INTEGER/SERIAL", "constraints": ["PRIMARY KEY"]},
    {"name": "model", "type": "TEXT/VARCHAR(255)", "constraints": ["NOT NULL", "UNIQUE"]},
    {"name": "context_window", "type": "INTEGER/BIGINT", "nullable": true},
    {"name": "max_output_tokens", "type": "INTEGER/BIGINT", "nullable": true},
    {"name": "supports_vision", "type": "BOOLEAN", "default": false},
    {"name": "supports_function_calling", "type": "BOOLEAN", "default": false},
    {"name": "input_price", "type": "REAL/DOUBLE PRECISION", "nullable": true},
    {"name": "output_price", "type": "REAL/DOUBLE PRECISION", "nullable": true},
    {"name": "synced_at", "type": "INTEGER/BIGINT", "nullable": true}
  ]
}
```

`input_price` and `output_price` are legacy USD projections in this mixed historical table.
Commerce remains canonical pricing truth.

The former HuggingFace-shaped `models` JSON reference was documentation for a table that is not
present in runtime migrations and has therefore been retired by #621.
