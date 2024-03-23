#![feature(lazy_cell)]

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
        "s3-object",
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

pub fn titlecase(mut s: String) -> Result<String> {
  Ok(s.remove(0).to_uppercase().to_string() + &s)
}
