FROM debian:stretch as common
ENV APT_KEY_DONT_WARN_ON_DANGEROUS_USAGE=DontWarn
RUN apt-get update && \
    apt-get install -y build-essential mesa-common-dev libgl1-mesa-glx \
                       cmake curl software-properties-common && \
    curl https://apt.llvm.org/llvm-snapshot.gpg.key -sSf | apt-key add - && \
    add-apt-repository "deb http://apt.llvm.org/stretch/ llvm-toolchain-stretch-6.0 main" && \
    apt-get update


FROM common as builder
RUN apt-get install -y libsqlite3-dev libclang-6.0-dev && \
    curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain stable-2019-04-11 -y

ENV LIBCLANG_PATH=/usr/lib/llvm-6.0/lib
ENV PATH=/root/.cargo/bin:$PATH
COPY . /app
WORKDIR /app
RUN cargo build --bin cluster_worker


FROM common as qt_downloader
RUN apt-get install -y python3-bs4 p7zip-full
RUN mkdir -p /opt/qt
WORKDIR /opt/qt
COPY scripts/install_qt.py /
RUN /install_qt.py


FROM common
RUN apt-get install -y libsqlite3-0 libclang1-6.0
COPY --from=qt_downloader /opt/qt /opt/qt
COPY --from=builder /app/target/debug/cluster_worker /root
COPY . /app

ENV QT_RITUAL_QMAKE_5_9_7=/opt/qt/5.9.7/gcc_64/bin/qmake
ENV QT_RITUAL_QMAKE_5_11_3=/opt/qt/5.11.3/gcc_64/bin/qmake
ENV QT_RITUAL_QMAKE_5_12_2=/opt/qt/5.12.2/gcc_64/bin/qmake
ENV QT_RITUAL_QMAKE_5_13_0=/opt/qt/5.13.0/gcc_64/bin/qmake

ENV CMAKE_PREFIX_PATH=/opt/qt/5.13.0/gcc_64/lib/cmake/Qt5Core

ENV RUST_BACKTRACE=1
ENV QT_RITUAL_WORKER_QUEUE_ADDRESS=amqp://localhost//
ENV QT_RITUAL_WORKER_RUN_TESTS=0
CMD /root/cluster_worker
