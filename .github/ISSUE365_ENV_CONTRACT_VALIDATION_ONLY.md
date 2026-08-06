# Issue 365 validator environment contract proof

Temporary static-only branch for Issue #730 / parent #365.

- Never merge this branch or its observation PR.
- Run only on GitHub-hosted `ubuntu-latest`.
- Consume exact artifact `8964153679` and historical workflow `f23bd310eb8812ff61e7ad7227b2a950bf695b59`.
- Extract and classify every required top-level validator environment input.
- Fail closed on any unresolved input or provenance mismatch.
- Do not allocate self-hosted/Synology runners or execute Docker, browsers, databases or product samples.
- Close the observation PR after the single terminal static result is recorded.
