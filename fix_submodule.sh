#!/bin/bash

# 서브모듈 문제 해결 스크립트
# 팀원들이 실행하면 자동으로 해결됩니다

echo "🔧 서브모듈 문제 해결 중..."
echo ""

# 1. 기존 잘못된 서브모듈 정보 제거
echo "1️⃣ 기존 서브모듈 정보 정리..."
git rm -rf --cached bitvmx_protocol 2>/dev/null || true
rm -rf bitvmx_protocol
rm -rf .git/modules/bitvmx_protocol

# 2. Git 인덱스 정리
echo "2️⃣ Git 인덱스 정리..."
git config --remove-section submodule.bitvmx_protocol 2>/dev/null || true

# 3. 최신 상태로 업데이트
echo "3️⃣ 최신 상태로 업데이트..."
git fetch --all
git reset --hard origin/dev

# 4. 서브모듈 새로 초기화
echo "4️⃣ 서브모듈 초기화..."
git submodule init
git submodule update

echo ""
echo "✅ 완료! bitvmx_protocol이 정상적으로 설정되었습니다."
echo ""
echo "확인:"
ls -la bitvmx_protocol/