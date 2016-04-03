
OUTPUT_DIR ?= $(realpath ../generated_output)
QT_DOC_DIR ?= $(realpath ../qt-doc/html)
QT_LIB_DIR ?= /home/ri/bin/Qt/5.5/gcc_64/lib
RUST_BINDGEN ?= /home/ri/rust/rust_qt/rust-bindgen/target/debug/bindgen
QTCW_MAKE_OPTIONS ?= -j10

CMAKE_PREFIX_PATH = $(QT_LIB_DIR)/cmake

all: build_rust_qt

$(OUTPUT_DIR)/doc_parse_result.json: $(wildcard $(QT_DOC_DIR)/**/*) qt_doc_parser.py
	echo "TARGET1"
	mkdir -p $(OUTPUT_DIR)
	./qt_doc_parser.py $(QT_DOC_DIR) $(OUTPUT_DIR)/doc_parse_result.json

$(OUTPUT_DIR)/qtcw: $(OUTPUT_DIR)/doc_parse_result.json $(wildcard qt_wrapper_generator/src/*.rs $(wildcard qtcw_template/**/*) $(wildcard rust_qt_template/**/*))
	echo "TARGET2"
	rm -rf $(OUTPUT_DIR)/qtcw
	cp -r qtcw_template $(OUTPUT_DIR)/qtcw
	rm -rf $(OUTPUT_DIR)/rust_qt
	cp -r rust_qt_template $(OUTPUT_DIR)/rust_qt
	cd qt_wrapper_generator && \
	cargo run $(OUTPUT_DIR)/doc_parse_result.json $(OUTPUT_DIR)/qtcw $(OUTPUT_DIR)/rust_qt

$(OUTPUT_DIR)/install_qtcw: $(OUTPUT_DIR)/qtcw
	echo "TARGET3"
	mkdir -p $(OUTPUT_DIR)/build_qtcw
	rm -rf $(OUTPUT_DIR)/install_qtcw
	cd $(OUTPUT_DIR)/build_qtcw && \
	export LIBRARY_PATH=$(QT_LIB_DIR) && \
	export LD_LIBRARY_PATH=$(QT_LIB_DIR) && \
	cmake ../qtcw -DCMAKE_PREFIX_PATH=$(CMAKE_PREFIX_PATH) -DCMAKE_INSTALL_PREFIX=$(OUTPUT_DIR)/install_qtcw && \
	make $(QTCW_MAKE_OPTIONS) install

.PHONY: run_c_tests
run_c_tests: $(OUTPUT_DIR)/install_qtcw
	echo "TARGET4"
	mkdir -p $(OUTPUT_DIR)/build_c_test
	export LIBRARY_PATH=$(QT_LIB_DIR) && \
	export LD_LIBRARY_PATH=$(QT_LIB_DIR) && \
	export C_TEST_OUTPUT_DIR=$(OUTPUT_DIR)/build_c_test && \
	export OUTPUT_DIR=$(OUTPUT_DIR) && \
	$(MAKE) -C c_test



.PHONY: build_rust_qt
build_rust_qt: run_c_tests
	echo "TARGET5"
	cd $(OUTPUT_DIR)/rust_qt/qt_core && \
	export OUTPUT_DIR=$(OUTPUT_DIR) && \
	export RUST_BINDGEN=$(RUST_BINDGEN) && \
	make

clean: 
	rm -rv $(OUTPUT_DIR)/*

clean_rust:
	rm -rv $(OUTPUT_DIR)/qtcw
	rm -rv $(OUTPUT_DIR)/install_qtcw


