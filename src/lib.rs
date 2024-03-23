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

static CATEGORIES: LazyLock<HashMap<&str, HashSet<&str>>> = LazyLock::new(|| {
  HashMap::from([
    (
      "data",
      HashSet::from([
        "batch",
        "efs",
        "elasticache",
        "fsx",
        "opensearch",
        "msk-kafka-cluster",
        "redshift",
        "rds",
        "rds-aurora",
        "s3-bucket",
        "emr",
        "dynamodb-table",
        "dms",
        "memory-db",
        "s3-object",
      ]),
    ),
    (
      "compute",
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
      "serverless",
      HashSet::from([
        "appconfig",
        "app-runner",
        "lambda",
        "eventbridge",
        "appsync",
        "cloudfront",
        "apigateway-v2",
        "step-functions",
        "sns",
        "sqs",
        "cloudwatch",
        "memory-db",
        "rds-proxy",
      ]),
    ),
    (
      "network",
      HashSet::from([
        "vpc",
        "security-group",
        "alb",
        "route53",
        "network-firewall",
        "global-accelerator",
        "customer-gateway",
        "elb",
        "transit-gateway",
        "vpn-gateway",
        "rds-proxy",
      ]),
    ),
    (
      "other",
      HashSet::from([
        "iam",
        "acm",
        "notify-slack",
        "kms",
        "pricing",
        "datadog-forwarders",
        "atlantis",
        "ssm-parameter",
        "managed-service-prometheus",
        "key-pair",
        "managed-service-grafana",
        "secrets-manager",
        "solutions",
        "ebs-optimized",
      ]),
    ),
  ])
});

pub fn titlecase(mut s: String) -> Result<String> {
  Ok(s.remove(0).to_uppercase().to_string() + &s)
}
