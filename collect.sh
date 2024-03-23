#!/usr/bin/env bash

cargo build --release

gh repo list terraform-aws-modules --source --limit 100 --json name --jq '.[].name' | \
  grep terraform-aws- | \
  sed 's/terraform-aws-//g' | \
  xargs -t -I {} target/release/tmd collect-data --module {}
