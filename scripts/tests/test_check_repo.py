"""Executable policy examples; no Cargo build or network dependency."""

from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from check_repo import check


class RepositoryChecks(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.write("Cargo.toml", '[workspace]\nmembers = ["crates/*", "games/*"]\n')
        self.write("games/example/Cargo.toml", '[package]\nname = "example"\nversion = "0.1.0"\n')

    def write(self, name, content):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)

    def test_valid_workspace_and_ignored_output(self):
        self.write("README.md", "[Game](games/example/Cargo.toml)\n```sh\n[x](absent)\n```\n")
        self.write("target/README.md", "[broken](absent)")
        self.assertEqual(check(self.root), [])

    def test_missing_and_escaping_links(self):
        self.write("README.md", "[missing](missing.md) [outside](../Cargo.toml)")
        self.assertEqual(len(check(self.root)), 2)

    def test_game_dependency_through_workspace_alias(self):
        self.write("Cargo.toml", '[workspace]\nmembers = []\n[workspace.dependencies]\nalias = { package = "example", path = "games/example" }\n')
        self.write("crates/cap/Cargo.toml", '[package]\nname = "cap"\nversion = "0.1.0"\n[dev-dependencies]\nalias.workspace = true\n')
        self.assertTrue(any("depends on game" in error for error in check(self.root)))

    def test_legacy_and_duplicate_workspace(self):
        self.write("src/lib.rs", "")
        self.write("games/example/Cargo.lock", "")
        self.write("games/example/Cargo.toml", '[workspace]\nmembers = []\n')
        self.assertEqual(len(check(self.root)), 3)


if __name__ == "__main__":
    unittest.main()
