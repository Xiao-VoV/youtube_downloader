#!/usr/bin/env python3
"""Create 200 synthetic metadata-only records at a new explicit SQLite path.

Never point this tool at application history. No media files or user data are read.
Example: python3 scripts/seed_history.py --output /tmp/shiguang-demo.sqlite3
"""
import argparse
import json
import os
from pathlib import Path
import sqlite3


def seed(output: Path) -> None:
    output = output.expanduser()
    if output.exists() or output.is_symlink():
        raise ValueError("Destination must not exist, including symlinks")
    output = output.resolve()
    protected = (Path.home() / "Library/Application Support/Shiguang").resolve()
    if output == protected or protected in output.parents:
        raise ValueError("Refusing the real application history directory")
    if not output.parent.is_dir():
        raise ValueError("Parent directory must already exist")
    # Exclusive creation refuses files, directories and symlinks, never replaces.
    descriptor = os.open(output, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
    os.close(descriptor)
    connection = sqlite3.connect(output)
    try:
        connection.execute("PRAGMA foreign_keys=ON")
        with connection:
            connection.execute("CREATE TABLE records(id TEXT PRIMARY KEY,title TEXT NOT NULL,url TEXT NOT NULL,started_at INTEGER NOT NULL,finished_at INTEGER,status TEXT NOT NULL,metadata TEXT NOT NULL)")
            connection.execute("CREATE INDEX records_started ON records(started_at)")
            connection.execute("CREATE TABLE artifacts(record_id TEXT NOT NULL REFERENCES records(id) ON DELETE CASCADE,ordinal INTEGER NOT NULL,path TEXT NOT NULL,size INTEGER NOT NULL,PRIMARY KEY(record_id,ordinal))")
            connection.execute("CREATE TABLE repository_flags(key TEXT PRIMARY KEY,value TEXT NOT NULL)")
            connection.execute("INSERT INTO repository_flags VALUES('legacy_imported','1')")
            connection.execute("PRAGMA user_version=1")
            statuses = ["Succeeded", "Failed", "Cancelled", "Interrupted", "Running"]
            for i in range(200):
                status = statuses[i % len(statuses)]
                height = [360, 720, 1080, 1440, 2160][i % 5]
                suffix = "mp4" if i % 3 else "mkv"
                metadata = {
                    "source_site": ["youtube", "YouTube", "demo-site"][i % 3],
                    "media_id": f"demo-{i:03d}",
                    "channel": f"Synthetic Channel {i % 12:02d}",
                    "duration": 35.5 + i * 7,
                    "original_url": "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
                    "requested_height": height,
                    "best_resolution": i % 9 == 0,
                    "requested_format": "Video",
                    "output_dir": "/tmp/shiguang-synthetic-media",
                    "width": height * 16 // 9,
                    "height": height,
                    "ext": suffix,
                    "vcodec": "h264" if suffix == "mp4" else "vp9",
                    "acodec": "aac" if suffix == "mp4" else "opus",
                }
                title = f"{199-i:03d} · 合成历史 Synthetic media — " + ("Long title for truncation / rendering checks " * (1 + i % 5))
                started = 1_800_000_000 + i * 3700
                connection.execute(
                    "INSERT INTO records VALUES(?,?,?,?,?,?,?)",
                    (f"synthetic-{i:03d}", title, metadata["original_url"], started,
                     None if status == "Running" else started + 30 + i,
                     status, json.dumps(metadata, ensure_ascii=False)),
                )
                if status == "Succeeded":
                    connection.execute("INSERT INTO artifacts VALUES(?,?,?,?)", (f"synthetic-{i:03d}", 0, f"/tmp/shiguang-synthetic-media/demo-{i:03d}.{suffix}", 1_000_000 + i * 735_111))
        print(f"Created 200 synthetic entries: {output}")
        print("Running entries intentionally become Interrupted when loaded by the app.")
    finally:
        connection.close()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True, help="Explicit nonexistent destination; parent must exist")
    args = parser.parse_args()
    try:
        seed(args.output)
    except (ValueError, OSError, sqlite3.Error) as error:
        parser.exit(1, f"Seed refused/failed: {error}\n")


if __name__ == "__main__":
    main()
