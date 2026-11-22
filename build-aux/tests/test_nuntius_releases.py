"""Tests of build-aux/nuntius-releases.py.

    python3 -m unittest discover -s build-aux/tests
"""

import importlib.util
import pathlib
import subprocess
import sys
import tempfile
import unittest
import xml.etree.ElementTree as ET

ROOT = pathlib.Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "build-aux" / "nuntius-releases.py"
RELEASES = ROOT / "data" / "de.nexumia.Nuntius.releases.xml"
METAINFO = ROOT / "data" / "org.gnome.Fractal.metainfo.xml.in.in"

spec = importlib.util.spec_from_file_location("nuntius_releases", SCRIPT)
nuntius_releases = importlib.util.module_from_spec(spec)
spec.loader.exec_module(nuntius_releases)

OWN = """<component>
  <releases>
    <release version="0.3.0~dev.20261122.abc1234" date="2026-11-22" type="development">
      <description>
        <ul>
          <li>Pending change.</li>
        </ul>
      </description>
    </release>
    <release version="0.2.0" date="2026-11-20">
      <description>
        <ul>
          <li>Something new &amp; good.</li>
        </ul>
      </description>
    </release>
    <release version="0.1.0" date="2026-11-04"/>
  </releases>
</component>
"""

UPSTREAM = """<?xml version="1.0" encoding="UTF-8"?>
<component type="desktop-application">
  <id>de.nexumia.Nuntius</id>
  <releases>
    <release version="14.3" type="stable" date="2026-10-26">
      <description>
        <p>Upstream text.</p>
      </description>
    </release>
    <release version="14.2" type="stable" date="2026-10-21"/>
  </releases>
</component>
"""


def versions(metainfo_text):
    root = ET.fromstring(metainfo_text)
    return [r.get("version") for r in root.findall("releases/release")]


class Version(unittest.TestCase):
    def test_newest_stable(self):
        self.assertEqual(nuntius_releases.version(OWN), "0.2.0")

    def test_no_stable_release(self):
        with self.assertRaises(nuntius_releases.Error):
            nuntius_releases.version("<component><releases/></component>")


class Merge(unittest.TestCase):
    def test_replaces_upstream_releases(self):
        merged = nuntius_releases.merge(UPSTREAM, OWN)
        self.assertEqual(
            versions(merged),
            ["0.3.0~dev.20261122.abc1234", "0.2.0", "0.1.0"])
        self.assertNotIn("Upstream text", merged)
        self.assertIn("<li>Something new &amp; good.</li>", merged)
        # Everything outside <releases> is kept as it was.
        head, tail = UPSTREAM.split("<releases>")[0], "  </releases>\n</component>\n"
        self.assertTrue(merged.startswith(head))
        self.assertTrue(merged.endswith(tail))

    def test_development_placeholder_is_dropped(self):
        # Upstream's template has meson's placeholder right after <releases>.
        template = UPSTREAM.replace("<releases>", "<releases>@development-release@")
        self.assertNotIn("@development-release@",
                         nuntius_releases.merge(template, OWN))

    def test_metainfo_without_releases(self):
        with self.assertRaises(nuntius_releases.Error):
            nuntius_releases.merge("<component/>", OWN)

    def test_empty_release_list(self):
        with self.assertRaises(nuntius_releases.Error):
            nuntius_releases.merge(
                UPSTREAM, "<component>\n  <releases>\n  </releases>\n</component>")


class Repository(unittest.TestCase):
    """The files of this repository, through the command line."""

    def run_script(self, *args):
        return subprocess.run([sys.executable, str(SCRIPT), *map(str, args)],
                              capture_output=True, text=True)

    def test_version_is_semver(self):
        result = self.run_script("version", RELEASES)
        self.assertEqual(result.returncode, 0, result.stderr)
        # phosh-toolkit's release.py needs MAJOR.MINOR.PATCH.
        self.assertRegex(result.stdout, r"^\d+\.\d+\.\d+\n$")

    def test_merge_into_upstream_template(self):
        own = versions(RELEASES.read_text(encoding="utf-8"))
        with tempfile.TemporaryDirectory() as tmp:
            out = pathlib.Path(tmp) / "metainfo.xml"
            result = self.run_script("merge", METAINFO, RELEASES, out)
            self.assertEqual(result.returncode, 0, result.stderr)
            merged = out.read_text(encoding="utf-8")
        self.assertEqual(versions(merged), own)
        # The release check of the Flatpak workflow reads the first one.
        self.assertEqual(
            versions(merged)[0],
            self.run_script("version", RELEASES).stdout.strip())

    def test_usage_error(self):
        self.assertEqual(self.run_script("merge").returncode, 2)

    def test_missing_file(self):
        result = self.run_script("version", "/nonexistent.xml")
        self.assertEqual(result.returncode, 1)
        self.assertIn("nuntius-releases.py", result.stderr)


if __name__ == "__main__":
    unittest.main()
