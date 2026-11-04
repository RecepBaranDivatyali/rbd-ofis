#!/usr/bin/env python3
"""
RBD OFIS Test Corpus Manifest Builder.
Fetches, validates, and records sha256 and size_bytes for all test files.
Outputs corpus/manifest.json according to AGENTS.md, ADR-0003, and Phase 0 requirements.
"""

import hashlib
import json
import os
import re
import sys
import time
import urllib.parse
import urllib.request
from pathlib import Path

ROOT_DIR = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT_DIR))

from corpus.generate import GENERATORS, validate_path_safety


def verify_magic_bytes(data: bytes, category: str, filename: str) -> None:
    """Verify document format magic bytes and strictly forbid HTML response pages."""
    data_strip = data.lstrip()
    if data_strip.startswith(b"<!DOCTYPE html") or data_strip.startswith(b"<html"):
        raise ValueError(f"File {filename} is an HTML page, not a valid document!")

    ext = Path(filename).suffix.lower()
    if ext == ".pdf":
        if category == "pdf-corrupt":
            return
        if not data_strip.startswith(b"%PDF-"):
            raise ValueError(f"File {filename} does not start with %PDF- header (got {data[:16]!r})")
    elif ext in (".docx", ".xlsx", ".pptx"):
        if not data.startswith(b"PK\x03\x04"):
            raise ValueError(f"File {filename} is not a valid OOXML/Zip package (does not start with PK\\x03\\x04)")


def get_candidates():
    candidates = []

    # =========================================================================
    # 1. pdf-basic (15 files)
    # =========================================================================
    pdfium_basic = [
        ("pdfium-about-blank", "testing/resources/about_blank.pdf", "Minimal blank PDF page"),
        ("pdfium-black", "testing/resources/black.pdf", "Solid black single page rendering"),
        ("pdfium-bookmarks", "testing/resources/bookmarks.pdf", "Document outline and bookmark navigation tree"),
        ("pdfium-rectangles", "testing/resources/rectangles.pdf", "Vector graphic rectangle fills and stroke paths"),
        ("pdfium-hello-world", "testing/resources/hello_world.pdf", "Standard Hello World text document"),
        ("pdfium-hello-world-2-pages", "testing/resources/hello_world_2_pages.pdf", "Two-page Hello World document"),
        ("pdfium-bigtable-mini", "testing/resources/bigtable_mini.pdf", "Table layout structure and border lines"),
        ("pdfium-named-dests", "testing/resources/named_dests.pdf", "Named destination targets for cross-document links"),
        ("pdfium-page-labels", "testing/resources/page_labels.pdf", "Custom Roman and decimal page label numbering"),
        ("pdfium-rectangles-multi", "testing/resources/rectangles_multi_pages.pdf", "Multi-page vector drawing test"),
        ("pdfium-weblinks", "testing/resources/weblinks.pdf", "External web hyperlinks and URL annotations"),
    ]
    for fid, p, note in pdfium_basic:
        candidates.append({
            "id": fid,
            "category": "pdf-basic",
            "url": f"https://raw.githubusercontent.com/chromium/pdfium/main/{p}",
            "license": "BSD-3-Clause",
            "notes": note,
        })

    pdfjs_basic = [
        ("pdfjs-tracemonkey", "test/pdfs/tracemonkey.pdf", "Academic paper with 14 pages, figures, two-column layout"),
        ("pdfjs-hello-world-rotated", "test/pdfs/hello_world_rotated.pdf", "Rotated text rendering with standard font"),
        ("pdfjs-basicapi", "test/pdfs/basicapi.pdf", "Basic API reference test document"),
        ("pdfjs-pages-tree-refs", "test/pdfs/Pages-tree-refs.pdf", "Hierarchical page tree references"),
    ]
    for fid, p, note in pdfjs_basic:
        candidates.append({
            "id": fid,
            "category": "pdf-basic",
            "url": f"https://raw.githubusercontent.com/mozilla/pdf.js/master/{p}",
            "license": "Apache-2.0",
            "notes": note,
        })

    # =========================================================================
    # 2. pdf-large (500+ pages) (5 files, ALL strictly 500+ verified pages)
    # =========================================================================
    candidates.extend([
        {
            "id": "large-911-report",
            "category": "pdf-large",
            "url": "https://www.govinfo.gov/content/pkg/GPO-911REPORT/pdf/GPO-911REPORT.pdf",
            "license": "Public Domain",
            "notes": "9/11 Commission Report, 585 pages, US Government official publication",
        },
        {
            "id": "large-fcic-report",
            "category": "pdf-large",
            "url": "https://www.govinfo.gov/content/pkg/GPO-FCIC/pdf/GPO-FCIC.pdf",
            "license": "Public Domain",
            "notes": "Financial Crisis Inquiry Report, 663 pages, US Government official publication",
        },
        {
            "id": "large-cfr-title12",
            "category": "pdf-large",
            "url": "https://www.govinfo.gov/content/pkg/CFR-2024-title12-vol1/pdf/CFR-2024-title12-vol1.pdf",
            "license": "Public Domain",
            "notes": "Code of Federal Regulations Title 12 (Banks and Banking), 1,192 pages, US Government publication",
        },
        {
            "id": "large-cfr-title26",
            "category": "pdf-large",
            "url": "https://www.govinfo.gov/content/pkg/CFR-2024-title26-vol1/pdf/CFR-2024-title26-vol1.pdf",
            "license": "Public Domain",
            "notes": "Code of Federal Regulations Title 26 (Internal Revenue), 780 pages, US Government publication",
        },
        {
            "id": "large-generated-2000-page",
            "category": "pdf-large",
            "url": "synthetic://corpus/generate.py/large-generated-2000-page.pdf",
            "license": "AGPL-3.0-or-later",
            "notes": "Synthetically generated 2000-page valid PDF to test large document handling",
        },
    ])

    # =========================================================================
    # 3. pdf-encrypted (13 files)
    # =========================================================================
    pdfium_enc = [
        ("pdfium-encrypted", "testing/resources/encrypted.pdf", "Standard 40-bit RC4 password encryption"),
        ("pdfium-encrypted-hello-r2", "testing/resources/encrypted_hello_world_r2.pdf", "Standard revision 2 RC4 40-bit encryption"),
        ("pdfium-encrypted-hello-r2-bad-okey", "testing/resources/encrypted_hello_world_r2_bad_okey.pdf", "Revision 2 with invalid owner password key"),
        ("pdfium-encrypted-hello-r3", "testing/resources/encrypted_hello_world_r3.pdf", "Standard revision 3 RC4 128-bit encryption"),
        ("pdfium-encrypted-hello-r3-bad-okey", "testing/resources/encrypted_hello_world_r3_bad_okey.pdf", "Revision 3 with invalid owner password key"),
        ("pdfium-encrypted-hello-r5", "testing/resources/encrypted_hello_world_r5.pdf", "Standard revision 5 AES-256 encryption"),
        ("pdfium-encrypted-hello-r6", "testing/resources/encrypted_hello_world_r6.pdf", "Standard revision 6 AES-256 encryption"),
    ]
    for fid, p, note in pdfium_enc:
        candidates.append({
            "id": fid,
            "category": "pdf-encrypted",
            "url": f"https://raw.githubusercontent.com/chromium/pdfium/main/{p}",
            "license": "BSD-3-Clause",
            "notes": note,
        })

    qpdf_enc = [
        ("qpdf-enc1", "compare-for-test/qtest/compare/enc1.pdf", "Standard encryption revision 2 test file"),
        ("qpdf-enc2", "compare-for-test/qtest/compare/enc2.pdf", "Standard encryption revision 3 test file"),
        ("qpdf-diff-data-enc", "compare-for-test/qtest/compare/diff-data-enc.pdf", "Encrypted stream data verification"),
        ("qpdf-bad-enc-len", "qpdf/qtest/qpdf/bad-encryption-length.pdf", "Invalid encryption key length edge case"),
        ("qpdf-enc-r2-v1-o-master", "qpdf/qtest/qpdf/enc-R2,V1,O=master.pdf", "R2 V1 master password verification"),
    ]
    for fid, p, note in qpdf_enc:
        candidates.append({
            "id": fid,
            "category": "pdf-encrypted",
            "url": f"https://raw.githubusercontent.com/qpdf/qpdf/main/{p}",
            "license": "Apache-2.0",
            "notes": note,
        })

    candidates.append({
        "id": "pdfjs-encrypted-attachment",
        "category": "pdf-encrypted",
        "url": "https://raw.githubusercontent.com/mozilla/pdf.js/master/test/pdfs/encrypted-attachment.pdf",
        "license": "Apache-2.0",
        "notes": "Encrypted PDF with embedded encrypted file attachment",
    })

    # =========================================================================
    # 4. pdf-corrupt (17 files)
    # =========================================================================
    pdfium_corrupt = [
        ("pdfium-bad-annots", "testing/resources/bad_annots_entry.pdf", "Malformed annotation array entry in page dict"),
        ("pdfium-bad-dict-keys", "testing/resources/bad_dict_keys.pdf", "Corrupt dictionary key syntax and types"),
        ("pdfium-bad-page-type", "testing/resources/bad_page_type.pdf", "Invalid page object /Type attribute"),
        ("pdfium-bug-1029", "testing/resources/bug_1029.pdf", "Corrupt font descriptor and stream"),
        ("pdfium-bug-113", "testing/resources/bug_113.pdf", "Parser crash regression test case"),
        ("pdfium-bug-1206", "testing/resources/bug_1206.pdf", "Damaged indirect object reference"),
    ]
    for fid, p, note in pdfium_corrupt:
        candidates.append({
            "id": fid,
            "category": "pdf-corrupt",
            "url": f"https://raw.githubusercontent.com/chromium/pdfium/main/{p}",
            "license": "BSD-3-Clause",
            "notes": note,
        })

    pdfjs_corrupt = [
        ("pdfjs-pdfbox-3148-2-fuzzed", "test/pdfs/PDFBOX-3148-2-fuzzed.pdf", "Fuzzed PDF from Apache PDFBox test suite"),
        ("pdfjs-ghostscript-698804-1-fuzzed", "test/pdfs/GHOSTSCRIPT-698804-1-fuzzed.pdf", "Fuzzed Ghostscript regression sample"),
        ("pdfjs-scan-bad", "test/pdfs/scan-bad.pdf", "Corrupted scanned image stream"),
        ("pdfjs-helloworld-bad", "test/pdfs/helloworld-bad.pdf", "Malformed hello world stream dictionary"),
        ("pdfjs-poppler-395-0-fuzzed", "test/pdfs/poppler-395-0-fuzzed.pdf", "Fuzzed poppler parser test case"),
    ]
    for fid, p, note in pdfjs_corrupt:
        candidates.append({
            "id": fid,
            "category": "pdf-corrupt",
            "url": f"https://raw.githubusercontent.com/mozilla/pdf.js/master/{p}",
            "license": "Apache-2.0",
            "notes": note,
        })

    qpdf_corrupt = [
        ("qpdf-bad-content", "qpdf/qtest/qpdf/bad-content.pdf", "Invalid operator sequence in page content stream"),
        ("qpdf-append-damaged", "qpdf/qtest/qpdf/append-page-content-damaged.pdf", "Damaged page content stream during append"),
        ("qpdf-bad-xref", "qpdf/qtest/qpdf/bad-xref.pdf", "Malformed cross-reference table"),
        ("qpdf-bad-xref-entry", "qpdf/qtest/qpdf/bad-xref-entry.pdf", "Invalid xref entry format and offset"),
    ]
    for fid, p, note in qpdf_corrupt:
        candidates.append({
            "id": fid,
            "category": "pdf-corrupt",
            "url": f"https://raw.githubusercontent.com/qpdf/qpdf/main/{p}",
            "license": "Apache-2.0",
            "notes": note,
        })

    candidates.extend([
        {
            "id": "corrupt-generated-truncated",
            "category": "pdf-corrupt",
            "url": "synthetic://corpus/generate.py/corrupt-generated-truncated.pdf",
            "license": "AGPL-3.0-or-later",
            "notes": "Synthetically generated truncated PDF (cut off mid-stream, no trailer or EOF)",
        },
        {
            "id": "corrupt-generated-broken-xref",
            "category": "pdf-corrupt",
            "url": "synthetic://corpus/generate.py/corrupt-generated-broken-xref.pdf",
            "license": "AGPL-3.0-or-later",
            "notes": "Synthetically generated PDF with corrupted xref table and bogus startxref offset",
        },
    ])

    # =========================================================================
    # 5. pdf-forms (AcroForm + 2 XFA) (14 files)
    # =========================================================================
    pdfium_forms = [
        ("pdfium-combobox-form", "testing/resources/combobox_form.pdf", "AcroForm combobox / dropdown selector"),
        ("pdfium-click-form", "testing/resources/click_form.pdf", "Interactive pushbutton and click action"),
        ("pdfium-text-form", "testing/resources/text_form.pdf", "Single-line interactive text field"),
        ("pdfium-text-form-multiline", "testing/resources/text_form_multiline.pdf", "Multiline interactive text field"),
        ("pdfium-listbox-form", "testing/resources/listbox_form.pdf", "AcroForm listbox with scrolling options"),
        ("pdfium-multiple-form-types", "testing/resources/multiple_form_types.pdf", "Mixed form fields: checkboxes, radios, text"),
        ("pdfium-form-object", "testing/resources/form_object.pdf", "Form XObject rendering and hierarchy"),
        ("pdfium-simple-xfa", "testing/resources/simple_xfa.pdf", "XML Forms Architecture (XFA) dynamic form"),
        ("pdfium-rectangles-multi-xfa", "testing/resources/rectangles_multi_page_xfa.pdf", "Multi-page XFA dynamic form"),
    ]
    for fid, p, note in pdfium_forms:
        candidates.append({
            "id": fid,
            "category": "pdf-forms",
            "url": f"https://raw.githubusercontent.com/chromium/pdfium/main/{p}",
            "license": "BSD-3-Clause",
            "notes": note,
        })

    pdfjs_forms = [
        ("pdfjs-prefilled-f1040", "test/pdfs/prefilled_f1040.pdf", "IRS Form 1040 federal tax form prefilled with AcroForm data"),
        ("pdfjs-form-two-pages", "test/pdfs/form_two_pages.pdf", "Two-page interactive AcroForm with various widgets"),
        ("pdfjs-acroform-calc-order", "test/pdfs/acroform_calculation_order.pdf", "Form fields with calculation order (CO) dependencies"),
        ("pdfjs-resetform", "test/pdfs/resetform.pdf", "ResetForm action button functionality"),
        ("pdfjs-xfa-filled-imm1344e", "test/pdfs/xfa_filled_imm1344e.pdf", "Canadian immigration official dynamic XFA form"),
    ]
    for fid, p, note in pdfjs_forms:
        candidates.append({
            "id": fid,
            "category": "pdf-forms",
            "url": f"https://raw.githubusercontent.com/mozilla/pdf.js/master/{p}",
            "license": "Apache-2.0",
            "notes": note,
        })

    # =========================================================================
    # 6. pdf-annotations (14 files)
    # =========================================================================
    pdfium_annots = [
        ("pdfium-annots", "testing/resources/annots.pdf", "Comprehensive annotation collection"),
        ("pdfium-annot-highlight-long", "testing/resources/annotation_highlight_long_content.pdf", "Highlight annotation across multiple lines of text"),
        ("pdfium-annot-highlight-square", "testing/resources/annotation_highlight_square_with_ap.pdf", "Highlight annotation with custom Appearance Stream (AP)"),
        ("pdfium-annot-ink-multiple", "testing/resources/annotation_ink_multiple.pdf", "Freehand ink annotation with multiple strokes"),
        ("pdfium-annot-stamp", "testing/resources/annotation_stamp_with_ap.pdf", "Stamp annotation with custom appearance stream"),
        ("pdfium-annot-fileattachment", "testing/resources/annotation_fileattachment.pdf", "Embedded file attachment annotation"),
        ("pdfium-annot-line", "testing/resources/line_annot.pdf", "Line markup annotation with arrow endpoints"),
        ("pdfium-annot-ink", "testing/resources/ink_annot.pdf", "Basic ink stroke annotation"),
        ("pdfium-annot-freetext-no-da", "testing/resources/freetext_annotation_without_da.pdf", "FreeText annotation lacking default appearance (DA) string"),
    ]
    for fid, p, note in pdfium_annots:
        candidates.append({
            "id": fid,
            "category": "pdf-annotations",
            "url": f"https://raw.githubusercontent.com/chromium/pdfium/main/{p}",
            "license": "BSD-3-Clause",
            "notes": note,
        })

    pdfjs_annots = [
        ("pdfjs-annot-highlight", "test/pdfs/annotation-highlight.pdf", "Standard text highlight annotation"),
        ("pdfjs-annot-freetext", "test/pdfs/annotation-freetext.pdf", "FreeText typewriter note annotation"),
        ("pdfjs-annot-caret-ink", "test/pdfs/annotation-caret-ink.pdf", "Caret insertion indicator and ink drawing"),
        ("pdfjs-annot-border-styles", "test/pdfs/annotation-border-styles.pdf", "Solid, dashed, and beveled annotation border styles"),
        ("pdfjs-annot-button-widget", "test/pdfs/annotation-button-widget.pdf", "Widget annotation for interactive buttons"),
    ]
    for fid, p, note in pdfjs_annots:
        candidates.append({
            "id": fid,
            "category": "pdf-annotations",
            "url": f"https://raw.githubusercontent.com/mozilla/pdf.js/master/{p}",
            "license": "Apache-2.0",
            "notes": note,
        })

    # =========================================================================
    # 7. pdf-scanned (8 files, including authentic Turkish scans)
    # =========================================================================
    candidates.extend([
        {
            "id": "scan-tr-rg-2741",
            "category": "pdf-scanned",
            "url": "https://upload.wikimedia.org/wikipedia/commons/7/7e/RG_-_2741.pdf",
            "license": "Public Domain",
            "notes": "Authentic Turkish historical scanned document: T.C. Resmî Gazete sayı 2741 (1934)",
        },
        {
            "id": "scan-tr-rg-2865",
            "category": "pdf-scanned",
            "url": "https://upload.wikimedia.org/wikipedia/commons/d/d8/RG-_2865.pdf",
            "license": "Public Domain",
            "notes": "Authentic Turkish historical scanned document: T.C. Resmî Gazete sayı 2865 (1934)",
        },
        {
            "id": "scan-tr-rg-2888",
            "category": "pdf-scanned",
            "url": "https://upload.wikimedia.org/wikipedia/commons/1/1c/RG_-_2888.pdf",
            "license": "Public Domain",
            "notes": "Authentic Turkish historical scanned document: T.C. Resmî Gazete sayı 2888 (1934, multi-page)",
        },
        {
            "id": "pdfium-embedded-images",
            "category": "pdf-scanned",
            "url": "https://raw.githubusercontent.com/chromium/pdfium/main/testing/resources/embedded_images.pdf",
            "license": "BSD-3-Clause",
            "notes": "Embedded image streams across multiple pages representing scanned document pages",
        },
        {
            "id": "pdfium-rotated-image",
            "category": "pdf-scanned",
            "url": "https://raw.githubusercontent.com/chromium/pdfium/main/testing/resources/rotated_image.pdf",
            "license": "BSD-3-Clause",
            "notes": "Scanned document page with 90-degree rotated image matrix",
        },
        {
            "id": "pdfium-form-image",
            "category": "pdf-scanned",
            "url": "https://raw.githubusercontent.com/chromium/pdfium/main/testing/resources/form_object_with_image.pdf",
            "license": "BSD-3-Clause",
            "notes": "Scanned form document with embedded bitmap image object",
        },
        {
            "id": "pdfium-xfa-image",
            "category": "pdf-scanned",
            "url": "https://raw.githubusercontent.com/chromium/pdfium/main/testing/resources/xfa/xfa_image_edit.pdf",
            "license": "BSD-3-Clause",
            "notes": "Scanned image element embedded in dynamic form layout",
        },
        {
            "id": "pdfjs-perf-images-jpeg",
            "category": "pdf-scanned",
            "url": "https://raw.githubusercontent.com/mozilla/pdf.js/master/test/pdfs/perf_images_jpeg_large.pdf",
            "license": "Apache-2.0",
            "notes": "High-resolution scanned image page document",
        },
    ])

    # =========================================================================
    # 8. pdf-pdfa (12 files)
    # =========================================================================
    verapdf_files = [
        ("verapdf-1a-unicode-pass-a", "PDF_A-1a/6.3 Fonts/6.3.8 Unicode character maps/veraPDF test suite 6-3-8-t01-pass-a.pdf", "PDF/A-1a Unicode character map valid conformance"),
        ("verapdf-1a-unicode-pass-b", "PDF_A-1a/6.3 Fonts/6.3.8 Unicode character maps/veraPDF test suite 6-3-8-t01-pass-b.pdf", "PDF/A-1a Unicode character map valid conformance (variant B)"),
        ("verapdf-1a-unicode-fail-a", "PDF_A-1a/6.3 Fonts/6.3.8 Unicode character maps/veraPDF test suite 6-3-8-t01-fail-a.pdf", "PDF/A-1a Unicode character map invalid conformance test"),
        ("verapdf-1a-unicode-fail-b", "PDF_A-1a/6.3 Fonts/6.3.8 Unicode character maps/veraPDF test suite 6-3-8-t01-fail-b.pdf", "PDF/A-1a Unicode character map invalid conformance test (variant B)"),
        ("verapdf-1b-header-pass-a", "PDF_A-1b/6.1 File structure/6.1.2 File header/veraPDF test suite 6-1-2-t01-pass-a.pdf", "PDF/A-1b File header valid conformance test"),
        ("verapdf-1b-header-fail-a", "PDF_A-1b/6.1 File structure/6.1.2 File header/veraPDF test suite 6-1-2-t01-fail-a.pdf", "PDF/A-1b File header invalid conformance test"),
        ("verapdf-1b-trailer-pass-a", "PDF_A-1b/6.1 File structure/6.1.3 File trailer/veraPDF test suite 6-1-3-t01-pass-a.pdf", "PDF/A-1b File trailer dictionary valid conformance test"),
        ("verapdf-1b-trailer-fail-a", "PDF_A-1b/6.1 File structure/6.1.3 File trailer/veraPDF test suite 6-1-3-t01-fail-a.pdf", "PDF/A-1b File trailer dictionary invalid conformance test"),
        ("verapdf-2b-transparency-pass-a", "PDF_A-2b/6.2 Graphics/6.2.10 Transparency/veraPDF test suite 6-2-10-t01-pass-a.pdf", "PDF/A-2b Graphics transparency valid conformance test"),
        ("verapdf-2b-transparency-fail-a", "PDF_A-2b/6.2 Graphics/6.2.10 Transparency/veraPDF test suite 6-2-10-t01-fail-a.pdf", "PDF/A-2b Graphics transparency invalid conformance test"),
        ("verapdf-3b-embedded-pass-a", "PDF_A-3b/6.8 Embedded files/veraPDF test suite 6-8-t02-pass-a.pdf", "PDF/A-3b Embedded files specification valid conformance"),
        ("verapdf-3b-embedded-fail-a", "PDF_A-3b/6.8 Embedded files/veraPDF test suite 6-8-t01-fail-a.pdf", "PDF/A-3b Embedded files specification invalid conformance"),
    ]
    for fid, p, note in verapdf_files:
        quoted = urllib.parse.quote(p)
        candidates.append({
            "id": fid,
            "category": "pdf-pdfa",
            "url": f"https://raw.githubusercontent.com/veraPDF/veraPDF-corpus/master/{quoted}",
            "license": "MPL-2.0",
            "notes": note,
        })

    # =========================================================================
    # 9. pdf-signed (9 files)
    # =========================================================================
    candidates.extend([
        {
            "id": "pdfium-sig-no-subfilter",
            "category": "pdf-signed",
            "url": "https://raw.githubusercontent.com/chromium/pdfium/main/testing/resources/signature_no_sub_filter.pdf",
            "license": "BSD-3-Clause",
            "notes": "Digital signature dictionary lacking /SubFilter entry",
        },
        {
            "id": "pdfium-sig-reason",
            "category": "pdf-signed",
            "url": "https://raw.githubusercontent.com/chromium/pdfium/main/testing/resources/signature_reason.pdf",
            "license": "BSD-3-Clause",
            "notes": "Digital signature containing Reason and Location strings",
        },
        {
            "id": "pdfium-sig-two",
            "category": "pdf-signed",
            "url": "https://raw.githubusercontent.com/chromium/pdfium/main/testing/resources/two_signatures.pdf",
            "license": "BSD-3-Clause",
            "notes": "Document containing two distinct digital signature fields",
        },
        {
            "id": "pdfjs-sig-verified",
            "category": "pdf-signed",
            "url": "https://raw.githubusercontent.com/mozilla/pdf.js/master/test/pdfs/signed_verified.pdf",
            "license": "Apache-2.0",
            "notes": "Cryptographically signed PDF with PKCS#7 detached signature",
        },
        {
            "id": "pdfjs-sig-cff-unsigned-bbox",
            "category": "pdf-signed",
            "url": "https://raw.githubusercontent.com/mozilla/pdf.js/master/test/pdfs/issue_cff_unsigned_bbox.pdf",
            "license": "Apache-2.0",
            "notes": "Signed document containing CFF font unsigned bounding box edge case",
        },
        {
            "id": "pyhanko-certified-indirect",
            "category": "pdf-signed",
            "url": "https://raw.githubusercontent.com/MatthiasValvekens/pyHanko/master/internal/common-test-utils/src/pyhanko_testing_commons/test_data/data/pdf/certified-with-indirect-refs-in-dir.pdf",
            "license": "MIT",
            "notes": "Certified PDF with DocMDP permissions dictionary and indirect object references",
        },
        {
            "id": "pyhanko-sig-twice",
            "category": "pdf-signed",
            "url": "https://raw.githubusercontent.com/MatthiasValvekens/pyHanko/master/internal/common-test-utils/src/pyhanko_testing_commons/test_data/data/pdf/minimal-signed-twice-both-created.pdf",
            "license": "MIT",
            "notes": "Incremental updates with two consecutive digital signatures",
        },
        {
            "id": "pyhanko-sig-two-fields",
            "category": "pdf-signed",
            "url": "https://raw.githubusercontent.com/MatthiasValvekens/pyHanko/master/internal/common-test-utils/src/pyhanko_testing_commons/test_data/data/pdf/minimal-two-fields-signed-twice.pdf",
            "license": "MIT",
            "notes": "Dual signature fields with incremental update signed twice",
        },
        {
            "id": "pyhanko-sig-gap-too-big",
            "category": "pdf-signed",
            "url": "https://raw.githubusercontent.com/MatthiasValvekens/pyHanko/master/internal/common-test-utils/src/pyhanko_testing_commons/test_data/data/pdf/coverage-anomalies/signature-gap-too-big.pdf",
            "license": "MIT",
            "notes": "Signature byte range gap anomaly boundary test case",
        },
    ])

    # =========================================================================
    # 10. pdf-cjk-rtl-turkish (12 files)
    # =========================================================================
    candidates.extend([
        {
            "id": "cjk-simfang-variant",
            "category": "pdf-cjk-rtl-turkish",
            "url": "https://raw.githubusercontent.com/mozilla/pdf.js/master/test/pdfs/SimFang-variant.pdf",
            "license": "Apache-2.0",
            "notes": "Simplified Chinese text rendering with FangSong font",
        },
        {
            "id": "cjk-90ms-rksj",
            "category": "pdf-cjk-rtl-turkish",
            "url": "https://raw.githubusercontent.com/mozilla/pdf.js/master/test/pdfs/90ms_rksj_h_sample.pdf",
            "license": "Apache-2.0",
            "notes": "Japanese Shift-JIS 90ms-RKSJ-H CIDFont sample",
        },
        {
            "id": "cjk-perf-text",
            "category": "pdf-cjk-rtl-turkish",
            "url": "https://raw.githubusercontent.com/mozilla/pdf.js/master/test/pdfs/perf_text_cjk_nonembedded.pdf",
            "license": "Apache-2.0",
            "notes": "CJK text rendering performance with non-embedded fonts",
        },
        {
            "id": "rtl-arabic-cid-truetype",
            "category": "pdf-cjk-rtl-turkish",
            "url": "https://raw.githubusercontent.com/mozilla/pdf.js/master/test/pdfs/ArabicCIDTrueType.pdf",
            "license": "Apache-2.0",
            "notes": "Arabic right-to-left (RTL) CID TrueType text rendering",
        },
        {
            "id": "cjk-arial-unicode-ab",
            "category": "pdf-cjk-rtl-turkish",
            "url": "https://raw.githubusercontent.com/mozilla/pdf.js/master/test/pdfs/arial_unicode_ab_cidfont.pdf",
            "license": "Apache-2.0",
            "notes": "Arial Unicode CIDFont subset character layout",
        },
        {
            "id": "cjk-arial-unicode-en",
            "category": "pdf-cjk-rtl-turkish",
            "url": "https://raw.githubusercontent.com/mozilla/pdf.js/master/test/pdfs/arial_unicode_en_cidfont.pdf",
            "license": "Apache-2.0",
            "notes": "Arial Unicode Latin and CJK CIDFont subset character layout",
        },
        {
            "id": "font-pdfium-utf8",
            "category": "pdf-cjk-rtl-turkish",
            "url": "https://raw.githubusercontent.com/chromium/pdfium/main/testing/resources/utf-8.pdf",
            "license": "BSD-3-Clause",
            "notes": "UTF-8 encoded Unicode text rendering test",
        },
        {
            "id": "font-pdfium-text-font",
            "category": "pdf-cjk-rtl-turkish",
            "url": "https://raw.githubusercontent.com/chromium/pdfium/main/testing/resources/text_font.pdf",
            "license": "BSD-3-Clause",
            "notes": "Font matrix transformations and character layout",
        },
        {
            "id": "tr-vikipedi-kutuphane",
            "category": "pdf-cjk-rtl-turkish",
            "url": "https://upload.wikimedia.org/wikipedia/commons/6/6c/Vikipedi_K%C3%BCt%C3%BCphanesi_Bilgilendirme.pdf",
            "license": "CC-BY-SA-4.0",
            "notes": "Authentic Turkish text document: Vikipedi Kütüphanesi Bilgilendirme rehberi (modern Turkish typography)",
        },
        {
            "id": "tr-vikipedi-maddesi",
            "category": "pdf-cjk-rtl-turkish",
            "url": "https://upload.wikimedia.org/wikipedia/commons/8/8e/Vikipedi_Maddesi_Olu%C5%9Fturuyorum.pdf",
            "license": "CC-BY-SA-4.0",
            "notes": "Authentic Turkish text document: Vikipedi Maddesi Oluşturuyorum kılavuzu (Turkish headings, lists, callouts)",
        },
        {
            "id": "tr-vikipedi-psikoloji",
            "category": "pdf-cjk-rtl-turkish",
            "url": "https://upload.wikimedia.org/wikipedia/commons/b/b1/Vikipedi_psikoloji_maddeleri_yazmak.pdf",
            "license": "CC-BY-SA-4.0",
            "notes": "Authentic Turkish text document: Vikipedi psikoloji maddeleri yazmak rehberi (multi-page article typography)",
        },
        {
            "id": "tr-generated-standard-fonts",
            "category": "pdf-cjk-rtl-turkish",
            "url": "synthetic://corpus/generate.py/tr-generated-standard-fonts.pdf",
            "license": "AGPL-3.0-or-later",
            "notes": "Synthetically generated PDF with Turkish characters (İıŞşĞğÇçÖöÜü) using standard Type 1 font Differences array and /ToUnicode CMap",
        },
    ])

    # =========================================================================
    # 11. docx (15 files)
    # =========================================================================
    poi_docx = [
        ("poi-docx-word-crash067", "test-data/document/51921-Word-Crash067.docx", "Word crash regression document"),
        ("poi-docx-52288", "test-data/document/52288.docx", "Paragraph formatting and text runs"),
        ("poi-docx-52449", "test-data/document/52449.docx", "Section breaks and page layout margins"),
        ("poi-docx-55733", "test-data/document/55733.docx", "Complex character styling and formatting"),
        ("poi-docx-55966", "test-data/document/55966.docx", "Numbered and bulleted list hierarchies"),
        ("poi-docx-56392", "test-data/document/56392.docx", "Header and footer references and layouts"),
        ("poi-docx-57312", "test-data/document/57312.docx", "Table grid cells, borders, and background shading"),
        ("poi-docx-58067", "test-data/document/58067.docx", "Embedded drawings and shape anchors"),
        ("poi-docx-58618", "test-data/document/58618.docx", "Hyperlinks and document bookmarks"),
        ("poi-docx-59030", "test-data/document/59030.docx", "Footnotes and endnotes structures"),
        ("poi-docx-59378", "test-data/document/59378.docx", "Multi-column text layout"),
        ("poi-docx-60293", "test-data/document/60293.docx", "Comments and tracked revisions"),
        ("poi-docx-60316", "test-data/document/60316.docx", "Document properties and metadata"),
        ("poi-docx-60329", "test-data/document/60329.docx", "Table column width calculations"),
        ("poi-docx-bar-chart-template", "poi-examples/src/main/java/org/apache/poi/examples/xwpf/usermodel/bar-chart-template.docx", "Embedded chart object and diagram template"),
    ]
    for fid, p, note in poi_docx:
        candidates.append({
            "id": fid,
            "category": "docx",
            "url": f"https://raw.githubusercontent.com/apache/poi/trunk/{p}",
            "license": "Apache-2.0",
            "notes": note,
        })

    # =========================================================================
    # 12. xlsx (13 files, including authentic Turkish open data)
    # =========================================================================
    poi_xlsx = [
        ("poi-xlsx-50154", "test-data/openxml4j/50154.xlsx", "Standard worksheet structure and shared strings"),
        ("poi-xlsx-51444", "test-data/openxml4j/51444.xlsx", "Worksheet numeric formulas and calculation engine"),
        ("poi-xlsx-53282", "test-data/openxml4j/53282.xlsx", "Cached formula results and recalculation flags"),
        ("poi-xlsx-hyperlinks", "test-data/openxml4j/ExcelWithHyperlinks.xlsx", "Cell hyperlinks and external references"),
        ("poi-xlsx-sample", "test-data/openxml4j/sample.xlsx", "Multi-sheet workbook structure and styles"),
        ("poi-xlsx-stress002", "test-data/integration/stress002.xlsx", "Large worksheet grid dimensions stress test"),
        ("poi-xlsx-stress020", "test-data/integration/stress020.xlsx", "Shared strings table volume stress test"),
        ("poi-xlsx-stress024", "test-data/integration/stress024.xlsx", "Complex cell formatting and number styles stress test"),
        ("poi-xlsx-opc-compliance", "test-data/openxml4j/OPCCompliance_NoCoreProperties.xlsx", "OPC package compliance without core properties"),
        ("poi-xlsx-64473", "test-data/openxml4j/64473.xlsx", "OpenXML4J issue 64473 workbook parts test"),
    ]
    for fid, p, note in poi_xlsx:
        candidates.append({
            "id": fid,
            "category": "xlsx",
            "url": f"https://raw.githubusercontent.com/apache/poi/trunk/{p}",
            "license": "Apache-2.0",
            "notes": note,
        })

    candidates.extend([
        {
            "id": "tr-ibb-kamulastirma-maliyeti",
            "category": "xlsx",
            "url": "https://data.ibb.gov.tr/dataset/c738353c-abcd-419d-bf9f-6083175c5dd6/resource/ad54340a-9a17-454d-a047-f0e3aaa6fd0c/download/yllara-gore-kamulatrma-maliyeti.xlsx",
            "license": "CC-BY-4.0",
            "notes": "Authentic Turkish open data spreadsheet: Yıllara Göre Kamulaştırma Maliyeti (tables, currency, Turkish headers)",
        },
        {
            "id": "tr-ibb-kamulastirilan-alan",
            "category": "xlsx",
            "url": "https://data.ibb.gov.tr/dataset/a0533a25-ed05-407e-9fad-882f635aa4f3/resource/d248ec2d-eeeb-4910-9162-c21a3ab53aea/download/2022-yl-proje-amacna-gore-kamulatrlan-alan-miktar.xlsx",
            "license": "CC-BY-4.0",
            "notes": "Authentic Turkish open data spreadsheet: Proje Amacına Göre Kamulaştırılan Alan (multi-column tables, Turkish characters)",
        },
        {
            "id": "tr-ibb-hane-halki",
            "category": "xlsx",
            "url": "https://data.ibb.gov.tr/dataset/2f8889dd-4e5d-46c6-ad75-922a6cd36a84/resource/a148a770-d20f-4e44-a4ef-894e0d4cdae4/download/ilce-bazl-ortalama-hane-halk-buyukluu_2008_2024.xlsx",
            "license": "CC-BY-4.0",
            "notes": "Authentic Turkish open data spreadsheet: İlçe Bazlı Ortalama Hane Halkı Büyüklüğü 2008-2024 (demographic time-series tables)",
        },
    ])

    # =========================================================================
    # 13. pptx (14 files)
    # =========================================================================
    poi_pptx = [
        ("poi-pptx-bar-chart", "poi-examples/src/main/java/org/apache/poi/examples/xslf/bar-chart-template.pptx", "Bar chart presentation template and data bindings"),
        ("poi-pptx-pie-chart", "poi-examples/src/main/java/org/apache/poi/examples/xslf/pie-chart-template.pptx", "Pie chart presentation template and data series"),
        ("poi-pptx-empty", "poi-ooxml/src/main/resources/org/apache/poi/xslf/usermodel/empty.pptx", "Minimal blank default slide master and presentation"),
        ("poi-pptx-stress013", "test-data/integration/stress013.pptx", "Multi-slide presentation layout stress test"),
        ("poi-pptx-stress021", "test-data/integration/stress021.pptx", "Complex drawing shapes and visual hierarchy stress test"),
        ("poi-pptx-stress022", "test-data/integration/stress022.pptx", "Rich text formatting runs and styling stress test"),
        ("poi-pptx-perf-up", "test-data/slideshow/2411-Performance_Up.pptx", "Slide presentation deck with gradient fills and vectors"),
        ("poi-pptx-footer", "test-data/slideshow/45541_Footer.pptx", "Master slide footer placeholders and formatting"),
        ("poi-pptx-header", "test-data/slideshow/45541_Header.pptx", "Master slide header placeholders and formatting"),
        ("poi-pptx-comment", "test-data/slideshow/45545_Comment.pptx", "Slide author comments and threaded notes"),
        ("poi-pptx-null-dates", "test-data/slideshow/49386-null_dates.pptx", "Date placeholders and null date handling in slides"),
        ("poi-pptx-51187", "test-data/slideshow/51187.pptx", "Group shape collections and line connectors"),
        ("poi-pptx-cropped-bitmap", "test-data/slideshow/54542_cropped_bitmap.pptx", "Cropped bitmap images and picture effects on slides"),
        ("poi-pptx-56812", "test-data/slideshow/56812.pptx", "Native table shapes on presentation slides"),
    ]
    for fid, p, note in poi_pptx:
        candidates.append({
            "id": fid,
            "category": "pptx",
            "url": f"https://raw.githubusercontent.com/apache/poi/trunk/{p}",
            "license": "Apache-2.0",
            "notes": note,
        })

    return candidates


def download_and_hash(entry: dict, target_dir: Path) -> tuple[str, int, str]:
    cat = entry["category"]
    url = entry["url"]

    # Scheme validation (Blocking Issue 4)
    parsed = urllib.parse.urlparse(url)
    if parsed.scheme not in ("https", "synthetic"):
        raise ValueError(f"Insecure URL scheme '{parsed.scheme}': only https:// and synthetic:// are permitted")

    ext = Path(parsed.path).suffix
    if not ext:
        if cat.startswith("pdf"):
            ext = ".pdf"
        elif cat in ["docx", "xlsx", "pptx"]:
            ext = f".{cat}"
        else:
            ext = ".dat"
    filename = f"{entry['id']}{ext}"

    # Path safety validation (Blocking Issue 2)
    target_path = validate_path_safety(cat, filename, target_dir)

    # Synthetic handling (Blocking Issue 3)
    if url.startswith("synthetic://"):
        gen_filename = url.split("/")[-1]
        if gen_filename in GENERATORS:
            data = GENERATORS[gen_filename]["func"]()
            verify_magic_bytes(data, cat, filename)
            sha256 = hashlib.sha256(data).hexdigest().lower()
            size = len(data)
            target_path.write_bytes(data)
            return sha256, size, filename
        raise ValueError(f"Unknown generator: {url}")

    # Check if already present, valid, and non-empty
    if target_path.exists() and target_path.stat().st_size > 0:
        data = target_path.read_bytes()
        try:
            verify_magic_bytes(data, cat, filename)
            sha256 = hashlib.sha256(data).hexdigest().lower()
            size = len(data)
            return sha256, size, filename
        except ValueError:
            target_path.unlink()  # Corrupted or HTML, re-download

    # Download with retries
    retries = 3
    last_err = None
    for attempt in range(1, retries + 1):
        try:
            req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) RBD-OFIS-Builder/1.0"})
            with urllib.request.urlopen(req, timeout=30) as resp:
                data = resp.read()
                verify_magic_bytes(data, cat, filename)
                sha256 = hashlib.sha256(data).hexdigest().lower()
                size = len(data)
                target_path.write_bytes(data)
                return sha256, size, filename
        except Exception as e:
            last_err = e
            time.sleep(1.5 * attempt)

    raise last_err


def main():
    print("Building RBD OFIS Test Corpus Manifest...")
    candidates = get_candidates()
    print(f"Total candidates: {len(candidates)}")

    target_dir = ROOT_DIR / "corpus" / "files"
    target_dir.mkdir(parents=True, exist_ok=True)

    manifest_entries = []
    failures = []
    total_bytes = 0

    for i, entry in enumerate(candidates, 1):
        fid = entry["id"]
        cat = entry["category"]
        url = entry["url"]
        sys.stdout.write(f"[{i:3d}/{len(candidates)}] Processing {cat}/{fid}...")
        sys.stdout.flush()

        try:
            sha256, size, filename = download_and_hash(entry, target_dir)
            total_bytes += size
            manifest_entry = {
                "id": fid,
                "category": cat,
                "filename": filename,
                "url": url,
                "sha256": sha256,
                "license": entry["license"],
                "size_bytes": size,
                "notes": entry["notes"],
            }
            manifest_entries.append(manifest_entry)
            print(f" OK ({size:,} bytes, {sha256[:10]}...)")
        except Exception as e:
            print(f" FAILED: {e}")
            failures.append((entry, str(e)))

    print("\n" + "=" * 60)
    print(f"Manifest build complete: {len(manifest_entries)} OK, {len(failures)} failed.")
    print(f"Total size: {total_bytes:,} bytes ({total_bytes / (1024 * 1024):.2f} MB)")

    if failures:
        print("\nFailures:")
        for entry, err in failures:
            print(f"  {entry['category']}/{entry['id']}: {entry['url']} -> {err}")
        sys.exit(1)

    manifest_path = ROOT_DIR / "corpus" / "manifest.json"
    with open(manifest_path, "w", encoding="utf-8") as f:
        json.dump(manifest_entries, f, indent=2, ensure_ascii=False)
    print(f"\nSaved verified manifest to {manifest_path}")


if __name__ == "__main__":
    main()
