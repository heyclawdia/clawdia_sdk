#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

if [[ ! -f .gitignore ]]; then
  echo "public-release-audit: missing .gitignore" >&2
  exit 1
fi

required_gitignore_patterns=(
  ".DS_Store"
  "target/"
  ".env"
  ".env.*"
  "!.env.example"
  ".netrc"
  ".npmrc"
  ".cargo/credentials"
  ".cargo/credentials.toml"
  "*.pem"
  "*.key"
  "*.p8"
  "*.p12"
  "id_rsa"
  "id_ed25519"
  "*.log"
  "tmp/"
  "coverage/"
  "dist/"
)

missing_patterns=()
for pattern in "${required_gitignore_patterns[@]}"; do
  if ! grep -Fxq "$pattern" .gitignore; then
    missing_patterns+=("$pattern")
  fi
done

if (( ${#missing_patterns[@]} > 0 )); then
  echo "public-release-audit: .gitignore is missing required public-repo patterns:" >&2
  printf '  - %s\n' "${missing_patterns[@]}" >&2
  exit 1
fi

sensitive_path_pattern='(^|/)(\.DS_Store|\.env(\..*)?|\.netrc|\.npmrc|id_rsa|id_ed25519|.*\.(pem|key|p8|p12)|\.cargo/credentials(\.toml)?)$'
if git ls-files --cached --others --exclude-standard | grep -E "$sensitive_path_pattern" >/tmp/public-release-sensitive-paths.txt; then
  echo "public-release-audit: tracked sensitive/local files found:" >&2
  sed 's/^/  - /' /tmp/public-release-sensitive-paths.txt >&2
  exit 1
fi
rm -f /tmp/public-release-sensitive-paths.txt

sensitive_content_pattern='-----BEGIN ([A-Z0-9 ]+ )?PRIVATE KEY-----|AKIA[0-9A-Z]{16}|ASIA[0-9A-Z]{16}|AIza[0-9A-Za-z_-]{35}|gh[pousr]_[A-Za-z0-9_]{36,}|github_pat_[A-Za-z0-9_]{40,}|xox[baprs]-[A-Za-z0-9-]{20,}|sk-[A-Za-z0-9_-]{32,}|cio[A-Za-z0-9]{20,}|/Users/[A-Za-z0-9._-]+|[A-Za-z0-9._%+-]+@(gmail|icloud|me|mac|protonmail|outlook|hotmail|yahoo)\.[A-Za-z]{2,}|\b[0-9]{3}-[0-9]{2}-[0-9]{4}\b'

set +e
git grep --untracked --exclude-standard -n -I -E -e "$sensitive_content_pattern" -- ':!Cargo.lock' >/tmp/public-release-sensitive-content.txt
grep_status="$?"
set -e

if [[ "$grep_status" == "0" ]]; then
  echo "public-release-audit: possible personal or sensitive content found:" >&2
  sed 's/^/  /' /tmp/public-release-sensitive-content.txt >&2
  exit 1
elif [[ "$grep_status" != "1" ]]; then
  echo "public-release-audit: sensitive-content grep failed with status ${grep_status}" >&2
  exit "$grep_status"
fi
rm -f /tmp/public-release-sensitive-content.txt

echo "public-release-audit: PASS"
