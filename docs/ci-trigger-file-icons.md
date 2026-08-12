# CI trigger

This file triggers Windows CI for PR #18 (file_icons).
Bot pushes via GITHUB_TOKEN don't trigger `on: pull_request` workflows,
so a PAT push is needed to run `Windows (stable + perf + fetchable logs)`.

See .github/workflows/ci.yml `on: pull_request: branches: [main]`.
