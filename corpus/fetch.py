#!/usr/bin/env python3
"""
RBD OFIS Test Corpus Fetcher.
AGPL-3.0-or-later. Standard library only (no external dependencies).

Downloads openly licensed test documents into corpus/files/<category>/,
verifies SHA-256 checksums, validates file format magic bytes, prevents
CWE-22 path traversal, restricts schemes to https/synthetic, is fully
idempotent and resumable, and prints detailed status summaries.
"""

import argparse
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

try:
    from corpus.generate import GENERATORS, validate_path_safety
except ImportError:
    GENERATORS = {}

    def validate_path_safety(category: str, filename: str, base_dir: Path) -> Path:
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


def compute_sha256(path: Path) -> str:
    """Compute SHA-256 checksum of a file in chunks."""
    h = hashlib.sha256()
    with open(path, "rb") as f:
        while chunk := f.read(65536):
            h.update(chunk)
    return h.hexdigest().lower()


def verify_magic_bytes(data: bytes, category: str, filename: str) -> None:
    """Verify document format magic bytes and strictly forbid HTML response pages."""
    data_strip = data.lstrip()
    if data_strip.startswith(b"<!DOCTYPE html") or data_strip.startswith(b"<html"):
        raise ValueError(f"File {filename} is an HTML error page, not a valid document!")

    ext = Path(filename).suffix.lower()
    if ext == ".pdf":
        if category == "pdf-corrupt":
            return
        if not data_strip.startswith(b"%PDF-"):
            raise ValueError(f"File {filename} does not start with %PDF- header (got {data[:16]!r})")
    elif ext in (".docx", ".xlsx", ".pptx"):
        if not data.startswith(b"PK\x03\x04"):
            raise ValueError(f"File {filename} is not a valid OOXML/Zip package (does not start with PK\\x03\\x04)")


def fetch_file(entry: dict, target_dir: Path, force: bool = False, timeout: int = 30, retries: int = 3) -> tuple[str, int]:
    """
    Fetch a single corpus entry.
    Returns (status, bytes_downloaded).
    Status is one of: 'skipped', 'downloaded', 'generated', 'failed'.
    """
    cat = entry["category"]
    url = entry["url"]

    # 1. Validate URL scheme (Blocking Issue 4)
    parsed = urllib.parse.urlparse(url)
    if parsed.scheme not in ("https", "synthetic"):
        raise ValueError(f"Disallowed URL scheme '{parsed.scheme}': only https:// and synthetic:// are permitted")

    # 2. Determine and validate filename & destination path (Blocking Issue 2)
    filename = entry.get("filename")
    if not filename:
        url_path = parsed.path
        ext = Path(url_path).suffix
        if not ext:
            if cat.startswith("pdf"):
                ext = ".pdf"
            elif cat in ["docx", "xlsx", "pptx"]:
                ext = f".{cat}"
            else:
                ext = ".dat"
        filename = f"{entry['id']}{ext}"

    target_path = validate_path_safety(cat, filename, target_dir)
    expected_sha256 = entry.get("sha256", "").lower()

    # 3. Check if already present and verified (Idempotent check)
    if not force and target_path.exists() and target_path.stat().st_size > 0:
        if expected_sha256:
            local_sha256 = compute_sha256(target_path)
            if local_sha256 == expected_sha256:
                return "skipped", 0

    # 4. Handle synthetically generated files (Blocking Issue 3)
    if url.startswith("synthetic://"):
        gen_filename = url.split("/")[-1]
        if gen_filename in GENERATORS:
            data = GENERATORS[gen_filename]["func"]()
            verify_magic_bytes(data, cat, filename)
            actual_sha = hashlib.sha256(data).hexdigest().lower()
            if expected_sha256 and actual_sha != expected_sha256:
                raise ValueError(f"Generated hash mismatch for {gen_filename}: {actual_sha} vs {expected_sha256}")
            target_path.write_bytes(data)
            return "generated", len(data)
        raise ValueError(f"Unknown generator requested: {url}")

    # 5. HTTPS download with resume support and retries
    part_path = target_path.with_name(f"{filename}.part")
    last_error = None

    for attempt in range(1, retries + 1):
        try:
            req_headers = {
                "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) RBD-OFIS-TestCorpus/1.0"
            }
            existing_bytes = 0
            open_mode = "wb"

            if part_path.exists() and part_path.stat().st_size > 0:
                existing_bytes = part_path.stat().st_size
                req_headers["Range"] = f"bytes={existing_bytes}-"
                open_mode = "ab"

            req = urllib.request.Request(url, headers=req_headers)
            with urllib.request.urlopen(req, timeout=timeout) as resp:
                status_code = getattr(resp, "status", 200)

                if status_code == 200 and existing_bytes > 0:
                    existing_bytes = 0
                    open_mode = "wb"

                downloaded_this_req = 0
                with open(part_path, open_mode) as out_f:
                    while chunk := resp.read(65536):
                        out_f.write(chunk)
                        downloaded_this_req += len(chunk)

            # Magic bytes verification (Blocking Issue 1)
            with open(part_path, "rb") as pf:
                sample_data = pf.read(1024)
            verify_magic_bytes(sample_data, cat, filename)

            # Checksum verification
            actual_sha256 = compute_sha256(part_path)
            if expected_sha256 and actual_sha256 != expected_sha256:
                part_path.unlink(missing_ok=True)
                raise ValueError(f"SHA-256 mismatch for {entry['id']}: expected {expected_sha256}, got {actual_sha256}")

            part_path.replace(target_path)
            return "downloaded", existing_bytes + downloaded_this_req

        except Exception as e:
            last_error = e
            time.sleep(1.0 * attempt)

    part_path.unlink(missing_ok=True)
    raise RuntimeError(f"Failed to fetch {url} after {retries} attempts: {last_error}")


def main():
    parser = argparse.ArgumentParser(description="Fetch and verify RBD OFIS Test Corpus documents")
    parser.add_argument("--manifest", default="corpus/manifest.json", help="Path to manifest.json (default: corpus/manifest.json)")
    parser.add_argument("--output-dir", default="corpus/files", help="Base directory for corpus files (default: corpus/files)")
    parser.add_argument("--category", default=None, help="Filter downloads to a specific category")
    parser.add_argument("--force", action="store_true", help="Force re-download even if files exist and match checksums")
    parser.add_argument("--timeout", type=int, default=30, help="Socket timeout in seconds (default: 30)")
    parser.add_argument("--retries", type=int, default=3, help="Max retry attempts for network downloads (default: 3)")
    args = parser.parse_args()

    manifest_path = Path(args.manifest)
    if not manifest_path.is_absolute():
        manifest_path = ROOT_DIR / manifest_path

    if not manifest_path.exists():
        print(f"Error: Manifest file not found: {manifest_path}", file=sys.stderr)
        sys.exit(1)

    with open(manifest_path, "r", encoding="utf-8") as f:
        entries = json.load(f)

    if args.category:
        entries = [e for e in entries if e.get("category") == args.category]
        if not entries:
            print(f"Error: No entries found for category '{args.category}'", file=sys.stderr)
            sys.exit(1)

    target_dir = Path(args.output_dir)
    if not target_dir.is_absolute():
        target_dir = ROOT_DIR / target_dir

    print(f"RBD OFIS Test Corpus Fetcher")
    print(f"Manifest:   {manifest_path} ({len(entries)} items)")
    print(f"Target dir: {target_dir}")
    if args.category:
        print(f"Filter:     category = {args.category}")
    print("-" * 65)

    stats = {
        "skipped": 0,
        "downloaded": 0,
        "generated": 0,
        "failed": 0,
        "bytes_downloaded": 0,
    }
    cat_stats = {}
    failures = []

    start_time = time.time()

    for i, entry in enumerate(entries, 1):
        cat = entry["category"]
        fid = entry["id"]
        if cat not in cat_stats:
            cat_stats[cat] = {"total": 0, "skipped": 0, "downloaded": 0, "generated": 0, "failed": 0, "bytes": 0}
        cat_stats[cat]["total"] += 1

        sys.stdout.write(f"[{i:3d}/{len(entries)}] {cat:<20} {fid:<30} ")
        sys.stdout.flush()

        try:
            status, bytes_dl = fetch_file(entry, target_dir, force=args.force, timeout=args.timeout, retries=args.retries)
            stats[status] += 1
            stats["bytes_downloaded"] += bytes_dl
            cat_stats[cat][status] += 1
            cat_stats[cat]["bytes"] += entry.get("size_bytes", 0)

            if status == "skipped":
                print("[SKIPPED (verified)]")
            elif status == "downloaded":
                print(f"[DOWNLOADED ({bytes_dl:,} b)]")
            elif status == "generated":
                print(f"[GENERATED ({bytes_dl:,} b)]")

        except Exception as e:
            stats["failed"] += 1
            cat_stats[cat]["failed"] += 1
            failures.append((entry, str(e)))
            print(f"[FAILED: {e}]")

    elapsed = time.time() - start_time

    print("\n" + "=" * 65)
    print("CATEGORY SUMMARY:")
    print(f"{'Category':<22} {'Total':>6} {'Cached':>7} {'Fetched':>8} {'Gen':>5} {'Fail':>5} {'Size (MB)':>10}")
    print("-" * 65)

    for cat in sorted(cat_stats.keys()):
        cs = cat_stats[cat]
        mb = cs["bytes"] / (1024 * 1024)
        print(f"{cat:<22} {cs['total']:>6} {cs['skipped']:>7} {cs['downloaded']:>8} {cs['generated']:>5} {cs['failed']:>5} {mb:>9.2f}")

    print("-" * 65)
    total_mb = sum(cs["bytes"] for cs in cat_stats.values()) / (1024 * 1024)
    print(f"{'TOTAL':<22} {len(entries):>6} {stats['skipped']:>7} {stats['downloaded']:>8} {stats['generated']:>5} {stats['failed']:>5} {total_mb:>9.2f}")
    print("=" * 65)
    print(f"Elapsed Time: {elapsed:.2f}s | Bytes Transferred: {stats['bytes_downloaded']:,} bytes")

    if failures:
        print("\nERRORS ENCOUNTERED:")
        for entry, err in failures:
            print(f"  - [{entry['category']}] {entry['id']}: {entry['url']} -> {err}")
        sys.exit(1)

    print("\nAll files successfully verified against manifest. Checksums intact.")
    sys.exit(0)


if __name__ == "__main__":
    main()
