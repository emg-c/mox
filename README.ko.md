# Mox

한국어 · **[English](README.md)**

Mox는 **macOS**와 **Windows**에서 쓰는 크로스플랫폼 마우스·키보드 매크로 녹화·재생기입니다.
하던 작업을 녹화하고, 딜레이를 조정하고, 다시 재생하고, 더블클릭으로 실행되는 **단독 실행 파일로 추출**까지 — GUI와 CLI를 갖춘 하나의 도구로 할 수 있습니다.

> 상태: 초기(알파). 핵심 기능은 macOS(Apple Silicon)와 Windows 10 x64 양쪽에서 실기 검증을 마쳤지만, 패키징과 마감은 진행 중입니다.

## 기능

- 마우스 이동·클릭(좌/우/가운데/뒤로/앞으로)·휠·키보드 입력을 밀리초 단위로 **녹화**.
- 녹화한 로그 **편집**:
  - 딜레이 배율 조정 / 상한 지정 / 일괄 지정 (재생을 빠르게·느리게)
  - 마우스 이동 이벤트 솎기 (절반 / 간격 기준 / 거리 기준)
  - 이동만 삭제(클릭 유지) 또는 마우스 이벤트 전부 삭제
  - 절대 ↔ 비율 좌표 변환 (다른 해상도에서도 재생되도록)
  - 키 자동반복 제거, Cmd ↔ Ctrl 맞바꿈 (macOS ↔ Windows 매크로 이식)
- 반복 횟수·배속·"원래 타이밍 유지" 옵션으로 **재생**. 중지 키나 마우스를 화면 모서리로 밀기(비상 정지)로 언제든 멈춤.
- 매크로를 Mox가 없는 PC에서도 도는 **단독 실행 파일**(Windows `.exe` / macOS `.app`(zip))로 **추출**. **한 OS에서 반대쪽 OS용까지** 추출 가능.
- 녹화하면 실시간으로 채워지는 이벤트 표가 있는 **GUI**(egui)와, 스크립트용 **CLI**.
- 사람이 바로 열어 고칠 수 있는 `.mox` 파일 형식(UTF-8 JSON).

## 지원 플랫폼

| | 녹화·재생 | 단독 추출 |
|---|---|---|
| macOS 12+ (Apple Silicon / Intel, universal) | ✅ | `.app` (zip) |
| Windows 10/11 x64 | ✅ | `.exe` |

## 설치·빌드

Mox는 Rust 워크스페이스입니다. 각 OS에서 최신 Rust 툴체인(1.90+)으로 네이티브 빌드합니다.

```sh
# 메인 앱(GUI + CLI). 인자 없이 `mox`를 실행하면 GUI가 열립니다.
cargo build --release -p mox-app
```

**추출** 기능은 재생기 바이너리를 앱에 내장합니다. 쓰려면 대상별 재생기를 먼저 빌드하고, 앱 빌드에 경로를 알려 줍니다.

```sh
# 헤드리스 재생기 빌드(대상별; 교차 빌드 또는 각 OS에서)
cargo build --release -p mox-player --target x86_64-pc-windows-gnullvm   # → mox-player.exe
cargo build --release -p mox-player --target aarch64-apple-darwin        # (lipo로 universal)

# 재생기를 내장해 앱 빌드
MOX_WIN_PLAYER=/경로/mox-player.exe \
MOX_MAC_PLAYER=/경로/mox-player-macos-universal \
cargo build --release -p mox-app
```

환경변수를 생략해도 앱은 빌드됩니다 — 해당 대상 추출만 안내 메시지와 함께 비활성화됩니다.

> CLI만 빌드할 때(GUI 제외, 예: Linux CI): `cargo build -p mox-app --no-default-features`.

## CLI

```sh
mox                                   # GUI 실행
mox record -o work.mox                # 중지 키를 누를 때까지 녹화
mox play work.mox --repeat 3 --speed 2
mox play work.mox --no-timing         # 최대 속도로 재생
mox edit work.mox scale-delay 0.5     # 모든 딜레이 절반
mox edit work.mox cap-delay 500       # 500ms 넘는 대기를 500ms로
mox edit work.mox thin-half           # 마우스 이동 하나 걸러 삭제
mox edit work.mox to-ratio            # 절대 → 비율 좌표
mox export work.mox --target windows -o work.exe
mox export work.mox --target macos   -o work.app.zip
mox selftest                          # 입력 계층 자가 진단
```

전체 옵션은 `mox <명령> --help`. 편집 명령: `scale-delay`, `cap-delay`, `set-delay`, `thin-half`, `thin-interval`, `thin-distance`, `rm-moves`, `rm-mouse`, `to-ratio`, `to-abs`, `rm-key-repeats`, `swap-cmd-ctrl`.

## `.mox` 파일 형식

UTF-8 JSON, 이벤트는 한 줄에 하나. 딜레이(`d`)는 직전 이벤트와의 간격(ms)이라, 한 딜레이를 고쳐도 뒤 이벤트들이 밀리지 않습니다.

```json
{
  "format": "mox",
  "version": 1,
  "meta": { "name": "주간보고", "created": "2026-10-10T14:03:00Z", "recorded_on": "macos",
            "screen": { "width": 1512, "height": 982, "scale": 2.0, "unit": "point" } },
  "play": { "repeat": 1, "speed": 1.0, "timed": true, "min_gap_ms": 5 },
  "events": [
    {"d": 0,   "type": "move",     "x": 512, "y": 384},
    {"d": 120, "type": "down",     "button": "left", "x": 512, "y": 384},
    {"d": 800, "type": "key_down", "key": "KeyA"}
  ]
}
```

키 이름은 웹 표준 `KeyboardEvent.code`(`KeyA`, `Enter`, `ShiftLeft`, `MetaLeft` …)를 따르므로, 매크로가 키보드 배열이나 언어에 묶이지 않습니다.

## 권한·플랫폼 참고

- **macOS**: 녹화에는 **입력 모니터링**, 재생에는 **손쉬운 사용** 권한이 필요합니다(시스템 설정 → 개인정보 보호 및 보안). 권한이 없으면 Mox가 해당 설정 창을 열어 줍니다.
- **macOS 추출 앱**은 ad-hoc 서명이며 공증(notarize)은 안 돼 있습니다. 받은 사람은 처음 실행 시 *시스템 설정 → 개인정보 보호 및 보안 → 그래도 열기*로 허용하고, 앱마다 손쉬운 사용 권한을 줘야 합니다.
- **Windows**: 입력 주입과 저수준 훅은 **로그인된 대화형 데스크톱 세션**에서만 동작합니다. 추출한 `.exe`를 더블클릭하면 되지만, 순수 SSH 세션에서 실행하면 입력이 먹히지 않습니다.
- **좌표**는 기본적으로 절대 픽셀/포인트로 녹화됩니다. 비율 좌표로 변환(메뉴 또는 `to-ratio`)하면 다른 화면 크기에서 재생하기 좋습니다.

## 구조

목적별로 나뉜 Cargo 워크스페이스:

| 크레이트 | 역할 |
|---|---|
| `mox-format` | `.mox` 데이터 모델, 읽기/쓰기, 검증 |
| `mox-edit` | 순수 편집 함수 + 되돌리기/다시하기 |
| `mox-engine` | `InputBackend`/`Clock` 트레이트 위의 녹화·재생 로직 (OS 무관, 단위 테스트 완비) |
| `mox-input` | OS 입력 계층: macOS는 Core Graphics, Windows는 `windows-sys` |
| `mox-payload` / `mox-export` | 단독 실행 파일 패키징 & ad-hoc 서명 |
| `mox-player` | 추출 실행 파일에 내장되는 헤드리스 재생기 |
| `mox-app` | `mox` 바이너리: egui GUI + CLI |

OS 의존 코드는 `mox-input`에만 있고, 나머지는 전부 이식 가능하며 Linux에서 테스트됩니다.

## 라이선스

미정. 라이선스가 추가되기 전까지는 어떠한 사용 권리도 부여되지 않습니다.
