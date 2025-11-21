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

RUN sed -i 's/"target-pointer-width": "64"/"target-pointer-width": 64/' x86_64-r2.json

RUN rustup target add x86_64-unknown-none && \
    cargo install bootimage --locked && \
    rustup component add rust-src --toolchain nightly-x86_64-unknown-linux-gnu && \
    rustup component add llvm-tools-preview --toolchain nightly-x86_64-unknown-linux-gnu

ENV RUST_BACKTRACE=1

RUN make build || echo "Build failed, but container will continue for debugging"

CMD ["bash"]
