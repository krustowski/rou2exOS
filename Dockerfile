FROM --platform=linux/amd64 ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y \
    git \
    curl \
    build-essential \
    xorriso \
    net-tools \
    grub-common \
    grub-pc-bin \
    qemu-system-x86 \
    mtools \
    lld \
    nasm \
    && rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain nightly
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /workspace
RUN git clone https://github.com/krustowski/rou2exOS.git .

# 타겟 스펙 파일 수정
RUN sed -i 's/"target-pointer-width": "64"/"target-pointer-width": 64/' x86_64-r2.json

# Rust 컴포넌트 및 타겟 설치
RUN rustup target add x86_64-unknown-none && \
    cargo install bootimage --locked && \
    rustup component add rust-src --toolchain nightly-x86_64-unknown-linux-gnu && \
    rustup component add llvm-tools-preview --toolchain nightly-x86_64-unknown-linux-gnu

# build.rs의 assertion 문제를 디버깅하기 위해 환경변수 설정
ENV RUST_BACKTRACE=1

# 빌드 시도 (실패할 수 있지만 더 자세한 오류 정보 확인)
RUN make build || echo "Build failed, but container will continue for debugging"

CMD ["bash"]