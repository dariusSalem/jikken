use crate::test;
use crate::test::file::Validated::Good;
use crate::test::{definition, http, variable};
use log::error;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::Display;
use std::fmt::{self, Formatter};
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
    fn check(
        &self,
        val: &Self::Item,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>>; //Validated<Vec<()>, String>;
}

impl<T> Specification<T>
where
    T: PartialEq,
    T: Display,
    T: PartialOrd,
    T: fmt::Debug,
{
    fn check_val(
        &self,
        actual: &T,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Validated<(), String> {
        match &self.val {
            Some(t) => {
                if t == actual {
                    Good(())
                } else {
                    Validated::fail(formatter(
                        format!("{}", t).as_str(),
                        format!("{}", actual).as_str(),
                    ))
                    //Validated::fail(format!("Val check failed: {actual} not equal {t}"))
                }
            }
            None => Good(()),
        }
    }

    fn check_min(
        &self,
        actual: &T,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Validated<(), String> {
        match &self.min {
            Some(t) => {
                if t <= actual {
                    Good(())
                } else {
                    Validated::fail(formatter(
                        format!("minimum of {}", t).as_str(),
                        format!("{}", actual).as_str(),
                    ))
                    //Validated::fail(format!(
                    //    "Minimum check failed: {actual} not greater than or equal to {t}"
                    //))
                }
            }
            None => Good(()),
        }
    }

    fn check_max(
        &self,
        actual: &T,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Validated<(), String> {
        match &self.max {
            Some(t) => {
                if t >= actual {
                    Good(())
                } else {
                    Validated::fail(formatter(
                        format!("maximum of {}", t).as_str(),
                        format!("{}", actual).as_str(),
                    ))
                    //Validated::fail(format!(
                    //    "Maximum check failed: {actual} not less than or equal to {t}"
                    //))
                }
            }
            None => Good(()),
        }
    }

    fn check_one_of(
        &self,
        actual: &T,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Validated<(), String> {
        match &self.one_of {
            Some(t) => {
                if t.contains(actual) {
                    Good(())
                } else {
                    Validated::fail(formatter(
                        format!("one of {:?}", t).as_str(),
                        format!("{}", actual).as_str(),
                    ))
                    //Validated::fail(format!("One_Of check failed: {actual} not in {:?}", t))
                }
            }
            None => Good(()),
        }
    }

    fn check_none_of(
        &self,
        actual: &T,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Validated<(), String> {
        match &self.none_of {
            Some(t) => {
                if !t.contains(actual) {
                    Good(())
                } else {
                    Validated::fail(formatter(
                        format!("none of {:?}", t).as_str(),
                        format!("{}", actual).as_str(),
                    ))
                    //Validated::fail(format!("None_Of check failed: {actual} in {:?}", t))
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
    fn check(
        &self,
        val: &T,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        //Validated<Vec<()>, String> {
        vec![
            self.check_val(&val, formatter),
            self.check_min(&val, formatter),
            self.check_max(&val, formatter),
            self.check_none_of(&val, formatter),
            self.check_one_of(&val, formatter),
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
        specification: Option<Specification<f64>>,
    },
    Int {
        #[serde(flatten)]
        specification: Option<Specification<i64>>,
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

impl DatumSchema {
    fn check(
        &self,
        actual: &serde_json::Value,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        let mut ret = self.check_float(actual, formatter);
        ret.append(self.check_int(actual, formatter).as_mut());
        ret.append(self.check_string(actual, formatter).as_mut());
        ret.append(self.check_list(actual, formatter).as_mut());
        ret.append(self.check_object(actual, formatter).as_mut());
        ret
    }

    fn check_float(
        &self,
        actual: &serde_json::Value,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        match self {
            DatumSchema::Float { specification } => {
                if !actual.is_f64() {
                    return vec![Validated::fail(formatter("float type", "different type"))];
                }

                specification
                    .as_ref()
                    .map(|s| s.check(&actual.as_f64().unwrap(), formatter))
                    .unwrap_or(vec![Good(())])
            }
            _ => vec![Good(())],
        }
    }

    fn check_int(
        &self,
        actual: &serde_json::Value,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        match self {
            DatumSchema::Int { specification } => {
                if !actual.is_i64() {
                    return vec![Validated::fail(formatter("int type", "different type"))];
                }

                specification
                    .as_ref()
                    .map(|s| s.check(&actual.as_i64().unwrap(), formatter))
                    .unwrap_or(vec![Good(())])
            }
            _ => vec![Good(())],
        }
    }

    fn check_string(
        &self,
        actual: &serde_json::Value,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        match self {
            DatumSchema::String { specification } => {
                if !actual.is_string() {
                    return vec![Validated::fail(formatter("string type", "different type"))];
                }

                specification
                    .as_ref()
                    .map(|s| s.check(&actual.as_str().unwrap().to_string(), formatter))
                    .unwrap_or(vec![Good(())])
            }
            _ => vec![Good(())],
        }
    }

    fn check_list(
        &self,
        actual: &serde_json::Value,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        match self {
            DatumSchema::List { schema } => {
                if !actual.is_array() {
                    return vec![Validated::fail(formatter("array type", "different type"))];
                }

                actual
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| self.check(v, formatter))
                    .flatten()
                    .collect()
            }
            _ => vec![Good(())],
        }
    }

    fn check_object(
        &self,
        actual: &serde_json::Value,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        match self {
            DatumSchema::Object { schema } => {
                if !actual.is_object() {
                    return vec![Validated::fail(formatter("object type", "different type"))];
                }

                let vals = actual.as_object().unwrap();
                schema
                    .iter()
                    .map(|(k, datum)| {
                        vals.get(k)
                            .map(|v| datum.check(v, formatter))
                            .unwrap_or(vec![Validated::fail(formatter(
                                format!(r#"member "{k}""#).as_str(),
                                format!(r#"object with "{k}" missing"#).as_str(),
                            ))])
                    })
                    .flatten()
                    .collect()
            }
            _ => vec![Good(())],
        }
    }
}

impl Checker for DatumSchema {
    type Item = serde_json::Value;
    fn check(
        &self,
        val: &Self::Item,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        self.check(val, formatter)
    }
}

#[derive(Serialize, Debug, Clone, Deserialize)]
pub struct DocumentSchema {
    #[serde(rename = "_jk_schema")]
    pub schema: DatumSchema,
}

impl Checker for DocumentSchema {
    type Item = serde_json::Value;
    fn check(
        &self,
        val: &serde_json::Value,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        self.schema.check(val, formatter)
    }
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
    fn check(
        &self,
        val: &Self::Item,
        formatter: &impl Fn(&str, &str) -> String,
    ) -> Vec<Validated<(), String>> {
        //Validated<Vec<()>, String> {
        match &self {
            ValueOrSpecification::Value(t) => {
                if t == val {
                    vec![Good(())]
                } else {
                    vec![Validated::fail(formatter(
                        format!("{}", t).as_str(),
                        format!("{}", val).as_str(), //format!("{val} not equal to {t}"))]
                    ))]
                }
            }
            ValueOrSpecification::Schema(s) => s.check(val, formatter),
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
    use nonempty_collections::*;

    fn get_example_file_path(p: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("example_tests")
            .join(p)
    }
    #[test]
    fn body_validation() {
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

        let res = s.check(&serde_json::json!({}), &|e, a| {
            format!("Expected object with {e} but received {a}!")
        });
        let expected: Validated<Vec<()>, String> = Validated::Fail(nev![
            r#"Expected object with member "cars" but received object with "cars" missing!"#
                .to_string(),
            r#"Expected object with member "name" but received object with "name" missing!"#
                .to_string()
        ]);
        assert_eq!(expected, res.into_iter().collect());
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
        let resp = UnvalidatedResponse {
            status: Some(ValueOrSpecification::<u16>::Value(200)),
            ..Default::default()
        };

        println!("{}", serde_json::to_string(&resp).unwrap());

        let foo: ValueOrSpecification<u16> = serde_json::from_str("200").unwrap();
        let ba: ValueOrSpecification<u16> = serde_json::from_str(r#"{"oneOf": [200]}"#).unwrap();
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
