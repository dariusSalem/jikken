use crate::test;
use crate::test::file::Validated::Good;
use crate::test::{definition, http, variable};
use log::error;
//use nonempty_collections::nev;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::fmt::Display;
use std::fs;
use std::hash::{Hash, Hasher};
use validated::Validated;

//add pattern
#[derive(Serialize, Debug, Clone, Deserialize, PartialEq, PartialOrd, Hash)]
#[serde(rename_all = "camelCase")]
pub struct Specification<T> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub val: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub one_of: Option<Vec<T>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub none_of: Option<Vec<T>>,
    //#[serde(skip_serializing_if = "Option::is_none")]
    //pub all_of: Option<Vec<T>>,
}

pub trait Checker {
    type Item;
    fn check(&self, val: &Self::Item) -> Vec<Validated<(), String>>; //Validated<Vec<()>, String>;
}

impl<T> Specification<T>
where
    T: PartialEq,
    T: Display,
    T: PartialOrd,
    T: fmt::Debug,
{
    fn check_val(&self, actual: &T) -> Validated<(), String> {
        match &self.val {
            Some(t) => {
                if t == actual {
                    Good(())
                } else {
                    Validated::fail(format!("Val check failed: {actual} not equal {t}"))
                }
            }
            None => Good(()),
        }
    }

    fn check_min(&self, actual: &T) -> Validated<(), String> {
        match &self.min {
            Some(t) => {
                if t <= actual {
                    Good(())
                } else {
                    Validated::fail(format!(
                        "Minimum check failed: {actual} not greater than or equal to {t}"
                    ))
                }
            }
            None => Good(()),
        }
    }

    fn check_max(&self, actual: &T) -> Validated<(), String> {
        match &self.max {
            Some(t) => {
                if t >= actual {
                    Good(())
                } else {
                    Validated::fail(format!(
                        "Maximum check failed: {actual} not less than or equal to {t}"
                    ))
                }
            }
            None => Good(()),
        }
    }

    fn check_one_of(&self, actual: &T) -> Validated<(), String> {
        match &self.one_of {
            Some(t) => {
                if t.contains(actual) {
                    Good(())
                } else {
                    Validated::fail(format!("One_Of check failed: {actual} not in {:?}", t))
                }
            }
            None => Good(()),
        }
    }

    fn check_none_of(&self, actual: &T) -> Validated<(), String> {
        match &self.none_of {
            Some(t) => {
                if !t.contains(actual) {
                    Good(())
                } else {
                    Validated::fail(format!("None_Of check failed: {actual} in {:?}", t))
                }
            }
            None => Good(()),
        }
    }
}

impl<T> Checker for Specification<T>
where
    T: PartialEq,
    T: Display,
    T: PartialOrd,
    T: fmt::Debug,
{
    type Item = T;
    fn check(&self, val: &T) -> Vec<Validated<(), String>> {
        //Validated<Vec<()>, String> {
        vec![
            self.check_val(&val),
            self.check_min(&val),
            self.check_max(&val),
            self.check_none_of(&val),
            self.check_one_of(&val),
        ]
        //.into_iter()
        //.collect()
    }
}

#[derive(Serialize, Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type")]
pub enum DatumSchema {
    Float {
        #[serde(flatten)]
        specification: Option<Specification<f32>>,
    },
    Int {
        #[serde(flatten)]
        specification: Option<Specification<i32>>,
    },
    String {
        #[serde(flatten)]
        specification: Option<Specification<String>>,
    },
    List {
        schema: Box<DatumSchema>,
    },
    Object {
        schema: BTreeMap<String, DatumSchema>,
    },
}

#[derive(Serialize, Debug, Clone, Deserialize)]
pub struct DocumentSchema {
    #[serde(rename = "_jk_schema")]
    pub schema: DatumSchema,
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

#[derive(Debug, Serialize, Clone, PartialEq, Deserialize, Hash)]
#[serde(untagged)]
pub enum ValueOrSpecification<T> {
    Value(T),
    Schema(Specification<T>),
}

impl<T> Checker for ValueOrSpecification<T>
where
    T: PartialEq,
    T: Display,
    T: PartialOrd,
    T: fmt::Debug,
{
    type Item = T;
    fn check(&self, val: &Self::Item) -> Vec<Validated<(), String>> {
        //Validated<Vec<()>, String> {
        match &self {
            ValueOrSpecification::Value(t) => {
                if t == val {
                    vec![Good(())]
                } else {
                    vec![Validated::fail(format!("{val} not equal to {t}"))]
                }
            }
            ValueOrSpecification::Schema(s) => s.check(val),
        }
    }
}

#[derive(Debug, Serialize, Clone, Deserialize)]
#[serde(untagged)]
pub enum BodyOrSchema {
    Value(serde_json::Value),
    Schema(DocumentSchema),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnvalidatedResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ValueOrSpecification<u16>>,
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
            status: Some(ValueOrSpecification::Value(200)),
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

    #[test]
    fn more() {
        let val = ValueOrSpecification::Value(202);
        println!("{}", serde_json::to_string(&val).unwrap());

        let val2 = ValueOrSpecification::<i32>::Schema(Specification {
            none_of: Some(vec![400, 500]),
            // all_of: None,
            max: None,
            min: None,
            one_of: None,
            val: None,
        });
        println!("{}", serde_json::to_string(&val2).unwrap());
    }
    /*
       The following tests may appear as though they are just
       testing serde functionality. However, they're a compile-time
       guarantee that we are not changing our data format in a breaking
       manner
    */
    #[test]
    fn build() {
        let mut schema = BTreeMap::<String, DatumSchema>::new();
        schema.insert(
            "name".to_string(),
            DatumSchema::String {
                specification: Some(Specification {
                    val: None,
                    min: None,
                    max: None,
                    one_of: Some(vec!["foo".to_string(), "bar".to_string()]),
                    none_of: None,
                    //all_of: None,
                }),
            },
        );
        schema.insert(
            "cars".to_string(),
            DatumSchema::List {
                schema: Box::from(DatumSchema::String {
                    specification: None,
                }),
            },
        );
        let s = DocumentSchema {
            schema: DatumSchema::Object { schema },
        };

        println!("{}", serde_json::to_string(&s).unwrap());
        let output = format!("{}", serde_json::to_string(&s).unwrap());
        let f: DocumentSchema = serde_json::from_str(&output).unwrap();
        println!("{}", serde_json::to_string(&f).unwrap())
    }
}
