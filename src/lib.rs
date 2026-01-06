use std::{marker::PhantomData, sync::Condvar};

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
struct Invariant<'a>(PhantomData<fn(&'a ()) -> &'a ()>);
impl Invariant<'_> {
    fn new() -> Self {
        Self(PhantomData)
    }
}
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
struct Covariant<'a>(PhantomData<fn() -> &'a ()>);
impl Covariant<'_> {
    fn new() -> Self {
        Self(PhantomData)
    }
}
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
struct Contravariant<'a>(PhantomData<fn(&'a ())>);
impl Contravariant<'_> {
    fn new() -> Self {
        Self(PhantomData)
    }
}

impl Default for Contravariant<'_> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct Arena(Vec<f64>);

impl Arena {
    pub fn new<'a>(capacity: usize) -> (Self, Gc<'a>) {
        // SAFETY: created with Arena.
        (Self(Vec::with_capacity(capacity)), unsafe { Gc::new() })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Handle<'a>(u32, Contravariant<'a>);

impl<'a> Handle<'a> {
    pub fn new<'gc>(arena: &mut Arena, value: f64, _gc: NoGc<'gc>) -> Handle<'a>
    where
        'gc: 'a,
    {
        let index = u32::try_from(arena.0.len()).unwrap();
        arena.0.push(value);
        Handle(index, Contravariant::new())
    }

    pub fn get(self, arena: &Arena) -> &f64 {
        arena.0.get(self.0 as usize).as_ref().unwrap()
    }

    pub fn local<'l>(self) -> Handle<'l> {
        Handle(self.0, Contravariant::new())
    }

    pub fn unbind(self) -> Handle<'static> {
        Handle(self.0, Contravariant::new())
    }
}

#[derive(Debug)]
pub struct Gc<'a>(Covariant<'a>);

impl<'gc> Gc<'gc> {
    /// # Safety
    ///
    /// Only one Gc must exist per
    unsafe fn new() -> Self {
        Gc(Covariant::new())
    }

    pub fn join(&'gc self, handle: Handle<'gc>) {}

    pub fn reborrow(&mut self) -> Gc<'_> {
        Gc(Covariant::new())
    }

    pub fn nogc(&self) -> NoGc<'_> {
        NoGc(Covariant::new())
    }
}

pub struct NoGcGuard<'a>(Contravariant<'a>);

impl<'gc> NoGcGuard<'gc> {
    pub fn nogc<'a>(&'a self) -> NoGc<'a> {
        NoGc(Covariant::new())
    }

    pub fn bind(self, handle: Handle<'gc>) -> NoGcGuard<'gc> {
        NoGcGuard(Contravariant::new())
    }
}

impl Drop for NoGcGuard<'_> {
    fn drop(&mut self) {
        todo!()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct NoGc<'a>(Covariant<'a>);

impl<'gc> NoGc<'gc> {
    pub fn bind(self, handle: Handle<'gc>) {}
}

impl<'a> From<Gc<'a>> for NoGc<'a> {
    fn from(_: Gc<'a>) -> Self {
        NoGc(Covariant::new())
    }
}

#[cfg(test)]
mod tests {
    use crate::{Arena, Contravariant, Covariant, Gc, Handle, NoGc};

    #[test]
    fn test_invalidation() {
        fn perform_gc<'a>(arena: &mut Arena, handle: Handle<'a>, gc: Gc<'a>) -> Gc<'a> {
            arena.0.clear();
            gc
        }

        fn inner<'gc>(
            arena: &mut Arena,
            a: Handle,
            b: Handle,
            gc: Gc<'gc>,
        ) -> Result<Handle<'gc>, Handle<'gc>> {
        }

        fn outer<'gc>(arena: &mut Arena, handle: Handle, mut gc: Gc<'gc>) -> Handle<'gc> {
            let handle = handle.local();
            gc.join(handle);
            perform_gc(arena, handle, gc.reborrow());
            // The following line fails: perform_gc invalidates handles.
            // handle.get(arena);
            let handle = Handle::new(arena, 3.0, gc.nogc());
            // gc.bind(handle);
            perform_gc(arena, handle, gc.reborrow());
            handle
        }
        let (mut arena, gc) = Arena::new(256);
        let handle = Handle::new(&mut arena, 1.0, gc.nogc());
        outer(&mut arena, handle, gc);
    }

    #[test]
    fn covariant_invalidation() {
        fn create_covariant(_: &mut ()) -> Covariant<'_> {
            Covariant::new()
        }

        let data = &mut ();
        let mut foo = create_covariant(data);
        let a = &foo;
        // let b = &mut foo;
        println!("{a:?}");
    }

    #[test]
    fn contraariant_invalidation() {
        fn create_contravariant(_: &mut ()) -> Contravariant<'_> {
            Contravariant::new()
        }
        let data = &mut ();
        let mut foo = create_contravariant(data);
        let a = &foo;
        // let b = &mut foo;
        println!("{a:?}");
        // let (mut arena, gc) = Arena::new(256);
        // let handle = Handle::new(&mut arena, 1.0, gc.nogc());
        // outer(&mut arena, handle, gc);
    }
}
