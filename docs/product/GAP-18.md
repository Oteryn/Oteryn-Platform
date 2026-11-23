# Public character discovery and comparison

Visitors can look up a full character name under **Community → Characters** or
select **Characters** in portal search. Both lookups use the same current
public profile and privacy authority. The profile links to comparison with the
first name filled in. Comparison accepts two names and keeps the existing
2–5 UUID links usable. It shows observed daily values in the selected date range.

## Operator status

- **Disabled.** The public profile directory and the character search provider
  require `PORTAL_PUBLIC_PROFILES_ENABLED=true`. The comparison page requires
  `PORTAL_CHARACTER_STATS_ENABLED=true` and uses the directory for results.
  Both flags default to off.
- **Unavailable (503).** Check the accepted `character_profile` family watermark
  and the ownership feed's source, epoch and freshness. Check that
  `OTERYN_CHARACTER_PROFILES_ENABLED` enables live Platform privacy decisions.
  A conflicted or stale profile family withholds results.
- **Ready.** A current, visible character resolves in both searches. Hidden and
  unknown subjects produce the same no-result behavior. Profile and comparison
  return 404 for these subjects.
- **Metric unavailable.** Check `PORTAL_STATS_ENABLED` and the `stats.poll` job.
  It materializes daily level observations from covered character profiles.
  An eligible character can have no observations in the requested window.

## Game qualification remaining

Production qualification needs a live accepted Game publisher for complete
character profiles and the current ownership feed. The existing public ownership
and privacy reader is limited to testing and preproduction. Production serving
also needs a qualified ownership and privacy contract.

The existing profile producer contract supplies level observations. Experience
and skill comparisons require accepted daily metric producers. Optional profile
fields also need an accepted details source. Missing observations use the
existing unavailable state. The portal uses read-only Platform projections for
lookups and comparison.
