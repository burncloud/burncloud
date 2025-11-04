# Database Fields JSON Format

This document describes all database fields in JSON format for the BurnCloud Database Models system.

## ModelInfo JSON Schema

The complete JSON representation of a model record in the database:

```json
{
  "model_id": "string",
  "private": false,
  "pipeline_tag": "string|null",
  "library_name": "string|null",
  "model_type": "string|null",
  "downloads": 0,
  "likes": 0,
  "sha": "string|null",
  "last_modified": "string|null",
  "gated": false,
  "disabled": false,
  "tags": "[]",
  "config": "{}",
  "widget_data": "[]",
  "card_data": "{}",
  "transformers_info": "{}",
  "siblings": "[]",
  "spaces": "[]",
  "safetensors": "{}",
  "used_storage": 0,
  "filename": "string|null",
  "size": 0,
  "created_at": "string",
  "updated_at": "string"
}
```

## Field Descriptions

### Basic Information
- **model_id**: Unique identifier for the model (e.g., "openai-community/gpt2")
- **private**: Boolean indicating if the model is private
- **pipeline_tag**: Model pipeline type (e.g., "text-generation")
- **library_name**: Library name (e.g., "transformers")
- **model_type**: Model type (e.g., "gpt2")

### Statistics
- **downloads**: Number of downloads (integer)
- **likes**: Number of likes (integer)

### Version Information
- **sha**: Git SHA hash
- **last_modified**: Last modification timestamp
- **gated**: Boolean indicating if authorization is required
- **disabled**: Boolean indicating if the model is disabled

### JSON Storage Fields
These fields store complex data as JSON strings:

- **tags**: Tag list
  ```json
  ["tag1", "tag2", "tag3"]
  ```

- **config**: Model configuration
  ```json
  {
    "architectures": ["GPT2LMHeadModel"],
    "model_type": "gpt2",
    "vocab_size": 50257
  }
  ```

- **widget_data**: Example data for model testing
  ```json
  [
    {
      "text": "My name is Mariama, my favorite"
    }
  ]
  ```

- **card_data**: Model card metadata
  ```json
  {
    "language": ["en"],
    "license": "mit",
    "datasets": ["openwebtext"]
  }
  ```

- **transformers_info**: Transformers library information
  ```json
  {
    "auto_model": "AutoModelForCausalLM",
    "pipeline_tag": "text-generation",
    "processor": "AutoTokenizer"
  }
  ```

- **siblings**: Related files list
  ```json
  [
    {
      "rfilename": "config.json"
    },
    {
      "rfilename": "pytorch_model.bin"
    }
  ]
  ```

- **spaces**: Associated Hugging Face Spaces
  ```json
  [
    "space1",
    "space2"
  ]
  ```

- **safetensors**: SafeTensors format information
  ```json
  {
    "parameters": {},
    "total": 124439808
  }
  ```

### Storage Information
- **used_storage**: Used storage space in bytes
- **filename**: File name
- **size**: File size in bytes

### Timestamps
- **created_at**: Creation timestamp (ISO 8601 format)
- **updated_at**: Last update timestamp (ISO 8601 format)

## Example Complete Record

```json
{
  "model_id": "openai-community/gpt2",
  "private": false,
  "pipeline_tag": "text-generation",
  "library_name": "transformers",
  "model_type": "gpt2",
  "downloads": 1234567,
  "likes": 890,
  "sha": "11c5a3d5811f50298f278a704980280950aedb10",
  "last_modified": "2023-10-15T14:30:00Z",
  "gated": false,
  "disabled": false,
  "tags": "[\"pytorch\", \"gpt2\", \"text-generation\", \"en\", \"dataset:openwebtext\", \"arxiv:1909.08053\", \"transformers\", \"causal-lm\"]",
  "config": "{\"architectures\": [\"GPT2LMHeadModel\"], \"model_type\": \"gpt2\", \"vocab_size\": 50257}",
  "widget_data": "[{\"text\": \"My name is Mariama, my favorite\"}]",
  "card_data": "{\"language\": [\"en\"], \"license\": \"mit\", \"datasets\": [\"openwebtext\"]}",
  "transformers_info": "{\"auto_model\": \"AutoModelForCausalLM\", \"pipeline_tag\": \"text-generation\", \"processor\": \"AutoTokenizer\"}",
  "siblings": "[{\"rfilename\": \"config.json\"}, {\"rfilename\": \"pytorch_model.bin\"}]",
  "spaces": "[]",
  "safetensors": "{\"parameters\": {}, \"total\": 124439808}",
  "used_storage": 548118077,
  "filename": "pytorch_model.bin",
  "size": 548118077,
  "created_at": "2023-10-15T10:00:00Z",
  "updated_at": "2023-10-15T14:30:00Z"
}
```

## Database Schema JSON

The SQLite table structure in JSON format:

```json
{
  "table_name": "models",
  "columns": [
    {
      "name": "model_id",
      "type": "TEXT",
      "constraints": ["PRIMARY KEY"]
    },
    {
      "name": "private",
      "type": "INTEGER",
      "constraints": ["NOT NULL"]
    },
    {
      "name": "pipeline_tag",
      "type": "TEXT"
    },
    {
      "name": "library_name",
      "type": "TEXT"
    },
    {
      "name": "model_type",
      "type": "TEXT"
    },
    {
      "name": "downloads",
      "type": "INTEGER",
      "constraints": ["NOT NULL"]
    },
    {
      "name": "likes",
      "type": "INTEGER",
      "constraints": ["NOT NULL"]
    },
    {
      "name": "sha",
      "type": "TEXT"
    },
    {
      "name": "last_modified",
      "type": "DATETIME"
    },
    {
      "name": "gated",
      "type": "INTEGER",
      "constraints": ["NOT NULL"]
    },
    {
      "name": "disabled",
      "type": "INTEGER",
      "constraints": ["NOT NULL"]
    },
    {
      "name": "tags",
      "type": "TEXT",
      "constraints": ["NOT NULL"]
    },
    {
      "name": "config",
      "type": "TEXT",
      "constraints": ["NOT NULL"]
    },
    {
      "name": "widget_data",
      "type": "TEXT",
      "constraints": ["NOT NULL"]
    },
    {
      "name": "card_data",
      "type": "TEXT",
      "constraints": ["NOT NULL"]
    },
    {
      "name": "transformers_info",
      "type": "TEXT",
      "constraints": ["NOT NULL"]
    },
    {
      "name": "siblings",
      "type": "TEXT",
      "constraints": ["NOT NULL"]
    },
    {
      "name": "spaces",
      "type": "TEXT",
      "constraints": ["NOT NULL"]
    },
    {
      "name": "safetensors",
      "type": "TEXT",
      "constraints": ["NOT NULL"]
    },
    {
      "name": "used_storage",
      "type": "INTEGER",
      "constraints": ["NOT NULL"]
    },
    {
      "name": "filename",
      "type": "TEXT"
    },
    {
      "name": "size",
      "type": "INTEGER",
      "constraints": ["NOT NULL"]
    },
    {
      "name": "created_at",
      "type": "DATETIME",
      "constraints": ["NOT NULL"]
    },
    {
      "name": "updated_at",
      "type": "DATETIME",
      "constraints": ["NOT NULL"]
    }
  ],
  "indexes": [
    {
      "name": "idx_models_pipeline_tag",
      "columns": ["pipeline_tag"]
    },
    {
      "name": "idx_models_library_name",
      "columns": ["library_name"]
    },
    {
      "name": "idx_models_downloads",
      "columns": ["downloads DESC"]
    },
    {
      "name": "idx_models_likes",
      "columns": ["likes DESC"]
    },
    {
      "name": "idx_models_created_at",
      "columns": ["created_at"]
    },
    {
      "name": "idx_models_private",
      "columns": ["private"]
    }
  ]
}
```