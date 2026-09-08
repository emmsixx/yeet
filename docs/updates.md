# Update plans

Update with Homebrew, rerun the installer, or pull and reinstall a source build,
depending on how you installed Yeet. There is no background check or self-updater.
See [installation](installation.md).

A future `yeet update` should be explicit, verify the download, and replace the
binary atomically. It must track install ownership and leave Homebrew, Cargo,
and distro-managed binaries to their package managers. Failed updates should
preserve the installed version and user configuration.
