FROM debian:stretch as common
ENV APT_KEY_DONT_WARN_ON_DANGEROUS_USAGE=DontWarn
RUN apt-get update && \
    apt-get install -y build-essential cmake curl software-properties-common && \
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


FROM common
RUN apt-get update
RUN apt-get install -y libsqlite3-0 libclang1-6.0
COPY --from=builder /app/target/debug/cluster_worker /root

ENV RUST_BACKTRACE=1
ENV QT_RITUAL_WORKER_QUEUE_ADDRESS=amqp://localhost//
ENV QT_RITUAL_WORKER_RUN_TESTS=1

RUN mkdir /root/qt_archives
WORKDIR /root/qt_archives

RUN curl http://download.qt.io/online/qtsdkrepository/linux_x64/desktop/qt5_597/qt.qt5.597.gcc_64/5.9.7-0-201810181452qtbase-Linux-RHEL_7_4-GCC-Linux-RHEL_7_4-X86_64.7z -sSfOJL
RUN curl http://download.qt.io/online/qtsdkrepository/linux_x64/desktop/qt5_5113/qt.qt5.5113.gcc_64/5.11.3-0-201811291858qtbase-Linux-RHEL_7_4-GCC-Linux-RHEL_7_4-X86_64.7z -sSfOJL
RUN curl http://download.qt.io/online/qtsdkrepository/linux_x64/desktop/qt5_5122/qt.qt5.5122.gcc_64/5.12.2-0-201903121858qtbase-Linux-RHEL_7_4-GCC-Linux-RHEL_7_4-X86_64.7z -sSfOJL
RUN curl http://download.qt.io/online/qtsdkrepository/linux_x64/desktop/qt5_5130/qt.qt5.5130.gcc_64/5.13.0-0-201903150614qtbase-Linux-RHEL_7_4-GCC-Linux-RHEL_7_4-X86_64.7z -sSfOJL
RUN curl http://download.qt.io/online/qtsdkrepository/linux_x64/desktop/qt5_597/qt.qt5.597.gcc_64/5.9.7-0-201810181452icu-linux-Rhel7.2-x64.7z -sSfOJL
RUN curl http://download.qt.io/online/qtsdkrepository/linux_x64/desktop/qt5_597/qt.qt5.597.gcc_64/5.9.7-0-201810181452qttools-Linux-RHEL_7_4-GCC-Linux-RHEL_7_4-X86_64.7z -sSfOJL
RUN curl http://download.qt.io/online/qtsdkrepository/linux_x64/desktop/qt5_597/qt.qt5.597.gcc_64/5.9.7-0-201810181452qt3d-Linux-RHEL_7_4-GCC-Linux-RHEL_7_4-X86_64.7z -sSfOJL
RUN curl http://download.qt.io/online/qtsdkrepository/linux_x64/desktop/qt5_597/qt.qt5.597.gcc_64/5.9.7-0-201810181452qtgamepad-Linux-RHEL_7_4-GCC-Linux-RHEL_7_4-X86_64.7z -sSfOJL


RUN mkdir -p /opt/qt
RUN apt-get install -y p7zip-full

WORKDIR /opt/qt
RUN for i in /root/qt_archives/*; do 7z x "$i"; done
RUN echo "[Paths]\nPrefix = /opt/qt/5.9.7/gcc_64" > /opt/qt/5.9.7/gcc_64/bin/qt.conf
RUN echo "[Paths]\nPrefix = /opt/qt/5.11.3/gcc_64" > /opt/qt/5.11.3/gcc_64/bin/qt.conf
RUN echo "[Paths]\nPrefix = /opt/qt/5.12.2/gcc_64" > /opt/qt/5.12.2/gcc_64/bin/qt.conf
RUN echo "[Paths]\nPrefix = /opt/qt/5.13.0/gcc_64" > /opt/qt/5.13.0/gcc_64/bin/qt.conf

RUN /opt/qt/5.9.7/gcc_64/bin/qmake -query
ENV QT_RITUAL_QMAKE_5_9_7=/opt/qt/5.9.7/gcc_64/bin/qmake

COPY . /app

RUN apt-get install -y mesa-common-dev libgl1-mesa-glx
CMD /root/cluster_worker
