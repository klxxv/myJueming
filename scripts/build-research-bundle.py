"""Build an independently installable offline resource bundle from managed inputs.

No system Python is used at runtime. --download-base optionally sets immutable
HTTPS file URLs for a thin installer; omit it for a fully offline installer.
"""
import argparse
import hashlib
import json
import os
import shutil
from pathlib import Path
from urllib.parse import quote

parser=argparse.ArgumentParser()
for name in ("runtime","packages","model","output"):
    parser.add_argument("--"+name,required=True,type=Path)
parser.add_argument("--download-base")
args=parser.parse_args()
if args.download_base and not args.download_base.startswith("https://"):
    raise SystemExit("--download-base must be immutable HTTPS hosting")
root=Path(__file__).resolve().parents[1]
destination=args.output.resolve()
destination.mkdir(parents=True,exist_ok=True)
for name,source in [("runtime",args.runtime),("packages",args.packages),("model",args.model)]:
    shutil.copytree(source,destination/name,dirs_exist_ok=True,ignore=shutil.ignore_patterns("__pycache__","*.pyc",".cache"))
# PyTorch's Windows wheel includes large static/import libraries for compiling
# C++ extensions. Our packaged worker only loads the DLLs and never compiles.
if (args.runtime/"python.exe").is_file():
    for library in (destination/"packages"/"torch"/"lib").glob("*.lib"):
        library.unlink()
for name in ("fuzzy-matching","xlmr-word-alignment"):
    shutil.copytree(root/"plugins"/name,destination/"plugins"/name,dirs_exist_ok=True,ignore=shutil.ignore_patterns("__pycache__","*.pyc"))
for name in ("THIRD_PARTY.md", "requirements-lock.txt"):
    shutil.copy2(root/"plugins"/name,destination/name)
for directory in destination.rglob("__pycache__"):
    shutil.rmtree(directory)
for bytecode in destination.rglob("*.pyc"):
    bytecode.unlink()
python="runtime/python.exe" if (destination/"runtime/python.exe").exists() else "runtime/bin/python3"
if not (destination/python).exists():
    raise SystemExit("runtime must be a complete relocatable managed Python distribution")
for name in ("model.safetensors","tokenizer.json","config.json"):
    if not (destination/"model"/name).exists():
        raise SystemExit("incomplete model: "+name)
files=[]
for path in sorted(destination.rglob("*")):
    if not path.is_file() or path.name=="bundle.json":
        continue
    relative=path.relative_to(destination).as_posix()
    if relative=="README.md": continue
    digest=hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda:stream.read(1024*1024),b""): digest.update(chunk)
    files.append({"path":relative,"sha256":digest.hexdigest(),"bytes":path.stat().st_size,"executable":bool(path.stat().st_mode&0o111),"url":args.download_base.rstrip("/")+"/"+quote(relative) if args.download_base else None})
manifest={"format_version":1,"release":"research-1.0.0","python":python,"packages":"packages","model":"model","model_revision":"e73636d4f797dec63c3081bb6ed5c7b0bb3f2089","files":files,"plugins":[json.loads((destination/"plugins"/name/"plugin.json").read_text()) for name in ("fuzzy-matching","xlmr-word-alignment")]}
(destination/"bundle.json").write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+"\n")
print(f"Built {len(files)} verified files, {sum(f['bytes'] for f in files):,} bytes at {destination}")
