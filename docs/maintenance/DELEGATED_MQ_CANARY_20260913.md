# Delegated Merge Queue canary — 2026-09-13

Purpose: provide one bounded, non-runtime Platform candidate for validating the protected META delegated Merge Queue executor from Oteryn/Oteryn#195 and control Issue Oteryn/Oteryn#196.

This file is intentionally operationally inert. It changes no runtime, deployment, protection, ruleset, workflow, credential, product behavior, or provider integration policy.

Acceptance for this canary is external to this file: exact-head provider qualification, one authorized delegated `/oteryn-mq-submit` request, accepted native `merge-async` receipt with UUID/readback, real Platform `merge_group` with `platform-gate` success, merged PR state, and protected-main readback.
