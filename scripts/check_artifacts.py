"""교차 빌드한 재생기를 검사한다.

- Windows: x86_64 PE이고, 시스템 DLL 외에 따로 들고 다녀야 할 DLL(libunwind, libc++ 등)을 쓰지 않는지
- macOS: universal(fat) Mach-O에 arm64와 x86_64가 모두 있는지

사용법: python3 check_artifacts.py <windows.exe> <macos-binary>   (pefile 필요)
"""
import struct
import sys

import pefile

BUNDLED_DLL_HINTS = ("libunwind", "libc++", "libgcc", "libwinpthread", "libstdc++")
CPU_X86_64 = 0x01000007
CPU_ARM64 = 0x0100000C


def check_pe(path):
    pe = pefile.PE(path)
    ok = pe.FILE_HEADER.Machine == 0x8664
    dlls = sorted(e.dll.decode().lower() for e in getattr(pe, "DIRECTORY_ENTRY_IMPORT", []))
    bad = [d for d in dlls if d.startswith(BUNDLED_DLL_HINTS)]
    print(f"PE {path}: machine={hex(pe.FILE_HEADER.Machine)} imports={dlls}")
    if bad:
        print(f"  FAIL: 함께 배포해야 하는 DLL에 의존함: {bad}")
    return ok and not bad


def check_fat(path):
    with open(path, "rb") as f:
        head = f.read(8)
        magic, n = struct.unpack(">II", head)
        if magic != 0xCAFEBABE:
            print(f"Mach-O {path}: FAIL universal 아님 (magic={hex(magic)})")
            return False
        cpus = [struct.unpack(">iiIII", f.read(20))[0] & 0xFFFFFFFF for _ in range(n)]
    names = {CPU_X86_64: "x86_64", CPU_ARM64: "arm64"}
    print(f"Mach-O {path}: archs={[names.get(c, hex(c)) for c in cpus]}")
    return CPU_X86_64 in cpus and CPU_ARM64 in cpus


ok = check_pe(sys.argv[1]) & check_fat(sys.argv[2])
print("OK" if ok else "FAIL")
sys.exit(0 if ok else 1)
