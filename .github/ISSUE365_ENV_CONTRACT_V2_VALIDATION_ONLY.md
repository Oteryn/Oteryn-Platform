# Issue 365 environment contract proof v2

Temporary static-only branch for Issue #733 / parent #365.

- Never merge this branch or its observation PR.
- Run on `ubuntu-latest` only.
- Consume exact artifact `8964153679` and historical workflow `f23bd310eb8812ff61e7ad7227b2a950bf695b59`.
- Construct the expected GitHub secret expression at runtime from separate fragments; do not embed an interpolatable expression in workflow source.
- Fail closed on unresolved validator environment inputs.
- Do not allocate self-hosted runners or execute product/browser code.
- Close the observation PR after the single terminal result.
