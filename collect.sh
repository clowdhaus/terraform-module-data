#!/usr/bin/env bash
set -uo pipefail

# Collect data for all modules
failed_modules=()
modules=$(gh repo list terraform-aws-modules --source --limit 100 --json name --jq '.[].name' | \
  grep terraform-aws- | \
  grep -v s3-object | \
  sed 's/terraform-aws-//g')

for module in $modules; do
  if ! target/release/tmd collect-data --module "$module"; then
    echo "WARNING: Failed to collect data for module: $module" >&2
    failed_modules+=("$module")
  fi
done

# Update graphs
target/release/tmd graph

# Report failures
if [ ${#failed_modules[@]} -gt 0 ]; then
  echo "WARNING: Failed to collect data for ${#failed_modules[@]} module(s):" >&2
  printf '  - %s\n' "${failed_modules[@]}" >&2
fi
