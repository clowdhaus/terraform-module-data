pub mod cli;
pub(crate) mod github;
pub mod graph;
pub(crate) mod registry;

use std::{
  collections::{HashMap, HashSet},
  sync::LazyLock,
};

use anyhow::Result;

const DATA: &str = "data";
const COMPUTE: &str = "compute";
const SERVERLESS: &str = "serverless";
const NETWORKING: &str = "networking";
const OTHER: &str = "other";

static CATEGORIES: LazyLock<HashMap<&str, HashSet<&str>>> = LazyLock::new(|| {
  HashMap::from([
    (
      DATA,
      HashSet::from([
        "batch",
        "dms",
        "dynamodb-table",
        "efs",
        "elasticache",
        "emr",
        "fsx",
        "memory-db",
        "msk-kafka-cluster",
        "opensearch",
        "rds",
        "rds-aurora",
        "redshift",
        "s3-bucket",
      ]),
    ),
    (
      COMPUTE,
      HashSet::from([
        "app-runner",
        "autoscaling",
        "ec2-instance",
        "ecr",
        "ecs",
        "eks",
        "eks-pod-identity",
        "lambda",
      ]),
    ),
    (
      SERVERLESS,
      HashSet::from([
        "apigateway-v2",
        "app-runner",
        "appconfig",
        "appsync",
        "cloudfront",
        "cloudwatch",
        "eventbridge",
        "lambda",
        "memory-db",
        "rds-proxy",
        "sns",
        "sqs",
        "step-functions",
      ]),
    ),
    (
      NETWORKING,
      HashSet::from([
        "alb",
        "customer-gateway",
        "elb",
        "global-accelerator",
        "network-firewall",
        "rds-proxy",
        "route53",
        "security-group",
        "transit-gateway",
        "vpc",
        "vpn-gateway",
      ]),
    ),
    (
      OTHER,
      HashSet::from([
        "acm",
        "atlantis",
        "datadog-forwarders",
        "ebs-optimized",
        "iam",
        "key-pair",
        "kms",
        "managed-service-grafana",
        "managed-service-prometheus",
        "notify-slack",
        "pricing",
        "secrets-manager",
        "solutions",
        "ssm-parameter",
      ]),
    ),
  ])
});

pub fn titlecase(s: String) -> Result<String> {
  let mut chars = s.chars();
  match chars.next() {
    None => Ok(String::new()),
    Some(c) => Ok(c.to_uppercase().to_string() + chars.as_str()),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_titlecase_normal() {
    assert_eq!(titlecase("hello".to_string()).unwrap(), "Hello");
  }

  #[test]
  fn test_titlecase_single_char() {
    assert_eq!(titlecase("a".to_string()).unwrap(), "A");
  }

  #[test]
  fn test_titlecase_empty() {
    assert_eq!(titlecase("".to_string()).unwrap(), "");
  }

  #[test]
  fn test_titlecase_already_uppercase() {
    assert_eq!(titlecase("Hello".to_string()).unwrap(), "Hello");
  }

  #[test]
  fn test_titlecase_all_uppercase() {
    assert_eq!(titlecase("HELLO".to_string()).unwrap(), "HELLO");
  }

  #[test]
  fn test_categories_exist() {
    assert!(CATEGORIES.contains_key(DATA));
    assert!(CATEGORIES.contains_key(COMPUTE));
    assert!(CATEGORIES.contains_key(SERVERLESS));
    assert!(CATEGORIES.contains_key(NETWORKING));
    assert!(CATEGORIES.contains_key(OTHER));
  }

  #[test]
  fn test_categories_not_empty() {
    for (name, modules) in CATEGORIES.iter() {
      assert!(!modules.is_empty(), "Category '{name}' should not be empty");
    }
  }

  #[test]
  fn test_s3_object_not_in_categories() {
    for (_, modules) in CATEGORIES.iter() {
      assert!(
        !modules.contains("s3-object"),
        "s3-object should not be in any category"
      );
    }
  }
}
