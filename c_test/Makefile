CC = gcc
LINKED_LIBS = -lQtCore -lstdc++ 

all: $(patsubst %.c, run_$(C_TEST_OUTPUT_DIR)/%, $(wildcard *.c))

$(C_TEST_OUTPUT_DIR)/%: %.c
	$(CC) $< -o $@ $(OUTPUT_DIR)/install_qtcw/lib/libqtcw.a -I$(OUTPUT_DIR)/install_qtcw/include $(LINKED_LIBS)

.PHONY:run_$(C_TEST_OUTPUT_DIR)/%
run_$(C_TEST_OUTPUT_DIR)/%: $(C_TEST_OUTPUT_DIR)/%
	$<

