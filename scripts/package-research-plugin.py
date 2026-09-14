"""Produce a standalone local algorithm package directory with file integrity data."""
import argparse
import hashlib
import json
import shutil
from pathlib import Path
parser=argparse.ArgumentParser()
parser.add_argument("source",type=Path)
parser.add_argument("destination",type=Path)
args=parser.parse_args()
if args.destination.exists(): raise SystemExit("destination must be new")
manifest=json.loads((args.source/"plugin.json").read_text())
shutil.copytree(args.source,args.destination,ignore=shutil.ignore_patterns("__pycache__","*.pyc"))
files=[]
for path in sorted(args.destination.rglob("*")):
    if not path.is_file() or path.name=="plugin.json":continue
    files.append({"path":path.relative_to(args.destination).as_posix(),"bytes":path.stat().st_size,"sha256":hashlib.sha256(path.read_bytes()).hexdigest(),"executable":False,"url":None})
manifest["files"]=files
(args.destination/"plugin.json").write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+"\n")
print(args.destination.resolve())
