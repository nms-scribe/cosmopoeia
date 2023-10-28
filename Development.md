# Rules For Developers

* Try to follow formatting standards already in use. 
* If you see something that doesn't match those standards, go ahead and fix that.
* If you see inconsistencies in style, you're welcome to bring them up and fix them if asked to do so.
* Comment well (not just doc comments) and use code practices (full-word identifiers, etc.) that turn the code into the comment.
* Run clippy as often as you want, but before pushing changes
* The additional clippy warnings are in main.rs, don't remove any of them.
* All warnings, both compilation and clippy, should be gone before pushing changes.
* Add to the changelog as new features are added, majore bugs are fixed.
  * Keep the changelog human-readable.
* To tag a new version: run `release.ers` using rust-script (`cargo install rust-script`).
* After tagging, create deployment packages on all platforms:
  * Arch Linux: `cargo aur` (`cargo install --git https://github.com/nms-scribe/cargo-aur` until the official repository gets files)
  * Windows: `deploy_windows.ers` (rust-script)
* Finally, run `release.ers` one last time with the `rc` version bump, so continuing edits are made with a `rc-1` suffix on the version number.

