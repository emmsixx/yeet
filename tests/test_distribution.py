"""Exercise packaging and the real installer without GitHub/network access."""

import os
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
import package  # noqa: E402
import prepare_release  # noqa: E402


class PackagingTests(unittest.TestCase):
    def test_platform_archives_contain_binary_license_and_documentation(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            binary = root / "binary"
            binary.write_bytes(b"release-binary")
            for target in package.TARGETS:
                with self.subTest(target=target):
                    archive = package.package(target, f"v{package.version()}", binary, root / "dist")
                    with tarfile.open(archive) as source:
                        names = source.getnames()
                        executable = next(item for item in source.getmembers() if item.name.endswith("/yeet"))
                        self.assertEqual(executable.mode & 0o777, 0o755)
                    for suffix in ("/LICENSE", "/README.md", "/CONTRIBUTING.md", "/SECURITY.md",
                                   "/docs/installation.md", "/examples/config.toml"):
                        self.assertTrue(any(name.endswith(suffix) for name in names), suffix)
            prepare_release.verify_archives(root / "dist", f"v{package.version()}")
            archive.write_bytes(b"tampered")
            with self.assertRaisesRegex(ValueError, "checksum mismatch"):
                prepare_release.verify_archives(root / "dist", f"v{package.version()}")

    def test_tag_must_match_manifest(self):
        with self.assertRaises(ValueError):
            package.validate_tag("v99.0.0")

    def test_partial_matrix_is_not_releasable(self):
        with tempfile.TemporaryDirectory() as temporary:
            with self.assertRaisesRegex(ValueError, "missing release archive"):
                prepare_release.verify_archives(Path(temporary), f"v{package.version()}")

    def test_complete_release_includes_tagged_source_and_checksums(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            repository = root / "repository"
            repository.mkdir()
            for name in ("Cargo.toml", "Cargo.lock", "install.sh", "LICENSE", "rust-toolchain.toml"):
                (repository / name).write_bytes((ROOT / name).read_bytes())
            subprocess.run(["git", "init", "--quiet", str(repository)], check=True)
            subprocess.run(["git", "add", "."], cwd=repository, check=True)
            subprocess.run(
                ["git", "-c", "user.name=Yeet Test", "-c", "user.email=yeet@example.invalid",
                 "-c", "commit.gpgsign=false", "-c", f"core.hooksPath={root / 'no-hooks'}",
                 "commit", "--quiet", "-m", "Source fixture"], cwd=repository, check=True,
            )
            tag = f"v{package.version()}"
            subprocess.run(["git", "-c", "tag.gpgsign=false", "tag", tag], cwd=repository, check=True)
            binary = root / "binary"
            binary.write_bytes(b"release-binary")
            output = root / "dist"
            for target in package.TARGETS:
                package.package(target, tag, binary, output)
            with mock.patch.object(prepare_release, "ROOT", repository):
                prepare_release.prepare(output, tag)
            with tarfile.open(output / f"yeet-{tag}-source.tar.gz") as source:
                names = source.getnames()
                self.assertIn(f"yeet-{tag}/Cargo.lock", names)
                self.assertIn(f"yeet-{tag}/LICENSE", names)
            manifest = (output / "SHA256SUMS").read_text(encoding="utf-8").splitlines()
            self.assertEqual(len(manifest), len(package.TARGETS) + 2)
            for line in manifest:
                checksum, name = line.split("  ")
                self.assertEqual(checksum, prepare_release.digest(output / name))


class InstallerTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="yeet-installer-test-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.fixture = self.root / "release"
        self.fixture.mkdir()
        self.bin_dir = self.root / "install with spaces"
        self.fake_bin = self.root / "commands"
        self.fake_bin.mkdir()
        self.tag = f"v{package.version()}"
        self.env = dict(os.environ, PATH=f"{self.fake_bin}:{os.environ['PATH']}",
                        YEET_FIXTURE=str(self.fixture), YEET_TEST_TAG=self.tag,
                        YEET_INSTALL_DIR=str(self.bin_dir), YEET_VERSION=self.tag)
        self.command("uname", '#!/bin/sh\ncase "$1" in -s) echo "${YEET_TEST_OS:-Linux}";; -m) echo "${YEET_TEST_ARCH:-x86_64}";; esac\n')
        self.command("curl", '''#!/bin/sh
set -eu
if [ "${YEET_TEST_DOWNLOAD_FAIL:-0}" = 1 ]; then exit 22; fi
output=
while [ "$#" -gt 0 ]; do
    case "$1" in
        --output) output=$2; shift 2 ;;
        *) url=$1; shift ;;
    esac
done
case "$url" in
    */releases/latest) printf 'https://github.com/emmsixx/yeet/releases/tag/%s' "$YEET_TEST_TAG" ;;
    *) cp "$YEET_FIXTURE/${url##*/}" "$output" ;;
esac
''')
        binary = self.root / "binary"
        binary.write_text(f"#!/bin/sh\necho 'yeet {package.version()}'\n", encoding="utf-8")
        for target in package.TARGETS:
            package.package(target, self.tag, binary, self.fixture)

    def command(self, name, content):
        path = self.fake_bin / name
        path.write_text(content, encoding="utf-8")
        path.chmod(0o755)

    def install(self, *args):
        return subprocess.run(["sh", str(ROOT / "install.sh"), *args], env=self.env,
                              text=True, capture_output=True, timeout=15)

    def archive(self):
        return self.fixture / f"yeet-{self.tag}-x86_64-unknown-linux-gnu.tar.gz"

    def test_install_and_replace_in_path_with_spaces(self):
        self.bin_dir.mkdir()
        (self.bin_dir / "yeet").write_text("old version", encoding="utf-8")
        result = self.install()
        self.assertEqual(result.returncode, 0, result.stderr)
        result = subprocess.run([str(self.bin_dir / "yeet"), "--version"], capture_output=True, text=True, check=True)
        self.assertEqual(result.stdout.strip(), f"yeet {package.version()}")
        self.assertFalse(list(self.bin_dir.glob(".yeet.*")))

    def test_all_supported_platform_mappings(self):
        for platform, architecture in (("Linux", "aarch64"), ("Darwin", "arm64"), ("Darwin", "x86_64")):
            with self.subTest(platform=platform, architecture=architecture):
                self.env.update(YEET_TEST_OS=platform, YEET_TEST_ARCH=architecture)
                result = self.install()
                self.assertEqual(result.returncode, 0, result.stderr)

    def test_latest_and_cli_overrides(self):
        self.env["YEET_VERSION"] = "invalid"
        custom = self.root / "custom"
        result = self.install("--version", "latest", "--bin-dir", str(custom))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((custom / "yeet").is_file())
        self.assertFalse(self.bin_dir.exists())

    def test_checksum_failure_preserves_existing_binary(self):
        self.bin_dir.mkdir()
        installed = self.bin_dir / "yeet"
        installed.write_bytes(b"existing binary")
        self.archive().write_bytes(b"tampered")
        result = self.install()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("checksum mismatch", result.stderr)
        self.assertEqual(installed.read_bytes(), b"existing binary")

    def test_missing_checksum_fails_closed(self):
        self.archive().with_name(self.archive().name + ".sha256").unlink()
        result = self.install()
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(self.bin_dir.exists())

    def test_download_failure_does_not_install(self):
        self.env["YEET_TEST_DOWNLOAD_FAIL"] = "1"
        self.assertNotEqual(self.install().returncode, 0)
        self.assertFalse(self.bin_dir.exists())

    def test_invalid_version_and_platform_fail(self):
        for value in ("../../main", "1.0.0\nevil", "v1.0.0?query"):
            self.assertNotEqual(self.install("--version", value).returncode, 0)
        self.env["YEET_TEST_OS"] = "Plan9"
        result = self.install()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("unsupported operating system", result.stderr)

    def test_symlink_is_not_replaced(self):
        self.bin_dir.mkdir()
        original = self.root / "managed-binary"
        original.write_bytes(b"managed binary")
        (self.bin_dir / "yeet").symlink_to(original)
        result = self.install()
        self.assertNotEqual(result.returncode, 0)
        self.assertTrue((self.bin_dir / "yeet").is_symlink())
        self.assertEqual(original.read_bytes(), b"managed binary")

    def test_checksum_valid_but_wrong_version_is_not_installed(self):
        binary = self.root / "wrong-version"
        binary.write_text("#!/bin/sh\necho 'yeet 99.0.0'\n", encoding="utf-8")
        package.package("x86_64-unknown-linux-gnu", self.tag, binary, self.fixture)
        result = self.install()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("version does not match", result.stderr)
        self.assertFalse(self.bin_dir.exists())


if __name__ == "__main__":
    unittest.main()
