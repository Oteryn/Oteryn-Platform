# Tibia worldmap reconstruction tool

This tool is the proprietary-data-free normalization and comparison layer for task `OTERYN-20260811-tibia-client-analysis`.

It does **not** contain or extract proprietary Tibia assets by itself. Runtime/asset evidence is transformed outside Git into normalized IDs and classifications; only safe normalized data may be passed to this tool.

## Contract

Input format is `oteryn-worldmap-normalized-v1`. Every tile has an explicit `(x,y,z)`, `observed` state, monotonic `sequence`, ordered contents, semantic class, client appearance ID and explicit mapping state.

Never use an empty tile to mean “not seen”. A tile that has not been received/observed must carry `observed: false` or be absent and is handled separately by comparison.

Semantic classes:

- `ground`
- `ground_border`
- `static_item`
- `dynamic_item`
- `creature`
- `npc`
- `effect_or_ephemeral`
- `unknown`

Mapping states:

- `MAPPED` — `server_otb_id` is proven and required;
- `UNMAPPED` — no server ID is guessed;
- `NOT_APPLICABLE` — the content is intentionally outside static OTBM item translation.

Creatures/NPCs belong in the document `entities` stream and are excluded from static OTBM comparison unless separately classified as static content. One sighting must never be converted into a proven spawn definition.

## CLI

```bash
PYTHONPATH=tools/tibia-worldmap-reconstruction \
  python3 tools/tibia-worldmap-reconstruction/run.py validate map.json

PYTHONPATH=tools/tibia-worldmap-reconstruction \
  python3 tools/tibia-worldmap-reconstruction/run.py compare reconstructed.json crystalserver.json --output diff.json

PYTHONPATH=tools/tibia-worldmap-reconstruction \
  python3 tools/tibia-worldmap-reconstruction/run.py merge accumulated.json next-capture.json --output merged.json

PYTHONPATH=tools/tibia-worldmap-reconstruction \
  python3 tools/tibia-worldmap-reconstruction/run.py otbm-plan reconstructed.json --output export-plan.json
```

`otbm-plan` is intentionally **not** a binary OTBM writer. It exits with status `2` while any observed tile has an unproven mapping or no proven ground. A real OTBM writer is allowed only after client/object ID -> server/OTB ID translation is proven.

## Reference comparison

The same normalized representation is used for reference inputs. Planned reference adapters are:

- CrystalServer world OTBM/content;
- Renemap;
- TibiaMaps geographic/coverage evidence.

Comparison is coordinate/structure based, never OCR/visual matching. Current result statuses are `MATCH`, `MISSING_IN_REFERENCE`, `MISSING_IN_RECONSTRUCTION`, `GROUND_MISMATCH`, `ITEM_MISMATCH`, `STACK_ORDER_MISMATCH`, `UNMAPPED_ID`, `REFERENCE_CONFLICT`, and `NOT_OBSERVED`.

See `docs/agents/reports/OTERYN-20260812-worldmap-reconstruction-comparison-plan.md` for the durable reconstruction and comparison contract.

## Validation

```bash
python3 -m compileall -q tools/tibia-worldmap-reconstruction
PYTHONPATH=tools/tibia-worldmap-reconstruction \
  python3 -m unittest discover -s tools/tibia-worldmap-reconstruction/tests -v
```
