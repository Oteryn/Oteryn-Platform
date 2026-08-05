# Security Policy

## Supported versions

Until versioned releases are published, only the current `main` branch is supported for security fixes. Historical branches, forks and unmerged pull requests are not supported release channels.

## Reporting a vulnerability

Do not disclose suspected vulnerabilities, credentials, personal data or exploit details in a public Issue, Discussion, pull request or commit.

Use GitHub's **Security → Report a vulnerability** flow when private vulnerability reporting is available. Include:

- the affected component and exact commit or release;
- reproducible steps or a minimal proof of concept;
- expected and actual security behavior;
- impact and prerequisites;
- suggested remediation, when known.

When the private reporting form is unavailable, open a public Issue containing only a request for a private reporting channel. Do not include technical vulnerability details in that Issue.

## Response process

The maintainer will acknowledge a private report, validate the affected boundary, assign severity, prepare a fix and regression coverage, and coordinate disclosure after remediation. Timelines depend on severity and reproducibility; no report is considered resolved until the resulting state is verified.

## Scope

Security-sensitive areas include authentication, sessions, MFA, recovery, authorization, administrator actions, uploads, webhooks, payments, wallet or currency state, cross-repository game contracts, deployment automation and secrets handling.

Reports concerning third-party services or dependencies may be redirected to the responsible upstream project while preserving confidential details.

## Safe-harbor expectations

Good-faith research must avoid privacy violations, service disruption, destructive data changes, persistence, credential misuse, social engineering and access beyond what is necessary to demonstrate the issue. Stop testing and report immediately when sensitive data or an unsafe production effect is encountered.
