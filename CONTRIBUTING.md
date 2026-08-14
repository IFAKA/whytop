# Contributing to whytop

Thanks for helping improve whytop. Keep changes focused and preserve its read-only, local-first behavior.

## Before opening a pull request

1. Create a focused branch from `master`.
2. Explain the user problem and the behavior change.
3. Run the local checks:

   ```sh
   cargo fmt --all -- --check
   cargo test
   cargo check
   ```

4. Include reproduction steps for bugs and note the operating system and local model runtime when relevant.

Avoid adding environment variables, secrets, or hosted AI dependencies to process evidence. New platform-specific behavior should keep the shared core portable and include a fallback for unavailable data.

## Pull requests

Small pull requests are easier to review. Update the README when user-facing behavior or setup changes, and describe any checks that could not be run locally.
