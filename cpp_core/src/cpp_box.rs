use crate::ops::{Begin, BeginMut, End, EndMut, Increment, Indirection};
use crate::vector_ops::{Data, DataMut, Size};
use crate::{cpp_iter, CppIterator, DynamicCast, Ptr, Ref, StaticDowncast, StaticUpcast};
use std::ops::Deref;
use std::{fmt, mem, ptr, slice};

/// Objects that can be deleted using C++'s `delete` operator.
///
/// This trait is automatically implemented for class types by `ritual`.
pub trait CppDeletable: Sized {
    /// Calls C++'s `delete x` on `self`.
    ///
    /// # Safety
    ///
    /// The caller must make sure `self` contains a valid pointer. This function
    /// may invoke arbitrary foreign code, so no safety guarantees can be made.
    /// Note that deleting an object multiple times is undefined behavior.
    unsafe fn delete(&self);
}

/// An owning pointer to a C++ object.
///
/// `CppBox` is automatically used in places where C++ class objects are passed by value
/// and in return values of constructors because the ownership is apparent in these cases.
/// However, sometimes an object is returned as a pointer but you must accept the ownership
/// of the object. It's not possible to automatically determine ownership semantics
/// of C++ code in this case, so you should manually convert `Ptr` to `CppBox`
/// using `to_box` method.
///
/// When `CppBox` is dropped, it will automatically delete the object using C++'s `delete`
/// operator.
///
/// Objects stored in `CppBox` are usually placed on the heap by the C++ code.
///
/// If a C++ API accepts an object by pointer and takes ownership of it, it's not possible to
/// automatically detect this, so you must manually convert `CppBox` to a non-owning `Ptr`
/// using `into_ptr` before passing it to such a function.
///
/// `&CppBox<T>` and `&mut CppBox<T>` implement operator traits and delegate them
/// to the corresponding C++ operators.
/// This means that you can use `&box1 + value` to access the object's `operator+`.
///
/// `CppBox` implements `Deref`, allowing to call the object's methods
/// directly. In addition, methods of the object's first base class are also directly available
/// thanks to nested `Deref` implementations.
///
/// If the object provides an iterator interface through `begin()` and `end()` functions,
/// `&CppBox<T>` and `&mut CppBox<T>` will implement `IntoIterator`,
/// so you can iterate on them directly.
///
/// ### Safety
///
/// It's not possible to automatically track the ownership of objects possibly managed by C++
/// libraries. The user must ensure that the object is alive while `CppBox` exists and that
/// no pointers to the object are used after the object is deleted
/// by `CppBox`'s `Drop` implementation. Note that with `CppBox`,
/// it's possible to call unsafe C++ code without using any more unsafe code, for example, by
/// using operator traits or simply dropping the box, so care should be taken when exposing
/// `CppBox` in a safe interface.
pub struct CppBox<T: CppDeletable>(ptr::NonNull<T>);

impl<T: CppDeletable> CppBox<T> {
    /// Encapsulates the object into a `CppBox`. Returns `None` if the pointer is null.
    ///
    /// The same operation can be done by calling `to_box` function on `Ptr`.
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
    ///
    /// ### Safety
    ///
    /// The pointer must point to an object that can be
    /// safely deleted using C++'s `delete` operator.
    /// The object must not be deleted by other means while `CppBox` exists.
    /// Any other pointers to the object must not be used after `CppBox` is dropped.
    pub unsafe fn new(ptr: Ptr<T>) -> Option<Self> {
        Self::from_raw(ptr.as_raw_ptr())
    }

    /// Encapsulates the object into a `CppBox`. Returns `None` if the pointer is null.
    ///
    /// See `CppBox::new` for more information.
    ///
    /// ### Safety
    ///
    /// The pointer must point to an object that can be
    /// safely deleted using C++'s `delete` operator.
    /// The object must not be deleted by other means while `CppBox` exists.
    /// Any other pointers to the object must not be used after `CppBox` is dropped.
    pub unsafe fn from_raw(ptr: *const T) -> Option<Self> {
        ptr::NonNull::new(ptr as *mut T).map(CppBox)
    }

    /// Returns a constant pointer to the value in the box.
    ///
    /// ### Safety
    ///
    /// This operation is safe as long as `self` is valid.
    pub unsafe fn as_ptr(&self) -> Ptr<T> {
        Ptr::from_raw(self.0.as_ptr())
    }

    /// Returns a constant raw pointer to the value in the box.
    pub fn as_mut_raw_ptr(&self) -> *mut T {
        self.0.as_ptr()
    }

    /// Returns a mutable raw pointer to the value in the box.
    pub fn as_raw_ptr(&self) -> *const T {
        self.0.as_ptr() as *const T
    }

    /// Destroys the box without deleting the object and returns a raw pointer to the content.
    /// The caller of the function becomes the owner of the object and should
    /// ensure that the object will be deleted at some point.
    pub fn into_raw_ptr(self) -> *mut T {
        let ptr = self.0.as_ptr();
        mem::forget(self);
        ptr
    }

    /// Destroys the box without deleting the object and returns a pointer to the content.
    /// The caller of the function becomes the owner of the object and should
    /// ensure that the object will be deleted at some point.
    ///
    /// ### Safety
    ///
    /// This operation is safe as long as `self` is valid.
    pub unsafe fn into_ptr(self) -> Ptr<T> {
        let ptr = Ptr::from_raw(self.0.as_ptr());
        mem::forget(self);
        ptr
    }

    /// Returns a constant reference to the value in the box.
    ///
    /// ### Safety
    ///
    /// This operation is safe as long as `self` is valid.
    #[allow(clippy::should_implement_trait)]
    pub unsafe fn as_ref(&self) -> Ref<T> {
        Ref::from_raw_non_null(self.0)
    }

    /// Returns a reference to the value.
    ///
    /// ### Safety
    ///
    /// `self` must be valid.
    /// The content must not be read or modified through other ways while the returned reference
    /// exists.See type level documentation.
    pub unsafe fn as_raw_ref<'a>(&self) -> &'a T {
        &*self.0.as_ptr()
    }

    /// Returns a mutable reference to the value.
    ///
    /// ### Safety
    ///
    /// `self` must be valid.
    /// The content must not be read or modified through other ways while the returned reference
    /// exists.See type level documentation.
    pub unsafe fn as_mut_raw_ref<'a>(&self) -> &'a mut T {
        &mut *self.0.as_ptr()
    }

    /// Returns a non-owning reference to the content converted to the base class type `U`.
    /// `CppBox` retains the ownership of the object.
    ///
    /// ### Safety
    ///
    /// This operation is safe as long as `self` is valid.
    pub unsafe fn static_upcast<U>(&self) -> Ref<U>
    where
        T: StaticUpcast<U>,
    {
        StaticUpcast::static_upcast(self.as_ptr())
            .as_ref()
            .expect("StaticUpcast returned null on CppBox input")
    }

    /// Returns a non-owning reference to the content converted to the derived class type `U`.
    /// `CppBox` retains the ownership of the object.
    ///
    /// It's recommended to use `dynamic_cast` instead because it performs a checked conversion.
    ///
    /// ### Safety
    ///
    /// This operation is safe as long as `self` is valid and it's type is `U` or inherits from `U`.
    pub unsafe fn static_downcast<U>(&self) -> Ref<U>
    where
        T: StaticDowncast<U>,
    {
        StaticDowncast::static_downcast(self.as_ptr())
            .as_ref()
            .expect("StaticDowncast returned null on CppBox input")
    }

    /// Returns a non-owning reference to the content converted to the derived class type `U`.
    /// `CppBox` retains the ownership of the object. Returns `None` if the object's type is not `U`
    /// and doesn't inherit `U`.
    ///
    /// ### Safety
    ///
    /// This operation is safe as long as `self` is valid.
    pub unsafe fn dynamic_cast<U>(&self) -> Option<Ref<U>>
    where
        T: DynamicCast<U>,
    {
        DynamicCast::dynamic_cast(self.as_ptr()).as_ref()
    }
}

impl<V, T> CppBox<V>
where
    V: Data<Output = *const T> + Size + CppDeletable,
{
    /// Returns the content of the object as a slice, based on `data()` and `size()` methods.
    ///
    /// # Safety
    ///
    /// The caller must make sure `self` contains a valid pointer. The content must
    /// not be read or modified through other ways while the returned slice exists.
    /// This function
    /// may invoke arbitrary foreign code, so no safety guarantees can be made.
    pub unsafe fn as_slice<'a>(&self) -> &'a [T] {
        let ptr = self.data();
        let size = self.size();
        slice::from_raw_parts(ptr, size)
    }
}

impl<V, T> CppBox<V>
where
    V: DataMut<Output = *mut T> + Size + CppDeletable,
{
    /// Returns the content of the vector as a mutable slice,
    /// based on `data()` and `size()` methods.
    ///
    /// # Safety
    ///
    /// The caller must make sure `self` contains a valid pointer. The content must
    /// not be read or modified through other ways while the returned slice exists.
    /// This function
    /// may invoke arbitrary foreign code, so no safety guarantees can be made.
    pub unsafe fn as_mut_slice<'a>(&self) -> &'a mut [T] {
        let ptr = self.data_mut();
        let size = self.size();
        slice::from_raw_parts_mut(ptr, size)
    }
}

impl<T, T1, T2> CppBox<T>
where
    T: Begin<Output = CppBox<T1>> + End<Output = CppBox<T2>> + CppDeletable,
    T1: CppDeletable + PartialEq<Ref<T2>> + Increment + Indirection,
    T2: CppDeletable,
{
    /// Returns an iterator over the content of the object,
    /// based on `begin()` and `end()` methods.
    ///
    /// # Safety
    ///
    /// The caller must make sure `self` contains a valid pointer. The content must
    /// not be read or modified through other ways while the returned slice exists.
    /// This function
    /// may invoke arbitrary foreign code, so no safety guarantees can be made.
    pub unsafe fn iter(&self) -> CppIterator<T1, T2> {
        cpp_iter(self.begin(), self.end())
    }
}

impl<T, T1, T2> CppBox<T>
where
    T: BeginMut<Output = CppBox<T1>> + EndMut<Output = CppBox<T2>> + CppDeletable,
    T1: CppDeletable + PartialEq<Ref<T2>> + Increment + Indirection,
    T2: CppDeletable,
{
    /// Returns a mutable iterator over the content of the object,
    /// based on `begin()` and `end()` methods.
    ///
    /// # Safety
    ///
    /// The caller must make sure `self` contains a valid pointer. The content must
    /// not be read or modified through other ways while the returned slice exists.
    /// This function
    /// may invoke arbitrary foreign code, so no safety guarantees can be made.
    pub unsafe fn iter_mut(&self) -> CppIterator<T1, T2> {
        cpp_iter(self.begin_mut(), self.end_mut())
    }
}

/// Allows to call member functions of `T` and its base classes directly on the pointer.
impl<T: CppDeletable> Deref for CppBox<T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { self.0.as_ref() }
    }
}

/// Deletes the stored object using C++'s `delete` operator.
impl<T: CppDeletable> Drop for CppBox<T> {
    fn drop(&mut self) {
        unsafe {
            T::delete(self.0.as_ref());
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
    use crate::{CppBox, CppDeletable, Ptr};
    use std::cell::RefCell;
    use std::rc::Rc;

    struct Struct1 {
        value: Rc<RefCell<i32>>,
    }

    unsafe extern "C" fn struct1_delete(this_ptr: *const Struct1) {
        (*this_ptr).value.borrow_mut().clone_from(&42);
    }

    impl CppDeletable for Struct1 {
        unsafe fn delete(&self) {
            struct1_delete(self);
        }
    }

    #[test]
    fn test_drop_calls_deleter() {
        let value1 = Rc::new(RefCell::new(10));
        let object1 = Struct1 {
            value: value1.clone(),
        };
        assert!(*value1.borrow() == 10);
        unsafe {
            CppBox::new(Ptr::from_raw(&object1));
        }
        assert!(*value1.borrow() == 42);
    }
}
