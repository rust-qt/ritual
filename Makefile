
OUTPUT_DIR ?= $(realpath ../generated_output)
QT_DOC_DIR ?= $(realpath ../qt-doc/html)
CMAKE_PREFIX_PATH ?= /home/ri/bin/Qt/5.5/gcc_64/lib/cmake

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

$(OUTPUT_DIR)/build_c_test: $(OUTPUT_DIR)/install_qtcw
	export LIBRARY_PATH=$(OUTPUT_DIR)/install_qtcw/lib && \
	export LD_LIBRARY_PATH=$(OUTPUT_DIR)/install_qtcw/lib && \
	export C_TEST_OUTPUT_DIR=$(OUTPUT_DIR)/build_c_test && \
	mkdir -p $(OUTPUT_DIR)/build_c_test && \
	cd c_test && \
	make 

clean: 
	rm -rv $(OUTPUT_DIR)/*

