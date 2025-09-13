use core::fmt;
use core::any::{Any, TypeId};
use core::mem;
#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

#[doc(hidden)]
pub trait CloneToAny {
    /// Clone `self` into a new `Box<dyn CloneAny>` object.
    fn clone_to_any(&self) -> Box<dyn CloneAny>;
}

impl<T: Any + Clone> CloneToAny for T {
    #[inline]
    fn clone_to_any(&self) -> Box<dyn CloneAny> {
        Box::new(self.clone())
    }
}

#[doc(hidden)]
pub trait CloneToAnySend {
    /// Clone `self` into a new `Box<dyn CloneAny + Send>` object.
    fn clone_to_any_send(&self) -> Box<dyn CloneAny + Send>;
}

impl<T: Any + Clone + Send> CloneToAnySend for T {
    #[inline]
    fn clone_to_any_send(&self) -> Box<dyn CloneAny + Send> {
        Box::new(self.clone())
    }
}

#[doc(hidden)]
pub trait CloneToAnySendSync {
    /// Clone `self` into a new `Box<dyn CloneAny + Send + Sync>` object.
    fn clone_to_any_send_sync(&self) -> Box<dyn CloneAny + Send + Sync>;
}

impl<T: Any + Clone + Send + Sync> CloneToAnySendSync for T {
    #[inline]
    fn clone_to_any_send_sync(&self) -> Box<dyn CloneAny + Send + Sync> {
        Box::new(self.clone())
    }
}

// Basic implementation for dyn CloneAny
impl Clone for Box<dyn CloneAny> {
    #[inline]
    fn clone(&self) -> Box<dyn CloneAny> {
        (**self).clone_to_any()
    }
}

// Implementation for dyn CloneAny + Send
impl Clone for Box<dyn CloneAny + Send> {
    #[inline]
    fn clone(&self) -> Box<dyn CloneAny + Send> {
        // We need to use transmute here because the trait object doesn't directly
        // implement CloneToAnySend, but the underlying concrete type does
        unsafe {
            let type_id = (**self).type_id();
            let clone_any = (**self).clone_to_any();
            
            // This is safe because:
            // 1. We know the original was Send (it's in a Box<dyn CloneAny + Send>)
            // 2. The clone has the same concrete type as the original
            // 3. Therefore the clone is also Send
            mem::transmute::<Box<dyn CloneAny>, Box<dyn CloneAny + Send>>(clone_any)
        }
    }
}

// Implementation for dyn CloneAny + Send + Sync  
impl Clone for Box<dyn CloneAny + Send + Sync> {
    #[inline]
    fn clone(&self) -> Box<dyn CloneAny + Send + Sync> {
        // Same logic as above, but for Send + Sync
        unsafe {
            let type_id = (**self).type_id();
            let clone_any = (**self).clone_to_any();
            
            // This is safe because:
            // 1. We know the original was Send + Sync
            // 2. The clone has the same concrete type as the original  
            // 3. Therefore the clone is also Send + Sync
            mem::transmute::<Box<dyn CloneAny>, Box<dyn CloneAny + Send + Sync>>(clone_any)
        }
    }
}

impl fmt::Debug for dyn CloneAny {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad("dyn CloneAny")
    }
}

impl fmt::Debug for dyn CloneAny + Send {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad("dyn CloneAny + Send")
    }
}

impl fmt::Debug for dyn CloneAny + Send + Sync {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad("dyn CloneAny + Send + Sync")
    }
}

/// Methods for downcasting from an `Any`-like trait object.
///
/// This should only be implemented on trait objects for subtraits of `Any`, though you can
/// implement it for other types and it'll work fine, so long as your implementation is correct.
pub trait Downcast {
    /// Gets the `TypeId` of `self`.
    fn type_id(&self) -> TypeId;

    // Note the bound through these downcast methods is 'static, rather than the inexpressible
    // concept of Self-but-as-a-trait (where Self is `dyn Trait`). This is sufficient, exceeding
    // TypeId's requirements. Sure, you *can* do CloneAny.downcast_unchecked::<NotClone>() and the
    // type system won't protect you, but that doesn't introduce any unsafety: the method is
    // already unsafe because you can specify the wrong type, and if this were exposing safe
    // downcasting, CloneAny.downcast::<NotClone>() would just return an error, which is just as
    // correct.
    //
    // Now in theory we could also add T: ?Sized, but that doesn't play nicely with the common
    // implementation, so I'm doing without it.

    /// Downcast from `&Any` to `&T`, without checking the type matches.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `T` matches the trait object, on pain of *undefined behaviour*.
    unsafe fn downcast_ref_unchecked<T: 'static>(&self) -> &T;

    /// Downcast from `&mut Any` to `&mut T`, without checking the type matches.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `T` matches the trait object, on pain of *undefined behaviour*.
    unsafe fn downcast_mut_unchecked<T: 'static>(&mut self) -> &mut T;

    /// Downcast from `Box<Any>` to `Box<T>`, without checking the type matches.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `T` matches the trait object, on pain of *undefined behaviour*.
    unsafe fn downcast_unchecked<T: 'static>(self: Box<Self>) -> Box<T>;
}

/// A trait for the conversion of an object into a boxed trait object.
pub trait IntoBox<A: ?Sized + Downcast>: Any {
    /// Convert self into the appropriate boxed form.
    fn into_box(self) -> Box<A>;
}

macro_rules! implement {
    ($any_trait:ident $(+ $auto_traits:ident)*) => {
        impl Downcast for dyn $any_trait $(+ $auto_traits)* {
            #[inline]
            fn type_id(&self) -> TypeId {
                self.type_id()
            }

            #[inline]
            unsafe fn downcast_ref_unchecked<T: 'static>(&self) -> &T {
                &*(self as *const Self as *const T)
            }

            #[inline]
            unsafe fn downcast_mut_unchecked<T: 'static>(&mut self) -> &mut T {
                &mut *(self as *mut Self as *mut T)
            }

            #[inline]
            unsafe fn downcast_unchecked<T: 'static>(self: Box<Self>) -> Box<T> {
                Box::from_raw(Box::into_raw(self) as *mut T)
            }
        }

        impl<T: $any_trait $(+ $auto_traits)*> IntoBox<dyn $any_trait $(+ $auto_traits)*> for T {
            #[inline]
            fn into_box(self) -> Box<dyn $any_trait $(+ $auto_traits)*> {
                Box::new(self)
            }
        }
    }
}

implement!(Any);
implement!(Any + Send);
implement!(Any + Send + Sync);

/// [`Any`], but with cloning.
///
/// Every type with no non-`'static` references that implements `Clone` implements `CloneAny`.
/// See [`core::any`] for more details on `Any` in general.
pub trait CloneAny: Any + CloneToAny {}
impl<T: Any + Clone> CloneAny for T {}

implement!(CloneAny);
implement!(CloneAny + Send);
implement!(CloneAny + Send + Sync);
