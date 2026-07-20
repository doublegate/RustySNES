#!/usr/bin/env python3
"""
snesdev_wiki_mirror.py — mirror the SNESdev Wiki into a local, offline,
agent-readable corpus at ``snesdev_wiki/``.

Background
==========

RustySNES's sibling project RustyNES keeps a gitignored ``nesdev_wiki/``
mirror: crawled ``.xhtml`` pages alongside an ``output/`` directory of
Markdown conversions, plus the images the pages reference. The Markdown
is the part that actually gets read day to day — it is what makes the
hardware reference greppable without a browser or a network round trip.

This script builds the equivalent for <https://snes.nesdev.org/wiki/>.
It does not crawl HTML: the wiki exposes a MediaWiki API, so pages are
enumerated and fetched through it, which is faster, more polite, and
gives us the wikitext as well as the rendered HTML.

The upstream wiki is small — roughly 195 pages and 32 images against
NESdev's ~2,800 — so a full mirror is quick and re-runnable.

Layout produced
===============

::

    snesdev_wiki/
        INDEX.md                 generated table of contents, by namespace
        html/<Title>.xhtml       rendered HTML, one file per page
        wikitext/<Title>.wiki    raw wikitext (the most faithful source form)
        output/<Title>.md        Markdown conversion  <- the useful one
        images/<name>            every image the pages reference

Filenames use RustyNES's transform: every character outside
``[A-Za-z0-9_]`` becomes ``_``, so ``Memory map`` becomes ``Memory_map``
and internal links can be rewritten mechanically.

Licensing
=========

The SNESdev Wiki's text is contributor-licensed; this mirror is a local
reference copy only. ``snesdev_wiki/`` is **gitignored** and must never
be committed or vendored into the MIT/Apache tree — exactly the posture
RustyNES takes with ``nesdev_wiki/``.

Usage
=====

::

    python3 scripts/snesdev_wiki_mirror.py              # full mirror
    python3 scripts/snesdev_wiki_mirror.py --dry-run    # list what would be fetched
    python3 scripts/snesdev_wiki_mirror.py --no-images  # skip image download
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import time
import urllib.parse
import urllib.request
from pathlib import Path

API = "https://snes.nesdev.org/w/api.php"
WIKI = "https://snes.nesdev.org/wiki/"
UA = "RustySNES-wiki-mirror/1.0 (https://github.com/doublegate/RustySNES)"

# Namespaces worth mirroring: 0 = articles, 6 = File pages, 14 = Category,
# 10 = Template (a few register tables live there), 4 = project pages.
NAMESPACES = [0, 4, 6, 10, 14]

ROOT = Path(__file__).resolve().parent.parent / "snesdev_wiki"

# Be a good citizen: the wiki is small and volunteer-hosted.
DELAY = 0.25


def api(**params) -> dict:
    params.setdefault("format", "json")
    url = f"{API}?{urllib.parse.urlencode(params)}"
    req = urllib.request.Request(url, headers={"User-Agent": UA})
    with urllib.request.urlopen(req, timeout=60) as r:
        return json.load(r)


def fetch_bytes(url: str) -> bytes:
    req = urllib.request.Request(url, headers={"User-Agent": UA})
    with urllib.request.urlopen(req, timeout=120) as r:
        return r.read()


def safe_name(title: str) -> str:
    """RustyNES's crawler transform: non-[A-Za-z0-9_] becomes '_'."""
    return re.sub(r"[^A-Za-z0-9_]", "_", title)


def list_pages() -> list[dict]:
    pages: list[dict] = []
    for ns in NAMESPACES:
        cont: dict = {}
        while True:
            d = api(
                action="query",
                list="allpages",
                apnamespace=ns,
                aplimit="500",
                **cont,
            )
            pages.extend(d["query"]["allpages"])
            if "continue" not in d:
                break
            cont = d["continue"]
            time.sleep(DELAY)
    return pages


def to_markdown(html: str, title: str, titles: set[str]) -> str:
    from bs4 import BeautifulSoup
    from markdownify import markdownify

    soup = BeautifulSoup(html, "html.parser")

    # Drop wiki chrome that carries no reference value.
    for sel in [".mw-editsection", ".navbox", "#toc", ".toc", ".printfooter", ".catlinks"]:
        for el in soup.select(sel):
            el.decompose()

    # Rewrite internal links to the local Markdown copies so the corpus is
    # navigable offline; leave everything else pointing upstream.
    for a in soup.find_all("a", href=True):
        href = a["href"]
        if href.startswith("/wiki/"):
            target = urllib.parse.unquote(href[len("/wiki/") :]).split("#")[0]
            # URLs use underscores where titles use spaces; compare on the title form.
            target = target.replace("_", " ")
            frag = href.split("#", 1)[1] if "#" in href else ""
            if target in titles:
                a["href"] = safe_name(target) + ".md" + (f"#{frag}" if frag else "")
            else:
                a["href"] = WIKI + urllib.parse.quote(target.replace(" ", "_"))
        elif href.startswith("/"):
            a["href"] = "https://snes.nesdev.org" + href

    # Point images at the local copies.
    for img in soup.find_all("img", src=True):
        img["src"] = "../images/" + Path(urllib.parse.unquote(img["src"])).name

    body = markdownify(str(soup), heading_style="ATX", strip=["script", "style"])
    body = re.sub(r"\n{3,}", "\n\n", body).strip()
    return f"# {title}\n\n*Mirrored from <{WIKI}{urllib.parse.quote(title)}>*\n\n{body}\n"


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--dry-run", action="store_true", help="list pages, fetch nothing")
    ap.add_argument("--no-images", action="store_true", help="skip image download")
    args = ap.parse_args()

    print(f"enumerating {API} ...", flush=True)
    pages = list_pages()
    titles = {p["title"] for p in pages}
    print(f"{len(pages)} pages across namespaces {NAMESPACES}")

    if args.dry_run:
        for p in sorted(pages, key=lambda p: p["title"]):
            print(f"  [{p['ns']}] {p['title']}")
        return 0

    for sub in ("html", "wikitext", "output", "images"):
        (ROOT / sub).mkdir(parents=True, exist_ok=True)

    written = 0
    failed: list[str] = []
    for i, p in enumerate(sorted(pages, key=lambda p: p["title"]), 1):
        title = p["title"]
        name = safe_name(title)
        try:
            d = api(action="parse", page=title, prop="text|wikitext", redirects=1)
            if "error" in d:
                failed.append(f"{title}: {d['error'].get('info', 'unknown')}")
                continue
            html = d["parse"]["text"]["*"]
            wikitext = d["parse"]["wikitext"]["*"]

            (ROOT / "html" / f"{name}.xhtml").write_text(html, encoding="utf-8")
            (ROOT / "wikitext" / f"{name}.wiki").write_text(wikitext, encoding="utf-8")
            (ROOT / "output" / f"{name}.md").write_text(
                to_markdown(html, title, titles), encoding="utf-8"
            )
            written += 1
            print(f"  [{i}/{len(pages)}] {title}", flush=True)
        except Exception as e:  # noqa: BLE001 - a mirror should not abort on one bad page
            failed.append(f"{title}: {e}")
        time.sleep(DELAY)

    images = 0
    if not args.no_images:
        cont: dict = {}
        while True:
            d = api(action="query", list="allimages", ailimit="500", **cont)
            for im in d["query"]["allimages"]:
                dest = ROOT / "images" / im["name"]
                try:
                    if not dest.exists():
                        dest.write_bytes(fetch_bytes(im["url"]))
                    images += 1
                except Exception as e:  # noqa: BLE001
                    failed.append(f"image {im['name']}: {e}")
                time.sleep(DELAY)
            if "continue" not in d:
                break
            cont = d["continue"]

    # Index, grouped by namespace.
    by_ns: dict[int, list[str]] = {}
    for p in pages:
        by_ns.setdefault(p["ns"], []).append(p["title"])
    ns_names = {0: "Articles", 4: "Project", 6: "Files", 10: "Templates", 14: "Categories"}
    lines = [
        "# SNESdev Wiki — local mirror",
        "",
        f"Mirrored from <{WIKI}> by `scripts/snesdev_wiki_mirror.py`.",
        "",
        "`output/*.md` is the readable form; `wikitext/*.wiki` is the most faithful source;",
        "`html/*.xhtml` is the rendered page. **This directory is gitignored** — it is a local",
        "reference copy, not vendored content.",
        "",
    ]
    for ns in sorted(by_ns):
        lines += [f"## {ns_names.get(ns, f'Namespace {ns}')}", ""]
        for t in sorted(by_ns[ns]):
            lines.append(f"- [{t}](output/{safe_name(t)}.md)")
        lines.append("")
    (ROOT / "INDEX.md").write_text("\n".join(lines), encoding="utf-8")

    print(f"\nwrote {written} pages, {images} images to {ROOT}")
    if failed:
        print(f"{len(failed)} failure(s):", file=sys.stderr)
        for f in failed[:20]:
            print(f"  {f}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
