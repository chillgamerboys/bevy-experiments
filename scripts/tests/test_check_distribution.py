"""Distribution-boundary failure cases; no network or Cargo invocation required."""

from pathlib import Path
import sys
import tempfile
import tomllib
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from check_distribution import library_manifest, package_sources, validate_graph


class DistributionChecks(unittest.TestCase):
    def test_staging_removes_game_membership_preserving_shared_settings(self):
        source = '''[workspace]
resolver = "3"
members = ["games/*", "crates/*"]
default-members = ["games/labyrinth"]
[workspace.package]
version = "0.1.0"
[workspace.dependencies]
cap = { path = "crates/cap" }
'''
        parsed = tomllib.loads(library_manifest(source))
        self.assertEqual(parsed["workspace"]["members"], ["crates/*"])
        self.assertNotIn("default-members", parsed["workspace"])
        self.assertEqual(parsed["workspace"]["dependencies"]["cap"]["path"], "crates/cap")

    def test_package_list_requires_library_and_rejects_escapes(self):
        with tempfile.TemporaryDirectory() as temporary:
            crate = Path(temporary).resolve()
            (crate / "src").mkdir()
            (crate / "src/lib.rs").write_text("")
            (crate / "Cargo.toml").write_text("")
            listing = "Cargo.toml\nsrc/lib.rs\nCargo.toml.orig\nCargo.lock\n.cargo_vcs_info.json\n"
            self.assertEqual(package_sources(crate, listing), [Path("Cargo.toml"), Path("src/lib.rs")])
            for bad in ("../game.rs", str(crate / "src/lib.rs"), "missing.rs", "Cargo.toml"):
                with self.subTest(bad=bad), self.assertRaises(ValueError):
                    package_sources(crate, bad)

    def test_graph_rejects_game_and_feature_leakage(self):
        base = {"gamekit_external_probe", "bevy-gamekit"}
        validate_graph("empty", base, {"labyrinth"})
        validate_graph("ui", base | {"bevy", "bevy_game_ui"}, {"labyrinth"})
        for case, extra in (("empty", "bevy"), ("pure", "bevy"),
                            ("ui", "bevy_game_multiplayer"), ("network", "labyrinth")):
            with self.subTest(case=case), self.assertRaises(ValueError):
                validate_graph(case, base | {extra}, {"labyrinth"})

    def test_package_rejects_symlinked_source(self):
        with tempfile.TemporaryDirectory() as temporary:
            crate = Path(temporary).resolve()
            (crate / "real.rs").write_text("")
            try:
                (crate / "linked.rs").symlink_to(crate / "real.rs")
            except OSError:
                self.skipTest("creating symlinks is unavailable on this platform")
            with self.assertRaisesRegex(ValueError, "symlinked"):
                package_sources(crate, "linked.rs")


if __name__ == "__main__":
    unittest.main()
