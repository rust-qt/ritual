all: $(patsubst %.c, run_$(C_TEST_OUTPUT_DIR)/%, $(wildcard *.c))

$(C_TEST_OUTPUT_DIR)/%: %.c
	$(CC) $< -o $@ -I$(OUTPUT_DIR)/install_qtcw/include -lqtcw

.PHONY:run_$(C_TEST_OUTPUT_DIR)/%
run_$(C_TEST_OUTPUT_DIR)/%: $(C_TEST_OUTPUT_DIR)/%
	$<

