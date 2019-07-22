use crate::{MutPtr, MutRef, Ref};
use std::ops::{Deref, DerefMut};
use std::{fmt, mem, ptr};

/// Indicates that the type can be put into a CppBox.
///
/// Example of implementation:
/// ```
/// use cpp_utils::{CppDeletable};
///
/// struct Struct1;
///
/// unsafe extern "C" fn struct1_delete(this_ptr: *mut Struct1) {
///     unimplemented!()
/// }
///
/// impl CppDeletable for Struct1 {
///     unsafe fn delete(&mut self) {
///         struct1_delete(self)
///     }
/// }
/// ```
pub trait CppDeletable: Sized {
    /// Calls C++ `delete x` on `self`.
    unsafe fn delete(&mut self);
}

/// A C++ pointer wrapper to manage deletion of objects.
///
/// Objects of CppBox should be created by calling into_box() for
/// types that implement CppDeletable trait. The object will
/// be deleted when corresponding CppBox is deleted.
pub struct CppBox<T: CppDeletable>(ptr::NonNull<T>);

impl<T: CppDeletable> CppBox<T> {
    /// Returns constant raw pointer to the value in the box.
    pub unsafe fn as_ptr(&self) -> MutPtr<T> {
        MutPtr::from_raw(self.0.as_ptr())
    }

    /// Returns mutable raw pointer to the value in the box.
    pub unsafe fn as_mut_ptr(&mut self) -> MutPtr<T> {
        MutPtr::from_raw(self.0.as_ptr())
    }

    pub unsafe fn as_raw_ptr(&mut self) -> *mut T {
        self.0.as_ptr()
    }

    /// Returns the pointer to the content and destroys the box.
    /// The caller of the function becomes the owner of the object and should
    /// ensure that the object will be deleted at some point.
    pub unsafe fn into_raw_ptr(self) -> *mut T {
        let ptr = self.0.as_ptr();
        mem::forget(self);
        ptr
    }

    /// Returns the pointer to the content and destroys the box.
    /// The caller of the function becomes the owner of the object and should
    /// ensure that the object will be deleted at some point.
    pub unsafe fn into_ptr(self) -> MutPtr<T> {
        let ptr = MutPtr::from_raw(self.0.as_ptr());
        mem::forget(self);
        ptr
    }

    #[allow(clippy::should_implement_trait)]
    pub unsafe fn as_ref(&self) -> Ref<T> {
        Ref::from_raw_non_null(self.0)
    }

    #[allow(clippy::should_implement_trait)]
    pub unsafe fn as_mut_ref(&mut self) -> MutRef<T> {
        MutRef::from_raw_non_null(self.0)
    }
}

impl<T: CppDeletable> CppBox<T> {
    /// Encapsulates the object into a CppBox. Returns `None` if the pointer is null.
    ///
    /// You should use this function only for
    /// pointers that were created on C++ side and passed through
    /// a FFI boundary to Rust. An object created with C++ `new`
    /// must be deleted using C++ `delete`, which is executed by `CppBox`.
    ///
    /// Do not use this function for objects that would be deleted by other means.
    /// If another C++ object is the owner of the passed object,
    /// it will attempt to delete it. If `CppBox` containing the object still exists,
    /// it would result in a double deletion, which must never happen.
    ///
    /// Use `CppBox::into_ptr` to unwrap the pointer before passing it to
    /// a function that takes ownership of the object.
    pub unsafe fn new(ptr: MutPtr<T>) -> Option<Self> {
        Self::from_raw(ptr.as_raw_ptr())
    }

    pub unsafe fn from_raw(ptr: *mut T) -> Option<Self> {
        ptr::NonNull::new(ptr).map(CppBox)
    }
}

impl<T: CppDeletable> Deref for CppBox<T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { self.0.as_ref() }
    }
}

impl<T: CppDeletable> DerefMut for CppBox<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { self.0.as_mut() }
    }
}

impl<T: CppDeletable> Drop for CppBox<T> {
    fn drop(&mut self) {
        unsafe {
            T::delete(self.0.as_mut());
        }
    }
}

impl<T: CppDeletable> fmt::Debug for CppBox<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CppBox({:?})", self.0)
    }
}

#[cfg(test)]
mod tests {
    use crate::{CppBox, CppDeletable, MutPtr};
    use std::cell::RefCell;
    use std::rc::Rc;

    struct Struct1 {
        value: Rc<RefCell<i32>>,
    }

    unsafe extern "C" fn struct1_delete(this_ptr: *mut Struct1) {
        (*this_ptr).value.borrow_mut().clone_from(&42);
    }

    impl CppDeletable for Struct1 {
        unsafe fn delete(&mut self) {
            struct1_delete(self);
        }
    }

    #[test]
    fn test_drop_calls_deleter() {
        let value1 = Rc::new(RefCell::new(10));
        let mut object1 = Struct1 {
            value: value1.clone(),
        };
        assert!(*value1.borrow() == 10);
        unsafe {
            // TODO: remove all "as *mut _" because it's automatic
            CppBox::new(MutPtr::from_raw(&mut object1));
        }
        assert!(*value1.borrow() == 42);
    }
}
