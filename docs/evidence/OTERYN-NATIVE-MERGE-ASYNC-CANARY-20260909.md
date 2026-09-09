# Native Merge Queue canary

This documentation-only carrier verifies the protected META 3.1 native exact-head Merge Queue executor after its protected integration.

- authority: `Oteryn/Oteryn#187`
- provider task: `Oteryn/Oteryn-Platform#1363`
- mutation under test: REST `merge-async` with the exact qualified head and explicit `merge_action=merge_queue`
- terminal proof required: accepted request/readback, real `merge_group` `platform-gate`, PR integration, and protected-main readback

No product/runtime/auth/database/payment/deployment/protection/ruleset/required-check behavior is changed by this carrier.
