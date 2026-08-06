# Security Policy

## Supported versions

Until versioned releases are published, only the current `main` branch is supported for security fixes. Historical branches, forks and unmerged pull requests are not supported release channels.

## Report a vulnerability privately

Do not disclose suspected vulnerabilities, credentials, personal data, exploit details or sensitive reproduction evidence in a public Issue, Discussion, pull request, commit or other public repository surface.

Use the canonical GitHub Private Vulnerability Reporting form:

**https://github.com/blakinio/Oteryn-Platform/security/advisories/new**

GitHub Private Vulnerability Reporting is enabled for this repository. The role-based contact is the Oteryn Platform repository security maintainers and administrators who can access private vulnerability reports. The repository owner is accountable for ensuring that at least one authorized maintainer monitors GitHub security notifications and that access is transferred or revoked when responsibilities change.

When the private form is unexpectedly unavailable, open a public Issue containing only a request to restore or provide a private reporting channel. Do not include technical vulnerability details in that Issue.

## Information to include

Provide only the information needed for safe validation:

- the affected Oteryn Platform component;
- the exact commit, branch or release when known;
- reproducible steps or a minimal proof of concept;
- expected and actual security behavior;
- impact, prerequisites and required privileges;
- whether sensitive data was encountered;
- suggested remediation or regression coverage when known;
- any disclosure timing constraints that should be considered.

Do not include unrelated personal data, production credentials, destructive payloads or persistence mechanisms.

## Response process

Repository security maintainers will:

1. acknowledge the private report as soon as practical;
2. validate repository scope, reproducibility and affected versions;
3. assign severity and a remediation owner;
4. preserve confidentiality while a fix and regression coverage are prepared;
5. provide material status updates when assessment or remediation state changes;
6. coordinate disclosure after remediation or an explicit risk decision.

Timelines depend on severity, reproducibility and maintainer availability. This policy does not promise a fixed acknowledgement or remediation service-level agreement.

## Scope

Security-sensitive Oteryn Platform areas include authentication, sessions, MFA, recovery, authorization, administrator actions, uploads, webhooks, payments, wallet or currency state, cross-repository game contracts, deployment automation, public-edge controls and secrets handling.

This policy governs findings in `blakinio/Oteryn-Platform` and repository-owned deployment or integration behavior documented here. Findings whose root cause belongs exclusively to another Oteryn repository, an upstream dependency or a third-party service may be confidentially redirected to the responsible owner. Sensitive details must not be copied into public records during that handoff.

## Good-faith research and safe harbor

Good-faith research must avoid:

- privacy violations or access to unrelated data;
- service disruption or resource exhaustion;
- destructive data changes;
- persistence or lateral movement;
- credential misuse;
- social engineering;
- testing beyond what is necessary to demonstrate the suspected issue safely.

Stop testing and report immediately when sensitive data, credentials or an unsafe production effect is encountered. This policy does not authorize activity prohibited by law, third-party terms or systems outside the stated repository scope.

## Route verification

The reporting route may be verified non-destructively through GitHub's private-vulnerability-reporting API state and the direct reporting URL. Do not create a fake advisory, disclose a real vulnerability or place sensitive material in public repository records merely to test reachability.
