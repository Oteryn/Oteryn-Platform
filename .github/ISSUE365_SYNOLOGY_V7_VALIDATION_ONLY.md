# Issue 365 Docker API 1.43 validation v7

Temporary validation-only branch for Issue #740 / parent #365.

- Never merge this branch or its observation PR.
- Consume exact validator artifact `8964153679` and environment proof artifact `8964791387`.
- Verify artifact metadata, outer digests and all internal hashes before transformation.
- Derive the runtime validator by adding exactly one line, `DOCKER_API_VERSION=1.43`, immediately after `CI=1` in the generated `.issue365.env` heredoc.
- Require original validator SHA-256 `5e89a700d85cb362e374a500bd923d52eea1a9b1b86d0fe657e07c0e134f5945` and derived SHA-256 `3280d961652b5aa6659d73fc8020fb8b6dba9d4879a1695d4323afe62e3d76b4`.
- Require an exact one-line diff, one compatibility-pin occurrence and valid Bash syntax.
- Invoke the derived validator exactly once on frozen target `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`.
- Retain full or partial matrix evidence and cleanup status.
- Do not modify application code, dependencies, deployment, production, Cloudflare, Canary or external repositories.
- Close the observation PR after terminal classification.
