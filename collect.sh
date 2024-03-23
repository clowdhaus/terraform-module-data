#!/usr/bin/env bash

cargo build --release

gh repo list terraform-aws-modules --source --json name --jq '.[].name' | sed 's/terraform-aws-//g' | xargs -t -I {} target/release/tmd collect-data --module {}
