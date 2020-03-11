use crate::QUiLoader;
use cpp_core::{CastInto, NullPtr, Ptr};
use qt_core::{QBox, QBuffer, QByteArray};
use qt_widgets::QWidget;

impl QUiLoader {
    #[inline]
    pub unsafe fn load_bytes(&mut self, bytes: &[u8]) -> QBox<QWidget> {
        self.load_bytes_with_parent(bytes, NullPtr)
    }

    pub unsafe fn load_bytes_with_parent(
        &mut self,
        bytes: &[u8],
        parent: impl CastInto<Ptr<QWidget>>,
    ) -> QBox<QWidget> {
        let mut byte_array = QByteArray::from_slice(bytes);
        let mut buffer = QBuffer::from_q_byte_array(&mut byte_array);
        self.load_2a(&mut buffer, parent).into_q_box()
    }
}
