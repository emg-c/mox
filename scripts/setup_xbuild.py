"""교차 빌드 도구를 /work/.mox-xtools 에 내려받는다. 이미 있으면 건너뛴다.

워크스테이션 컨테이너에서 실행한다: rdev run python:3.12-slim 'python3 -I /work/motion-capture/scripts/setup_xbuild.py'
"""
import hashlib
import os
import shutil
import sys
import tarfile
import urllib.request

ROOT = "/work/.mox-xtools"
TOOLS = [
    (
        "zig",
        "https://ziglang.org/download/0.16.0/zig-x86_64-linux-0.16.0.tar.xz",
        "70e49664a74374b48b51e6f3fdfbf437f6395d42509050588bd49abe52ba3d00",
    ),
    (
        "llvm-mingw",
        "https://github.com/mstorsjo/llvm-mingw/releases/download/20260922/llvm-mingw-20260922-ucrt-ubuntu-22.04-x86_64.tar.xz",
        "bb7bb7654b33d5aa8712acb837c963b2e0c56352560c76105270a3268c665c21",
    ),
    (
        "bin",
        "https://github.com/rust-cross/cargo-zigbuild/releases/download/v0.23.4/cargo-zigbuild-x86_64-unknown-linux-musl.tar.xz",
        "9e3cf73485edbd45905c8aadbc0fdf869c7ddc3848f0c898229f2680db52e44b",
    ),
]


def fetch(name, url, sha256):
    dest = os.path.join(ROOT, name)
    if os.path.isdir(dest):
        print(f"{name}: 이미 있음")
        return
    os.makedirs(ROOT, exist_ok=True)
    archive = dest + ".tar.xz"
    urllib.request.urlretrieve(url, archive)
    with open(archive, "rb") as f:
        digest = hashlib.sha256(f.read()).hexdigest()
    if digest != sha256:
        os.remove(archive)
        sys.exit(f"{name}: sha256 불일치 (받은 값 {digest})")
    staging = dest + ".staging"
    shutil.rmtree(staging, ignore_errors=True)
    with tarfile.open(archive) as t:
        t.extractall(staging, filter="data")
    entries = os.listdir(staging)
    inner = os.path.join(staging, entries[0]) if len(entries) == 1 else None
    if inner and os.path.isdir(inner):
        os.rename(inner, dest)  # 최상위 폴더 하나로 묶인 압축
        os.rmdir(staging)
    else:
        os.rename(staging, dest)  # 파일이 바로 들어 있는 압축 (cargo-zigbuild)
    os.remove(archive)
    print(f"{name}: {dest}")


for tool in TOOLS:
    fetch(*tool)
