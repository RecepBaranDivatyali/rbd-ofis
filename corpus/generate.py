#!/usr/bin/env python3
"""
RBD OFIS Test Corpus - Synthetic Edge-Case PDF Generator.
AGPL-3.0-or-later. Standard library only (no external dependencies).

Generates:
1. large-generated-2000-page.pdf (pdf-large): 2000-page valid PDF to test engine scalability.
2. corrupt-generated-truncated.pdf (pdf-corrupt): PDF abruptly truncated mid-stream without trailer/EOF.
3. corrupt-generated-broken-xref.pdf (pdf-corrupt): PDF with corrupted xref table entries and bogus startxref offset.
4. tr-generated-standard-fonts.pdf (pdf-cjk-rtl-turkish): PDF with Turkish characters (İıŞşĞğÇçÖöÜü) using
   standard Type 1 Helvetica, custom /Differences array, corrected octal escapes, and a valid /ToUnicode CMap stream.
"""

import argparse
import hashlib
import io
import os
import re
import sys
from pathlib import Path


def validate_path_safety(category: str, filename: str, base_dir: Path) -> Path:
    """Validate path against CWE-22 Path Traversal and return resolved safe Path."""
    if not re.match(r"^[a-z0-9_-]+$", category):
        raise ValueError(f"Insecure category identifier: {category!r}")
    if not re.match(r"^[a-zA-Z0-9_.-]+$", filename) or ".." in filename:
        raise ValueError(f"Insecure filename: {filename!r}")
    target_dir = base_dir / category
    target_dir.mkdir(parents=True, exist_ok=True)
    target_path = (target_dir / filename).resolve()
    base_resolved = base_dir.resolve()
    if not target_path.is_relative_to(base_resolved):
        raise ValueError(f"Path traversal detected: {target_path} is outside base {base_resolved}")
    return target_path


def generate_2000_page_pdf() -> bytes:
    """Generate a valid, fully conformant 2000-page PDF."""
    buf = io.BytesIO()
    buf.write(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n")
    page_count = 2000
    offsets = {}

    # Obj 1: Catalog
    offsets[1] = buf.tell()
    buf.write(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n")

    # Obj 2: Pages root
    offsets[2] = buf.tell()
    kids_str = " ".join(f"{i} 0 R" for i in range(5, page_count + 5))
    buf.write(f"2 0 obj\n<< /Type /Pages /Kids [{kids_str}] /Count {page_count} >>\nendobj\n".encode("ascii"))

    # Obj 3: Font
    offsets[3] = buf.tell()
    buf.write(b"3 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n")

    # Obj 4: Content stream shared across pages
    offsets[4] = buf.tell()
    content = (
        b"BT\n"
        b"/F1 20 Tf\n"
        b"50 740 Td\n"
        b"(RBD OFIS - 2000-Page Stress Test Document) Tj\n"
        b"0 -30 Td\n"
        b"/F1 12 Tf\n"
        b"(This page is generated synthetically to verify engine scalability.) Tj\n"
        b"ET\n"
    )
    buf.write(f"4 0 obj\n<< /Length {len(content)} >>\nstream\n".encode("ascii") + content + b"\nendstream\nendobj\n")

    # Obj 5..page_count+4: Page objects
    for i in range(5, page_count + 5):
        offsets[i] = buf.tell()
        page_obj = (
            f"{i} 0 obj\n"
            f"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
            f"/Contents 4 0 R /Resources << /Font << /F1 3 0 R >> >> >>\n"
            f"endobj\n"
        )
        buf.write(page_obj.encode("ascii"))

    total_objs = page_count + 4
    xref_offset = buf.tell()
    buf.write(f"xref\n0 {total_objs + 1}\n0000000000 65535 f \n".encode("ascii"))
    for i in range(1, total_objs + 1):
        buf.write(f"{offsets[i]:010d} 00000 n \n".encode("ascii"))
    buf.write(
        f"trailer\n<< /Size {total_objs + 1} /Root 1 0 R >>\n"
        f"startxref\n{xref_offset}\n%%EOF\n".encode("ascii")
    )
    return buf.getvalue()


def generate_turkish_pdf() -> bytes:
    """Generate a valid PDF containing Turkish characters with standard Type 1 Helvetica font.
    
    Mapping of character codes 192..203 (octal \\300..\\313, hex 0xC0..0xCB):
      192 (0xC0, \\300): /Idotaccent -> U+0130 (İ)
      193 (0xC1, \\301): /dotlessi   -> U+0131 (ı)
      194 (0xC2, \\302): /Scedilla   -> U+015E (Ş)
      195 (0xC3, \\303): /scedilla   -> U+015F (ş)
      196 (0xC4, \\304): /Gbreve     -> U+011E (Ğ)
      197 (0xC5, \\305): /gbreve     -> U+011F (ğ)
      198 (0xC6, \\306): /Ccedilla   -> U+00C7 (Ç)
      199 (0xC7, \\307): /ccedilla   -> U+00E7 (ç)
      200 (0xC8, \\310): /Odieresis  -> U+00D6 (Ö)
      201 (0xC9, \\311): /odieresis  -> U+00F6 (ö)
      202 (0xCA, \\312): /Udieresis  -> U+00DC (Ü)
      203 (0xCB, \\313): /udieresis  -> U+00FC (ü)
    
    Includes a /ToUnicode CMap stream so text extraction engines (MuPDF, PDFium, lopdf)
    cleanly extract UTF-8 text.
    """
    buf = io.BytesIO()
    buf.write(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n")
    offsets = {}

    # Obj 1: Catalog
    offsets[1] = buf.tell()
    buf.write(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n")

    # Obj 2: Pages root
    offsets[2] = buf.tell()
    buf.write(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n")

    # Obj 3: Page 1
    offsets[3] = buf.tell()
    buf.write(
        b"3 0 obj\n"
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
        b"/Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>\n"
        b"endobj\n"
    )

    # Obj 4: Content stream with verified octal escapes
    stream_content = (
        b"BT\n"
        b"/F1 20 Tf\n"
        b"50 730 Td\n"
        b"(RBD OFIS - Turkce Standart Yazi Tipi Testi) Tj\n"
        b"0 -35 Td\n"
        b"/F1 16 Tf\n"
        b"(Turkce Harfler: \\300\\301 \\302\\303 \\304\\305 \\306\\307 \\310\\311 \\312\\313) Tj\n"
        b"0 -30 Td\n"
        b"/F1 13 Tf\n"
        b"(Pangram 1: Pijamal\\301 hasta, ya\\305\\301z \\303of\\311re \\307abucak g\\313vendi.) Tj\n"
        b"0 -25 Td\n"
        b"(Pangram 2: \\302a\\303\\301rt\\301c\\301 ve fevkalade \\307abuklu\\305uyla g\\311\\305e y\\313kseldi.) Tj\n"
        b"0 -25 Td\n"
        b"(Tekerleme: \\302emsi Pa\\303a pasaj\\301nda sesi bo\\305uk \\307avu\\303...) Tj\n"
        b"0 -35 Td\n"
        b"/F1 11 Tf\n"
        b"(Font: Type 1 Helvetica with WinAnsiEncoding Differences array and ToUnicode CMap) Tj\n"
        b"0 -20 Td\n"
        b"(Glyphs: Idotaccent dotlessi Scedilla scedilla Gbreve gbreve Ccedilla ccedilla Odieresis odieresis Udieresis udieresis) Tj\n"
        b"ET\n"
    )
    offsets[4] = buf.tell()
    buf.write(f"4 0 obj\n<< /Length {len(stream_content)} >>\nstream\n".encode("ascii") + stream_content + b"\nendstream\nendobj\n")

    # Obj 5: Font object referencing Encoding 6 0 R and ToUnicode 7 0 R
    offsets[5] = buf.tell()
    buf.write(b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding 6 0 R /ToUnicode 7 0 R >>\nendobj\n")

    # Obj 6: Encoding dictionary with Differences array
    offsets[6] = buf.tell()
    differences = (
        b"6 0 obj\n"
        b"<< /Type /Encoding /BaseEncoding /WinAnsiEncoding\n"
        b"/Differences [\n"
        b"  192 /Idotaccent /dotlessi /Scedilla /scedilla /Gbreve /gbreve /Ccedilla /ccedilla /Odieresis /odieresis /Udieresis /udieresis\n"
        b"] >>\n"
        b"endobj\n"
    )
    buf.write(differences)

    # Obj 7: ToUnicode CMap stream mapping codes 192..203 (0xC0..0xCB) to UTF-16BE Unicode
    cmap_data = (
        b"/CIDInit /ProcSet findresource begin\n"
        b"12 dict begin\n"
        b"begincmap\n"
        b"/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n"
        b"/CMapName /Custom-ToUnicode def\n"
        b"/CMapType 2 def\n"
        b"1 begincodespacerange\n"
        b"<00> <FF>\n"
        b"endcodespacerange\n"
        b"12 beginbfrange\n"
        b"<C0> <C0> <0130>\n"
        b"<C1> <C1> <0131>\n"
        b"<C2> <C2> <015E>\n"
        b"<C3> <C3> <015F>\n"
        b"<C4> <C4> <011E>\n"
        b"<C5> <C5> <011F>\n"
        b"<C6> <C6> <00C7>\n"
        b"<C7> <C7> <00E7>\n"
        b"<C8> <C8> <00D6>\n"
        b"<C9> <C9> <00F6>\n"
        b"<CA> <CA> <00DC>\n"
        b"<CB> <CB> <00FC>\n"
        b"endbfrange\n"
        b"endcmap\n"
        b"CMapName currentdict /CMap defineresource pop\n"
        b"end\n"
        b"end\n"
    )
    offsets[7] = buf.tell()
    buf.write(f"7 0 obj\n<< /Length {len(cmap_data)} >>\nstream\n".encode("ascii") + cmap_data + b"\nendstream\nendobj\n")

    total_objs = 7
    xref_offset = buf.tell()
    buf.write(f"xref\n0 {total_objs + 1}\n0000000000 65535 f \n".encode("ascii"))
    for i in range(1, total_objs + 1):
        buf.write(f"{offsets[i]:010d} 00000 n \n".encode("ascii"))
    buf.write(
        f"trailer\n<< /Size {total_objs + 1} /Root 1 0 R >>\n"
        f"startxref\n{xref_offset}\n%%EOF\n".encode("ascii")
    )
    return buf.getvalue()


def generate_truncated_pdf() -> bytes:
    """Generate a truncated PDF: a valid PDF cut off mid-stream with no trailer or EOF."""
    valid_pdf = generate_turkish_pdf()
    cutoff = valid_pdf.find(b"stream\n") + 120
    return valid_pdf[:cutoff]


def generate_broken_xref_pdf() -> bytes:
    """Generate a PDF with corrupted xref table and invalid startxref offset."""
    buf = io.BytesIO()
    buf.write(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n")
    offsets = {}

    offsets[1] = buf.tell()
    buf.write(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n")

    offsets[2] = buf.tell()
    buf.write(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n")

    offsets[3] = buf.tell()
    buf.write(
        b"3 0 obj\n"
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
        b"/Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>\n"
        b"endobj\n"
    )

    stream_content = b"BT /F1 16 Tf 50 700 Td (RBD OFIS Broken XREF Test Case) Tj ET"
    offsets[4] = buf.tell()
    buf.write(f"4 0 obj\n<< /Length {len(stream_content)} >>\nstream\n".encode("ascii") + stream_content + b"\nendstream\nendobj\n")

    offsets[5] = buf.tell()
    buf.write(b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n")

    total_objs = 5
    buf.write(f"xref\n0 {total_objs + 1}\n".encode("ascii"))
    buf.write(b"0000000000 65535 f \n")
    buf.write(b"CORRUPTED_OFFSET_A n \n")
    buf.write(b"9999999999 00000 n \n")
    buf.write(b"-000000001 00000 f \n")
    buf.write(b"0000000000 99999 ? \n")
    buf.write(b"BAD_ENTRY_RECORD!! \n")
    buf.write(
        f"trailer\n<< /Size {total_objs + 1} /Root 1 0 R >>\n"
        f"startxref\n999999999\n%%EOF\n".encode("ascii")
    )
    return buf.getvalue()


# Canonical mapping: unique canonical filename, category, and generator function
GENERATORS = {
    "large-generated-2000-page.pdf": {
        "category": "pdf-large",
        "id": "large-generated-2000-page",
        "func": generate_2000_page_pdf,
        "description": "Synthetically generated 2000-page valid PDF to test large document handling",
    },
    "corrupt-generated-truncated.pdf": {
        "category": "pdf-corrupt",
        "id": "corrupt-generated-truncated",
        "func": generate_truncated_pdf,
        "description": "Synthetically generated truncated PDF (cut off mid-stream, no trailer or EOF)",
    },
    "corrupt-generated-broken-xref.pdf": {
        "category": "pdf-corrupt",
        "id": "corrupt-generated-broken-xref",
        "func": generate_broken_xref_pdf,
        "description": "Synthetically generated PDF with corrupted xref table entries and bogus startxref offset",
    },
    "tr-generated-standard-fonts.pdf": {
        "category": "pdf-cjk-rtl-turkish",
        "id": "tr-generated-standard-fonts",
        "func": generate_turkish_pdf,
        "description": "Synthetically generated PDF with Turkish characters (İıŞşĞğÇçÖöÜü) using standard Helvetica, Differences array, and /ToUnicode CMap",
    },
}


def generate_all(base_dir: Path) -> dict:
    """Generate all synthetic files at their single canonical category paths."""
    results = {}
    for filename, meta in GENERATORS.items():
        data = meta["func"]()
        sha256 = hashlib.sha256(data).hexdigest().lower()
        size_bytes = len(data)
        category = meta["category"]

        target_path = validate_path_safety(category, filename, base_dir)
        target_path.write_bytes(data)

        results[filename] = {
            "category": category,
            "id": meta["id"],
            "path": str(target_path),
            "sha256": sha256,
            "size_bytes": size_bytes,
            "description": meta["description"],
        }
        print(f"Generated {filename} -> {target_path} ({size_bytes:,} bytes, sha256: {sha256[:12]}...)")

    return results


def main():
    parser = argparse.ArgumentParser(description="Generate synthetic PDF edge cases for RBD OFIS")
    parser.add_argument("--output-dir", default="corpus/files", help="Base output directory (default: corpus/files)")
    args = parser.parse_args()

    base_dir = Path(args.output_dir)
    print(f"Generating synthetic edge cases into: {base_dir}")
    results = generate_all(base_dir)
    print(f"\nSuccessfully generated {len(results)} canonical files.")


if __name__ == "__main__":
    main()
