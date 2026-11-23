# GAP-02 · Native login qualification

Production native login is blocked pending [N4P-5 §17A](../contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md#17a-amendment-n4p-5-proposed-production-issuance). The account overview shows game sign-in as unavailable. The staff panel links to `/admin/native-login` for staff with MFA, `admin.access` and `audit.view`. The page lists the release decisions, outstanding evidence and local Platform switch settings. The switch values describe configuration in the web process. Readiness requires the evidence below.

## Decisions to record

| Decision | Owner | Required outcome |
| --- | --- | --- |
| U3 | Architect and owner | Choose the gameplay TLS trust anchor and agree on the released client's name verification and trust rotation. |
| D3 | Architect | After U3, accept the route record and its revision digest. |
| U7 | Platform | Approve how production expresses per-scope login policy. Missing, ambiguous and disabled policy must refuse issuance. |
| U8 | Platform and owner | Approve production topology and scope issuance, including operator and owner approval for route publication. |
| U9 | Owner | Approve production key custody, delivery, rotation and revocation. |

## Platform after decision approval

1. Implement the accepted production paths for ticket issuance, topology, route publication, policy and signing trust behind separate default-off settings. Keep production and preproduction evidence and keys isolated, and audit production publication with two-person approval.
2. Require the full [§5.4 ownership check](../contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md#54-issuance-check), a fresh ownership-bound runtime report and a matching route revision before granting admission. Publish the first approved scope with its policy off and verify issuer and Game trust before enabling that scope.
3. Run [§16](../contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md#16-required-contract-tests-before-activation) and [§17A.4](../contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md#17a4-required-tests-before-activation) on the exact candidate. Keep rollout scoped to the first approved route until the joint test passes. Roll back a scope through its policy and the full rollout through the Gateway switch, per §17A.3.

## Game and joint acceptance

- Ship a signed native client that completes S256 PKCE and verifies the gameplay certificate against U3 and the route TLS name. Qualify build acceptance and update policy with the actual release.
- Deploy the Game listener with the approved certificate and exact route revision. Publish ownership and runtime evidence through their existing contracts, and prove trust distribution and grant verification, including rotation and revocation.
- On a production-shaped stack, sign in through the client, issue a ticket, request admission for an owned playable character, and observe the first successful Game connection. Also prove that a suspended account or replay is refused, and that invalid builds, foreign characters and unavailable or mismatched routes yield no grant. Record the exact Platform and Game candidates and the route revision with the result. Use the [§14 joint rollout test](../contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md#14-rollout-rollback-and-mixed-versions).
