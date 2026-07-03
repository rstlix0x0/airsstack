---
description: Provision an OKF bundle root (default knowledge/) with the okf_version marker index.md and an empty log.md. Idempotent; never overwrites.
---

# okf-setup

Provision an OKF v0.1 bundle in this repository. `$ARGUMENTS` may carry an
explicit bundle-root path; the default is `knowledge/` at the repo root.
The bundle is meant to be committed WITH the repo — do not gitignore it.

## Steps

1. Resolve the target directory: `$ARGUMENTS` if non-empty, else
   `knowledge/` under `git rev-parse --show-toplevel` (or the cwd when not
   inside git).

2. Idempotence check: if `<target>/index.md` exists and its frontmatter
   carries `okf_version:`, report "bundle already provisioned at
   <target>" and STOP. Never overwrite existing files.

3. Create the skeleton in one Bash call, substituting today's date from
   `date +%F`:

   ```sh
   mkdir -p "<target>"
   cat > "<target>/index.md" <<'EOF'
   ---
   okf_version: "0.1"
   ---

   # Index
   EOF
   cat > "<target>/log.md" <<EOF
   ## $(date +%F)

   - **Creation** — provisioned empty OKF bundle.
   EOF
   ```

4. Report the bundle root path and the two files created, and remind the
   user to commit the bundle with the repo. Do not commit yourself.
