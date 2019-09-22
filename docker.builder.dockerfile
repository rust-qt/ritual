FROM debian:buster as ritual_builder
ENV APT_KEY_DONT_WARN_ON_DANGEROUS_USAGE=DontWarn
RUN apt-get update && \
    apt-get install -y build-essential mesa-common-dev libgl1-mesa-glx \
                       cmake curl software-properties-common && \
    curl https://apt.llvm.org/llvm-snapshot.gpg.key -sSf | apt-key add - && \
    add-apt-repository "deb http://apt.llvm.org/buster/ llvm-toolchain-buster main" && \
    apt-get update && \
    apt-get install -y libsqlite3-dev libclang-6.0-dev
ENV LIBCLANG_PATH=/usr/lib/llvm-6.0/lib

COPY rust-toolchain /tmp/rust-toolchain
RUN curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain $(cat /tmp/rust-toolchain) -y
ENV PATH=/root/.cargo/bin:$PATH
RUN rustup component add rustfmt

ENV CARGO_HOME=/build/cargo_home
ENV CARGO_TARGET_DIR=/build/target
ENV RITUAL_WORKSPACE_TARGET_DIR=/build/workspace_target
ENV RITUAL_STD_HEADERS=/usr/include/c++/8
