# Native Oteryn protocol contract migration

Coordination ID: `OTS-20260804-native-protocol-selection`  
Canonical contract revision: `2`  
Schema revision: `2`  
Schema SHA-256: `9c67f19525400fb9890d2a3541ceb6d02eb955061540ad39ca1c1d891c06eba9`

## Required migration

The disabled transitional producer used a native `profile` string. The corrected model uses exactly `family = oteryn` and `native_protocol_version = 1`. No profile column, table, alias, placeholder, registry, factory, order or selector may remain in the final active native model.

The runtime implementation package must inspect deployed state before migration, add explicit version storage without reinterpreting serialized profile bytes, migrate only proven legacy native-v1 rows to version `1`, fail closed on any other value, validate readiness/session projections, and remove the transitional field within the same bounded migration programme. Native rows stay disabled and no candidate or endpoint is seeded.

Rollback must be rehearsed against disposable data. It may restore the pre-correction schema only while no deployed component requires the corrected model; it must never enable native advertisement or infer a profile.

## Protobuf compatibility

Removed `gameplay_profile` fields retain their numbers and names as `reserved`. New `native_protocol_version` fields use new numbers. The transport field keeps its wire number and semantics but is renamed from `transport_profile` to `transport`.
