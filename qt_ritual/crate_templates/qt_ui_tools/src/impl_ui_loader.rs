use crate::QUiLoader;
use cpp_core::{CastInto, NullPtr, Ptr};
use qt_core::{QBox, QBuffer, QByteArray};
use qt_widgets::QWidget;

impl QUiLoader {
    /// Loads a form from the given UI file data and creates a new widget
    /// to hold its contents.
    #[inline]
    pub unsafe fn load_bytes(&self, bytes: &[u8]) -> QBox<QWidget> {
        self.load_bytes_with_parent(bytes, NullPtr)
    }

    /// Loads a form from the given UI file data and creates a new widget
    /// with the given `parent` to hold its contents.
    pub unsafe fn load_bytes_with_parent(
        &self,
        bytes: &[u8],
        parent: impl CastInto<Ptr<QWidget>>,
    ) -> QBox<QWidget> {
        let byte_array = QByteArray::from_slice(bytes);
        let buffer = QBuffer::from_q_byte_array(&byte_array);
        self.load_2a(&buffer, parent).into_q_box()
    }
}
