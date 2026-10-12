"""m8-polish-and-ship/53, ADR-0057 amendment §11: our Homebrew formula, filled
from dist's manifest.

Usage: formula.py <dist-manifest.json> <out.rb>

Reads packaging/homebrew/mergewright.rb.in and replaces each placeholder:
  @VERSION@          the release's app_version
  @HOMEPAGE@         https://github.com/<owner>/<repo>, from the release's
                     GitHub hosting (owner and repo URL-decoded)
  @URL:<archive>@    <artifact_base_url><artifact_download_path>/<archive>
  @SHA256:<archive>@ the archive's sha256, from the manifest when dist put
                     it there, else from the `<archive>.sha256` file dist
                     wrote beside the manifest
The template's leading comment lines are dropped.

Exit 0: the formula is written. Exit 2: an archive the template names is
not in the manifest, or has no checksum; the message names its target.
Exit 1: the manifest is not one dist wrote (no release, no GitHub hosting).

Standard library only; run with `python3 -I`.
"""

import json
import os
import re
import sys
from urllib.parse import unquote

TEMPLATE = os.path.join(
    os.path.dirname(os.path.abspath(__file__)),
    "..",
    "packaging",
    "homebrew",
    "mergewright.rb.in",
)
PLACEHOLDER = re.compile(r"@(URL|SHA256):([^@]+)@")


def fail(code, message):
    print("formula.py: " + message, file=sys.stderr)
    sys.exit(code)


def the_release(manifest):
    releases = manifest.get("releases") or []
    if len(releases) != 1:
        fail(1, "expected one release in the manifest, found %d" % len(releases))
    release = releases[0]
    github = (release.get("hosting") or {}).get("github")
    if not github:
        fail(1, "the release has no GitHub hosting")
    return release, github


def target_of(archive, entry):
    triples = (entry or {}).get("target_triples") or []
    if triples:
        return triples[0]
    # Not in the manifest: the target is the archive name between the
    # package and the extension.
    return archive.split("-", 1)[-1].replace(".tar.xz", "")


def sha256_of(archive, entry, manifest_dir):
    checksums = (entry or {}).get("checksums") or {}
    if checksums.get("sha256"):
        return checksums["sha256"]
    beside = os.path.join(manifest_dir, archive + ".sha256")
    if os.path.isfile(beside):
        with open(beside, encoding="utf-8") as handle:
            words = handle.read().split()
        if words:
            return words[0]
    return None


def render(manifest_path):
    with open(manifest_path, encoding="utf-8") as handle:
        manifest = json.load(handle)
    release, github = the_release(manifest)
    artifacts = manifest.get("artifacts") or {}
    manifest_dir = os.path.dirname(os.path.abspath(manifest_path))

    with open(TEMPLATE, encoding="utf-8") as handle:
        lines = handle.read().splitlines(keepends=True)
    while lines and lines[0].startswith("#"):
        lines.pop(0)
    text = "".join(lines)

    def fill(match):
        kind, archive = match.group(1), match.group(2)
        entry = artifacts.get(archive)
        if entry is None or archive not in (release.get("artifacts") or []):
            fail(2, "no archive for target %s (%s) in the manifest" % (target_of(archive, entry), archive))
        if kind == "URL":
            return github["artifact_base_url"] + github["artifact_download_path"] + "/" + archive
        sha = sha256_of(archive, entry, manifest_dir)
        if not sha:
            fail(2, "no sha256 for target %s (%s)" % (target_of(archive, entry), archive))
        return sha

    text = PLACEHOLDER.sub(fill, text)
    text = text.replace("@VERSION@", release["app_version"])
    homepage = "https://github.com/%s/%s" % (unquote(github["owner"]), unquote(github["repo"]))
    text = text.replace("@HOMEPAGE@", homepage)
    left = re.findall(r"@[A-Z0-9]+[:@]", text)
    if left:
        fail(1, "placeholders left unfilled: %s" % ", ".join(sorted(set(left))))
    return text


def main(argv):
    if len(argv) != 3:
        print(__doc__.split("\n\n")[1], file=sys.stderr)
        return 1
    text = render(argv[1])
    with open(argv[2], "w", encoding="utf-8") as handle:
        handle.write(text)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
