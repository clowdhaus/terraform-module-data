#!/usr/bin/env bash

cargo build --release

# Collect data for all modules
gh repo list terraform-aws-modules --source --limit 100 --json name --jq '.[].name' | \
  grep terraform-aws- | \
  grep -v s3-object | \
  sed 's/terraform-aws-//g' | \
  xargs -I {} target/release/tmd collect-data --module {}

# Update graphs

target/release/tmd graph
