"""Reject incomplete or ambiguous release metadata before updating a tap."""

from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
from homebrew_formula import formula
from package import TARGETS, version


class FormulaTests(unittest.TestCase):
    def manifest(self):
        tag = f"v{version()}"
        return "".join(
            f"{'a' * 64}  yeet-{tag}-{target}.tar.gz\n"
            for target in TARGETS if not target.endswith("windows-msvc")
        )

    def test_requires_all_four_unix_assets(self):
        with self.assertRaisesRegex(ValueError, "missing checksum"):
            formula(f"v{version()}", "\n".join(self.manifest().splitlines()[:-1]))

    def test_rejects_duplicate_or_malformed_checksums(self):
        for manifest in (self.manifest() * 2, "invalid checksum\n"):
            with self.assertRaises(ValueError):
                formula(f"v{version()}", manifest)

    def test_formula_selects_verified_assets_and_contains_smoke_test(self):
        output = formula(f"v{version()}", self.manifest())
        self.assertEqual(output.count('      url "https://github.com/emmsixx/yeet/releases/download/'), 4)
        self.assertIn('license "GPL-3.0-or-later"', output)
        self.assertIn('testpath/"config.toml"', output)


if __name__ == "__main__":
    unittest.main()
