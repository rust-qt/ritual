all: $(C_TEST_OUTPUT_DIR)/c_empty_test $(C_TEST_OUTPUT_DIR)/c_qpoint_test $(C_TEST_OUTPUT_DIR)/c_qrect_test

$(C_TEST_OUTPUT_DIR)/c_empty_test: c_empty_test.c
	$(CC) c_empty_test.c -o $(C_TEST_OUTPUT_DIR)/c_empty_test -I$(OUTPUT_DIR)/install_qtcw/include -lqtcw

$(C_TEST_OUTPUT_DIR)/c_qpoint_test: c_qpoint_test.c
	$(CC) c_qpoint_test.c -o $(C_TEST_OUTPUT_DIR)/c_qpoint_test -I$(OUTPUT_DIR)/install_qtcw/include -lqtcw

$(C_TEST_OUTPUT_DIR)/c_qrect_test: c_qrect_test.c
	$(CC) c_qrect_test.c -o $(C_TEST_OUTPUT_DIR)/c_qrect_test -I$(OUTPUT_DIR)/install_qtcw/include -lqtcw

