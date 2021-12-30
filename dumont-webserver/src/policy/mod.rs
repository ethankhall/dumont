use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::convert::TryFrom;
use thiserror::Error;
use tracing_attributes::instrument;
use derivative::Derivative;

#[derive(Error, Debug)]
pub enum PolicyError {
    #[error(transparent)]
    RegexError {
        #[from]
        error: regex::Error,
    },
    #[error("Policy `{policy_name}` defined the label `{label_name}` twice.")]
    DuplicateLabel {
        policy_name: String,
        label_name: String,
    },
    #[error("Policy `{policy_name}` required that label `{label_name}` be set, however it was not and no default was specified.")]
    LabelNotDefined {
        policy_name: String,
        label_name: String,
    },
    #[error("Policy `{policy_name}` required that label `{label_name}` be one of a set values, however `{value}` was not in that set.")]
    LabelNotInSet {
        policy_name: String,
        label_name: String,
        value: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDefinitionContainer {
    #[serde(rename = "policy", default)]
    policies: Vec<PolicyDefinition>,
}

impl Default for PolicyDefinitionContainer {
    fn default() -> Self {
        Self {
            policies: Default::default(),
        }
    }
}

#[test]
fn parse_policy_container() {
    let input = r#"
[[policy]]
repository_pattern = "example/.*-service"
name = "service"
required_repo_labels = [
  {name = "owners"}
]
required_version_labels = [
  {name = "git_hash"},
  {name = "image_name"},
  {name = "release_state", one_of = ["built", "canary", "deployed", "replaced"]},
]

[[policy]]
repository_pattern = "example/.*"
name = "library"
required_repo_labels = [
  {name = "owners"}
]
required_version_labels = [
  {name = "git_hash"},
  {name = "release_state", one_of = ["pre-release", "released", "deprecated", "end-of-life"], default_value = "released"},
]
"#;

    let parsed: PolicyDefinitionContainer = toml::from_str(input).unwrap();
    assert_eq!(parsed.policies[0].name, "service");
    assert_eq!(parsed.policies[0].required_repo_labels[0], RequiredLabel::new("owners", Vec::new(), None));
    assert_eq!(parsed.policies[0].required_version_labels[0], RequiredLabel::new("git_hash", Vec::new(), None));
    assert_eq!(parsed.policies[0].required_version_labels[1], RequiredLabel::new("image_name", Vec::new(), None));
    assert_eq!(parsed.policies[0].required_version_labels[2], RequiredLabel::new("release_state", vec!["built", "canary", "deployed", "replaced"], None));

    assert_eq!(parsed.policies[1].name, "library");
    assert_eq!(parsed.policies[1].required_repo_labels[0], RequiredLabel::new("owners", Vec::new(), None));
    assert_eq!(parsed.policies[1].required_version_labels[0], RequiredLabel::new("git_hash", Vec::new(), None));
    assert_eq!(parsed.policies[1].required_version_labels[1], RequiredLabel::new("release_state", vec!["pre-release", "released", "deprecated", "end-of-life"], Some("released")));
    let realized_container = RealizedPolicyContainer::try_from(parsed.clone()).unwrap();

    assert_eq!(parsed.policies.len(), realized_container.policies.len())
}

#[test]
fn handle_unknown_fields() {
    let input = r#"
[[policy]]
repository_pattern = "example/.*-service"
name = "service"
required_repo_labels = [
  {name = "owners"}
]
required_version_labels = [
  {name = "release_state", default = "foo"},
]
"#;

    let parsed: Result<PolicyDefinitionContainer, toml::de::Error> = toml::from_str(input);
    assert!(parsed.is_err());
    assert_eq!(parsed.unwrap_err().to_string(), r#"unknown field `default`, expected one of `name`, `one_of`, `default_value` for key `policy.required_version_labels` at line 9 column 3"#);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDefinition {
    name: String,
    repository_pattern: String,
    required_repo_labels: Vec<RequiredLabel>,
    required_version_labels: Vec<RequiredLabel>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Derivative)]
#[derivative(PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RequiredLabel {
    name: String,
    #[serde(default)]
    one_of: Vec<String>,
    #[serde(default)]
    default_value: Option<String>,
}

impl RequiredLabel {
    #[cfg(test)]
    pub fn new(name: &str, one_of: Vec<&str>, default_value: Option<&str>) -> Self {
        let one_of: Vec<String> = one_of.into_iter().map(str::to_string).collect();
        RequiredLabel {
            name: name.to_owned(),
            one_of,
            default_value: default_value.map(str::to_string),
        }
    }
    pub fn process_label(
        &self,
        policy_name: &str,
        all_labels: &mut BTreeMap<String, String>,
    ) -> Result<(), PolicyError> {
        let label_name = self.name.clone();
        let value = match (all_labels.get(&label_name), &self.default_value) {
            (None, None) => {
                return Err(PolicyError::LabelNotDefined {
                    policy_name: policy_name.to_owned(),
                    label_name: label_name.clone(),
                });
            }
            (None, Some(default)) => {
                all_labels.insert(label_name.clone(), default.to_string());
                default.to_string()
            }
            (Some(value), _) => value.clone(),
        };

        if !self.one_of.is_empty() && !self.one_of.contains(&value) {
            return Err(PolicyError::LabelNotInSet {
                policy_name: policy_name.to_owned(),
                label_name: label_name.clone(),
                value,
            });
        }

        Ok(())
    }
}

#[test]
fn test_validate_label() {
    let ut = RequiredLabel {
        name: "test".to_owned(),
        one_of: vec!["true".to_owned(), "false".to_owned()],
        default_value: None,
    };

    let mut value = BTreeMap::from_iter(vec![("test".to_owned(), "true".to_owned())]);
    assert!(ut.process_label("foo", &mut value).is_ok());

    let mut value = BTreeMap::from_iter(vec![("test".to_owned(), "bar".to_owned())]);
    assert_eq!(ut.process_label("foo", &mut value).unwrap_err().to_string(), "Policy `foo` required that label `test` be one of a set values, however `bar` was not in that set.");

    let mut value = BTreeMap::default();
    assert_eq!(ut.process_label("foo", &mut value).unwrap_err().to_string(), "Policy `foo` required that label `test` be set, however it was not and no default was specified.");
}

#[test]
fn test_will_set_default_label() {
    let ut = RequiredLabel::new("test", vec!["true", "false"], Some("bar"));

    let mut value = BTreeMap::default();
    assert_eq!(ut.process_label("foo", &mut value).unwrap_err().to_string(), "Policy `foo` required that label `test` be one of a set values, however `bar` was not in that set.");

    let ut = RequiredLabel {
        name: "test".to_owned(),
        one_of: vec!["true".to_owned(), "false".to_owned()],
        default_value: Some("true".to_owned()),
    };

    let mut value = BTreeMap::default();
    assert!(ut.process_label("foo", &mut value).is_ok());
    assert_eq!(value.get("test"), Some(&"true".to_owned()));
}

#[derive(Debug, Serialize)]
pub struct RealizedPolicyContainer {
    pub policies: Vec<RealizedPolicy>,
}

impl Default for RealizedPolicyContainer {
    fn default() -> Self {
        Self {
            policies: Default::default(),
        }
    }
}

impl RealizedPolicyContainer {
    pub fn execute_repo_policies(
        &self,
        org: &str,
        repo: &str,
        labels: &mut BTreeMap<String, String>,
    ) -> Result<(), PolicyError> {
        let repo_path = format!("{}/{}", org, repo);
        for policy in &self.policies {
            if policy.policy_matches_repo(&repo_path) {
                policy.process_repo_labels(labels)?;
                break;
            }
        }

        Ok(())
    }

    pub fn execute_version_policies(
        &self,
        org: &str,
        repo: &str,
        labels: &mut BTreeMap<String, String>,
    ) -> Result<(), PolicyError> {
        let repo_path = format!("{}/{}", org, repo);
        for policy in &self.policies {
            if policy.policy_matches_repo(&repo_path) {
                policy.process_version_labels(labels)?;
                break;
            }
        }

        Ok(())
    }
}

impl TryFrom<PolicyDefinitionContainer> for RealizedPolicyContainer {
    type Error = PolicyError;

    fn try_from(container: PolicyDefinitionContainer) -> Result<Self, Self::Error> {
        let mut policies = Vec::new();
        for policy in container.policies {
            policies.push(RealizedPolicy::try_from(policy)?);
        }

        Ok(Self { policies })
    }
}

#[derive(Debug, Serialize)]
pub struct RealizedPolicy {
    name: String,
    repository_regex: String,
    #[serde(skip)]
    repository_pattern: Regex,
    required_repo_labels: Vec<RequiredLabel>,
    required_version_labels: Vec<RequiredLabel>,
}

impl RealizedPolicy {
    fn new(
        name: &str,
        pattern: &str,
        required_repo_labels: Vec<RequiredLabel>,
        required_version_labels: Vec<RequiredLabel>,
    ) -> Result<Self, PolicyError> {
        RealizedPolicy::validate_only_one_label(name, &required_repo_labels)?;
        RealizedPolicy::validate_only_one_label(name, &required_version_labels)?;

        let formatted_pattern = format!("^{}$", pattern);
        let repository_pattern = Regex::new(&formatted_pattern)?;

        Ok(RealizedPolicy {
            name: name.to_owned(),
            repository_pattern,
            repository_regex: formatted_pattern,
            required_repo_labels: required_repo_labels.clone(),
            required_version_labels: required_version_labels.clone(),
        })
    }

    fn validate_only_one_label(
        policy_name: &str,
        labels: &Vec<RequiredLabel>,
    ) -> Result<(), PolicyError> {
        let mut label_names = BTreeSet::default();
        for label in labels {
            if label_names.contains(&label.name) {
                return Err(PolicyError::DuplicateLabel {
                    policy_name: policy_name.to_string(),
                    label_name: label.name.clone(),
                });
            }
            label_names.insert(label.name.to_string());
        }

        Ok(())
    }

    #[cfg(test)]
    pub fn test_new(pattern: &str, labels: Vec<RequiredLabel>) -> Self {
        RealizedPolicy::test_new_different_labels(pattern, labels.clone(), labels.clone())
    }

    #[cfg(test)]
    pub fn test_new_different_labels(
        pattern: &str,
        required_repo_labels: Vec<RequiredLabel>,
        required_version_labels: Vec<RequiredLabel>,
    ) -> Self {
        RealizedPolicy::new(
            "test",
            pattern,
            required_repo_labels,
            required_version_labels,
        )
        .unwrap()
    }

    #[instrument(skip(self, repo_path))]
    pub fn policy_matches_repo(&self, repo_path: &str) -> bool {
        self.repository_pattern.is_match(repo_path)
    }

    #[instrument(skip(self, declared_labels))]
    pub fn process_repo_labels(
        &self,
        declared_labels: &mut BTreeMap<String, String>,
    ) -> Result<(), PolicyError> {
        for label in &self.required_repo_labels {
            label.process_label(&self.name, declared_labels)?;
        }

        Ok(())
    }

    #[instrument(skip(self, declared_labels))]
    pub fn process_version_labels(
        &self,
        declared_labels: &mut BTreeMap<String, String>,
    ) -> Result<(), PolicyError> {
        for label in &self.required_version_labels {
            label.process_label(&self.name, declared_labels)?;
        }

        Ok(())
    }
}

#[test]
fn realized_pattern_will_match() {
    let policy = RealizedPolicy::test_new("example/.*", Vec::new());
    assert!(policy.policy_matches_repo("example/foo"));
    assert!(policy.policy_matches_repo("example/bar"));
    assert!(!policy.policy_matches_repo("another-example/foo"));
}

impl TryFrom<PolicyDefinition> for RealizedPolicy {
    type Error = PolicyError;

    fn try_from(policy: PolicyDefinition) -> Result<Self, Self::Error> {
        RealizedPolicy::new(
            &policy.name,
            &policy.repository_pattern,
            policy.required_repo_labels.clone(),
            policy.required_version_labels.clone(),
        )
    }
}
