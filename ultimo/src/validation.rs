//! Validation helpers for request data
//!
//! Integrates with the validator crate to provide automatic validation
//! with structured error responses.

use crate::error::{Result, UltimoError, ValidationError};
use validator::{Validate, ValidationErrors};

/// Validate a struct and convert errors to UltimoError
pub fn validate<T: Validate>(data: &T) -> Result<()> {
    data.validate().map_err(|errors| UltimoError::Validation {
        message: "Validation failed".to_string(),
        details: validation_errors_to_details(errors),
    })
}

/// Convert validator ValidationErrors to our ValidationError format
fn validation_errors_to_details(errors: ValidationErrors) -> Vec<ValidationError> {
    let mut details = Vec::new();

    for (field, field_errors) in errors.field_errors() {
        for error in field_errors {
            let message = error
                .message
                .as_ref()
                .map(|m| m.to_string())
                .unwrap_or_else(|| format!("Validation failed for field: {}", field));

            details.push(ValidationError {
                field: field.to_string(),
                message,
            });
        }
    }

    details
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use validator::Validate;

    #[derive(Debug, Deserialize, Validate)]
    struct TestData {
        #[validate(length(min = 3, max = 10))]
        name: String,
        #[validate(email)]
        email: String,
    }

    #[test]
    fn test_validation_success() {
        let data = TestData {
            name: "John".to_string(),
            email: "john@example.com".to_string(),
        };

        assert!(validate(&data).is_ok());
    }

    #[test]
    fn test_validation_failure() {
        let data = TestData {
            name: "AB".to_string(),       // Too short
            email: "invalid".to_string(), // Invalid email
        };

        let result = validate(&data);
        assert!(result.is_err());

        if let Err(UltimoError::Validation { details, .. }) = result {
            assert!(!details.is_empty());
        } else {
            panic!("Expected ValidationError");
        }
    }
}
