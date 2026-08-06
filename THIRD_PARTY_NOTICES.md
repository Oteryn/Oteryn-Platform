# Third-party rights and provenance boundary

This document records the legal boundary for material referenced, installed, generated or bundled by Oteryn Platform. It is not a complete bill of materials and does not grant rights to any third-party content.

## Dependency software

PHP and Composer dependencies, including Laravel and other packages declared through `composer.json` and `composer.lock`, remain governed by their upstream licenses and notices. Dependency manifests and lock files identify technical versions; they are not replacement license texts and do not transfer third-party rights to Oteryn Platform.

Development tools, operating-system packages, database engines, browsers, GitHub Actions and external services are likewise governed by their respective terms.

## Assets, data and compatibility material

The proprietary notice in `LICENSE.md` does not claim ownership of or grant rights to third-party:

- fonts, icons, images, audio, video or other media;
- game names, characters, creatures, items, maps, texts, statistics or other game data;
- fixtures, samples, screenshots and test evidence derived from external systems;
- network, protocol or compatibility information derived from Canary, OpenTibia, Tibia or other projects;
- copied, adapted or generated material that carries a separate notice or whose provenance is not yet proven.

A technical need to interoperate with another component does not imply permission to redistribute that component or its protected content.

## File-specific notices

A license, copyright or attribution notice attached to a specific file or directory governs that material within its stated scope. Such a notice is not broadened to the rest of the repository, and the repository's proprietary policy does not override valid third-party terms.

Do not remove, obscure or replace file-specific notices without a verified provenance review and explicit authorization.

## Current provenance status

The repository does not claim that every dependency, asset, fixture, game-data sample or protocol-derived material has completed a legal provenance audit. Unknown provenance is a restriction, not permission.

Before distributing a release or separately publishing bundled material, the responsible release owner must:

1. inventory the exact material included in the distribution;
2. identify its source, rights holder and applicable terms;
3. verify compatibility with the intended distribution model;
4. preserve required license texts, copyright notices and attributions;
5. exclude or replace material whose rights remain unknown or incompatible.

Future work may replace this boundary document with a versioned software bill of materials and an evidence-backed asset provenance register. Until then, absence from this document must never be interpreted as a license grant or a claim of ownership.
