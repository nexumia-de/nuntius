#!/usr/bin/env python3
"""Nuntius' own release list for the build (docs/decisions/version-scheme.md).

    nuntius-releases.py version RELEASES
        print the version of the newest stable release
    nuntius-releases.py merge METAINFO RELEASES OUTPUT
        write METAINFO with the content of its <releases> element replaced
        by the one of RELEASES

RELEASES is data/de.nexumia.Nuntius.releases.xml. Upstream's metainfo
template stays untouched, so merges of upstream releases do not conflict.
Standard library only: meson runs this inside the Flatpak SDK.
"""

import re
import sys
import xml.etree.ElementTree as ET

RELEASES = re.compile(r"(<releases>).*?([ \t]*</releases>)", re.S)


class Error(Exception):
    pass


def version(releases_text):
    """Version of the newest stable release; the list is newest first."""
    root = ET.fromstring(releases_text)
    for release in root.findall("releases/release"):
        if release.get("type", "stable") == "stable":
            return release.get("version")
    raise Error("the release list has no stable <release>")


def merge(metainfo_text, releases_text):
    """The metainfo with the releases of the release list."""
    ET.fromstring(releases_text)
    own = RELEASES.search(releases_text)
    if not own or "<release " not in own[0]:
        raise Error("the release list has no <release>")
    if len(RELEASES.findall(metainfo_text)) != 1:
        raise Error("the metainfo needs exactly one <releases> element")
    inner = own[0][len(own[1]):-len(own[2])].rstrip()
    return RELEASES.sub(lambda m: m[1] + inner + "\n" + m[2], metainfo_text)


def main(argv):
    try:
        if len(argv) == 3 and argv[1] == "version":
            with open(argv[2], encoding="utf-8") as f:
                print(version(f.read()))
        elif len(argv) == 5 and argv[1] == "merge":
            with open(argv[2], encoding="utf-8") as f:
                metainfo = f.read()
            with open(argv[3], encoding="utf-8") as f:
                merged = merge(metainfo, f.read())
            ET.fromstring(merged)
            with open(argv[4], "w", encoding="utf-8") as f:
                f.write(merged)
        else:
            print(__doc__, file=sys.stderr)
            return 2
    except (Error, ET.ParseError, OSError) as error:
        print(f"nuntius-releases.py: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
