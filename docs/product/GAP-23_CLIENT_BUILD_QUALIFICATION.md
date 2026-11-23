# GAP-23 minimum client build qualification

The [minimum client build gate proposal](../contracts/OTERYN_V2_MINIMUM_CLIENT_BUILD_GATE_CONTRACT_PROPOSAL.md)
is awaiting joint Platform, Game and client approval. The staff preview supplies
Platform evidence for that decision. The proposed rule remains unenforced.

## Platform evidence

Open **Staff panel → Client build qualification** with an MFA-authenticated
staff account holding `admin.access` and `downloads.manage`.

- Read the active, verified Downloads publication for each update channel.
  The view distinguishes missing publications, expired publications and
  current publications with or without a Windows x86-64 installer. An expired
  publication still supplies its minimum and revocations.
- Preview a known release at and below a proposed minimum, an unregistered
  release identity, and a legacy offer with both release fields blank.
  The two update channels have independent release histories.
- Use the effective settings for the game scope being evaluated. Under the
  proposal, ignore open scope rows, take maintenance over enforcing, take
  the highest applicable minimum, and let refuse win for legacy offers.
  Enter those effective settings in the preview. The verified Downloads
  minimum is included automatically. The minimum warning means the active
  publication cannot supply a qualifying installer.
- Record the publication revision and input values alongside the displayed result in
  the rollout review. The preview evaluates compatibility. Game admission
  readiness for the rollout is shown as unavailable until joint evidence is
  available.

## Decisions and evidence required to activate the gate

1. Ratify the native login and recovery wire contract, including the release
   identity rules, old-client behavior, update response and maintenance
   behavior. Decide which caller identity guarantees are required, including
   whether rollout policy requires client attestation.
2. Qualify the Platform admission implementation after ratification. Prove
   exact Gateway relay, unchanged legacy bodies, candidate filtering,
   rollback of refused attempts, and transactional gate updates with
   operator authorization and audit. Deploy support with enforcement off.
   Record the minimum compatible Gateway and issuer versions as the rollback
   floor before shipping any client that uses the wire extension.
3. Game and client owners must qualify the compiled release identity and
   updater trust, choose the overlap serving strategy, and demonstrate a
   session on both chains with the overlap client. Verify the update prompt
   and independent updater path for older builds.
4. For a planned rotation, raise the minimum only after every affected
   update channel has a verified installable overlap release and the target
   Game scope has passed overlap qualification. Game operators drain or end
   affected live sessions and verify the next-only release before retiring
   the old chain. For a compromise, agree on the maintenance scope, drain
   sequence and replacement release before returning it to enforcing.
5. Keep enforcement off until the required evidence is signed off. After
   activation, lowering a minimum or disabling enforcement must preserve the
   recorded wire-compatible rollback floor.

The [design](../architecture/MINIMUM_CLIENT_BUILD_GATE_DESIGN.md) and proposal
list the Platform and joint acceptance cases for activation.
