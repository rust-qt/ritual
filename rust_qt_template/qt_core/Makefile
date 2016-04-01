
all: $(patsubst $(OUTPUT_DIR)/install_qtcw/include/qtcw_%.h, src/ffi/%.rs, $(wildcard $(OUTPUT_DIR)/install_qtcw/include/*.h)) 
	cargo test

src/ffi/global.rs:
	true
src/ffi/exports.rs:
	true 
src/ffi/sizes.rs:
	true 
src/ffi/enum_values.rs:
	true 

src/ffi/%.rs: $(OUTPUT_DIR)/install_qtcw/include/qtcw_%.h
	$(RUST_BINDGEN) -static-link qtcw -o $@ $< -DBINDGEN_RUNNING


