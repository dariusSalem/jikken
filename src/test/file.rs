use crate::test;
use crate::test::{definition, http, variable};
use log::error;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::hash::{Hash, Hasher};

#[derive(Serialize, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
enum Specification<T: PartialOrd> {
    //Idea here was regex matching;
    //need to think more about how to discriminate type
    //Pattern { pattern: String },
    Value { val: T },
    Range { min: T, max: T },
    OneOf { one_of: Vec<T> },
    NoneOf { none_of: Vec<T> },
    AllOf { all_of: Vec<T> },
}

#[derive(Serialize, Debug, Deserialize)]
#[serde(untagged)]
enum ValidSpecifications {
    StringSpecification(Specification<String>),
    IntSpecification(Specification<i32>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SpecificationType {
    Int,
    Float,
    String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnvalidatedSpecification {
    #[serde(rename = "type")]
    pub type_id: SpecificationType,
    pub spec: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnvalidatedRequest {
    pub method: Option<http::Verb>,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Vec<http::Parameter>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<http::Header>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
}

impl Default for UnvalidatedRequest {
    fn default() -> Self {
        Self {
            method: None,
            url: "".to_string(),
            params: None,
            headers: None,
            body: None,
        }
    }
}

impl Hash for UnvalidatedRequest {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.method.hash(state);
        self.url.hash(state);
        self.params.hash(state);
        self.headers.hash(state);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnvalidatedCompareRequest {
    pub method: Option<http::Verb>,
    pub url: String,
    pub params: Option<Vec<http::Parameter>>,
    pub add_params: Option<Vec<http::Parameter>>,
    pub ignore_params: Option<Vec<String>>,
    pub headers: Option<Vec<http::Header>>,
    pub add_headers: Option<Vec<http::Header>>,
    pub ignore_headers: Option<Vec<String>>,
    pub body: Option<serde_json::Value>,
}

impl Hash for UnvalidatedCompareRequest {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.method.hash(state);
        self.url.hash(state);
        self.params.hash(state);
        self.add_params.hash(state);
        self.ignore_params.hash(state);
        self.headers.hash(state);
        self.add_headers.hash(state);
        self.ignore_headers.hash(state);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnvalidatedResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<http::Header>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extract: Option<Vec<definition::ResponseExtraction>>,
}

impl Default for UnvalidatedResponse {
    fn default() -> Self {
        Self {
            status: Some(200),
            headers: None,
            body: None,
            ignore: None,
            extract: None,
        }
    }
}

impl Hash for UnvalidatedResponse {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.status.hash(state);
        self.headers.hash(state);
        self.ignore.hash(state);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "camelCase")]
pub struct UnvalidatedVariable {
    pub name: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub data_type: Option<variable::Type>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_yaml::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modifier: Option<variable::Modifier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct UnvalidatedStage {
    pub request: UnvalidatedRequest,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compare: Option<UnvalidatedCompareRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<UnvalidatedResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<Vec<UnvalidatedVariable>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct UnvalidatedRequestResponse {
    pub request: UnvalidatedRequest,
    pub response: Option<UnvalidatedResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct UnvalidatedCleanup {
    pub onsuccess: Option<UnvalidatedRequest>,
    pub onfailure: Option<UnvalidatedRequest>,
    pub always: Option<UnvalidatedRequest>,
}

pub fn load(filename: &str) -> Result<test::File, Box<dyn Error + Send + Sync>> {
    let file_data = fs::read_to_string(filename)?;
    let result: Result<test::File, serde_yaml::Error> = serde_yaml::from_str(&file_data);
    match result {
        Ok(mut file) => {
            file.filename = String::from(filename);
            Ok(file)
        }
        Err(e) => {
            error!("unable to parse file ({}) data: {}", filename, e);
            Err(Box::from(e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_example_file_path(p: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("example_tests")
            .join(p)
    }

    //We use untagged serialization for a smoother user experience.
    //Maintain these tests to make sure serde's in-order, best-effort attemps
    //behave as expected
    #[test]
    fn verify_int_oneof_spec_serde_properly() {
        let foo = ValidSpecifications::IntSpecification(Specification::OneOf {
            one_of: vec![200, 201],
        });
        let output = format!("{}", serde_json::to_string(&foo).unwrap());
        assert_eq!(r#"{"one_of":[200,201]}"#, output.as_str());
        let bar: ValidSpecifications = serde_json::from_str(output.as_str()).unwrap();
        let expected = match bar {
            ValidSpecifications::IntSpecification(s) => match s {
                Specification::OneOf { .. } => true,
                _ => false,
            },
            _ => false,
        };

        assert!(expected);
    }

    #[test]
    fn verify_int_noneof_spec_serde_properly() {
        let foo = ValidSpecifications::IntSpecification(Specification::NoneOf {
            none_of: vec![200, 201],
        });
        let output = format!("{}", serde_json::to_string(&foo).unwrap());
        assert_eq!(r#"{"none_of":[200,201]}"#, output.as_str());
        let bar: ValidSpecifications = serde_json::from_str(output.as_str()).unwrap();
        let expected = match bar {
            ValidSpecifications::IntSpecification(s) => match s {
                Specification::NoneOf { .. } => true,
                _ => false,
            },
            _ => false,
        };

        assert!(expected);
    }

    #[test]
    fn verify_int_allof_spec_serde_properly() {
        let foo = ValidSpecifications::IntSpecification(Specification::AllOf {
            all_of: vec![200, 201],
        });
        let output = format!("{}", serde_json::to_string(&foo).unwrap());
        assert_eq!(r#"{"all_of":[200,201]}"#, output.as_str());
        let bar: ValidSpecifications = serde_json::from_str(output.as_str()).unwrap();
        let expected = match bar {
            ValidSpecifications::IntSpecification(s) => match s {
                Specification::AllOf { .. } => true,
                _ => false,
            },
            _ => false,
        };

        assert!(expected);
    }

    #[test]
    fn verify_string_oneof_spec_serde_properly() {
        let foo = ValidSpecifications::StringSpecification(Specification::OneOf {
            one_of: vec!["200".to_string(), "201".to_string()],
        });
        let output = format!("{}", serde_json::to_string(&foo).unwrap());
        assert_eq!(r#"{"one_of":["200","201"]}"#, output.as_str());
        let bar: ValidSpecifications = serde_json::from_str(output.as_str()).unwrap();
        let expected = match bar {
            ValidSpecifications::StringSpecification(s) => match s {
                Specification::OneOf { .. } => true,
                _ => false,
            },
            _ => false,
        };

        assert!(expected);
    }

    #[test]
    fn verify_string_noneof_spec_serde_properly() {
        let foo = ValidSpecifications::StringSpecification(Specification::NoneOf {
            none_of: vec!["200".to_string(), "201".to_string()],
        });
        let output = format!("{}", serde_json::to_string(&foo).unwrap());
        assert_eq!(r#"{"none_of":["200","201"]}"#, output.as_str());
        let bar: ValidSpecifications = serde_json::from_str(output.as_str()).unwrap();
        let expected = match bar {
            ValidSpecifications::StringSpecification(s) => match s {
                Specification::NoneOf { .. } => true,
                _ => false,
            },
            _ => false,
        };

        assert!(expected);
    }

    #[test]
    fn verify_string_allof_spec_serde_properly() {
        let foo = ValidSpecifications::StringSpecification(Specification::AllOf {
            all_of: vec!["200".to_string(), "201".to_string()],
        });
        let output = format!("{}", serde_json::to_string(&foo).unwrap());
        assert_eq!(r#"{"all_of":["200","201"]}"#, output.as_str());
        let bar: ValidSpecifications = serde_json::from_str(output.as_str()).unwrap();
        let expected = match bar {
            ValidSpecifications::StringSpecification(s) => match s {
                Specification::AllOf { .. } => true,
                _ => false,
            },
            _ => false,
        };

        assert!(expected);
    }

    #[test]
    fn verify_int_value_spec_serde_properly() {
        let foo = ValidSpecifications::IntSpecification(Specification::Value { val: 12 });
        let output = format!("{}", serde_json::to_string(&foo).unwrap());
        assert_eq!(r#"{"val":12}"#, output.as_str());
        let bar: ValidSpecifications = serde_json::from_str(output.as_str()).unwrap();
        let expected = match bar {
            ValidSpecifications::IntSpecification(s) => match s {
                Specification::Value { .. } => true,
                _ => false,
            },
            _ => false,
        };

        assert!(expected);
    }

    #[test]
    fn verify_int_range_spec_serde_properly() {
        let foo = ValidSpecifications::IntSpecification(Specification::Range { min: 12, max: 100 });
        let output = format!("{}", serde_json::to_string(&foo).unwrap());
        assert_eq!(r#"{"min":12,"max":100}"#, output.as_str());
        let bar: ValidSpecifications = serde_json::from_str(output.as_str()).unwrap();
        let expected = match bar {
            ValidSpecifications::IntSpecification(s) => match s {
                Specification::Range { .. } => true,
                _ => false,
            },
            _ => false,
        };

        assert!(expected);
    }

    #[test]
    fn verify_string_range_spec_serde_properly() {
        let foo = ValidSpecifications::StringSpecification(Specification::Range {
            min: "aa".to_string(),
            max: "bb".to_string(),
        });
        let output = format!("{}", serde_json::to_string(&foo).unwrap());
        assert_eq!(r#"{"min":"aa","max":"bb"}"#, output.as_str());
        let bar: ValidSpecifications = serde_json::from_str(output.as_str()).unwrap();
        let expected = match bar {
            ValidSpecifications::StringSpecification(s) => match s {
                Specification::Range { .. } => true,
                _ => false,
            },
            _ => false,
        };

        assert!(expected);
    }

    #[test]
    fn verify_string_value_spec_serde_properly() {
        let foo = ValidSpecifications::StringSpecification(Specification::Value {
            val: "foo".to_string(),
        });
        let output = format!("{}", serde_json::to_string(&foo).unwrap());
        assert_eq!(r#"{"val":"foo"}"#, output.as_str());
        let bar: ValidSpecifications = serde_json::from_str(output.as_str()).unwrap();
        let expected = match bar {
            ValidSpecifications::StringSpecification(s) => match s {
                Specification::Value { .. } => true,
                _ => false,
            },
            _ => false,
        };

        assert!(expected);
    }
}
