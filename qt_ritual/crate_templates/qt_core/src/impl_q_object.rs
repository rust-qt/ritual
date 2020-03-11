use crate::{QObject, QPtr, QString};
use cpp_core::{DynamicCast, StaticUpcast};
use std::error::Error;
use std::fmt;

pub struct FindChildError(FindChildErrorInner);

enum FindChildErrorInner {
    NotFound {
        name: String,
    },
    TypeMismatch {
        name: String,
        target_type: &'static str,
    },
}

impl fmt::Display for FindChildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            FindChildErrorInner::NotFound { name } => write!(f, "child \"{}\" not found", name),
            FindChildErrorInner::TypeMismatch { name, target_type } => write!(
                f,
                "child \"{}\" cannot be converted to type {}",
                name, target_type
            ),
        }
    }
}

impl fmt::Debug for FindChildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl Error for FindChildError {}

impl QObject {
    pub unsafe fn find_child<T>(&self, name: &str) -> Result<QPtr<T>, FindChildError>
    where
        QObject: DynamicCast<T>,
        T: StaticUpcast<QObject>,
    {
        let ptr = self.find_child_q_object_1a(&QString::from_std_str(name));
        if ptr.is_null() {
            return Err(FindChildError(FindChildErrorInner::NotFound {
                name: name.into(),
            }));
        }

        let ptr = ptr.dynamic_cast();
        if ptr.is_null() {
            return Err(FindChildError(FindChildErrorInner::TypeMismatch {
                name: name.into(),
                target_type: std::any::type_name::<T>(),
            }));
        }
        Ok(ptr)
    }
}
