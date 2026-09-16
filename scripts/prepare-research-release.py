"""Prepare full Windows x64 or macOS Universal offline research resources.

Builder requirements: Python and uv 0.8.22. User installations need neither.
The generated manifest includes SHA-256 for each platform runtime/model file.
"""
import argparse
import json
import os
import platform
import shutil
import subprocess
import sys
import urllib.request
from pathlib import Path

parser=argparse.ArgumentParser()
parser.add_argument("--uv",default="uv")
parser.add_argument("--universal",action="store_true")
parser.add_argument("--output",type=Path,default=Path("apps/desktop/src-tauri/resources/research"))
args=parser.parse_args()
root=Path(__file__).resolve().parents[1]
cache=root/"target/research-release"
cache.mkdir(parents=True,exist_ok=True)
system="windows" if sys.platform=="win32" else "macos"
architectures=["x86_64","aarch64"] if args.universal else ["aarch64" if platform.machine().lower() in ("arm64","aarch64") else "x86_64"]
revision="e73636d4f797dec63c3081bb6ed5c7b0bb3f2089"
model=cache/"model"
model.mkdir(exist_ok=True)
for name in ["config.json","model.safetensors","tokenizer.json","tokenizer_config.json","sentencepiece.bpe.model","README.md"]:
    path=model/name
    if not path.exists():
        temporary=path.with_suffix(path.suffix+".partial")
        urllib.request.urlretrieve(f"https://huggingface.co/FacebookAI/xlm-roberta-base/resolve/{revision}/{name}",temporary)
        temporary.replace(path)
env={**os.environ,"UV_PYTHON_INSTALL_DIR":str(cache/"python")}
for architecture in architectures:
    target=f"cpython-3.12.11-{system}-{architecture}-"+"none"
    subprocess.run([args.uv,"python","install",target,"--no-bin","--no-registry"],env=env,check=True)
    runtime=cache/"python"/target
    if not runtime.is_dir():
        matches=list((cache/"python").glob(f"cpython-3.12.11-{system}-{architecture}-*"))
        if len(matches)!=1: raise SystemExit("managed runtime not found")
        runtime=matches[0]
    packages=cache/("packages-"+architecture)
    triple=f"{architecture}-"+("pc-windows-msvc" if system=="windows" else "apple-darwin")
    install=[args.uv,"pip","install","--python-version","3.12","--python-platform",triple,"--only-binary",":all:","--target",str(packages),"-r",str(root/"plugins/requirements-lock.txt")]
    if system=="windows":
        # The worker runs on CPU. CUDA libraries exceed NSIS's 2 GiB data limit.
        # Pin the CPython 3.12 x64 CPU wheel and its official SHA-256.
        install.append("torch @ https://download-r2.pytorch.org/whl/cpu/torch-2.2.2%2Bcpu-cp312-cp312-win_amd64.whl#sha256=2b0cf041f878607a361116945f82ce2dba4b7a747151da7619a63cb5fccb72df")
    subprocess.run(install,env=env,check=True)
    destination=args.output/architecture if args.universal else args.output
    subprocess.run([sys.executable,str(root/"scripts/build-research-bundle.py"),"--runtime",str(runtime),"--packages",str(packages),"--model",str(model),"--output",str(destination)],check=True)
if args.universal:
    # Platform manifests share one model and plugin source tree; no duplicated 1.1 GB weights.
    for shared in ("model", "plugins"):
        target=args.output/shared
        if target.exists(): shutil.rmtree(target)
        shutil.move(str(args.output/architectures[0]/shared),str(target))
    for architecture in architectures:
        directory=args.output/architecture
        manifest=json.loads((directory/"bundle.json").read_text())
        for item in manifest["files"]:
            if not item["path"].startswith(("model/", "plugins/")):
                item["path"]=architecture+"/"+item["path"]
        manifest["python"]=architecture+"/"+manifest["python"]
        manifest["packages"]=architecture+"/"+manifest["packages"]
        (args.output/("bundle."+architecture+".json")).write_text(json.dumps(manifest,indent=2)+"\n")
        (directory/"bundle.json").unlink()
        for shared in ("model", "plugins"):
            if (directory/shared).exists(): shutil.rmtree(directory/shared)
print(json.dumps({"prepared_architectures":architectures,"output":str(args.output)}))
