use serde_json::Value;

use crate::error::AppError;

pub fn validate_json_schema(schema: &Value, value: &Value) -> Result<(), AppError> {
    if schema.as_object().is_some_and(|object| object.is_empty()) {
        return Ok(());
    }

    let compiled = jsonschema::JSONSchema::compile(schema)
        .map_err(|e| AppError::bad_request(format!("invalid profile JSON schema: {e}")))?;

    let result = compiled.validate(value);
    match result {
        Ok(()) => Ok(()),
        Err(errors) => {
            let messages = errors.map(|e| e.to_string()).collect::<Vec<_>>();
            Err(AppError::bad_request(format!(
                "attributes failed profile schema validation: {}",
                messages.join("; ")
            )))
        }
    }
}
