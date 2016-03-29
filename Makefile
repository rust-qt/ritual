
OUTPUT_DIR ?= $(realpath ../generated_output)
QT_DOC_DIR ?= $(realpath ../qt-doc/html)
QT_LIB_DIR ?= /home/ri/bin/Qt/5.5/gcc_64/lib

CMAKE_PREFIX_PATH = $(QT_LIB_DIR)/cmake

all: $(OUTPUT_DIR)/build_c_test

$(OUTPUT_DIR)/doc_parse_result.json: $(wildcard $(QT_DOC_DIR)/**/*) qt_doc_parser.py
	mkdir -p $(OUTPUT_DIR)
	./qt_doc_parser.py $(QT_DOC_DIR) $(OUTPUT_DIR)/doc_parse_result.json

$(OUTPUT_DIR)/qtcw: $(OUTPUT_DIR)/doc_parse_result.json $(wildcard qt_wrapper_generator/src/*.rs $(wildcard qtcw_template/**/*))
	cd qt_wrapper_generator && \
	cargo run ../qtcw_template $(OUTPUT_DIR)

$(OUTPUT_DIR)/install_qtcw: $(OUTPUT_DIR)/qtcw
	mkdir -p $(OUTPUT_DIR)/build_qtcw
	cd $(OUTPUT_DIR)/build_qtcw && \
	cmake ../qtcw -DCMAKE_PREFIX_PATH=$(CMAKE_PREFIX_PATH) -DCMAKE_INSTALL_PREFIX=$(OUTPUT_DIR)/install_qtcw && \
	make install

.PHONY: $(OUTPUT_DIR)/build_c_test
$(OUTPUT_DIR)/build_c_test: $(OUTPUT_DIR)/install_qtcw
	mkdir -p $(OUTPUT_DIR)/build_c_test
	export LIBRARY_PATH=$(OUTPUT_DIR)/install_qtcw/lib:$(QT_LIB_DIR) && \
	export LD_LIBRARY_PATH=$(OUTPUT_DIR)/install_qtcw/lib:$(QT_LIB_DIR) && \
	export C_TEST_OUTPUT_DIR=$(OUTPUT_DIR)/build_c_test && \
	export OUTPUT_DIR=$(OUTPUT_DIR) && \
	$(MAKE) -C c_test

clean: 
	rm -rv $(OUTPUT_DIR)/*

clean_rust:
	rm -rv $(OUTPUT_DIR)/qtcw
	rm -rv $(OUTPUT_DIR)/install_qtcw

