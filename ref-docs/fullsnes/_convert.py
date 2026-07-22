#!/usr/bin/env python3
"""Convert nocash fullsnes.htm into a reorganized, split Markdown reference.

Faithful structural converter for the nocash XED2HTM format:
  - gray-banner tables (FONT SIZE=+2 + <A NAME>)  -> Markdown headings
  - <TABLE><TR><TD><PRE>...</TD>                   -> ```text fenced blocks (exact ASCII)
  - <B>..</B> / <BR> / <P> / <A HREF="#..">        -> **bold** / newlines / links
All 759 links are intra-document and are remapped to file#slug targets.
"""
import re, html as H, sys, os

SRC = sys.argv[1]
OUT = sys.argv[2]
FETCH_DATE = "2026-07-22"
raw = open(SRC, encoding='utf-8', errors='replace').read()

# ---- 1. Find all section headers (anchor, title, span) --------------------
HDR = re.compile(
    r'<TABLE WIDTH=100%[^>]*><TR bgcolor="#cccccc"><TD><FONT SIZE=\+2>\s*'
    r'<A NAME="([^"]+)"></A>&nbsp;\s*(.*?)\s*</FONT></TD></TR></TABLE>',
    re.S | re.I)
heads = [(m.group(1), H.unescape(re.sub(r'<[^>]+>', '', m.group(2)).strip()),
          m.start(), m.end()) for m in HDR.finditer(raw)]
print(f"sections: {len(heads)}", file=sys.stderr)

# body of section i = raw[end_i : start_{i+1}]
sections = []
for i, (anc, title, s, e) in enumerate(heads):
    body_end = heads[i + 1][2] if i + 1 < len(heads) else raw.find('</BODY>')
    if body_end < 0:
        body_end = len(raw)
    sections.append({'anchor': anc, 'title': title, 'body': raw[e:body_end]})

# ---- 2. File assignment ---------------------------------------------------
# ordered boundaries: (start_anchor, file_key, file_title)
BOUNDS = [
    ('snesiomap',                     '10-memory-and-io-map',  'Memory Map & I/O Map'),
    ('snesdmatransfers',              '20-dma-hdma',           'DMA & HDMA Transfers'),
    ('snespictureprocessingunitppu',  '30-ppu',                'Picture Processing Unit (PPU)'),
    ('snesaudioprocessingunitapu',    '40-apu-dsp',            'Audio Processing Unit (APU / SPC700 / S-DSP) & Maths'),
    ('snescontrollers',               '50-controllers',        'Controllers & Input Peripherals'),
    ('snescartridges',                '60-cartridge-header-and-mapping', 'Cartridge Header, PCBs, CIC & Memory Mapping'),
    ('snescartsa1programmable65c816cpuakasuperaccelerator35games',
                                      '61-coprocessors',       'Cartridge Coprocessors (SA-1, Super FX, CX4, DSP, ST018/ARM, OBC1, S-DD1, SPC7110, S-RTC)'),
    ('snescartsupergameboy',          '62-cartridge-addons-satellaview-modems',
                                                               'Cartridge Add-Ons (Super Game Boy, Satellaview, Data Pack, Nintendo Power, Sufami Turbo, X-Band Modem)'),
    ('snescartflashbackup',           '63-copiers-cheat-devices-cdrom',
                                                               'FLASH Backup, Cheat Devices, Tri-Star, Pirate Multicarts, Copiers & CD-ROM Drive'),
    ('sneshotelboxesandarcademachines','70-hotel-arcade-nss-sfcbox',
                                                               'Hotel Boxes & Arcade Machines (NSS, SFC-Box, Z80, HD64180) & Decompression Formats'),
    ('snesunpredictablethings',       '80-timings-unpredictable-pinouts',
                                                               'Unpredictable Things, Timings, Pinouts, Chipset & Mods'),
    ('cpu65xxmicroprocessor',         '90-cpu-65816',          'CPU 65XX / 65C816 Microprocessor Reference'),
    ('aboutcredits',                  '99-about-and-index',    'About / Credits & Index'),
]
bound_at = {b[0]: b for b in BOUNDS}
cur = None
file_order = []
for sec in sections:
    if sec['anchor'] in bound_at:
        cur = bound_at[sec['anchor']]
        file_order.append(cur[1])
    sec['file'] = cur[1]
    sec['file_title'] = cur[2]

# ---- 3. Heading level + slug ---------------------------------------------
def slugify(t):
    s = t.lower()
    s = s.replace('&', '')                 # GitHub drops & entirely
    s = re.sub(r'[^a-z0-9 \-/]', '', s)
    s = s.replace('/', '')
    s = re.sub(r'\s+', '-', s.strip())
    s = re.sub(r'-+', '-', s)
    return s

# per-file slug dedup + level assignment
file_slugs = {}          # file -> set of used slugs
anchor_map = {}          # orig anchor -> (file, slug)
last_h2 = {}             # file -> last level-2 title
for sec in sections:
    f = sec['file']
    used = file_slugs.setdefault(f, {})
    base = slugify(sec['title'])
    slug = base
    n = 1
    while slug in used:
        slug = f"{base}-{n}"; n += 1
    used[slug] = True
    sec['slug'] = slug
    anchor_map[sec['anchor']] = (f, slug)
    # level: 3 if strict prefix-child of last H2 in same file, else 2
    prev = last_h2.get(f)
    if prev and sec['title'].startswith(prev + ' '):
        sec['level'] = 3
    else:
        sec['level'] = 2
        last_h2[f] = sec['title']

anchor_map['contents'] = ('00-index', None)
anchor_map['index'] = anchor_map.get('index', ('99-about-and-index', 'index'))

# ---- 4. Body HTML -> Markdown --------------------------------------------
TABLE = re.compile(r'<TABLE[^>]*><TR><TD><PRE>(.*?)</TD>\s*</TR>\s*</TABLE>', re.S | re.I)

unresolved = set()

def fix_links(text, cur_file):
    def repl(m):
        tgt, label = m.group(1), m.group(2)
        label = re.sub(r'<[^>]+>', '', label)
        label = H.unescape(label).strip()
        if tgt in anchor_map:
            f, slug = anchor_map[tgt]
            if slug is None:
                dest = '00-index.md'
            elif f == cur_file:
                dest = f'#{slug}'
            else:
                dest = f'{f}.md#{slug}'
            return f'[{label}]({dest})'
        unresolved.add(tgt)
        return label
    return re.sub(r'<A HREF="#([^"]*)">(.*?)</A>', repl, text, flags=re.S | re.I)

def conv_text(seg, cur_file):
    seg = fix_links(seg, cur_file)
    seg = re.sub(r'<B>(.*?)</B>', lambda m: '**' + re.sub(r'<[^>]+>', '', m.group(1)).strip() + '**',
                 seg, flags=re.S | re.I)
    seg = re.sub(r'<P>', '\n\n', seg, flags=re.I)
    seg = re.sub(r'<BR>', '\n', seg, flags=re.I)
    seg = re.sub(r'<[^>]+>', '', seg)          # drop any stray real tags
    seg = H.unescape(seg)                       # -> fully literal text
    # Re-escape angle brackets/ampersands for Markdown PROSE so nocash's literal
    # <imm>, <input>, <output> notation is not swallowed as HTML. Markdown link
    # and bold syntax already inserted above contain none of & < >, so they survive.
    seg = seg.replace('&', '&amp;').replace('<', '&lt;').replace('>', '&gt;')
    seg = re.sub(r'[ \t]+\n', '\n', seg)       # trailing ws
    seg = re.sub(r'\n{3,}', '\n\n', seg)
    return seg.strip()

def conv_fence(inner):
    inner = re.sub(r'</?B>', '', inner, flags=re.I)   # strip decorative bold
    inner = re.sub(r'<[^>]+>', '', inner)
    inner = H.unescape(inner)
    inner = inner.strip('\n')
    inner = re.sub(r'[ \t]+\n', '\n', inner)          # kill trailing ws only
    fence = '```'
    if '```' in inner:
        fence = '~~~'
    return f'{fence}text\n{inner}\n{fence}'

def conv_body(body, cur_file):
    out = []
    pos = 0
    for m in TABLE.finditer(body):
        pre = body[pos:m.start()]
        t = conv_text(pre, cur_file)
        if t:
            out.append(t)
        out.append(conv_fence(m.group(1)))
        pos = m.end()
    tail = conv_text(body[pos:], cur_file)
    if tail:
        out.append(tail)
    return '\n\n'.join(out).strip()

# ---- 4b. Enhancement notes (clearly-marked supplements, not edits) --------
NOTES = {
 'snesmathsmultiplydivide':
   "> **Note (RustySNES ref):** Both units deliver their result *progressively*, "
   "so an emulator that must pass cycle-accurate tests should model the delay, not "
   "just the final value. The multiply (write to `WRMPYB` $4203) completes 8 CPU "
   "cycles after the write, and reading `RDMPYL/H` earlier returns a valid "
   "*intermediate* product; the result is valid one cycle earlier per leading zero "
   "bit in `WRMPYA`. The divide (write to `WRDIVB` $4206) takes up to 16 CPU cycles, "
   "with the quotient (`RDDIV`) and remainder (`RDMPY`) updated as it runs. "
   "See [SNESdev Wiki: Multiplication](https://snes.nesdev.org/wiki/Multiplication) "
   "and [Division](https://snes.nesdev.org/wiki/Division).",
 'snesppucontrol':
   "> **Note (RustySNES ref):** On the open question in the text about *when* forced "
   "blank takes effect: the practical, test-ROM-verified rule is that the CPU may "
   "freely access VRAM and OAM during Vblank **or** force-blank, and CGRAM during "
   "Vblank, Hblank **or** force-blank. Accesses outside those windows are dropped "
   "(see the VRAM/CGRAM notes below). "
   "See [SNESdev Wiki: Reading and writing PPU memory](https://snes.nesdev.org/wiki/Reading_and_writing_PPU_memory).",
 'snesppuvideomemoryvram':
   "> **Note (RustySNES ref):** A VRAM write attempted during active display is "
   "*silently ignored* — the VRAM address still increments per the `VMAIN` ($2115) "
   "setting, but no data is stored. Emulators must reproduce the dropped write plus "
   "the address increment, not skip the access entirely. "
   "See [SNESdev Wiki: Reading and writing PPU memory](https://snes.nesdev.org/wiki/Reading_and_writing_PPU_memory).",
 'snesppucolorpalettememorycgramanddirectcolors':
   "> **Note (RustySNES ref):** Bit 15 of a 16-bit CGRAM color read is open bus (the "
   "MDR), not a defined 0, and should be masked. CGRAM is only reliably writable "
   "during Vblank/Hblank/force-blank; a write during active display lands at the "
   "wrong CGRAM address. "
   "See [SNESdev Wiki: PPU registers](https://snes.nesdev.org/wiki/PPU_registers).",
 'snestimings':
   "> **Note (RustySNES ref):** For an authoritative, independently-cross-checked "
   "account of dot/scanline counts, the 5A22 memory-access cycle map, and the exact "
   "H/V-counter latch behavior used to validate accuracy work, cross-reference "
   "[SNESdev Wiki: Timing](https://snes.nesdev.org/wiki/Timing) alongside the numbers "
   "below.",
}
for sec in sections:
    sec['note'] = NOTES.get(sec['anchor'])

for sec in sections:
    sec['md'] = conv_body(sec['body'], sec['file'])

# ---- 5. Emit files --------------------------------------------------------
os.makedirs(OUT, exist_ok=True)
SRC_URL = 'https://problemkaputt.de/fullsnes.htm'

HEADER_NOTE = (
    f"> **Reformatted mirror for RustySNES development reference.** This is nocash's "
    f"*Fullsnes — Nocash SNES Specs*, converted from the [official HTML]({SRC_URL}) "
    f"(fetched {FETCH_DATE}) into navigable Markdown. Content is reproduced faithfully; "
    f"only structure (headings, a table of contents, fenced code blocks, cross-file links) "
    f"was added. Register/timing tables are preserved verbatim as `text` code fences so that "
    f"column alignment, hex notation, and bit-field layouts are never mangled.\n>\n"
    f"> **Original copyright 2012 by Martin Korth (nocash)**; this document is part of the "
    f"no$sns emulator/debugger help text. PPU and APU specs are largely based on Anomie's "
    f"specs. See [About / Credits](99-about-and-index.md#aboutcredits) for the full "
    f"attribution. Updates: <{SRC_URL}> (HTML) and <https://problemkaputt.de/fullsnes.txt> (text).\n>\n"
    f"> Additions made for this mirror are always fenced as `> **Note (RustySNES ref):**` "
    f"blockquotes and never alter nocash's text."
)

# group files by their file_title, in order
files = []
seen = set()
for sec in sections:
    if sec['file'] not in seen:
        seen.add(sec['file'])
        files.append((sec['file'], sec['file_title']))

secs_by_file = {}
for sec in sections:
    secs_by_file.setdefault(sec['file'], []).append(sec)

def emit_section(sec):
    h = '#' * sec['level']
    parts = [f'<a id="{sec["anchor"]}"></a>\n\n{h} {sec["title"]}']
    if sec.get('note'):
        parts.append(sec['note'])
    if sec['md']:
        parts.append(sec['md'])
    return '\n\n'.join(parts)

# per-file pages
for fkey, ftitle in files:
    fsecs = secs_by_file[fkey]
    idx = [f[0] for f in files].index(fkey)
    prev_f = files[idx - 1] if idx > 0 else None
    next_f = files[idx + 1] if idx + 1 < len(files) else None
    lines = [f'# Fullsnes — {ftitle}', '']
    nav = ['[Index](00-index.md)']
    if prev_f:
        nav.append(f'[« {prev_f[1].split("(")[0].strip()}]({prev_f[0]}.md)')
    if next_f:
        nav.append(f'[{next_f[1].split("(")[0].strip()} »]({next_f[0]}.md)')
    lines.append(' · '.join(nav))
    lines.append('')
    # local TOC
    lines.append('**Sections in this file:**')
    lines.append('')
    for sec in fsecs:
        indent = '  ' * (sec['level'] - 2)
        lines.append(f'{indent}- [{sec["title"]}](#{sec["slug"]})')
    lines.append('')
    lines.append('---')
    lines.append('')
    for sec in fsecs:
        lines.append(emit_section(sec))
        lines.append('')
    open(os.path.join(OUT, fkey + '.md'), 'w', encoding='utf-8').write('\n'.join(lines).rstrip() + '\n')

# master index
il = ['# Fullsnes — Nocash SNES Specs (RustySNES Reference Mirror)', '',
      HEADER_NOTE, '',
      f'- **Source:** <{SRC_URL}>',
      f'- **Fetched:** {FETCH_DATE}',
      f'- **Author:** Martin Korth (nocash), 2012',
      f'- **Sections:** {len(sections)} across {len(files)} section files (plus this index)',
      '',
      '## Why this is split into multiple files', '',
      'The original is one flat ~1.5 MB HTML page. Converted to Markdown it exceeds 400 KB, '
      'so it is split into logical section files (below). This index is the linked master '
      'table of contents. Every heading carries an HTML `id` matching nocash\'s original anchor '
      'name, so old `#anchor` cross-references resolve, and a GitHub-style slug for normal '
      'Markdown linking.', '',
      '## Table of Contents', '']
for fkey, ftitle in files:
    il.append(f'### [{ftitle}]({fkey}.md)')
    il.append('')
    for sec in secs_by_file[fkey]:
        indent = '  ' * (sec['level'] - 2)
        il.append(f'{indent}- [{sec["title"]}]({fkey}.md#{sec["slug"]})')
    il.append('')
open(os.path.join(OUT, '00-index.md'), 'w', encoding='utf-8').write('\n'.join(il).rstrip() + '\n')

print("unresolved links:", sorted(unresolved), file=sys.stderr)
print("files written:", len(files) + 1, file=sys.stderr)
