# Update plans

Update source builds by pulling and reinstalling. Once releases are available,
rerun the installer to update. There is no background check or self-updater.
See [installation](installation.md).

A future `yeet update` should be explicit, verify the download, and replace the
binary atomically. It must track install ownership and leave Homebrew, Cargo,
and distro-managed binaries to their package managers. Failed updates should
preserve the installed version and user configuration.
